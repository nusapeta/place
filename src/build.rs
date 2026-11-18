//! OSM PBF to in-memory index data pipeline.

use crate::model::{Admin, Counters, IndexData, Place, WayLine};
use crate::place_id::encode_i64;
use anyhow::{Context, Result};
use osmpbf::{Element, ElementReader, RelMemberType};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const ADMIN_LEVELS: std::ops::RangeInclusive<u8> = 4..=7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Node = 1,
    Way = 2,
    Relation = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Outer,
    Inner,
}

#[derive(Clone, Debug)]
struct Member {
    kind: Kind,
    id: i64,
    role: Role,
}

#[derive(Clone, Debug)]
struct Draft {
    kind: Kind,
    osm_id: i64,
    name: String,
    variants: Vec<(String, String)>,
    class: String,
    place_type: String,
    importance: f32,
    admin_level: Option<u8>,
    position: Option<(i32, i32)>,
    refs: Vec<i64>,
    members: Vec<Member>,
}

#[derive(Default)]
struct TagInfo {
    name: Option<String>,
    variants: Vec<(String, String)>,
    admin_level: Option<u8>,
    boundary: Option<String>,
    place: Option<String>,
    highway: Option<String>,
    amenity: Option<String>,
    shop: Option<String>,
    tourism: Option<String>,
    leisure: Option<String>,
    natural: Option<String>,
    waterway: Option<String>,
    railway: Option<String>,
    aeroway: Option<String>,
    landuse: Option<String>,
    building: Option<String>,
    man_made: Option<String>,
    historic: Option<String>,
    office: Option<String>,
    craft: Option<String>,
    healthcare: Option<String>,
    public_transport: Option<String>,
    power: Option<String>,
    population: Option<u64>,
    capital: bool,
    wikidata: bool,
    brand: Option<String>,
}

#[derive(Default)]
struct Pass1 {
    nodes: Vec<Draft>,
    ways: Vec<Draft>,
    relations: Vec<Draft>,
    relation_way_ids: HashSet<i64>,
    needed_nodes: Vec<i64>,
    skipped: u64,
    long_names: u64,
}

#[derive(Default)]
struct Geoms {
    coords: Vec<(i64, i32, i32)>,
    way_refs: HashMap<i64, Vec<i64>>,
    missing_nodes: u64,
}

impl Geoms {
    fn insert_coord(&mut self, id: i64, point: (i32, i32)) {
        self.coords.push((id, point.0, point.1));
    }

    fn coordinate(&self, id: i64) -> Option<(i32, i32)> {
        self.coords
            .binary_search_by_key(&id, |entry| entry.0)
            .ok()
            .map(|index| (self.coords[index].1, self.coords[index].2))
    }

    fn sort_coordinates(&mut self) {
        self.coords.sort_unstable_by_key(|entry| entry.0);
    }
}

