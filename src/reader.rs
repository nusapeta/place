//! Read-only, validated access to a format v1 or v2 index.

use crate::format::*;
use crate::normalize::normalize;
use crate::place_id::decode;
use memmap2::{Mmap, MmapOptions};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid index: {0}")]
    Invalid(String),
    #[error("index checksum mismatch")]
    ChecksumMismatch,
}

fn invalid<T>(message: impl Into<String>) -> Result<T, IndexError> {
    Err(IndexError::Invalid(message.into()))
}

#[derive(Clone, Copy, Debug)]
pub struct GridInfo {
    pub cell_size_e7: i32,
    pub min_lat_e7: i32,
    pub min_lon_e7: i32,
    pub cols: i32,
    pub rows: i32,
    pub cell_count: u32,
}

impl GridInfo {
    pub fn key(&self, lat_e7: i32, lon_e7: i32) -> Option<u32> {
        if self.cell_size_e7 <= 0 {
            return None;
        }
        let col = (i64::from(lon_e7) - i64::from(self.min_lon_e7))
            .div_euclid(i64::from(self.cell_size_e7));
        let row = (i64::from(lat_e7) - i64::from(self.min_lat_e7))
            .div_euclid(i64::from(self.cell_size_e7));
        let col = i64::from(col);
        let row = i64::from(row);
        if col < 0 || col >= i64::from(self.cols) || row < 0 || row >= i64::from(self.rows) {
            return None;
        }
        u32::try_from(row * i64::from(self.cols) + col).ok()
    }

    pub fn max_lat_e7(&self) -> i32 {
        let value = i64::from(self.min_lat_e7)
            + i64::from(self.rows) * i64::from(self.cell_size_e7);
        value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }

    pub fn max_lon_e7(&self) -> i32 {
        let value = i64::from(self.min_lon_e7)
            + i64::from(self.cols) * i64::from(self.cell_size_e7);
        value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
}

/// Boundary rings for one admin, owned by the mapped index.
pub struct AdminPolygonData<'a> {
    pub place_id: u64,
    pub rings: Vec<&'a [i32]>,
}

pub struct Index {
    data: Mmap,
    pub header: Header,
    path: PathBuf,
}

