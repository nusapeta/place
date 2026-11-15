//! Build a format v1 index from an OSM PBF file.

use anyhow::{Context, Result, bail};
use clap::Parser;
use nusa_place::build::build_data;
use nusa_place::writer::write_index;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "nusa-place-build",
    version,
    about = "Build a nusapeta-place index from OpenStreetMap PBF"
)]
struct Args {
    /// Input OSM PBF file.
    #[arg(long, short)]
    input: PathBuf,
    /// Output index path.
    #[arg(long, short)]
    output: PathBuf,
    /// Optional manifest output path.
    #[arg(long)]
    manifest: Option<PathBuf>,
    /// Verify the written index before returning.
    #[arg(long)]
    verify: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.input == args.output {
        bail!("input and output must differ");
    }
    eprintln!("reading {}", args.input.display());
    let (data, report) = build_data(&args.input)?;
    eprintln!(
        "selected {} features, {} admins, skipped {} features, {} missing refs",
        report.counters.features,
        report.counters.admins,
        report.counters.skipped_features,
        report.counters.missing_node_refs
    );
    let timings = report.timings;
    eprintln!(
        "timings features={:.2}s way_geometry={:.2}s node_coordinates={:.2}s assemble={:.2}s",
        timings.features, timings.way_geometry, timings.node_coordinates, timings.assemble
    );
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    let summary = write_index(&data, &args.output)
        .with_context(|| format!("write {}", args.output.display()))?;
    eprintln!(
        "wrote {} ({} bytes, sha256 {}, {} features, {} admins)",
        summary.index_path.display(),
        summary.bytes,
        summary.sha256,
        summary.feature_count,
        summary.admin_count
    );
    if let Some(manifest) = args.manifest {
        nusa_place::manifest::write_manifest(&manifest, &args.input, &summary, &report)
            .with_context(|| format!("write manifest {}", manifest.display()))?;
        eprintln!("wrote {}", manifest.display());
    }
    if args.verify {
        let index = nusa_place::Index::open(&args.output, true)?;
        eprintln!(
            "verified {} features and checksum",
            index.header.feature_count
        );
    }
    Ok(())
}
