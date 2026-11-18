//! Deterministic index writer for format v1.

use crate::format::*;
use crate::model::{IndexData, Place, WriteSummary};
use crate::normalize::{normalize, tokens};
use anyhow::{Context, Result, bail};
use fst::{MapBuilder, Streamer};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const CELL_SIZE_E7: i32 = 100_000;
struct StringTable {
    bytes: Vec<u8>,
    offsets: BTreeMap<String, (u32, u16)>,
}

impl StringTable {
    fn new(strings: BTreeSet<String>) -> Result<Self> {
        let mut bytes = Vec::new();
        let mut offsets = BTreeMap::new();
        for value in strings {
            if value.as_bytes().contains(&0) {
                bail!("string contains NUL byte");
            }
            let len = value.len();
            if len > u16::MAX as usize {
                bail!("string longer than {} bytes", u16::MAX);
            }
            if bytes.len() > u32::MAX as usize {
                bail!("string section exceeds 4 GiB");
            }
            let offset = bytes.len() as u32;
            bytes.extend_from_slice(value.as_bytes());
            offsets.insert(value, (offset, len as u16));
        }
        Ok(Self { bytes, offsets })
    }

    fn get(&self, value: &str) -> Result<(u32, u16)> {
        self.offsets
            .get(value)
            .copied()
            .with_context(|| format!("missing string {value:?}"))
    }
}

struct VariantWriter {
    bytes: Vec<u8>,
    count: u32,
}

impl VariantWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            count: 0,
        }
    }

    fn append(
        &mut self,
        strings: &StringTable,
        variants: &[(String, String)],
    ) -> Result<(u32, u8)> {
        let off = self.bytes.len() as u32;
        let mut sorted: Vec<_> = variants
            .iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(lang, value)| (normalize_lang(lang), value.clone()))
            .collect();
        sorted.sort();
        sorted.dedup();
        if sorted.len() > u8::MAX as usize {
            bail!("more than 255 name variants");
        }
        for (lang, value) in &sorted {
            let (str_off, str_len) = strings.get(value)?;
            let mut record = VariantRecord {
                str_off,
                str_len,
                ..Default::default()
            };
            for (slot, byte) in record.lang.iter_mut().zip(lang.as_bytes()) {
                *slot = *byte;
            }
            self.bytes.extend_from_slice(bytemuck::bytes_of(&record));
        }
        self.count += sorted.len() as u32;
        Ok((off, sorted.len() as u8))
    }
}

fn normalize_lang(lang: &str) -> String {
    let mut value: String = lang
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .take(3)
        .collect::<String>()
        .to_ascii_lowercase();
    if value.is_empty() {
        value.push_str("und");
    }
    value
}

fn build_fst(entries: &BTreeMap<String, u64>) -> Result<Vec<u8>> {
    let mut builder = MapBuilder::memory();
    for (key, value) in entries {
        if key.is_empty() {
            bail!("empty FST key");
        }
        builder.insert(key, *value)?;
    }
    Ok(builder.into_inner()?)
}

type GridBuckets = (Vec<u8>, Vec<(u32, Vec<u32>)>);