struct AdminDraft {
    osm_id: i64,
    level: u8,
    name: String,
    variants: Vec<(String, String)>,
    rings: Vec<Vec<(i32, i32)>>,
    bbox: [i32; 4],
    centroid: (i32, i32),
    area: f64,
    admin_id: u32,
    parent_id: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BuildTimings {
    pub features: f64,
    pub way_geometry: f64,
    pub node_coordinates: f64,
    pub assemble: f64,
}

#[derive(Debug, Default)]
pub struct BuildReport {
    pub counters: Counters,
    pub warnings: Vec<String>,
    pub timings: BuildTimings,
    pub kinds: [u64; 3],
    pub classes: BTreeMap<String, u64>,
}

pub fn build_data(pbf: &Path) -> Result<(IndexData, BuildReport)> {
    let features_started = std::time::Instant::now();
    let mut pass1 = Pass1 {
        nodes: Vec::new(),
        ways: Vec::new(),
        relations: Vec::new(),
        relation_way_ids: HashSet::new(),
        needed_nodes: Vec::new(),
        skipped: 0,
        long_names: 0,
    };

    ElementReader::from_path(pbf)
        .with_context(|| format!("open {}", pbf.display()))?
        .for_each(|element| match element {
            Element::Node(node) => {
                let tags = read_tags(node.tags());
                let Some(info) = tags else { return };
                let position = Some((
                    (node.lat() * 1e7).round() as i32,
                    (node.lon() * 1e7).round() as i32,
                ));
                push_draft(
                    &mut pass1,
                    Kind::Node,
                    node.id(),
                    info,
                    position,
                    Vec::new(),
                    Vec::new(),
                );
            }
            Element::DenseNode(node) => {
                let tags = read_tags(node.tags());
                let Some(info) = tags else { return };
                push_draft(
                    &mut pass1,
                    Kind::Node,
                    node.id,
                    info,
                    Some((node.decimicro_lat(), node.decimicro_lon())),
                    Vec::new(),
                    Vec::new(),
                );
            }
            Element::Way(way) => {
                let Some(info) = read_tags(way.tags()) else {
                    return;
                };
                let refs: Vec<i64> = way.refs().collect();
                for node in &refs {
                    pass1.needed_nodes.push(*node);
                }
                push_draft(
                    &mut pass1,
                    Kind::Way,
                    way.id(),
                    info,
                    None,
                    refs,
                    Vec::new(),
                );
            }
            Element::Relation(relation) => {
                let Some(info) = read_tags(relation.tags()) else {
                    return;
                };
                let mut members = Vec::new();
                for member in relation.members() {
                    let Ok(role) = member.role() else {
                        continue;
                    };
                    let role = if role == "inner" {
                        Role::Inner
                    } else {
                        Role::Outer
                    };
                    let kind = match member.member_type {
                        RelMemberType::Node => Kind::Node,
                        RelMemberType::Way => Kind::Way,
                        RelMemberType::Relation => continue,
                    };
                    if kind == Kind::Way {
                        pass1.relation_way_ids.insert(member.member_id);
                    } else {
                        pass1.needed_nodes.push(member.member_id);
                    }
                    members.push(Member {
                        kind,
                        id: member.member_id,
                        role,
                    });
                }
                push_draft(
                    &mut pass1,
                    Kind::Relation,
                    relation.id(),
                    info,
                    None,
                    Vec::new(),
                    members,
                );
            }
        })?;

    let features_seconds = features_started.elapsed().as_secs_f64();
    eprintln!("pass 1 selected features in {features_seconds:.2}s");
    let way_geometry_started = std::time::Instant::now();
    pass1.needed_nodes.sort_unstable();
    pass1.needed_nodes.dedup();
    let mut geoms = Geoms::default();
    ElementReader::from_path(pbf)
        .with_context(|| format!("open {} for way geometry", pbf.display()))?
        .for_each(|element| {
            if let Element::Way(way) = element
                && pass1.relation_way_ids.contains(&way.id())
            {
                let refs: Vec<i64> = way.refs().collect();
                for node in &refs {
                    pass1.needed_nodes.push(*node);
                }
                geoms.way_refs.insert(way.id(), refs);
            }
        })?;

    let way_geometry_seconds = way_geometry_started.elapsed().as_secs_f64();
    eprintln!("pass 2 read relation way geometry in {way_geometry_seconds:.2}s");
    pass1.needed_nodes.sort_unstable();
    pass1.needed_nodes.dedup();
    let node_coordinates_started = std::time::Instant::now();
    ElementReader::from_path(pbf)
        .with_context(|| format!("open {} for node coordinates", pbf.display()))?
        .for_each(|element| match element {
            Element::Node(node) if pass1.needed_nodes.binary_search(&node.id()).is_ok() => {
                geoms.insert_coord(
                    node.id(),
                    (
                        (node.lat() * 1e7).round() as i32,
                        (node.lon() * 1e7).round() as i32,
                    ),
                );
            }
            Element::DenseNode(node) if pass1.needed_nodes.binary_search(&node.id).is_ok() => {
                geoms.insert_coord(node.id, (node.decimicro_lat(), node.decimicro_lon()));
            }
            _ => {}
        })?;

    let node_coordinates_seconds = node_coordinates_started.elapsed().as_secs_f64();
    eprintln!("pass 3 read node coordinates in {node_coordinates_seconds:.2}s");
    geoms.sort_coordinates();
    let assemble_started = std::time::Instant::now();
    geoms.missing_nodes = pass1
        .needed_nodes
        .iter()
        .filter(|id| geoms.coordinate(**id).is_none())
        .count() as u64;

    let mut warnings = Vec::new();
    if geoms.missing_nodes > 0 {
        warnings.push(format!(
            "{} node references were missing; geometries that required them were skipped",
            geoms.missing_nodes
        ));
    }
    if pass1.long_names > 0 {
        warnings.push(format!(
            "{} names or variants exceeded the format limit and were skipped",
            pass1.long_names
        ));
    }

    let (mut admins, invalid_polygons) = build_admins(&pass1.relations, &geoms, &mut warnings);
    let mut admin_index = AdminIndex::new(&admins);
    admin_index.invalid_polygons = invalid_polygons;
    assign_parents(&mut admins, &admin_index);
    let mut places =
        Vec::with_capacity(pass1.nodes.len() + pass1.ways.len() + pass1.relations.len());
    let mut skipped = pass1.skipped;
    let mut way_lines = Vec::new();

    for draft in &pass1.nodes {
        let Some(position) = draft.position else {
            skipped += 1;
            continue;
        };
        let bbox = [position.0, position.1, position.0, position.1];
        let admin_id = admin_index.lookup(position.0, position.1, &admins);
        if let Some(place) = to_place(draft, position, bbox, admin_id) {
            places.push(place);
        } else {
            skipped += 1;
        }
    }

    for draft in &pass1.ways {
        let Some((centroid, bbox)) = way_geometry(&draft.refs, &geoms) else {
            skipped += 1;
            continue;
        };
        let admin_id = admin_index.lookup(centroid.0, centroid.1, &admins);
        if let Some(place) = to_place(draft, centroid, bbox, admin_id) {
            if place.class == "highway"
                && let Some(points) = way_line_for_reverse(&draft.refs, bbox, &geoms)
            {
                way_lines.push(WayLine {
                    place_id: place.place_id,
                    points,
                });
            }
            places.push(place);
        } else {
            skipped += 1;
        }
    }

    for draft in &pass1.relations {
        if draft.admin_level.is_some() {
            continue;
        }
        let Some((centroid, bbox)) = relation_geometry(&draft.members, &geoms) else {
            skipped += 1;
            continue;
        };
        let admin_id = admin_index.lookup(centroid.0, centroid.1, &admins);
        if let Some(place) = to_place(draft, centroid, bbox, admin_id) {
            places.push(place);
        } else {
            skipped += 1;
        }
    }

    let mut admin_places = Vec::with_capacity(admins.len());
    for admin in &admins {
        let place_id = encode_i64(3, admin.osm_id)?;
        admin_places.push(Place {
            place_id,
            kind: 3,
            osm_id: admin.osm_id as u64,
            name: admin.name.clone(),
            variants: admin.variants.clone(),
            lat_e7: admin.centroid.0,
            lon_e7: admin.centroid.1,
            importance: admin_importance(admin.level),
            admin_id: admin.parent_id,
            bbox_min_lat_e7: admin.bbox[0],
            bbox_min_lon_e7: admin.bbox[1],
            bbox_max_lat_e7: admin.bbox[2],
            bbox_max_lon_e7: admin.bbox[3],
            class: "boundary".to_string(),
            r#type: "administrative".to_string(),
        });
    }
    places.extend(admin_places);

    places.sort_by_key(|place| place.place_id);
    places.dedup_by_key(|place| place.place_id);

    let admin_count = admins.len() as u32;
    for (index, admin) in admins.iter_mut().enumerate() {
        admin.admin_id = index as u32 + 1;
    }
    let admin_map: Vec<Admin> = admins
        .iter()
        .map(|admin| Admin {
            admin_id: admin.admin_id,
            parent_id: admin.parent_id,
            level: admin.level,
            name: admin.name.clone(),
            variants: admin.variants.clone(),
            bbox_min_lat_e7: admin.bbox[0],
            bbox_min_lon_e7: admin.bbox[1],
            bbox_max_lat_e7: admin.bbox[2],
            bbox_max_lon_e7: admin.bbox[3],
        })
        .collect();

    let names = places
        .iter()
        .map(|place| 1 + place.variants.len() as u64)
        .sum();
    let counters = Counters {
        features: places.len() as u64,
        names,
        admins: admin_count as u64,
        skipped_features: skipped,
        invalid_polygons: admin_index.invalid_polygons,
        missing_node_refs: geoms.missing_nodes,
        ..Default::default()
    };
    let mut kinds = [0u64; 3];
    let mut classes = BTreeMap::new();
    for place in &places {
        if (1..=3).contains(&place.kind) {
            kinds[place.kind as usize - 1] += 1;
        }
        *classes.entry(place.class.clone()).or_insert(0) += 1;
    }
    let report = BuildReport {
        counters: counters.clone(),
        warnings: warnings.clone(),
        kinds,
        classes,
        timings: BuildTimings {
            features: features_seconds,
            way_geometry: way_geometry_seconds,
            node_coordinates: node_coordinates_seconds,
            assemble: assemble_started.elapsed().as_secs_f64(),
        },
    };
    let data = IndexData {
        places,
        admins: admin_map,
        admin_polygons: Vec::new(),
        way_lines,
        counters,
        warnings,
        ..Default::default()
    };
    Ok((data, report))
}

fn push_draft(
    pass: &mut Pass1,
    kind: Kind,
    osm_id: i64,
    info: TagInfo,
    position: Option<(i32, i32)>,
    refs: Vec<i64>,
    members: Vec<Member>,
) {
    let Some((class, place_type)) = classify(&info, kind) else {
        return;
    };
    let Some(name) = info.name.as_deref().filter(|name| !name.is_empty()) else {
        pass.skipped += 1;
        return;
    };
    if name.len() > u16::MAX as usize {
        pass.long_names += 1;
        pass.skipped += 1;
        return;
    }
    let importance = importance(&class, &place_type, &info);
    let mut variants = Vec::new();
    for (lang, value) in &info.variants {
        if value.len() > u16::MAX as usize {
            pass.long_names += 1;
            continue;
        }
        if value != name {
            variants.push((lang.clone(), value.clone()));
        }
    }
    if let Some(brand) = info.brand.as_deref().filter(|brand| *brand != name) {
        if brand.len() > u16::MAX as usize {
            pass.long_names += 1;
        } else {
            variants.push(("und".to_string(), brand.to_string()));
        }
    }
    variants.sort();
    variants.dedup();
    if variants.len() > u8::MAX as usize {
        variants.truncate(u8::MAX as usize);
        pass.long_names += 1;
    }
    let draft = Draft {
        kind,
        osm_id,
        name: name.to_string(),
        variants,
        class,
        place_type,
        importance,
        admin_level: if info.boundary.as_deref() == Some("administrative") {
            info.admin_level
        } else {
            None
        },
        position,
        refs,
        members,
    };
    match kind {
        Kind::Node => pass.nodes.push(draft),
        Kind::Way => pass.ways.push(draft),
        Kind::Relation => pass.relations.push(draft),
    }
}

fn to_place(draft: &Draft, centroid: (i32, i32), bbox: [i32; 4], admin_id: u32) -> Option<Place> {
    let place_id = encode_i64(draft.kind as u8, draft.osm_id).ok()?;
    Some(Place {
        place_id,
        kind: draft.kind as u8,
        osm_id: draft.osm_id as u64,
        name: draft.name.clone(),
        variants: draft.variants.clone(),
        lat_e7: centroid.0,
        lon_e7: centroid.1,
        importance: draft.importance,
        admin_id,
        bbox_min_lat_e7: bbox[0],
        bbox_min_lon_e7: bbox[1],
        bbox_max_lat_e7: bbox[2],
        bbox_max_lon_e7: bbox[3],
        class: draft.class.clone(),
        r#type: draft.place_type.clone(),
    })
}

fn read_tags<'a>(tags: impl Iterator<Item = (&'a str, &'a str)>) -> Option<TagInfo> {
    let mut info = TagInfo::default();
    for (key, value) in tags {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key {
            "name" => info.name = Some(value.to_string()),
            "brand" => info.brand = Some(value.to_string()),
            "boundary" => info.boundary = Some(value.to_string()),
            "admin_level" => info.admin_level = value.parse().ok(),
            "place" => info.place = Some(value.to_string()),
            "highway" => info.highway = Some(value.to_string()),
            "amenity" => info.amenity = Some(value.to_string()),
            "shop" => info.shop = Some(value.to_string()),
            "tourism" => info.tourism = Some(value.to_string()),
            "leisure" => info.leisure = Some(value.to_string()),
            "natural" => info.natural = Some(value.to_string()),
            "waterway" => info.waterway = Some(value.to_string()),
            "railway" => info.railway = Some(value.to_string()),
            "aeroway" => info.aeroway = Some(value.to_string()),
            "landuse" => info.landuse = Some(value.to_string()),
            "building" => info.building = Some(value.to_string()),
            "man_made" => info.man_made = Some(value.to_string()),
            "historic" => info.historic = Some(value.to_string()),
            "office" => info.office = Some(value.to_string()),
            "craft" => info.craft = Some(value.to_string()),
            "healthcare" => info.healthcare = Some(value.to_string()),
            "public_transport" => info.public_transport = Some(value.to_string()),
            "power" => info.power = Some(value.to_string()),
            "population" => info.population = value.replace(',', "").parse().ok(),
            "capital" => info.capital = value == "yes",
            "wikidata" => info.wikidata = true,
            _ if key.starts_with("name:") => {
                if let Some(lang) = language_code(&key[5..]) {
                    info.variants.push((lang, value.to_string()));
                }
            }
            "alt_name" | "old_name" | "official_name" | "short_name" | "loc_name" | "int_name"
            | "nat_name" | "reg_name" => {
                for part in value.split(';') {
                    let part = part.trim();
                    if !part.is_empty() {
                        info.variants.push(("und".to_string(), part.to_string()));
                    }
                }
            }
            "brand:en" | "brand:id" => {
                if let Some(lang) = language_code(&key[6..]) {
                    info.variants.push((lang, value.to_string()));
                }
            }
            _ => {}
        }
    }
    if info.name.is_none() {
        info.name = info.brand.clone();
    }
    Some(info)
}

