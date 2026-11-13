//! Compare absolute u32 postings with delta varint encoding.

use fst::Streamer;
use placeid::Index;
use std::time::Instant;

fn varint(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: postings_spike INDEX");
    let index = Index::open(&path, false)?;
    let fst = index.token_fst()?;
    let mut stream = fst.stream();
    let mut absolute: Vec<u8> = Vec::new();
    let mut delta = Vec::new();
    let mut boundaries = Vec::new();
    let mut postings = 0u64;
    while let Some((_key, value)) = stream.next() {
        let rows = index.postings_for(value).unwrap();
        postings += rows.len() as u64;
        absolute.extend_from_slice(bytemuck::cast_slice(rows));
        let mut previous = 0u32;
        for (position, row) in rows.iter().enumerate() {
            if position == 0 {
                varint(u64::from(*row), &mut delta);
            } else {
                varint(u64::from(row - previous), &mut delta);
            }
            previous = *row;
        }
        boundaries.push(delta.len());
    }
    let started = Instant::now();
    let mut sink = 0u64;
    for _ in 0..20 {
        let mut cursor = 0;
        for &end in &boundaries {
            let mut previous = 0u32;
            let mut first = true;
            while cursor < end {
                let mut value = 0u64;
                let mut shift = 0;
                loop {
                    let byte = delta[cursor];
                    cursor += 1;
                    value |= u64::from(byte & 0x7f) << shift;
                    if byte & 0x80 == 0 {
                        break;
                    }
                    shift += 7;
                }
                let row = if first {
                    value as u32
                } else {
                    previous + value as u32
                };
                sink = sink.wrapping_add(u64::from(row));
                previous = row;
                first = false;
            }
        }
    }
    let decode_ns = started.elapsed().as_nanos() as f64 / (postings as f64 * 20.0);
    println!(
        "postings={postings} absolute_bytes={} delta_varint_bytes={} ratio={:.3} decode_ns_per_posting={decode_ns:.2} checksum={sink}",
        absolute.len(),
        delta.len(),
        absolute.len() as f64 / delta.len() as f64
    );
    Ok(())
}
