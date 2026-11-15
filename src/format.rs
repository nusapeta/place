//! Core data structures for the nusapeta-place index format v1.
//!
//! The on-disk layout is documented in `docs/INDEX-FORMAT.md`. All integers
//! are little-endian. Records are fixed-size and contain no pointers.

use bytemuck::{Pod, Zeroable};

/// On-disk format identifier, not a brand: indexes built before the nusapeta
/// rename still start with it, so it only changes with a format version bump.
pub const MAGIC: [u8; 8] = *b"PLACEID\0";
pub const FORMAT_VERSION: u16 = 1;
pub const HEADER_BYTES: usize = 256;
pub const SECTION_COUNT: usize = 12;
pub const SECTION_TABLE_OFFSET: usize = 64;
pub const SECTION_ENTRY_BYTES: usize = 16;
pub const TRAILER_BYTES: usize = 32;
pub const PLACE_BYTES: usize = 64;
pub const ADMIN_BYTES: usize = 40;
pub const VARIANT_BYTES: usize = 12;
pub const GRID_HEADER_BYTES: usize = 40;
pub const GRID_ENTRY_BYTES: usize = 12;
pub const DICT_HEADER_BYTES: usize = 8;
pub const DICT_ENTRY_BYTES: usize = 8;
pub const MAX_OSM_ID: u64 = 1 << 51;

pub const SECTION_PLACES: usize = 0;
pub const SECTION_CLASSES: usize = 1;
pub const SECTION_TYPES: usize = 2;
pub const SECTION_STRINGS: usize = 3;
pub const SECTION_VARIANTS: usize = 4;
pub const SECTION_TOKEN_FST: usize = 5;
pub const SECTION_POSTINGS: usize = 6;
pub const SECTION_NAME_FST: usize = 7;
pub const SECTION_GRID: usize = 8;
pub const SECTION_ADMIN: usize = 9;
pub const SECTION_POLYGONS: usize = 10;
pub const SECTION_RESERVED: usize = 11;
pub const SECTION_NAMES: [&str; SECTION_COUNT] = [
    "places",
    "classes",
    "types",
    "strings",
    "variants",
    "token_fst",
    "postings",
    "name_fst",
    "grid",
    "admin",
    "polygons",
    "reserved",
];

pub const FLAG_ADMIN_POLYGONS: u16 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PlaceRecord {
    pub place_id: u64,
    pub name_off: u32,
    pub name_len: u16,
    pub kind: u8,
    pub flags: u8,
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub importance: f32,
    pub admin_id: u32,
    pub bbox_min_lat_e7: i32,
    pub bbox_min_lon_e7: i32,
    pub bbox_max_lat_e7: i32,
    pub bbox_max_lon_e7: i32,
    pub class_id: u16,
    pub type_id: u16,
    pub names_off: u32,
    pub names_count: u8,
    pub reserved: [u8; 7],
}

/// Admin records are unaligned on disk (`names_off` starts at byte 18), so
/// they are kept as bytes and decoded with explicit little-endian accessors.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct AdminRecord(pub [u8; ADMIN_BYTES]);

impl Default for AdminRecord {
    fn default() -> Self {
        Self([0; ADMIN_BYTES])
    }
}

impl AdminRecord {
    pub fn admin_id(&self) -> u32 {
        get_u32(&self.0, 0)
    }
    pub fn parent_id(&self) -> u32 {
        get_u32(&self.0, 4)
    }
    pub fn level(&self) -> u8 {
        self.0[8]
    }
    pub fn name_off(&self) -> u32 {
        get_u32(&self.0, 12)
    }
    pub fn name_len(&self) -> u16 {
        get_u16(&self.0, 16)
    }
    pub fn names_off(&self) -> u32 {
        get_u32(&self.0, 18)
    }
    pub fn names_count(&self) -> u8 {
        self.0[22]
    }
    pub fn bbox_min_lat_e7(&self) -> i32 {
        get_i32(&self.0, 24)
    }
    pub fn bbox_min_lon_e7(&self) -> i32 {
        get_i32(&self.0, 28)
    }
    pub fn bbox_max_lat_e7(&self) -> i32 {
        get_i32(&self.0, 32)
    }
    pub fn bbox_max_lon_e7(&self) -> i32 {
        get_i32(&self.0, 36)
    }
}

pub fn get_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub fn get_i32(bytes: &[u8], offset: usize) -> i32 {
    get_u32(bytes, offset) as i32
}

pub fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    let mut value = [0u8; 8];
    value.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(value)
}

pub fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    put_u32(bytes, offset, value as u32);
}

pub fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct VariantRecord {
    pub lang: [u8; 3],
    pub pad0: u8,
    pub str_off: u32,
    pub str_len: u16,
    pub pad1: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct DictRecord {
    pub str_off: u32,
    pub str_len: u16,
    pub pad: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GridEntry {
    pub key: u32,
    pub posting_off: u32,
    pub posting_count: u32,
}

const _: () = assert!(size_of::<PlaceRecord>() == PLACE_BYTES);
const _: () = assert!(size_of::<AdminRecord>() == ADMIN_BYTES);
const _: () = assert!(size_of::<VariantRecord>() == VARIANT_BYTES);
const _: () = assert!(size_of::<DictRecord>() == DICT_ENTRY_BYTES);
const _: () = assert!(size_of::<GridEntry>() == GRID_ENTRY_BYTES);
const _: () = assert!(SECTION_TABLE_OFFSET + SECTION_COUNT * SECTION_ENTRY_BYTES == HEADER_BYTES);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Section {
    pub offset: u64,
    pub len: u64,
}

#[derive(Clone, Debug)]
pub struct Header {
    pub flags: u16,
    pub feature_count: u32,
    pub admin_count: u32,
    pub class_count: u32,
    pub type_count: u32,
    pub min_lat_e7: i32,
    pub min_lon_e7: i32,
    pub max_lat_e7: i32,
    pub max_lon_e7: i32,
    pub string_bytes: u64,
    pub posting_count: u64,
    pub sections: [Section; SECTION_COUNT],
}

impl Default for Header {
    fn default() -> Self {
        Self {
            flags: 0,
            feature_count: 0,
            admin_count: 0,
            class_count: 0,
            type_count: 0,
            min_lat_e7: 0,
            min_lon_e7: 0,
            max_lat_e7: 0,
            max_lon_e7: 0,
            string_bytes: 0,
            posting_count: 0,
            sections: [Section::default(); SECTION_COUNT],
        }
    }
}

pub fn align8(value: usize) -> usize {
    (value + 7) & !7
}

pub fn kind_name(kind: u8) -> Option<&'static str> {
    match kind {
        1 => Some("node"),
        2 => Some("way"),
        3 => Some("relation"),
        _ => None,
    }
}

pub fn kind_code(name: &str) -> Option<u8> {
    match name.to_ascii_lowercase().as_str() {
        "n" | "node" => Some(1),
        "w" | "way" => Some(2),
        "r" | "relation" => Some(3),
        _ => None,
    }
}
