//! In-memory model shared by the builder and the index writer.

use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct Place {
    pub place_id: u64,
    pub kind: u8,
    pub osm_id: u64,
    pub name: String,
    pub variants: Vec<(String, String)>,
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub importance: f32,
    pub admin_id: u32,
    pub bbox_min_lat_e7: i32,
    pub bbox_min_lon_e7: i32,
    pub bbox_max_lat_e7: i32,
    pub bbox_max_lon_e7: i32,
    pub class: String,
    pub r#type: String,
}

#[derive(Clone, Debug, Default)]
pub struct Admin {
    pub admin_id: u32,
    pub parent_id: u32,
    pub level: u8,
    pub name: String,
    pub variants: Vec<(String, String)>,
    pub bbox_min_lat_e7: i32,
    pub bbox_min_lon_e7: i32,
    pub bbox_max_lat_e7: i32,
    pub bbox_max_lon_e7: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Counters {
    pub features: u64,
    pub admins: u64,
    pub names: u64,
    pub postings: u64,
    pub skipped_features: u64,
    pub invalid_polygons: u64,
    pub missing_node_refs: u64,
}

#[derive(Clone, Debug, Default)]
pub struct IndexData {
    pub places: Vec<Place>,
    pub admins: Vec<Admin>,
    pub counters: Counters,
    pub warnings: Vec<String>,
    pub min_lat_e7: i32,
    pub min_lon_e7: i32,
    pub max_lat_e7: i32,
    pub max_lon_e7: i32,
}

#[derive(Clone, Debug)]
pub struct WriteSummary {
    pub index_path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    pub feature_count: u32,
    pub admin_count: u32,
    pub class_count: u32,
    pub type_count: u32,
    pub posting_count: u64,
    pub string_bytes: u64,
    pub bbox: [i32; 4],
}
