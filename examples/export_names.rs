//! Export normalized names and importance for engine spikes.

use placeid::Index;
use std::io::{BufWriter, Write};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: export_names INDEX");
    let index = Index::open(&path, false)?;
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for row in 0..index.header.feature_count {
        let place = index.place(row);
        let name = index.normalized_name(row);
        if name.is_empty() {
            continue;
        }
        writeln!(out, "{name}\t{}", place.importance)?;
    }
    Ok(())
}