fn language_code(value: &str) -> Option<String> {
    let cut = value.split(['-', '_']).next().unwrap_or(value);
    if !(2..=3).contains(&cut.len()) || !cut.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }
    Some(cut.to_ascii_lowercase())
}

fn classify(info: &TagInfo, kind: Kind) -> Option<(String, String)> {
    if info.boundary.as_deref() == Some("administrative") {
        let level = info.admin_level?;
        if ADMIN_LEVELS.contains(&level) {
            return Some(("boundary".to_string(), "administrative".to_string()));
        }
        return None;
    }
    let (class, value) = if let Some(value) = &info.place {
        ("place", value)
    } else if let Some(value) = &info.highway {
        ("highway", value)
    } else if let Some(value) = &info.amenity {
        ("amenity", value)
    } else if let Some(value) = &info.shop {
        ("shop", value)
    } else if let Some(value) = &info.tourism {
        ("tourism", value)
    } else if let Some(value) = &info.leisure {
        ("leisure", value)
    } else if let Some(value) = &info.natural {
        ("natural", value)
    } else if let Some(value) = &info.waterway {
        ("waterway", value)
    } else if let Some(value) = &info.railway {
        ("railway", value)
    } else if let Some(value) = &info.aeroway {
        ("aeroway", value)
    } else if let Some(value) = &info.landuse {
        ("landuse", value)
    } else if let Some(value) = &info.building {
        ("building", value)
    } else if let Some(value) = &info.man_made {
        ("man_made", value)
    } else if let Some(value) = &info.historic {
        ("historic", value)
    } else if let Some(value) = &info.office {
        ("office", value)
    } else if let Some(value) = &info.craft {
        ("craft", value)
    } else if let Some(value) = &info.healthcare {
        ("healthcare", value)
    } else if let Some(value) = &info.public_transport {
        ("public_transport", value)
    } else if let Some(value) = &info.power {
        ("power", value)
    } else {
        return None;
    };
    if kind == Kind::Way && class == "building" && value == "no" {
        return None;
    }
    Some((class.to_string(), value.to_string()))
}

