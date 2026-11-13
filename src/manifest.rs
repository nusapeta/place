//! Deterministic, atomic manifest for a built index.

use crate::build::BuildReport;
use crate::model::WriteSummary;
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct Counts {
    pub features: u64,
    pub admins: u64,
    pub names: u64,
    pub postings: u64,
    pub skipped_features: u64,
    pub invalid_polygons: u64,
    #[serde(default)]
    pub missing_node_refs: u64,
    pub nodes: u64,
    pub ways: u64,
    pub relations: u64,
    pub classes: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct Manifest {
    pub format_version: u16,
    pub builder: String,
    pub built_at: String,
    pub pbf: FileInfo,
    pub index: FileInfo,
    pub counts: Counts,
    pub bbox: [f64; 4],
    pub warnings: Vec<String>,
}

pub fn write_manifest(
    path: &Path,
    pbf: &Path,
    summary: &WriteSummary,
    report: &BuildReport,
) -> Result<Manifest> {
    let pbf_bytes = fs::metadata(pbf)
        .with_context(|| format!("stat {}", pbf.display()))?
        .len();
    let pbf_sha = file_sha256(pbf)?;
    let manifest = Manifest {
        format_version: crate::format::FORMAT_VERSION,
        builder: format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        built_at: rfc3339_now(),
        pbf: FileInfo {
            name: file_name(pbf),
            sha256: pbf_sha,
            bytes: pbf_bytes,
        },
        index: FileInfo {
            name: file_name(&summary.index_path),
            sha256: summary.sha256.clone(),
            bytes: summary.bytes,
        },
        counts: Counts {
            features: u64::from(summary.feature_count),
            admins: u64::from(summary.admin_count),
            names: report.counters.names,
            postings: summary.posting_count,
            skipped_features: report.counters.skipped_features,
            invalid_polygons: report.counters.invalid_polygons,
            missing_node_refs: report.counters.missing_node_refs,
            nodes: report.kinds[0],
            ways: report.kinds[1],
            relations: report.kinds[2],
            classes: report.classes.clone(),
        },
        bbox: summary.bbox.map(|value| value as f64 / 1e7),
        warnings: report.warnings.clone(),
    };
    write_atomic(path, &serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn file_sha256(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = temporary_path(path);
    fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .with_context(|| format!("rename {} to {}", tmp.display(), path.display()))?;
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".tmp");
    PathBuf::from(value)
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

fn rfc3339_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's civil-from-days algorithm, proleptic Gregorian.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
    }

    #[test]
    fn failed_manifest_write_preserves_previous_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("index.idx.json");
        fs::write(&path, b"previous manifest").unwrap();
        let summary = WriteSummary {
            index_path: dir.path().join("index.idx"),
            bytes: 128,
            sha256: "00".repeat(32),
            feature_count: 2,
            admin_count: 1,
            class_count: 2,
            type_count: 2,
            posting_count: 3,
            string_bytes: 32,
            bbox: [-800_000_000, 100_000_000, -700_000_000, 110_000_000],
        };
        let report = BuildReport {
            counters: crate::model::Counters::default(),
            warnings: vec![],
            timings: Default::default(),
            kinds: [0; 3],
            classes: BTreeMap::new(),
        };
        let missing = dir.path().join("missing.osm.pbf");
        assert!(write_manifest(&path, &missing, &summary, &report).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"previous manifest");
        assert!(!dir.path().join("index.idx.json.tmp").exists());
    }

    #[test]
    fn atomic_write_replaces_and_removes_temporary_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("manifest.json");
        fs::write(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!temporary_path(&path).exists());
    }

    #[test]
    fn hex_is_lowercase() {
        assert_eq!(hex(&[0x00, 0xab, 0xff]), "00abff");
    }
}
