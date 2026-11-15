# Contributing

Nusapeta Place is an OSM geocoder for Indonesia: one Rust crate, one mmap index
file, no database. Small contributions matter, especially missing place-name
reports and wrong reverse results.

## Before changing code

1. Read [docs/README.md](docs/README.md) and the document related to your
   change.
2. Check requirement `FR-xxx`/`NFR-xxx` in [docs/SRS.md](docs/SRS.md) and
   [docs/BACKLOG.md](docs/BACKLOG.md).
3. Index format, `place_id` derivation, or HTTP contract changes need an issue
   discussion and an ADR in [docs/decisions/](docs/decisions/).
4. Do not edit measurements in `docs/BENCHMARKS.md` or `docs/releases/`; those
   are snapshots. Explanation fixes belong in other documents.
5. Keep route-engine changes in the Route repository.

## Setup

The toolchain is pinned in `rust-toolchain.toml`: Rust 1.99.1 with `rustfmt`
and `clippy`.

```bash
git clone https://github.com/nusapeta/place.git
cd place
rustup show
cargo build --release --bins
cargo test --all-features
```

Commands used by CI:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo audit
cargo deny check licenses
python3 scripts/check_consistency.py
```

## Number rules

Every pull request that changes search or reverse quality MUST include:

- the query or coordinate under test;
- the PBF/index used (hash from the manifest);
- before and after output, not just "better";
- for performance claims: machine, commit, and reproduction command.

Numbers without a machine, commit, and command are rejected. NFR claims follow
[docs/BENCHMARKS.md](docs/BENCHMARKS.md); do not change frozen numbers.

## Code

- `cargo fmt` is required; do not reformat files you did not touch.
- Every behavior bug needs a regression test.
- `unsafe` is limited to mmap and fixed-size record reads. Every `unsafe` block
  needs a reason comment and a test.
- Do not add a dependency without a reason. New dependencies MUST pass
  `cargo deny check licenses` and be recorded in
  [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
- Do not commit private domains, LAN IPs, tokens, PBFs, or indexes. Use
  `place.example.com` and `127.0.0.1`.

## Pull request

- [ ] `cargo fmt`, `cargo clippy`, and `cargo test` pass.
- [ ] Quality changes include query + index + before/after output.
- [ ] API or format documentation is updated when the contract changes.
- [ ] No PBF, index, or large data file is included.
- [ ] Commits carry `Signed-off-by` (DCO).

Use imperative commit messages that name the area, for example
`search: cap prefix expansion at 64 terms`.

## DCO

The repository uses Developer Certificate of Origin 1.1, with no CLA. Every
commit MUST include a sign-off line:

```text
Signed-off-by: Your Name <you@example.com>
```

Add it with `git commit -s`. Contributions without a sign-off are not merged.

## License

By submitting a contribution, you agree to license it under MIT. OSM data and
derived indexes remain ODbL 1.0; see
[docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