fn importance(class: &str, place_type: &str, info: &TagInfo) -> f32 {
    let mut score = match class {
        "boundary" => 0.95,
        "place" => match place_type {
            "city" => 0.9,
            "town" => 0.86,
            "village" => 0.76,
            "hamlet" => 0.68,
            "suburb" => 0.72,
            "neighbourhood" => 0.68,
            "quarter" => 0.66,
            "island" => 0.7,
            "locality" => 0.62,
            _ => 0.64,
        },
        "highway" => 0.55,
        "natural" | "waterway" => 0.5,
        "railway" | "aeroway" => 0.48,
        "amenity" | "shop" | "tourism" | "leisure" | "healthcare" | "public_transport" => 0.4,
        "historic" | "office" | "craft" | "power" => 0.38,
        "man_made" | "landuse" => 0.3,
        "building" => 0.24,
        _ => 0.3,
    };
    if let Some(population) = info.population {
        let bonus = (population as f64).max(1.0).log10() as f32 / 12.0;
        score += bonus.min(0.08);
    }
    if info.capital && class == "place" {
        score += 0.06;
    }
    if info.wikidata {
        score += 0.01;
    }
    if info.brand.is_some() {
        score += 0.01;
    }
    score.clamp(0.05, 0.99)
}

fn admin_importance(level: u8) -> f32 {
    (0.92 + f32::from(7u8.saturating_sub(level)) * 0.015).min(0.99)
}