impl Index {
    pub fn open(path: impl AsRef<Path>, verify_checksum: bool) -> Result<Self, IndexError> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        let index = Self::from_mmap(data, path.clone(), verify_checksum)?;
        Ok(index)
    }

    fn from_mmap(data: Mmap, path: PathBuf, verify_checksum: bool) -> Result<Self, IndexError> {
        if data.len() < HEADER_BYTES + TRAILER_BYTES {
            return invalid("file is shorter than header plus trailer");
        }
        if data[0..8] != MAGIC {
            return invalid("bad magic");
        }
        let version = get_u16(&data, 8);
        if !(MIN_FORMAT_VERSION..=FORMAT_VERSION).contains(&version) {
            return invalid(format!("unsupported format_version {version}"));
        }
        if version == 1 && get_u16(&data, 10) != 0 {
            return invalid("format v1 cannot have optional section flags");
        }
        let header_bytes = get_u32(&data, 12) as usize;
        if header_bytes < HEADER_BYTES || header_bytes > data.len() - TRAILER_BYTES {
            return invalid("header_bytes out of range");
        }
        if header_bytes != HEADER_BYTES {
            return invalid("index requires header_bytes = 256");
        }
        let mut sections = [Section::default(); SECTION_COUNT];
        for (index, section) in sections.iter_mut().enumerate() {
            let base = SECTION_TABLE_OFFSET + index * SECTION_ENTRY_BYTES;
            section.offset = get_u64(&data, base);
            section.len = get_u64(&data, base + 8);
        }
        let header = Header {
            flags: get_u16(&data, 10),
            feature_count: get_u32(&data, 16),
            admin_count: get_u32(&data, 20),
            class_count: get_u32(&data, 24),
            type_count: get_u32(&data, 28),
            min_lat_e7: get_i32(&data, 32),
            min_lon_e7: get_i32(&data, 36),
            max_lat_e7: get_i32(&data, 40),
            max_lon_e7: get_i32(&data, 44),
            string_bytes: get_u64(&data, 48),
            posting_count: get_u64(&data, 56),
            sections,
        };
        let index = Self { data, header, path };
        index.validate_layout()?;
        index.validate_contents()?;
        if verify_checksum {
            index.verify_checksum()?;
        }
        index.release_resident_pages();
        Ok(index)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn section(&self, index: usize) -> &[u8] {
        let section = self.header.sections[index];
        &self.data[section.offset as usize..(section.offset + section.len) as usize]
    }

    pub fn strings(&self) -> &str {
        // Validated once in validate_contents.
        std::str::from_utf8(self.section(SECTION_STRINGS)).expect("validated UTF-8")
    }

    pub fn str_at(&self, offset: u32, len: u16) -> &str {
        let start = offset as usize;
        let end = start + len as usize;
        std::str::from_utf8(&self.section(SECTION_STRINGS)[start..end]).expect("validated UTF-8")
    }

    pub fn places(&self) -> &[PlaceRecord] {
        bytemuck::cast_slice(self.section(SECTION_PLACES))
    }

    pub fn place(&self, row: u32) -> &PlaceRecord {
        &self.places()[row as usize]
    }

    pub fn find_row(&self, place_id: u64) -> Option<u32> {
        self.places()
            .binary_search_by_key(&place_id, |place| place.place_id)
            .ok()
            .map(|row| row as u32)
    }

    pub fn class(&self, row: u32) -> &str {
        let place = self.place(row);
        self.dict_value(SECTION_CLASSES, place.class_id)
    }

    pub fn place_type(&self, row: u32) -> &str {
        let place = self.place(row);
        self.dict_value(SECTION_TYPES, place.type_id)
    }

    fn dict_value(&self, section: usize, id: u16) -> &str {
        let bytes = self.section(section);
        let base = DICT_HEADER_BYTES + id as usize * DICT_ENTRY_BYTES;
        let record = bytemuck::from_bytes::<DictRecord>(&bytes[base..base + DICT_ENTRY_BYTES]);
        self.str_at(record.str_off, record.str_len)
    }

    pub fn variants_at(&self, offset: u32, count: u8) -> Vec<(String, String)> {
        let bytes = self.section(SECTION_VARIANTS);
        let mut result = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let base = offset as usize + index * VARIANT_BYTES;
            let record = bytemuck::from_bytes::<VariantRecord>(&bytes[base..base + VARIANT_BYTES]);
            let lang = lang_string(&record.lang);
            result.push((
                lang,
                self.str_at(record.str_off, record.str_len).to_string(),
            ));
        }
        result
    }

    pub fn place_variants(&self, row: u32) -> Vec<(String, String)> {
        let place = self.place(row);
        self.variants_at(place.names_off, place.names_count)
    }

    pub fn admin_records(&self) -> impl Iterator<Item = AdminRecord> + '_ {
        self.section(SECTION_ADMIN)
            .as_chunks::<ADMIN_BYTES>()
            .0
            .iter()
            .map(|bytes| AdminRecord(*bytes))
    }

    pub fn admin(&self, admin_id: u32) -> Option<AdminRecord> {
        if admin_id == 0 || admin_id > self.header.admin_count {
            return None;
        }
        let bytes = self.section(SECTION_ADMIN);
        let base = (admin_id as usize - 1) * ADMIN_BYTES;
        Some(AdminRecord(
            bytes[base..base + ADMIN_BYTES]
                .try_into()
                .expect("fixed record"),
        ))
    }

    pub fn admin_name(&self, admin: &AdminRecord) -> &str {
        self.str_at(admin.name_off(), admin.name_len())
    }

    pub fn admin_variants(&self, admin: &AdminRecord) -> Vec<(String, String)> {
        self.variants_at(admin.names_off(), admin.names_count())
    }

    pub fn token_fst(&self) -> Result<fst::Map<&[u8]>, IndexError> {
        fst::Map::new(self.section(SECTION_TOKEN_FST))
            .map_err(|error| IndexError::Invalid(format!("token FST: {error}")))
    }

    pub fn name_fst(&self) -> Result<fst::Map<&[u8]>, IndexError> {
        fst::Map::new(self.section(SECTION_NAME_FST))
            .map_err(|error| IndexError::Invalid(format!("name FST: {error}")))
    }

    pub fn postings(&self) -> &[u32] {
        bytemuck::cast_slice(self.section(SECTION_POSTINGS))
    }

    pub fn postings_for(&self, value: u64) -> Option<&[u32]> {
        let offset = (value >> 32) as usize;
        let count = (value & 0xffff_ffff) as usize;
        self.postings().get(offset..offset + count)
    }

    pub fn grid_info(&self) -> GridInfo {
        let bytes = self.section(SECTION_GRID);
        GridInfo {
            cell_size_e7: get_i32(bytes, 0),
            min_lat_e7: get_i32(bytes, 4),
            min_lon_e7: get_i32(bytes, 8),
            cols: get_i32(bytes, 12),
            rows: get_i32(bytes, 16),
            cell_count: get_u32(bytes, 20),
        }
    }

    pub fn grid_entries(&self) -> &[GridEntry] {
        let bytes = self.section(SECTION_GRID);
        bytemuck::cast_slice(&bytes[GRID_HEADER_BYTES..])
    }

    pub fn grid_postings(&self, key: u32) -> Option<&[u32]> {
        let entries = self.grid_entries();
        let index = entries.binary_search_by_key(&key, |entry| entry.key).ok()?;
        let entry = entries[index];
        self.postings()
            .get(entry.posting_off as usize..(entry.posting_off + entry.posting_count) as usize)
    }

    pub fn has_way_lines(&self) -> bool {
        self.header.flags & FLAG_WAY_LINES != 0
    }

    /// Returns the simplified polyline for a place row as flat `(lat, lon)`
    /// pairs in e7. `None` when the artifact has no way lines for that row.
    pub fn way_points(&self, row: u32) -> Option<&[i32]> {
        if !self.has_way_lines() {
            return None;
        }
        let bytes = self.section(SECTION_WAY_LINES);
        let count = get_u32(bytes, 0) as usize;
        let entries = &bytes[4..4 + count * 12];
        let index = entries
            .as_chunks::<12>()
            .0
            .binary_search_by_key(&row, |entry| get_u32(entry, 0))
            .ok()?;
        let entry = &entries[index * 12..index * 12 + 12];
        let offset = get_u32(entry, 4) as usize;
        let points = get_u32(entry, 8) as usize;
        let base = 4 + count * 12 + offset;
        Some(bytemuck::cast_slice(&bytes[base..base + points * 4]))
    }

    pub fn has_admin_polygons(&self) -> bool {
        self.header.flags & FLAG_ADMIN_POLYGONS != 0
    }

    /// Returns boundary rings as flat `(lat, lon)` pairs in e7.
    pub fn admin_polygon(&self, admin_id: u32) -> Option<AdminPolygonData<'_>> {
        if !self.has_admin_polygons() {
            return None;
        }
        let bytes = self.section(SECTION_POLYGONS);
        let count = get_u32(bytes, 0) as usize;
        let entries = &bytes[4..4 + count * 20];
        let index = entries
            .as_chunks::<20>()
            .0
            .binary_search_by_key(&admin_id, |entry| get_u32(entry, 0))
            .ok()?;
        let entry = &entries[index * 20..index * 20 + 20];
        let place_id = get_u64(entry, 4);
        let offset = get_u32(entry, 12) as usize;
        let len = get_u32(entry, 16) as usize;
        let mut position = 4 + count * 20 + offset;
        let end = position + len;
        let ring_count = get_u32(bytes, position) as usize;
        position += 4;
        let mut rings = Vec::with_capacity(ring_count);
        for _ in 0..ring_count {
            let points = get_u32(bytes, position) as usize;
            position += 4;
            rings.push(bytemuck::cast_slice(&bytes[position..position + points * 4]));
            position += points * 4;
        }
        debug_assert_eq!(position, end);
        Some(AdminPolygonData { place_id, rings })
    }

    fn validate_layout(&self) -> Result<(), IndexError> {
        let trailer_start = self.data.len() - TRAILER_BYTES;
        let mut previous_end = HEADER_BYTES;
        for (index, section) in self.header.sections.iter().enumerate() {
            let offset = section.offset as usize;
            let len = section.len as usize;
            if !offset.is_multiple_of(8) {
                return invalid(format!(
                    "section {} is not 8-byte aligned",
                    SECTION_NAMES[index]
                ));
            }
            if offset < previous_end {
                return invalid(format!(
                    "section {} overlaps previous section",
                    SECTION_NAMES[index]
                ));
            }
            let end = offset.checked_add(len).ok_or_else(|| {
                IndexError::Invalid(format!("section {} overflows", SECTION_NAMES[index]))
            })?;
            if end > trailer_start {
                return invalid(format!("section {} exceeds body", SECTION_NAMES[index]));
            }
            if len > 0 {
                previous_end = end;
            }
        }
        if self.header.flags & !(FLAG_ADMIN_POLYGONS | FLAG_WAY_LINES) != 0 {
            return invalid("unknown header flags");
        }
        if self.header.flags & FLAG_ADMIN_POLYGONS != 0
            && self.header.sections[SECTION_POLYGONS].len == 0
        {
            return invalid("admin polygon flag set with empty polygon section");
        }
        if self.header.flags & FLAG_ADMIN_POLYGONS == 0
            && self.header.sections[SECTION_POLYGONS].len != 0
        {
            return invalid("polygon section must be empty when flag is unset");
        }
        if self.header.flags & FLAG_WAY_LINES != 0
            && self.header.sections[SECTION_WAY_LINES].len == 0
        {
            return invalid("way line flag set with empty way_lines section");
        }
        if self.header.flags & FLAG_WAY_LINES == 0
            && self.header.sections[SECTION_WAY_LINES].len != 0
        {
            return invalid("way_lines section must be empty when flag is unset");
        }
        if self.data.len() < HEADER_BYTES + TRAILER_BYTES {
            return invalid("truncated file");
        }
        Ok(())
    }

    fn validate_contents(&self) -> Result<(), IndexError> {
        let strings = self.section(SECTION_STRINGS);
        if strings.contains(&0) {
            return invalid("strings section contains NUL");
        }
        if std::str::from_utf8(strings).is_err() {
            return invalid("strings section is not UTF-8");
        }
        if self.header.string_bytes != strings.len() as u64 {
            return invalid("string_bytes does not match section");
        }

        let places = self.places();
        if places.len() != self.header.feature_count as usize {
            return invalid("places section size does not match feature_count");
        }
        let admins = self.section(SECTION_ADMIN);
        if admins.len() != self.header.admin_count as usize * ADMIN_BYTES {
            return invalid("admin section size does not match admin_count");
        }
        self.validate_dict(SECTION_CLASSES, self.header.class_count)?;
        self.validate_dict(SECTION_TYPES, self.header.type_count)?;
        self.validate_variants()?;
        self.validate_places()?;
        self.validate_postings()?;
        self.validate_grid()?;
        self.validate_admin()?;
        self.validate_polygons()?;
        self.validate_way_lines()?;
        Ok(())
    }

    fn validate_dict(&self, section: usize, expected: u32) -> Result<(), IndexError> {
        let bytes = self.section(section);
        if bytes.len() < DICT_HEADER_BYTES {
            return invalid("dictionary is truncated");
        }
        let count = get_u32(bytes, 0);
        if count != expected {
            return invalid("dictionary count does not match header");
        }
        if bytes.len() != DICT_HEADER_BYTES + count as usize * DICT_ENTRY_BYTES {
            return invalid("dictionary size mismatch");
        }
        let mut previous: Option<&str> = None;
        for index in 0..count as usize {
            let base = DICT_HEADER_BYTES + index * DICT_ENTRY_BYTES;
            let record = bytemuck::from_bytes::<DictRecord>(&bytes[base..base + DICT_ENTRY_BYTES]);
            let value = self.checked_str(record.str_off, record.str_len)?;
            if value.is_empty() {
                return invalid("empty dictionary value");
            }
            if let Some(previous) = previous
                && previous >= value
            {
                return invalid("dictionary is not strictly sorted");
            }
            previous = Some(value);
        }
        Ok(())
    }

    fn validate_variants(&self) -> Result<(), IndexError> {
        let bytes = self.section(SECTION_VARIANTS);
        if !bytes.len().is_multiple_of(VARIANT_BYTES) {
            return invalid("variants section has a partial record");
        }
        for chunk in bytes.as_chunks::<VARIANT_BYTES>().0 {
            let record = bytemuck::from_bytes::<VariantRecord>(chunk);
            if record.lang == [0, 0, 0] || record.pad0 != 0 || record.pad1 != 0 {
                return invalid("invalid variant padding or language");
            }
            self.checked_str(record.str_off, record.str_len)?;
        }
        Ok(())
    }

    fn validate_places(&self) -> Result<(), IndexError> {
        let strings_len = self.section(SECTION_STRINGS).len() as u32;
        let variants_len = self.section(SECTION_VARIANTS).len() as u32;
        let mut previous = 0u64;
        for (row, place) in self.places().iter().enumerate() {
            if row > 0 && place.place_id <= previous {
                return invalid("places are not strictly sorted by place_id");
            }
            previous = place.place_id;
            let (kind, _) = decode(place.place_id)
                .map_err(|error| IndexError::Invalid(format!("row {row}: {error}")))?;
            if kind != place.kind {
                return invalid(format!("row {row}: kind does not match place_id"));
            }
            if place.name_len == 0
                || place.name_off as u64 + place.name_len as u64 > strings_len as u64
            {
                return invalid(format!("row {row}: name offset out of bounds"));
            }
            if !(-900_000_000..=900_000_000).contains(&place.lat_e7)
                || !(-1_800_000_000..=1_800_000_000).contains(&place.lon_e7)
            {
                return invalid(format!("row {row}: coordinate out of range"));
            }
            if !place.importance.is_finite() || !(0.0..=1.0).contains(&place.importance) {
                return invalid(format!("row {row}: importance out of range"));
            }
            if place.admin_id > self.header.admin_count {
                return invalid(format!("row {row}: admin_id out of range"));
            }
            if u32::from(place.class_id) >= self.header.class_count
                || u32::from(place.type_id) >= self.header.type_count
            {
                return invalid(format!("row {row}: class or type id out of range"));
            }
            if place.bbox_min_lat_e7 > place.lat_e7
                || place.lat_e7 > place.bbox_max_lat_e7
                || place.bbox_min_lon_e7 > place.lon_e7
                || place.lon_e7 > place.bbox_max_lon_e7
            {
                return invalid(format!("row {row}: bbox does not contain point"));
            }
            if place.bbox_min_lat_e7 < self.header.min_lat_e7
                || place.bbox_max_lat_e7 > self.header.max_lat_e7
                || place.bbox_min_lon_e7 < self.header.min_lon_e7
                || place.bbox_max_lon_e7 > self.header.max_lon_e7
            {
                return invalid(format!("row {row}: bbox outside index bbox"));
            }
            if place.reserved != [0; 7] {
                return invalid(format!("row {row}: reserved bytes are not zero"));
            }
            if place.names_count > 0
                && place.names_off as u64 + place.names_count as u64 * VARIANT_BYTES as u64
                    > variants_len as u64
            {
                return invalid(format!("row {row}: name variants out of range"));
            }
        }
        Ok(())
    }

    fn validate_postings(&self) -> Result<(), IndexError> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.validate_postings_inner()
        })) {
            Ok(result) => result,
            Err(_) => invalid("FST data is corrupt"),
        }
    }

    fn validate_postings_inner(&self) -> Result<(), IndexError> {
        let postings = self.postings();
        if postings.len() as u64 != self.header.posting_count {
            return invalid("posting_count does not match section");
        }
        if postings.iter().any(|row| *row >= self.header.feature_count) {
            return invalid("posting row out of range");
        }
        let token_fst = self.token_fst()?;
        use fst::Streamer;
        let mut stream = token_fst.stream();
        let mut previous: Option<Vec<u8>> = None;
        while let Some((key, value)) = stream.next() {
            if key.is_empty() {
                return invalid("empty token FST key");
            }
            if let Some(previous) = &previous
                && previous.as_slice() >= key
            {
                return invalid("token FST keys are not strictly sorted");
            }
            previous = Some(key.to_vec());
            let offset = (value >> 32) as u32;
            let count = value as u32;
            if count == 0 || offset as u64 + count as u64 > self.header.posting_count {
                return invalid("token FST value points outside postings");
            }
            self.validate_posting_range(offset, count)?;
        }
        let name_fst = self.name_fst()?;
        let mut stream = name_fst.stream();
        while let Some((key, value)) = stream.next() {
            if key.is_empty() {
                return invalid("empty name FST key");
            }
            if value >= self.header.feature_count as u64 {
                return invalid("name FST value out of range");
            }
        }
        Ok(())
    }

    fn validate_posting_range(&self, offset: u32, count: u32) -> Result<(), IndexError> {
        let rows = &self.postings()[offset as usize..(offset + count) as usize];
        if rows.windows(2).any(|pair| pair[0] >= pair[1]) {
            return invalid("posting range is not strictly increasing");
        }
        Ok(())
    }

    fn validate_grid(&self) -> Result<(), IndexError> {
        let bytes = self.section(SECTION_GRID);
        if bytes.len() < GRID_HEADER_BYTES {
            return invalid("grid section is truncated");
        }
        let grid = self.grid_info();
        if grid.cell_size_e7 <= 0 || grid.cols <= 0 || grid.rows <= 0 {
            return invalid("invalid grid dimensions");
        }
        if bytes.len() != GRID_HEADER_BYTES + grid.cell_count as usize * GRID_ENTRY_BYTES {
            return invalid("grid section size mismatch");
        }
        if grid.cell_count as i64 > grid.cols as i64 * grid.rows as i64 {
            return invalid("grid cell_count exceeds cols * rows");
        }
        if grid.min_lat_e7 > self.header.min_lat_e7
            || grid.min_lon_e7 > self.header.min_lon_e7
            || grid.max_lat_e7() < self.header.max_lat_e7
            || grid.max_lon_e7() < self.header.max_lon_e7
        {
            return invalid("grid does not cover index bbox");
        }
        let mut previous = None;
        let max_key = grid.cols as i64 * grid.rows as i64;
        for entry in self.grid_entries() {
            if entry.key as i64 >= max_key {
                return invalid("grid key out of range");
            }
            if let Some(previous) = previous
                && previous >= entry.key
            {
                return invalid("grid keys are not strictly sorted");
            }
            previous = Some(entry.key);
            if entry.posting_count == 0
                || entry.posting_off as u64 + entry.posting_count as u64 > self.header.posting_count
            {
                return invalid("grid posting range out of bounds");
            }
            self.validate_posting_range(entry.posting_off, entry.posting_count)?;
        }
        Ok(())
    }

    fn validate_admin(&self) -> Result<(), IndexError> {
        let count = self.header.admin_count;
        let mut seen = vec![false; count as usize + 1];
        for admin in self.admin_records() {
            let id = admin.admin_id();
            if id == 0 || id > count || seen[id as usize] {
                return invalid("admin_id is zero, out of range, or duplicated");
            }
            seen[id as usize] = true;
            if !(4..=7).contains(&admin.level()) {
                return invalid("admin level outside 4..=7");
            }
            if admin.parent_id() > count {
                return invalid("admin parent out of range");
            }
            self.checked_str(admin.name_off(), admin.name_len())?;
            if admin.bbox_min_lat_e7() > admin.bbox_max_lat_e7()
                || admin.bbox_min_lon_e7() > admin.bbox_max_lon_e7()
            {
                return invalid("invalid admin bbox");
            }
        }
        for admin in self.admin_records() {
            let mut current = admin;
            let mut steps = 0u32;
            while current.parent_id() != 0 {
                if steps > count {
                    return invalid("admin parent cycle");
                }
                let parent = self.admin(current.parent_id()).ok_or_else(|| {
                    IndexError::Invalid("admin parent does not exist".to_string())
                })?;
                if parent.level() >= current.level() {
                    return invalid("admin parent level is not lower");
                }
                current = parent;
                steps += 1;
            }
        }
        Ok(())
    }

    fn validate_polygons(&self) -> Result<(), IndexError> {
        if !self.has_admin_polygons() {
            return Ok(());
        }
        let bytes = self.section(SECTION_POLYGONS);
        if bytes.len() < 4 {
            return invalid("polygons section is truncated");
        }
        let count = get_u32(bytes, 0) as usize;
        let directory_end = 4 + count * 20;
        if directory_end > bytes.len() {
            return invalid("polygon directory out of range");
        }
        let mut previous: Option<u32> = None;
        for chunk in bytes[4..directory_end].as_chunks::<20>().0 {
            let admin_id = get_u32(chunk, 0);
            if admin_id == 0 || admin_id > self.header.admin_count {
                return invalid("polygon admin_id out of range");
            }
            if previous.is_some_and(|value| value >= admin_id) {
                return invalid("polygon admin_ids are not sorted and unique");
            }
            previous = Some(admin_id);
            let offset = get_u32(chunk, 12) as usize;
            let len = get_u32(chunk, 16) as usize;
            let base = directory_end + offset;
            let Some(end) = base.checked_add(len) else {
                return invalid("polygon body offset overflow");
            };
            if end > bytes.len() {
                return invalid("polygon body out of range");
            }
            let mut position = base;
            if position + 4 > end {
                return invalid("polygon ring count out of range");
            }
            let ring_count = get_u32(bytes, position) as usize;
            position += 4;
            for _ in 0..ring_count {
                if position + 4 > end {
                    return invalid("polygon point count out of range");
                }
                let points = get_u32(bytes, position) as usize;
                position += 4;
                if points < 6 || points % 2 != 0 {
                    return invalid("polygon ring needs at least three points");
                }
                let Some(next) = position.checked_add(points * 4) else {
                    return invalid("polygon ring offset overflow");
                };
                if next > end {
                    return invalid("polygon ring out of range");
                }
                position = next;
            }
            if position != end {
                return invalid("polygon body length mismatch");
            }
        }
        Ok(())
    }

    fn validate_way_lines(&self) -> Result<(), IndexError> {
        if !self.has_way_lines() {
            return Ok(());
        }
        let bytes = self.section(SECTION_WAY_LINES);
        if bytes.len() < 4 {
            return invalid("way_lines section is truncated");
        }
        let count = get_u32(bytes, 0) as usize;
        let directory_end = 4 + count * 12;
        if directory_end > bytes.len() {
            return invalid("way_lines directory out of range");
        }
        let mut previous: Option<u32> = None;
        for chunk in bytes[4..directory_end].as_chunks::<12>().0 {
            let row = get_u32(chunk, 0);
            if row >= self.header.feature_count {
                return invalid("way_lines row out of range");
            }
            if previous.is_some_and(|value| value >= row) {
                return invalid("way_lines rows are not sorted and unique");
            }
            previous = Some(row);
            let offset = get_u32(chunk, 4) as usize;
            let points = get_u32(chunk, 8) as usize;
            if points < 4 || points % 2 != 0 {
                return invalid("way_lines point count must be even and at least two points");
            }
            let base = directory_end + offset;
            if base.checked_add(points * 4).is_none_or(|end| end > bytes.len()) {
                return invalid("way_lines polyline out of range");
            }
        }
        Ok(())
    }

    fn checked_str(&self, offset: u32, len: u16) -> Result<&str, IndexError> {
        let end = offset as u64 + len as u64;
        if end > self.section(SECTION_STRINGS).len() as u64 {
            return invalid("string offset out of bounds");
        }
        Ok(self.str_at(offset, len))
    }

    fn release_resident_pages(&self) {
        #[cfg(target_os = "linux")]
        {
            // Validation touches every section; drop clean pages so idle RSS
            // reflects live data rather than a warm page cache.
            unsafe {
                libc::madvise(
                    self.data.as_ptr() as *mut libc::c_void,
                    self.data.len(),
                    libc::MADV_DONTNEED,
                );
            }
        }
    }

    pub fn verify_checksum(&self) -> Result<(), IndexError> {
        let body_end = self.data.len() - TRAILER_BYTES;
        let expected = &self.data[body_end..];
        let actual = Sha256::digest(&self.data[..body_end]);
        if expected != actual.as_slice() {
            return Err(IndexError::ChecksumMismatch);
        }
        Ok(())
    }

    pub fn normalized_name(&self, row: u32) -> String {
        let place = self.place(row);
        normalize(self.str_at(place.name_off, place.name_len))
    }
}

