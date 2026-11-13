//! Print index counts used by Indonesia build acceptance checks.

use fst::Streamer;
use placeid::Index;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: inspect_index INDEX");
    let index = Index::open(&path, true)?;
    let mut classes = BTreeMap::new();
    let mut types = BTreeMap::new();
    let mut variants = 0usize;
    let mut named = 0usize;
    let mut importance_min = f32::MAX;
    let mut importance_max = f32::MIN;
    for row in 0..index.header.feature_count {
        let place = index.place(row);
        *classes.entry(index.class(row).to_string()).or_insert(0u64) += 1;
        *types
            .entry(index.place_type(row).to_string())
            .or_insert(0u64) += 1;
        variants += index.place_variants(row).len();
        if !index.normalized_name(row).is_empty() {
            named += 1;
        }
        importance_min = importance_min.min(place.importance);
        importance_max = importance_max.max(place.importance);
    }
    let mut name_keys = 0u64;
    let name_fst = index.name_fst()?;
    let mut stream = name_fst.stream();
    while stream.next().is_some() {
        name_keys += 1;
    }
    let mut token_keys = 0u64;
    let token_fst = index.token_fst()?;
    let mut stream = token_fst.stream();
    while stream.next().is_some() {
        token_keys += 1;
    }
    let admin_levels: BTreeMap<u8, u64> = (1..=index.header.admin_count)
        .filter_map(|number| index.admin(number))
        .fold(BTreeMap::new(), |mut levels, admin| {
            *levels.entry(admin.level()).or_insert(0) += 1;
            levels
        });
    println!(
        "features={} named={} admins={} name_keys={} token_keys={} postings={} variants={} grid_cells={} bbox=[{},{},{},{}] importance=[{:.3},{:.3}]",
        index.header.feature_count,
        named,
        index.header.admin_count,
        name_keys,
        token_keys,
        index.header.posting_count,
        variants,
        index.grid_info().cell_count,
        index.header.min_lat_e7,
        index.header.min_lon_e7,
        index.header.max_lat_e7,
        index.header.max_lon_e7,
        importance_min,
        importance_max,
    );
    println!("classes={classes:?}");
    println!("types={types:?}");
    println!("admin_levels={admin_levels:?}");
    Ok(())
}