fn build_grid(places: &[Place]) -> Result<GridBuckets> {
    if places.is_empty() {
        bail!("cannot build grid for empty index");
    }
    let min_lat = places.iter().map(|p| p.bbox_min_lat_e7).min().unwrap();
    let min_lon = places.iter().map(|p| p.bbox_min_lon_e7).min().unwrap();
    let max_lat = places.iter().map(|p| p.bbox_max_lat_e7).max().unwrap();
    let max_lon = places.iter().map(|p| p.bbox_max_lon_e7).max().unwrap();
    let cell_min_lat = div_floor(min_lat, CELL_SIZE_E7) * CELL_SIZE_E7;
    let cell_min_lon = div_floor(min_lon, CELL_SIZE_E7) * CELL_SIZE_E7;
    let cell_max_lat = div_floor(max_lat, CELL_SIZE_E7) * CELL_SIZE_E7;
    let cell_max_lon = div_floor(max_lon, CELL_SIZE_E7) * CELL_SIZE_E7;
    let rows = (cell_max_lat - cell_min_lat) / CELL_SIZE_E7 + 1;
    let cols = (cell_max_lon - cell_min_lon) / CELL_SIZE_E7 + 1;
    if rows <= 0 || cols <= 0 {
        bail!("invalid grid dimensions");
    }
    if rows as u64 * cols as u64 > u32::MAX as u64 {
        bail!("grid dimensions overflow u32");
    }

    let mut cells: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (row, place) in places.iter().enumerate() {
        let min_row = div_floor(place.bbox_min_lat_e7 - cell_min_lat, CELL_SIZE_E7);
        let max_row = div_floor(place.bbox_max_lat_e7 - cell_min_lat, CELL_SIZE_E7);
        let min_col = div_floor(place.bbox_min_lon_e7 - cell_min_lon, CELL_SIZE_E7);
        let max_col = div_floor(place.bbox_max_lon_e7 - cell_min_lon, CELL_SIZE_E7);
        for row_idx in min_row.max(0)..=max_row.min(rows - 1) {
            for col_idx in min_col.max(0)..=max_col.min(cols - 1) {
                let key = (row_idx as u32) * (cols as u32) + col_idx as u32;
                cells.entry(key).or_default().push(row as u32);
            }
        }
    }
    let mut entries = Vec::with_capacity(cells.len());
    for (key, mut rows) in cells {
        rows.sort_unstable();
        rows.dedup();
        entries.push((key, rows));
    }

    let mut bytes = Vec::with_capacity(GRID_HEADER_BYTES);
    let header = [
        CELL_SIZE_E7,
        cell_min_lat,
        cell_min_lon,
        cols,
        rows,
        entries.len() as i32,
    ];
    for value in header {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[0u8; 16]);
    Ok((bytes, entries))
}

fn div_floor(value: i32, divisor: i32) -> i32 {
    value.div_euclid(divisor)
}

fn push_record(out: &mut Vec<u8>, record: &impl bytemuck::NoUninit) {
    out.extend_from_slice(bytemuck::bytes_of(record));
}

fn write_with_alignment(
    file: &mut BufWriter<File>,
    data: &[u8],
    sections: &mut [Section; SECTION_COUNT],
    index: usize,
    position: &mut usize,
) -> Result<()> {
    let aligned = align8(*position);
    if aligned > *position {
        file.write_all(&vec![0u8; aligned - *position])?;
    }
    sections[index] = Section {
        offset: aligned as u64,
        len: data.len() as u64,
    };
    file.write_all(data)?;
    *position = aligned + data.len();
    Ok(())
}