fn way_geometry(refs: &[i64], geoms: &Geoms) -> Option<((i32, i32), [i32; 4])> {
    let mut points = Vec::with_capacity(refs.len());
    for id in refs {
        points.push(geoms.coordinate(*id)?);
    }
    points.dedup();
    if points.is_empty() {
        return None;
    }
    bbox_of(&points).map(|bbox| (mean_point(&points), bbox))
}

/// Retain a simplified geometry only for ways whose bbox is long enough for
/// centroid distance to be misleading. Points stay in e7 `(lat, lon)` order.
fn way_line_for_reverse(refs: &[i64], bbox: [i32; 4], geoms: &Geoms) -> Option<Vec<(i32, i32)>> {
    let mid_lat = (bbox[0] as f64 + bbox[2] as f64) / 2.0 / 1e7;
    let lat_span = (bbox[2] - bbox[0]) as f64 / 1e7 * 111_320.0;
    let lon_span = (bbox[3] - bbox[1]) as f64 / 1e7 * 111_320.0 * mid_lat.to_radians().cos();
    if lat_span.hypot(lon_span) < 1_000.0 {
        return None;
    }
    let mut points = Vec::with_capacity(refs.len());
    for id in refs {
        let Some(point) = geoms.coordinate(*id) else {
            continue;
        };
        if points.last() != Some(&point) {
            points.push(point);
        }
    }
    if points.len() < 2 {
        return None;
    }
    if points.len() > 16 {
        let last = points.len() - 1;
        let step = last as f64 / 15.0;
        let mut simplified = Vec::with_capacity(16);
        for index in 0..15 {
            simplified.push(points[(index as f64 * step).round() as usize]);
        }
        simplified.push(points[last]);
        points = simplified;
    }
    Some(points)
}

fn relation_geometry(members: &[Member], geoms: &Geoms) -> Option<((i32, i32), [i32; 4])> {
    let mut points = Vec::new();
    for member in members {
        match member.kind {
            Kind::Node => {
                if let Some(point) = geoms.coordinate(member.id) {
                    points.push(point);
                }
            }
            Kind::Way => {
                if let Some(refs) = geoms.way_refs.get(&member.id) {
                    for id in refs {
                        if let Some(point) = geoms.coordinate(*id) {
                            points.push(point);
                        }
                    }
                }
            }
            Kind::Relation => {}
        }
    }
    points.dedup();
    if points.is_empty() {
        return None;
    }
    bbox_of(&points).map(|bbox| (mean_point(&points), bbox))
}

fn bbox_of(points: &[(i32, i32)]) -> Option<[i32; 4]> {
    let mut iter = points.iter();
    let first = *iter.next()?;
    let mut bbox = [first.0, first.1, first.0, first.1];
    for (lat, lon) in iter {
        bbox[0] = bbox[0].min(*lat);
        bbox[1] = bbox[1].min(*lon);
        bbox[2] = bbox[2].max(*lat);
        bbox[3] = bbox[3].max(*lon);
    }
    Some(bbox)
}

fn mean_point(points: &[(i32, i32)]) -> (i32, i32) {
    let mut lat = 0i128;
    let mut lon = 0i128;
    for point in points {
        lat += point.0 as i128;
        lon += point.1 as i128;
    }
    let count = points.len() as i128;
    ((lat / count) as i32, (lon / count) as i32)
}

