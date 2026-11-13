//! Measure reverse-geocoding cell and candidate counts from a built index.

use placeid::Index;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: grid_spike INDEX");
    let index = Index::open(&path, false)?;
    let info = index.grid_info();
    let mut occupancy = Vec::new();
    for row in 0..info.rows {
        for col in 0..info.cols {
            let key = row as u32 * info.cols as u32 + col as u32;
            if let Some(postings) = index.grid_postings(key)
                && !postings.is_empty()
            {
                occupancy.push((row, col, postings.len()));
            }
        }
    }
    let mut sample = Vec::new();
    let step = (index.header.feature_count as usize / 200).max(1);
    for row in (0..index.header.feature_count).step_by(step).take(200) {
        let place = index.place(row);
        sample.push((place.lat_e7, place.lon_e7));
    }
    println!(
        "{{\"occupied_cells\":{},\"cells_total\":{},\"sampled\":{},\"radii\":{{",
        occupancy.len(),
        info.rows as i64 * info.cols as i64,
        sample.len()
    );
    for (number, radius) in [1_000.0f64, 10_000.0].into_iter().enumerate() {
        let mut cells = Vec::new();
        let mut candidates = Vec::new();
        for (lat, lon) in &sample {
            let lat_delta = (radius / 111_320.0 * 1e7) as i32;
            let lon_delta = (radius
                / (111_320.0 * (*lat as f64 / 1e7).to_radians().cos().max(0.01))
                * 1e7) as i32;
            let min_row = ((lat - lat_delta - info.min_lat_e7) / info.cell_size_e7).max(0);
            let max_row =
                ((lat + lat_delta - info.min_lat_e7) / info.cell_size_e7).min(info.rows - 1);
            let min_col = ((lon - lon_delta - info.min_lon_e7) / info.cell_size_e7).max(0);
            let max_col =
                ((lon + lon_delta - info.min_lon_e7) / info.cell_size_e7).min(info.cols - 1);
            let cell_count = ((max_row - min_row + 1) * (max_col - min_col + 1)).max(0) as usize;
            let mut found = 0usize;
            for (row, col, count) in &occupancy {
                if *row >= min_row && *row <= max_row && *col >= min_col && *col <= max_col {
                    found += count;
                }
            }
            cells.push(cell_count);
            candidates.push(found);
        }
        cells.sort_unstable();
        candidates.sort_unstable();
        if number > 0 {
            print!(",");
        }
        println!(
            "\"{radius:.0}\":{{\"mean_cells\":{:.2},\"p95_cells\":{},\"max_cells\":{},\"mean_candidates\":{:.2},\"p95_candidates\":{},\"max_candidates\":{}}}",
            cells.iter().sum::<usize>() as f64 / cells.len() as f64,
            cells[cells.len() * 95 / 100 - 1],
            cells[cells.len() - 1],
            candidates.iter().sum::<usize>() as f64 / candidates.len() as f64,
            candidates[candidates.len() * 95 / 100 - 1],
            candidates[candidates.len() - 1]
        );
    }
    println!("}}}}");
    Ok(())
}