pub fn write_index(data: &IndexData, out: &Path) -> Result<WriteSummary> {
    if data.places.is_empty() {
        bail!("index must contain at least one place");
    }
    let mut places = data.places.clone();
    places.sort_by_key(|place| place.place_id);
    for pair in places.windows(2) {
        if pair[0].place_id == pair[1].place_id {
            bail!("duplicate place_id {}", pair[0].place_id);
        }
    }
    let mut admins = data.admins.clone();
    admins.sort_by_key(|admin| admin.admin_id);
    for (index, admin) in admins.iter().enumerate() {
        if admin.admin_id != index as u32 + 1 {
            bail!("admin_id must be contiguous starting at 1");
        }
    }

    let class_values: BTreeSet<String> = places.iter().map(|p| p.class.clone()).collect();
    let type_values: BTreeSet<String> = places.iter().map(|p| p.r#type.clone()).collect();
    if class_values.len() > u16::MAX as usize || type_values.len() > u16::MAX as usize {
        bail!("too many class or type values");
    }
    let class_ids: BTreeMap<_, _> = class_values
        .iter()
        .enumerate()
        .map(|(id, value)| (value.clone(), id as u16))
        .collect();
    let type_ids: BTreeMap<_, _> = type_values
        .iter()
        .enumerate()
        .map(|(id, value)| (value.clone(), id as u16))
        .collect();

    let mut all_strings = BTreeSet::new();
    for place in &places {
        all_strings.insert(place.name.clone());
        for (_, value) in &place.variants {
            all_strings.insert(value.clone());
        }
    }
    for admin in &admins {
        all_strings.insert(admin.name.clone());
        for (_, value) in &admin.variants {
            all_strings.insert(value.clone());
        }
    }
    all_strings.extend(class_values.iter().cloned());
    all_strings.extend(type_values.iter().cloned());
    let strings = StringTable::new(all_strings)?;

    let mut variants = VariantWriter::new();
    let mut place_variants = Vec::with_capacity(places.len());
    for place in &places {
        let mut list = place.variants.clone();
        list.push(("und".to_string(), place.name.clone()));
        place_variants.push(variants.append(&strings, &list)?);
    }
    let mut admin_variants = Vec::with_capacity(admins.len());
    for admin in &admins {
        let mut list = admin.variants.clone();
        list.push(("und".to_string(), admin.name.clone()));
        admin_variants.push(variants.append(&strings, &list)?);
    }

    // Token postings and exact-name FST. Both are built from normalized names
    // and variants so aliases participate in every query path.
    let mut token_rows: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let mut name_best: BTreeMap<String, (u64, f32)> = BTreeMap::new();
    for (row, place) in places.iter().enumerate() {
        let mut names = vec![place.name.clone()];
        names.extend(place.variants.iter().map(|(_, value)| value.clone()));
        let mut seen = BTreeSet::new();
        for name in names {
            let normalized = normalize(&name);
            if normalized.is_empty() || !seen.insert(normalized.clone()) {
                continue;
            }
            for token in tokens(&normalized) {
                token_rows.entry(token).or_default().push(row as u32);
            }
            let entry = name_best
                .entry(normalized)
                .or_insert((row as u64, place.importance));
            if place.importance > entry.1
                || (place.importance == entry.1
                    && place.place_id < places[entry.0 as usize].place_id)
            {
                *entry = (row as u64, place.importance);
            }
        }
    }
    for rows in token_rows.values_mut() {
        rows.sort_unstable();
        rows.dedup();
    }

    let (mut grid_bytes, grid_cells) = build_grid(&places)?;
    let mut postings: Vec<u32> = Vec::new();
    let mut token_fst: BTreeMap<String, u64> = BTreeMap::new();
    for (token, rows) in &token_rows {
        let off = postings.len();
        if off > u32::MAX as usize || rows.len() > u32::MAX as usize {
            bail!("postings exceed 32-bit offset");
        }
        postings.extend_from_slice(rows);
        token_fst.insert(token.clone(), ((off as u64) << 32) | rows.len() as u64);
    }
    let mut grid_entries = Vec::with_capacity(grid_cells.len());
    for (key, rows) in grid_cells {
        let off = postings.len();
        postings.extend_from_slice(&rows);
        grid_entries.push(GridEntry {
            key,
            posting_off: off as u32,
            posting_count: rows.len() as u32,
        });
    }
    for entry in grid_entries {
        grid_bytes.extend_from_slice(bytemuck::bytes_of(&entry));
    }

    let token_fst_bytes = build_fst(&token_fst)?;
    let name_fst_values: BTreeMap<String, u64> = name_best
        .iter()
        .map(|(key, (row, _))| (key.clone(), *row))
        .collect();
    let name_fst_bytes = build_fst(&name_fst_values)?;

    let mut places_bytes = Vec::with_capacity(places.len() * PLACE_BYTES);
    for (row, place) in places.iter().enumerate() {
        let (name_off, name_len) = strings.get(&place.name)?;
        let (names_off, names_count) = place_variants[row];
        let record = PlaceRecord {
            place_id: place.place_id,
            name_off,
            name_len,
            kind: place.kind,
            flags: 0,
            lat_e7: place.lat_e7,
            lon_e7: place.lon_e7,
            importance: place.importance,
            admin_id: place.admin_id,
            bbox_min_lat_e7: place.bbox_min_lat_e7,
            bbox_min_lon_e7: place.bbox_min_lon_e7,
            bbox_max_lat_e7: place.bbox_max_lat_e7,
            bbox_max_lon_e7: place.bbox_max_lon_e7,
            class_id: class_ids[&place.class],
            type_id: type_ids[&place.r#type],
            names_off,
            names_count,
            reserved: [0; 7],
        };
        push_record(&mut places_bytes, &record);
    }

    let row_by_id: BTreeMap<u64, u32> = places
        .iter()
        .enumerate()
        .map(|(row, place)| (place.place_id, row as u32))
        .collect();
    let mut way_lines: Vec<_> = data
        .way_lines
        .iter()
        .filter(|line| row_by_id.contains_key(&line.place_id))
        .collect();
    way_lines.sort_by_key(|line| row_by_id[&line.place_id]);
    let mut way_lines_bytes = Vec::new();
    if !way_lines.is_empty() {
        way_lines_bytes.extend_from_slice(&(way_lines.len() as u32).to_le_bytes());
        let directory_bytes = way_lines.len() * 12;
        let mut payload = Vec::new();
        for line in &way_lines {
            let row = row_by_id[&line.place_id];
            way_lines_bytes.extend_from_slice(&row.to_le_bytes());
            way_lines_bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            way_lines_bytes.extend_from_slice(&((line.points.len() * 2) as u32).to_le_bytes());
            for (lat, lon) in &line.points {
                payload.extend_from_slice(&lat.to_le_bytes());
                payload.extend_from_slice(&lon.to_le_bytes());
            }
        }
        debug_assert_eq!(way_lines_bytes.len(), 4 + directory_bytes);
        way_lines_bytes.extend_from_slice(&payload);
    }

    let mut classes_bytes = Vec::with_capacity(DICT_HEADER_BYTES + class_values.len() * 8);
    classes_bytes.extend_from_slice(&(class_values.len() as u32).to_le_bytes());
    classes_bytes.extend_from_slice(&0u32.to_le_bytes());
    for value in &class_values {
        let (str_off, str_len) = strings.get(value)?;
        let record = DictRecord {
            str_off,
            str_len,
            pad: 0,
        };
        classes_bytes.extend_from_slice(bytemuck::bytes_of(&record));
    }
    let mut types_bytes = Vec::with_capacity(DICT_HEADER_BYTES + type_values.len() * 8);
    types_bytes.extend_from_slice(&(type_values.len() as u32).to_le_bytes());
    types_bytes.extend_from_slice(&0u32.to_le_bytes());
    for value in &type_values {
        let (str_off, str_len) = strings.get(value)?;
        let record = DictRecord {
            str_off,
            str_len,
            pad: 0,
        };
        types_bytes.extend_from_slice(bytemuck::bytes_of(&record));
    }

    let mut admin_bytes = Vec::with_capacity(admins.len() * ADMIN_BYTES);
    for (index, admin) in admins.iter().enumerate() {
        let (name_off, name_len) = strings.get(&admin.name)?;
        let (names_off, names_count) = admin_variants[index];
        let mut record = [0u8; ADMIN_BYTES];
        put_u32(&mut record, 0, admin.admin_id);
        put_u32(&mut record, 4, admin.parent_id);
        record[8] = admin.level;
        put_u32(&mut record, 12, name_off);
        put_u16(&mut record, 16, name_len);
        put_u32(&mut record, 18, names_off);
        record[22] = names_count;
        put_i32(&mut record, 24, admin.bbox_min_lat_e7);
        put_i32(&mut record, 28, admin.bbox_min_lon_e7);
        put_i32(&mut record, 32, admin.bbox_max_lat_e7);
        put_i32(&mut record, 36, admin.bbox_max_lon_e7);
        admin_bytes.extend_from_slice(&record);
    }

    let mut postings_bytes = Vec::with_capacity(postings.len() * 4);
    for row in &postings {
        postings_bytes.extend_from_slice(&row.to_le_bytes());
    }

    let min_lat = places.iter().map(|p| p.bbox_min_lat_e7).min().unwrap();
    let min_lon = places.iter().map(|p| p.bbox_min_lon_e7).min().unwrap();
    let max_lat = places.iter().map(|p| p.bbox_max_lat_e7).max().unwrap();
    let max_lon = places.iter().map(|p| p.bbox_max_lon_e7).max().unwrap();

    let mut header = Header {
        flags: if way_lines_bytes.is_empty() {
            0
        } else {
            FLAG_WAY_LINES
        },
        feature_count: places.len() as u32,
        admin_count: admins.len() as u32,
        class_count: class_values.len() as u32,
        type_count: type_values.len() as u32,
        min_lat_e7: min_lat,
        min_lon_e7: min_lon,
        max_lat_e7: max_lat,
        max_lon_e7: max_lon,
        string_bytes: strings.bytes.len() as u64,
        posting_count: postings.len() as u64,
        sections: [Section::default(); SECTION_COUNT],
    };

    let tmp = tmp_path(out);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&tmp)
        .with_context(|| format!("create {}", tmp.display()))?;
    let mut writer = BufWriter::new(file);
    let header_bytes = serialize_header(&header)?;
    writer.write_all(&header_bytes)?;
    let mut position = HEADER_BYTES;
    let sections_data: [&[u8]; SECTION_COUNT] = [
        &places_bytes,
        &classes_bytes,
        &types_bytes,
        &strings.bytes,
        &variants.bytes,
        &token_fst_bytes,
        &postings_bytes,
        &name_fst_bytes,
        &grid_bytes,
        &admin_bytes,
        &[],
        &way_lines_bytes,
    ];
    for (index, bytes) in sections_data.iter().enumerate() {
        write_with_alignment(
            &mut writer,
            bytes,
            &mut header.sections,
            index,
            &mut position,
        )?;
    }
    let padding = align8(position) - position;
    if padding > 0 {
        writer.write_all(&vec![0u8; padding])?;
        position += padding;
    }
    writer.flush()?;
    let mut file = writer.into_inner()?;
    file.sync_all()?;

    // The header contains the final section table, so rewrite the first 256
    // bytes after all section offsets are known.
    let header_bytes = serialize_header(&header)?;
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&header_bytes)?;
    file.flush()?;
    file.sync_all()?;

    let mut body = Vec::with_capacity(position);
    {
        use std::io::Read;
        file.seek(SeekFrom::Start(0))?;
        file.read_to_end(&mut body)?;
    }
    if body.len() != position {
        bail!("internal size mismatch while hashing index");
    }
    let digest = Sha256::digest(&body);
    file.seek(SeekFrom::Start(position as u64))?;
    file.write_all(&digest)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, out).with_context(|| format!("rename {}", tmp.display()))?;

    let bytes = (position + TRAILER_BYTES) as u64;
    let sha256 = hex(&digest);
    Ok(WriteSummary {
        index_path: out.to_path_buf(),
        bytes,
        sha256,
        feature_count: header.feature_count,
        admin_count: header.admin_count,
        class_count: header.class_count,
        type_count: header.type_count,
        posting_count: header.posting_count,
        string_bytes: header.string_bytes,
        bbox: [min_lat, min_lon, max_lat, max_lon],
    })
}

