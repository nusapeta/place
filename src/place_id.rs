//! Stable 53-bit place identifiers.
//!
//! `place_id = (type_code << 51) | osm_id`, with node=1, way=2, relation=3.
//! Keeping the result below 2^53 makes it safe to parse as a JavaScript
//! number. See `docs/decisions/0003-place-id.md`.

use crate::format::MAX_OSM_ID;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceIdError {
    UnknownKind(String),
    NegativeOsmId(i64),
    OsmIdTooLarge(u64),
    InvalidBits(u64),
}

impl fmt::Display for PlaceIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKind(value) => write!(f, "unknown OSM kind: {value}"),
            Self::NegativeOsmId(value) => write!(f, "negative OSM id: {value}"),
            Self::OsmIdTooLarge(value) => {
                write!(f, "OSM id {value} does not fit in 51 bits")
            }
            Self::InvalidBits(value) => write!(f, "invalid place_id bits: {value}"),
        }
    }
}

impl std::error::Error for PlaceIdError {}

pub fn encode(kind: u8, osm_id: u64) -> Result<u64, PlaceIdError> {
    if !(1..=3).contains(&kind) {
        return Err(PlaceIdError::UnknownKind(kind.to_string()));
    }
    if osm_id >= MAX_OSM_ID {
        return Err(PlaceIdError::OsmIdTooLarge(osm_id));
    }
    Ok(((kind as u64) << 51) | osm_id)
}

pub fn encode_i64(kind: u8, osm_id: i64) -> Result<u64, PlaceIdError> {
    if osm_id < 0 {
        return Err(PlaceIdError::NegativeOsmId(osm_id));
    }
    encode(kind, osm_id as u64)
}

pub fn decode(place_id: u64) -> Result<(u8, u64), PlaceIdError> {
    let kind = (place_id >> 51) as u8;
    if !(1..=3).contains(&kind) {
        return Err(PlaceIdError::InvalidBits(place_id));
    }
    let osm_id = place_id & (MAX_OSM_ID - 1);
    Ok((kind, osm_id))
}

pub fn encode_text(kind: &str, osm_id: u64) -> Result<u64, PlaceIdError> {
    let kind = crate::format::kind_code(kind)
        .ok_or_else(|| PlaceIdError::UnknownKind(kind.to_string()))?;
    encode(kind, osm_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_all_kinds() {
        assert_eq!(encode(1, 123).unwrap(), (1u64 << 51) | 123);
        assert_eq!(encode(2, 123).unwrap(), (2u64 << 51) | 123);
        assert_eq!(encode(3, 123).unwrap(), (3u64 << 51) | 123);
    }

    #[test]
    fn round_trips_limit() {
        let id = encode(3, MAX_OSM_ID - 1).unwrap();
        assert_eq!(id, (3u64 << 51) | (MAX_OSM_ID - 1));
        assert!(id < (1u64 << 53));
        assert_eq!(decode(id).unwrap(), (3, MAX_OSM_ID - 1));
    }

    #[test]
    fn rejects_overflow_and_bad_kind() {
        assert!(matches!(
            encode(1, MAX_OSM_ID),
            Err(PlaceIdError::OsmIdTooLarge(_))
        ));
        assert!(matches!(encode(0, 1), Err(PlaceIdError::UnknownKind(_))));
        assert!(matches!(encode(4, 1), Err(PlaceIdError::UnknownKind(_))));
        assert!(matches!(decode(0), Err(PlaceIdError::InvalidBits(_))));
    }

    #[test]
    fn rejects_negative_and_high_bits() {
        assert!(matches!(
            encode_i64(1, -1),
            Err(PlaceIdError::NegativeOsmId(-1))
        ));
        assert!(matches!(
            decode(1u64 << 63),
            Err(PlaceIdError::InvalidBits(_))
        ));
        assert!(matches!(
            decode(4u64 << 51),
            Err(PlaceIdError::InvalidBits(_))
        ));
    }

    #[test]
    fn accepts_every_boundary_osm_id() {
        for kind in 1..=3 {
            for osm_id in [0, 1, MAX_OSM_ID - 1] {
                assert_eq!(
                    decode(encode(kind, osm_id).unwrap()).unwrap(),
                    (kind, osm_id)
                );
            }
        }
    }

    #[test]
    fn is_stable_across_rebuilds() {
        let first = encode(2, 987_654).unwrap();
        let second = encode(2, 987_654).unwrap();
        assert_eq!(first, second);
    }
}