pub fn lang_string(lang: &[u8; 3]) -> String {
    let end = lang.iter().position(|byte| *byte == 0).unwrap_or(3);
    String::from_utf8_lossy(&lang[..end]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Admin, IndexData, Place};
    use crate::place_id::encode;
    use crate::writer::write_index;
    use std::fs;
    use tempfile::TempDir;

    pub(crate) fn sample() -> IndexData {
        let mut data = IndexData::default();
        data.places.push(Place {
            place_id: encode(1, 10).unwrap(),
            kind: 1,
            osm_id: 10,
            name: "Kota Contoh".to_string(),
            variants: vec![("en".to_string(), "Example City".to_string())],
            lat_e7: -780_000_000,
            lon_e7: 110_000_000,
            importance: 0.9,
            admin_id: 2,
            bbox_min_lat_e7: -781_000_000,
            bbox_min_lon_e7: 109_900_000,
            bbox_max_lat_e7: -779_000_000,
            bbox_max_lon_e7: 110_100_000,
            class: "place".to_string(),
            r#type: "city".to_string(),
        });
        data.places.push(Place {
            place_id: encode(2, 20).unwrap(),
            kind: 2,
            osm_id: 20,
            name: "Jalan Contoh".to_string(),
            variants: vec![],
            lat_e7: -780_500_000,
            lon_e7: 110_050_000,
            importance: 0.5,
            admin_id: 1,
            bbox_min_lat_e7: -781_000_000,
            bbox_min_lon_e7: 110_000_000,
            bbox_max_lat_e7: -780_000_000,
            bbox_max_lon_e7: 110_100_000,
            class: "highway".to_string(),
            r#type: "residential".to_string(),
        });
        data.admins.push(Admin {
            admin_id: 1,
            parent_id: 0,
            level: 4,
            name: "Provinsi Contoh".to_string(),
            variants: vec![],
            bbox_min_lat_e7: -790_000_000,
            bbox_min_lon_e7: 109_000_000,
            bbox_max_lat_e7: -770_000_000,
            bbox_max_lon_e7: 111_000_000,
        });
        data.admins.push(Admin {
            admin_id: 2,
            parent_id: 1,
            level: 5,
            name: "Kota Contoh".to_string(),
            variants: vec![],
            bbox_min_lat_e7: -785_000_000,
            bbox_min_lon_e7: 109_500_000,
            bbox_max_lat_e7: -775_000_000,
            bbox_max_lon_e7: 110_500_000,
        });
        data
    }

    fn fixture() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("places.idx");
        write_index(&sample(), &path).unwrap();
        (dir, path)
    }

    #[test]
    fn round_trips_all_sections() {
        let (_dir, path) = fixture();
        let index = Index::open(&path, true).unwrap();
        assert_eq!(index.header.feature_count, 2);
        assert_eq!(index.header.admin_count, 2);
        assert_eq!(index.place(0).name_len, "Kota Contoh".len() as u16);
        assert_eq!(index.class(0), "place");
        assert_eq!(index.place_type(0), "city");
        assert_eq!(index.find_row(index.place(1).place_id), Some(1));
        assert_eq!(index.admin_name(&index.admin(2).unwrap()), "Kota Contoh");
        assert_eq!(
            index
                .grid_postings(index.grid_info().key(-780_500_000, 110_050_000).unwrap())
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            index
                .grid_postings(index.grid_info().key(-780_000_000, 110_000_000).unwrap())
                .unwrap()
                .len(),
            2
        );
        let token = index.token_fst().unwrap().get("jalan").unwrap();
        assert_eq!(index.postings_for(token).unwrap().len(), 1);
    }

    #[test]
    fn all_sections_are_aligned_and_inside_the_file() {
        let (_dir, path) = fixture();
        let bytes = fs::read(&path).unwrap();
        let index = Index::open(&path, true).unwrap();
        for (number, section) in index.header.sections.iter().enumerate() {
            if section.len == 0 {
                continue;
            }
            assert_eq!(section.offset % 8, 0, "section {number} alignment");
            assert!(
                section.offset + section.len <= bytes.len() as u64,
                "section {number} bounds"
            );
        }
        assert_eq!(index.place(0).name_len as usize, "Kota Contoh".len());
        assert_eq!(index.place(1).name_len as usize, "Jalan Contoh".len());
    }

    #[test]
    fn two_builds_are_identical() {
        let (_dir_a, a) = fixture();
        let (_dir_b, b) = fixture();
        assert_eq!(fs::read(a).unwrap(), fs::read(b).unwrap());
    }

    #[test]
    fn open_is_read_only_and_creates_no_files() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, path) = fixture();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        let before: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        {
            let index = Index::open(&path, true).unwrap();
            assert_eq!(index.header.feature_count, 2);
        }
        let after: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(before, after);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    }

    #[test]
    fn checksum_detects_mutation() {
        let (_dir, path) = fixture();
        let mut bytes = fs::read(&path).unwrap();
        let index = Index::open(&path, true).unwrap();
        let section = index.header.sections[SECTION_STRINGS].offset as usize;
        bytes[section] ^= 1;
        let bad = path.with_extension("bad");
        fs::write(&bad, bytes).unwrap();
        assert!(matches!(
            Index::open(&bad, true),
            Err(IndexError::ChecksumMismatch)
        ));
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::format::{
        SECTION_CLASSES, SECTION_GRID, SECTION_PLACES, SECTION_POSTINGS, SECTION_TOKEN_FST,
        get_u32, get_u64, put_u16, put_u32,
    };
    use crate::writer::write_index;
    use std::fs;
    use tempfile::TempDir;

    fn valid_bytes() -> (TempDir, Vec<u8>) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("places.idx");
        write_index(&super::tests::sample(), &path).unwrap();
        let bytes = fs::read(path).unwrap();
        (dir, bytes)
    }

    fn section(bytes: &[u8], index: usize) -> (usize, usize) {
        let base = crate::format::SECTION_TABLE_OFFSET + index * crate::format::SECTION_ENTRY_BYTES;
        (
            get_u64(bytes, base) as usize,
            get_u64(bytes, base + 8) as usize,
        )
    }

    fn reject(bytes: &[u8], message: &str) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.idx");
        fs::write(&path, bytes).unwrap();
        let result = Index::open(&path, false);
        match result {
            Err(IndexError::Invalid(reason)) => {
                assert!(!reason.is_empty(), "{message}: empty reason");
            }
            Err(error) => panic!("{message}: unexpected error {error}"),
            Ok(_) => panic!("{message}: mutation was accepted"),
        }
    }

    #[test]
    fn rejects_bad_magic() {
        let (_dir, mut bytes) = valid_bytes();
        bytes[0] = b'X';
        reject(&bytes, "magic");
    }

    #[test]
    fn rejects_bad_version() {
        let (_dir, mut bytes) = valid_bytes();
        put_u16(&mut bytes, 8, 3);
        reject(&bytes, "version");
    }

    #[test]
    fn rejects_v1_with_optional_flags() {
        let (_dir, mut bytes) = valid_bytes();
        put_u16(&mut bytes, 8, 1);
        put_u16(&mut bytes, 10, FLAG_ADMIN_POLYGONS);
        reject(&bytes, "v1 flags");
    }

    #[test]
    fn rejects_bad_header_size() {
        let (_dir, mut bytes) = valid_bytes();
        put_u32(&mut bytes, 12, 512);
        reject(&bytes, "header size");
    }

    #[test]
    fn rejects_misaligned_section() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, _) = section(&bytes, SECTION_PLACES);
        put_u64(
            &mut bytes,
            crate::format::SECTION_TABLE_OFFSET,
            offset as u64 + 1,
        );
        reject(&bytes, "alignment");
    }

    #[test]
    fn rejects_section_out_of_bounds() {
        let (_dir, mut bytes) = valid_bytes();
        let base = crate::format::SECTION_TABLE_OFFSET
            + SECTION_PLACES * crate::format::SECTION_ENTRY_BYTES;
        let len = bytes.len() as u64;
        put_u64(&mut bytes, base + 8, len);
        reject(&bytes, "bounds");
    }

    #[test]
    fn rejects_feature_count_mismatch() {
        let (_dir, mut bytes) = valid_bytes();
        let count = get_u32(&bytes, 16) + 1;
        put_u32(&mut bytes, 16, count);
        reject(&bytes, "feature count");
    }

    #[test]
    fn rejects_place_kind_mismatch() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, _) = section(&bytes, SECTION_PLACES);
        bytes[offset + 14] = 2;
        reject(&bytes, "place kind");
    }

    #[test]
    fn rejects_name_offset_out_of_bounds() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, _) = section(&bytes, SECTION_PLACES);
        put_u32(&mut bytes, offset + 8, u32::MAX);
        reject(&bytes, "name offset");
    }

    #[test]
    fn rejects_dictionary_count_mismatch() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, len) = section(&bytes, SECTION_CLASSES);
        assert!(len >= 8);
        let count = get_u32(&bytes, offset) + 1;
        put_u32(&mut bytes, offset, count);
        reject(&bytes, "dictionary count");
    }

    #[test]
    fn rejects_grid_count_mismatch() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, _) = section(&bytes, SECTION_GRID);
        let count = get_u32(&bytes, offset + 20) + 1;
        put_u32(&mut bytes, offset + 20, count);
        reject(&bytes, "grid count");
    }

    #[test]
    fn rejects_unsorted_grid_keys() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, len) = section(&bytes, SECTION_GRID);
        assert!(len >= crate::format::GRID_HEADER_BYTES + 2 * crate::format::GRID_ENTRY_BYTES);
        let second = offset + crate::format::GRID_HEADER_BYTES + crate::format::GRID_ENTRY_BYTES;
        put_u32(&mut bytes, second, 0);
        reject(&bytes, "grid order");
    }

    #[test]
    fn rejects_posting_row_out_of_range() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, len) = section(&bytes, SECTION_POSTINGS);
        assert!(len >= 4);
        put_u32(&mut bytes, offset, 2);
        reject(&bytes, "posting row");
    }

    #[test]
    fn rejects_corrupt_token_fst() {
        let (_dir, mut bytes) = valid_bytes();
        let (offset, len) = section(&bytes, SECTION_TOKEN_FST);
        assert!(len > 0);
        bytes[offset] = 0xff;
        reject(&bytes, "token fst");
    }

    #[test]
    fn fuzz_smoke_never_panics() {
        let (_dir, base) = valid_bytes();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("fuzz.idx");
        for seed in 0..400u32 {
            let mut bytes = base.clone();
            let len = bytes.len();
            for step in 0..4 {
                let index = (seed as usize * 31 + step * 97) % len;
                bytes[index] ^= (seed as u8).wrapping_add(step as u8) | 1;
            }
            fs::write(&path, &bytes).unwrap();
            let result = std::panic::catch_unwind(|| Index::open(&path, false));
            assert!(result.is_ok(), "seed {seed} panicked");
        }
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let (_dir, mut bytes) = valid_bytes();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.idx");
        fs::write(&path, bytes).unwrap();
        assert!(matches!(
            Index::open(&path, true),
            Err(IndexError::ChecksumMismatch)
        ));
    }
}