fn build_admins(
    relations: &[Draft],
    geoms: &Geoms,
    warnings: &mut Vec<String>,
) -> (Vec<AdminDraft>, u64) {
    let mut admins = Vec::new();
    let mut invalid = 0u64;
    for draft in relations {
        let Some(level) = draft.admin_level else {
            continue;
        };
        if !ADMIN_LEVELS.contains(&level) {
            continue;
        }
        let rings = assemble_admin_rings(&draft.members, geoms);
        let Some(rings) = rings else {
            invalid += 1;
            continue;
        };
        let points: Vec<(i32, i32)> = rings.iter().flatten().copied().collect();
        let Some(bbox) = bbox_of(&points) else {
            invalid += 1;
            continue;
        };
        let area = rings.iter().map(|ring| ring_area(ring).abs()).sum::<f64>();
        if !area.is_finite() || area == 0.0 {
            invalid += 1;
            continue;
        }
        let mut centroid = mean_point(&points);
        if !point_in_rings(centroid, &rings) {
            centroid = points[0];
        }
        admins.push(AdminDraft {
            osm_id: draft.osm_id,
            level,
            name: draft.name.clone(),
            variants: draft.variants.clone(),
            rings,
            bbox,
            centroid,
            area,
            admin_id: 0,
            parent_id: 0,
        });
    }
    if invalid > 0 {
        warnings.push(format!(
            "{invalid} administrative polygons could not be assembled"
        ));
    }
    admins.sort_by_key(|admin| (admin.level, admin.osm_id));
    for (index, admin) in admins.iter_mut().enumerate() {
        admin.admin_id = index as u32 + 1;
    }
    (admins, invalid)
}

fn assemble_admin_rings(members: &[Member], geoms: &Geoms) -> Option<Vec<Vec<(i32, i32)>>> {
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    for member in members {
        if member.kind != Kind::Way {
            continue;
        }
        let refs = geoms.way_refs.get(&member.id)?;
        let mut points = Vec::new();
        for id in refs {
            points.push(geoms.coordinate(*id)?);
        }
        points.dedup();
        if points.len() < 2 {
            continue;
        }
        match member.role {
            Role::Outer => outer.push(points),
            Role::Inner => inner.push(points),
        }
    }
    let mut rings = Vec::new();
    for segments in [outer, inner] {
        let mut remaining = segments;
        while !remaining.is_empty() {
            let mut ring = remaining.remove(0);
            let mut closed = false;
            for _ in 0..=remaining.len() {
                if ring.len() >= 4 && ring.first() == ring.last() {
                    closed = true;
                    break;
                }
                let tail = *ring.last()?;
                let Some(position) = remaining.iter().position(|segment| {
                    segment.first() == Some(&tail) || segment.last() == Some(&tail)
                }) else {
                    break;
                };
                let mut segment = remaining.remove(position);
                if segment.last() == Some(&tail) {
                    segment.reverse();
                }
                if segment.first() == Some(&tail) {
                    ring.extend_from_slice(&segment[1..]);
                }
            }
            if closed && ring.len() >= 4 {
                rings.push(ring);
            } else {
                return None;
            }
        }
    }
    if rings.is_empty() { None } else { Some(rings) }
}

fn ring_area(ring: &[(i32, i32)]) -> f64 {
    if ring.len() < 4 {
        return 0.0;
    }
    let mut sum = 0.0;
    for window in ring.windows(2) {
        let (lat1, lon1) = window[0];
        let (lat2, lon2) = window[1];
        sum += lon1 as f64 * lat2 as f64 - lon2 as f64 * lat1 as f64;
    }
    sum / 2.0
}

fn point_in_rings(point: (i32, i32), rings: &[Vec<(i32, i32)>]) -> bool {
    let mut inside = false;
    for ring in rings {
        if point_in_ring(point, ring) {
            inside = !inside;
        }
    }
    inside
}

