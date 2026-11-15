//! Compare prefix, Levenshtein automaton, and SymSpell recall on typos.

use fst::automaton::Levenshtein;
use fst::{Automaton, IntoStreamer, Streamer};
use nusa_place::Index;
use std::collections::HashMap;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: fuzzy_spike INDEX");
    let index = Index::open(&path, false)?;
    let fst = index.token_fst()?;
    let mut tokens = Vec::new();
    let mut stream = fst.stream();
    while let Some((key, value)) = stream.next() {
        let frequency = index.postings_for(value).map_or(0, |rows| rows.len());
        tokens.push((String::from_utf8_lossy(key).to_string(), frequency));
    }
    tokens.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let alphabet: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(_, (token, _))| {
            token.len() >= 6 && token.chars().all(|ch| ch.is_ascii_alphabetic())
        })
        .map(|(index, _)| index)
        .take(5_000)
        .collect();
    let mut queries = Vec::new();
    for (number, index) in alphabet
        .iter()
        .step_by(alphabet.len() / 35)
        .take(35)
        .enumerate()
    {
        let target = &tokens[*index].0;
        let mut bytes = target.as_bytes().to_vec();
        match number % 3 {
            0 => {
                bytes.remove(bytes.len() / 2);
            }
            1 => {
                let middle = bytes.len() / 2;
                bytes[middle] = b'x';
            }
            _ => {
                let middle = bytes.len() / 2;
                bytes.swap(middle, middle - 1);
            }
        }
        queries.push((String::from_utf8(bytes).unwrap(), target.clone()));
    }
    for (alias, target) in [
        ("jogja", "yogyakarta"),
        ("solo", "surakarta"),
        ("menmon", "monumen nasional"),
        ("brom", "bromo"),
        ("tob", "toba"),
        ("sudirman", "jenderal sudirman"),
        ("malioboro", "malioboro"),
        ("soeta", "soekarno hatta"),
        ("juand", "juanda"),
        ("ngurah", "ngurah rai"),
    ] {
        queries.push((alias.to_string(), target.to_string()));
    }

    let mut deletes: HashMap<String, Vec<usize>> = HashMap::new();
    for (number, (token, _)) in tokens.iter().enumerate().take(50_000) {
        let bytes = token.as_bytes();
        for position in 0..bytes.len() {
            let mut value = bytes.to_vec();
            value.remove(position);
            deletes
                .entry(String::from_utf8_lossy(&value).to_string())
                .or_default()
                .push(number);
        }
    }

    let mut prefix_hits = 0;
    let mut lev1_hits = 0;
    let mut lev2_hits = 0;
    let mut sym_hits = 0;
    let mut lev2_latencies = Vec::new();
    let mut lev1_latencies = Vec::new();
    let mut prefix_latencies = Vec::new();
    let mut sym_latencies = Vec::new();
    for (query, expected) in &queries {
        let started = Instant::now();
        let automaton = Levenshtein::new(query, 2).unwrap();
        let mut stream = fst.search(automaton).into_stream();
        let mut candidates = Vec::new();
        while let Some((key, value)) = stream.next() {
            candidates.push((
                String::from_utf8_lossy(key).to_string(),
                index.postings_for(value).map_or(0, |rows| rows.len()),
            ));
        }
        candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        candidates.truncate(5);
        lev2_latencies.push(started.elapsed().as_secs_f64() * 1000.0);
        if candidates.iter().any(|(token, _)| token == expected) {
            lev2_hits += 1;
        }
        let started = Instant::now();
        let mut lev1_candidates = Vec::new();
        let automaton = Levenshtein::new(query, 1).unwrap();
        let mut stream = fst.search(automaton).into_stream();
        while let Some((key, value)) = stream.next() {
            lev1_candidates.push((
                String::from_utf8_lossy(key).to_string(),
                index.postings_for(value).map_or(0, |rows| rows.len()),
            ));
        }
        lev1_candidates
            .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        lev1_candidates.truncate(5);
        lev1_latencies.push(started.elapsed().as_secs_f64() * 1000.0);
        if lev1_candidates.iter().any(|(token, _)| token == expected) {
            lev1_hits += 1;
        }
        let started = Instant::now();
        let mut prefix_candidates = Vec::new();
        let automaton = fst::automaton::Str::new(query).starts_with();
        let mut stream = fst.search(automaton).into_stream();
        while let Some((key, value)) = stream.next() {
            prefix_candidates.push((
                String::from_utf8_lossy(key).to_string(),
                index.postings_for(value).map_or(0, |rows| rows.len()),
            ));
        }
        prefix_candidates
            .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        prefix_candidates.truncate(5);
        prefix_latencies.push(started.elapsed().as_secs_f64() * 1000.0);
        if prefix_candidates.iter().any(|(token, _)| token == expected) {
            prefix_hits += 1;
        }
        let started = Instant::now();
        let mut sym_candidates = Vec::new();
        if let Some(rows) = deletes.get(query) {
            for number in rows.iter().take(20) {
                sym_candidates.push((tokens[*number].0.clone(), tokens[*number].1));
            }
        }
        sym_candidates
            .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        sym_candidates.truncate(5);
        sym_latencies.push(started.elapsed().as_secs_f64() * 1000.0);
        if sym_candidates.iter().any(|(token, _)| token == expected) {
            sym_hits += 1;
        }
    }
    lev2_latencies.sort_by(f64::total_cmp);
    lev1_latencies.sort_by(f64::total_cmp);
    prefix_latencies.sort_by(f64::total_cmp);
    sym_latencies.sort_by(f64::total_cmp);
    println!(
        "queries={} prefix_recall_at_5={prefix_hits} prefix_p95_ms={:.3} lev1_recall_at_5={lev1_hits} lev1_p95_ms={:.3} lev2_recall_at_5={lev2_hits} lev2_p95_ms={:.3} symspell1_recall_at_5={sym_hits} symspell1_p95_ms={:.3} tokens={}",
        queries.len(),
        prefix_latencies[prefix_latencies.len() * 95 / 100 - 1],
        lev1_latencies[lev1_latencies.len() * 95 / 100 - 1],
        lev2_latencies[lev2_latencies.len() * 95 / 100 - 1],
        sym_latencies[sym_latencies.len() * 95 / 100 - 1],
        tokens.len()
    );
    Ok(())
}