fn serialize_header(header: &Header) -> Result<Vec<u8>> {
    let mut bytes = vec![0u8; HEADER_BYTES];
    bytes[0..8].copy_from_slice(&MAGIC);
    put_u16(&mut bytes, 8, FORMAT_VERSION);
    put_u16(&mut bytes, 10, header.flags);
    put_u32(&mut bytes, 12, HEADER_BYTES as u32);
    put_u32(&mut bytes, 16, header.feature_count);
    put_u32(&mut bytes, 20, header.admin_count);
    put_u32(&mut bytes, 24, header.class_count);
    put_u32(&mut bytes, 28, header.type_count);
    put_i32(&mut bytes, 32, header.min_lat_e7);
    put_i32(&mut bytes, 36, header.min_lon_e7);
    put_i32(&mut bytes, 40, header.max_lat_e7);
    put_i32(&mut bytes, 44, header.max_lon_e7);
    put_u64(&mut bytes, 48, header.string_bytes);
    put_u64(&mut bytes, 56, header.posting_count);
    for (index, section) in header.sections.iter().enumerate() {
        let base = SECTION_TABLE_OFFSET + index * SECTION_ENTRY_BYTES;
        put_u64(&mut bytes, base, section.offset);
        put_u64(&mut bytes, base + 8, section.len);
    }
    Ok(bytes)
}

fn tmp_path(out: &Path) -> PathBuf {
    PathBuf::from(format!("{}.tmp", out.display()))
}

pub fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push_str(&format!("{byte:02x}"));
    }
    value
}

/// Count entries in an FST without allocating keys.
pub fn fst_len(bytes: &[u8]) -> Result<u64> {
    let map = fst::Map::new(bytes)?;
    Ok(map.len() as u64)
}

/// Iterate FST entries; used by validation and diagnostics.
pub fn fst_entries(bytes: &[u8]) -> Result<Vec<(String, u64)>> {
    let map = fst::Map::new(bytes)?;
    let mut stream = map.stream();
    let mut entries = Vec::with_capacity(map.len());
    while let Some((key, value)) = stream.next() {
        entries.push((String::from_utf8(key.to_vec())?, value));
    }
    Ok(entries)
}