fn point_in_ring(point: (i32, i32), ring: &[(i32, i32)]) -> bool {
    let (py, px) = (point.0 as f64, point.1 as f64);
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (yi, xi) = (ring[i].0 as f64, ring[i].1 as f64);
        let (yj, xj) = (ring[j].0 as f64, ring[j].1 as f64);
        if (yi > py) != (yj > py) {
            let x = (xj - xi) * (py - yi) / (yj - yi) + xi;
            if px < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

const EDGE_BUCKET_E7: i32 = 500_000;

#[derive(Clone, Copy)]
struct Edge {
    yi: i32,
    xi: i32,
    yj: i32,
    xj: i32,
}

#[derive(Default)]
struct AdminSpatial {
    rings: Vec<HashMap<i32, Vec<Edge>>>,
}

impl AdminSpatial {
    fn new(rings: &[Vec<(i32, i32)>]) -> Self {
        let mut result = Vec::with_capacity(rings.len());
        for ring in rings {
            let mut buckets: HashMap<i32, Vec<Edge>> = HashMap::new();
            let mut previous = *ring.last().expect("non-empty ring");
            for point in ring {
                let edge = if point.0 <= previous.0 {
                    Edge {
                        yi: point.0,
                        xi: point.1,
                        yj: previous.0,
                        xj: previous.1,
                    }
                } else {
                    Edge {
                        yi: previous.0,
                        xi: previous.1,
                        yj: point.0,
                        xj: point.1,
                    }
                };
                if edge.yi != edge.yj {
                    for bucket in
                        edge.yi.div_euclid(EDGE_BUCKET_E7)..=edge.yj.div_euclid(EDGE_BUCKET_E7)
                    {
                        buckets.entry(bucket).or_default().push(edge);
                    }
                }
                previous = *point;
            }
            result.push(buckets);
        }
        Self { rings: result }
    }

    fn contains(&self, point: (i32, i32)) -> bool {
        let bucket = point.0.div_euclid(EDGE_BUCKET_E7);
        let (py, px) = (point.0 as f64, point.1 as f64);
        let mut inside = false;
        for ring in &self.rings {
            let mut ring_inside = false;
            if let Some(edges) = ring.get(&bucket) {
                for edge in edges {
                    let (yi, yj) = (edge.yi as f64, edge.yj as f64);
                    if (yi > py) != (yj > py) {
                        let x = (edge.xj - edge.xi) as f64 * (py - yi) / (yj - yi) + edge.xi as f64;
                        if px < x {
                            ring_inside = !ring_inside;
                        }
                    }
                }
            }
            if ring_inside {
                inside = !inside;
            }
        }
        inside
    }
}

struct AdminIndex {
    buckets: HashMap<(i32, i32), Vec<usize>>,
    spatial: Vec<AdminSpatial>,
    invalid_polygons: u64,
}

impl AdminIndex {
    fn new(admins: &[AdminDraft]) -> Self {
        let mut buckets: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (index, admin) in admins.iter().enumerate() {
            let min = cell(admin.bbox[0], admin.bbox[1]);
            let max = cell(admin.bbox[2], admin.bbox[3]);
            for row in min.0..=max.0 {
                for col in min.1..=max.1 {
                    buckets.entry((row, col)).or_default().push(index);
                }
            }
        }
        Self {
            buckets,
            spatial: admins
                .iter()
                .map(|admin| AdminSpatial::new(&admin.rings))
                .collect(),
            invalid_polygons: 0,
        }
    }

    fn lookup(&self, lat_e7: i32, lon_e7: i32, admins: &[AdminDraft]) -> u32 {
        let Some(candidates) = self.buckets.get(&cell(lat_e7, lon_e7)) else {
            return 0;
        };
        let mut best: Option<&AdminDraft> = None;
        for index in candidates {
            let admin = &admins[*index];
            if !self.spatial[*index].contains((lat_e7, lon_e7)) {
                continue;
            }
            let better = best.is_none_or(|current| {
                admin.level > current.level
                    || (admin.level == current.level && admin.area < current.area)
            });
            if better {
                best = Some(admin);
            }
        }
        best.map_or(0, |admin| admin.admin_id)
    }
}

fn cell(lat_e7: i32, lon_e7: i32) -> (i32, i32) {
    (lat_e7.div_euclid(10_000_000), lon_e7.div_euclid(10_000_000))
}

fn assign_parents(admins: &mut [AdminDraft], index: &AdminIndex) {
    for position in 0..admins.len() {
        let level = admins[position].level;
        if level == *ADMIN_LEVELS.start() {
            continue;
        }
        let point = admins[position].centroid;
        let mut parent: Option<usize> = None;
        if let Some(candidates) = index.buckets.get(&cell(point.0, point.1)) {
            for candidate in candidates {
                if *candidate == position {
                    continue;
                }
                let admin = &admins[*candidate];
                if admin.level >= level || !index.spatial[*candidate].contains(point) {
                    continue;
                }
                if parent.is_none_or(|current| admin.area < admins[current].area) {
                    parent = Some(*candidate);
                }
            }
        }
        if let Some(parent) = parent {
            let id = admins[parent].admin_id;
            admins[position].parent_id = id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_features_without_a_name() {
        let mut pass = Pass1::default();
        let info = read_tags(
            [
                ("building", "yes"),
                ("addr:housenumber", "17"),
                ("addr:street", "Jalan Contoh"),
            ]
            .into_iter(),
        )
        .unwrap();
        push_draft(
            &mut pass,
            Kind::Node,
            1,
            info,
            Some((-6_200_000, 106_800_000)),
            Vec::new(),
            Vec::new(),
        );
        assert!(pass.nodes.is_empty());
        assert_eq!(pass.skipped, 1);
    }

    #[test]
    fn indexes_declared_variants_and_brand() {
        let mut pass = Pass1::default();
        let info = read_tags(
            [
                ("name", "Monumen Nasional"),
                ("name:en", "National Monument"),
                ("alt_name", "Monas"),
                ("old_name", "Batavia"),
                ("official_name", "Tugu Nasional"),
                ("short_name", "Monas"),
                ("brand", "Monas Brand"),
                ("tourism", "attraction"),
            ]
            .into_iter(),
        )
        .unwrap();
        push_draft(
            &mut pass,
            Kind::Node,
            2,
            info,
            Some((-6_175_000, 106_827_000)),
            Vec::new(),
            Vec::new(),
        );
        let variants: Vec<(String, String)> = pass.nodes[0].variants.clone();
        assert!(variants.contains(&("en".to_string(), "National Monument".to_string())));
        assert!(variants.contains(&("und".to_string(), "Monas".to_string())));
        assert!(variants.contains(&("und".to_string(), "Batavia".to_string())));
        assert!(variants.contains(&("und".to_string(), "Tugu Nasional".to_string())));
        assert!(variants.contains(&("und".to_string(), "Monas Brand".to_string())));
    }

    #[test]
    fn importance_follows_feature_class() {
        let mut info = TagInfo {
            name: Some("Contoh".to_string()),
            boundary: Some("administrative".to_string()),
            admin_level: Some(4),
            ..Default::default()
        };
        let admin = importance("boundary", "administrative", &info);
        info.place = Some("city".to_string());
        let place = importance("place", "city", &info);
        info.highway = Some("primary".to_string());
        let highway = importance("highway", "primary", &info);
        info.amenity = Some("cafe".to_string());
        let poi = importance("amenity", "cafe", &info);
        info.building = Some("yes".to_string());
        let building = importance("building", "yes", &info);
        assert!(admin > place, "{admin} {place}");
        assert!(place > highway, "{place} {highway}");
        assert!(highway > poi, "{highway} {poi}");
        assert!(poi > building, "{poi} {building}");
        assert!((0.0..=1.0).contains(&building));
    }

    #[test]
    fn edge_buckets_match_brute_force_point_test() {
        let rings = vec![
            vec![
                (0, 0),
                (0, 1_000_000),
                (1_000_000, 1_000_000),
                (1_000_000, 0),
            ],
            vec![
                (300_000, 300_000),
                (300_000, 600_000),
                (600_000, 600_000),
                (600_000, 300_000),
            ],
        ];
        let spatial = AdminSpatial::new(&rings);
        for lat in (-200_000..1_200_000).step_by(100_000) {
            for lon in (-200_000..1_200_000).step_by(100_000) {
                assert_eq!(
                    spatial.contains((lat, lon)),
                    point_in_rings((lat, lon), &rings),
                    "point {lat},{lon}"
                );
            }
        }
    }

    fn admin(osm_id: i64, level: u8, size: i32, area: f64) -> AdminDraft {
        let ring = vec![(0, 0), (0, size), (size, size), (size, 0)];
        AdminDraft {
            osm_id,
            level,
            name: format!("Admin {osm_id}"),
            variants: Vec::new(),
            rings: vec![ring],
            bbox: [0, 0, size, size],
            centroid: (size / 2, size / 2),
            area,
            admin_id: 0,
            parent_id: 0,
        }
    }

    #[test]
    fn assigns_deepest_smallest_parent() {
        let mut admins = vec![
            admin(1, 4, 1_000_000, 1_000_000.0),
            admin(2, 6, 600_000, 600_000.0),
            admin(3, 5, 400_000, 400_000.0),
        ];
        for (index, admin) in admins.iter_mut().enumerate() {
            admin.admin_id = index as u32 + 1;
        }
        let index = AdminIndex::new(&admins);
        assign_parents(&mut admins, &index);
        assert_eq!(admins[1].parent_id, 3);
        assert_eq!(admins[2].parent_id, 1);
    }

    #[test]
    fn assembles_reversed_segments() {
        let mut geoms = Geoms::default();
        for (index, point) in [(0, 0), (0, 10), (10, 10), (10, 0)].iter().enumerate() {
            geoms.insert_coord(index as i64, *point);
        }
        geoms.way_refs.insert(1, vec![0, 1]);
        geoms.way_refs.insert(2, vec![1, 2]);
        geoms.way_refs.insert(3, vec![3, 2]);
        geoms.way_refs.insert(4, vec![0, 3]);
        geoms.sort_coordinates();
        let members = vec![
            Member {
                kind: Kind::Way,
                id: 1,
                role: Role::Outer,
            },
            Member {
                kind: Kind::Way,
                id: 2,
                role: Role::Outer,
            },
            Member {
                kind: Kind::Way,
                id: 3,
                role: Role::Outer,
            },
            Member {
                kind: Kind::Way,
                id: 4,
                role: Role::Outer,
            },
        ];
        let rings = assemble_admin_rings(&members, &geoms).unwrap();
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 5);
        assert_eq!(rings[0].first(), rings[0].last());
        assert!(point_in_ring((5, 5), &rings[0]));
        assert!(!point_in_ring((15, 15), &rings[0]));
    }

    #[test]
    fn classifies_and_scores() {
        let info = TagInfo {
            name: Some("Contoh".to_string()),
            place: Some("city".to_string()),
            population: Some(1_000_000),
            ..Default::default()
        };
        assert_eq!(
            classify(&info, Kind::Node),
            Some(("place".to_string(), "city".to_string()))
        );
        assert!(importance("place", "city", &info) > 0.9);
    }
}
