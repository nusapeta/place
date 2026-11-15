# Summary

What does this change?

## Type

- [ ] Bug fix
- [ ] Feature
- [ ] Documentation
- [ ] Data or index
- [ ] CI or build

## Evidence

For search or reverse quality changes, include the query/coordinate, index
hash, and before/after output.

```text
query= index_sha256=
before= after=
```

For performance claims, include machine, commit, and reproduction command.

## Checklist

- [ ] `cargo fmt`, `cargo clippy`, `cargo test` pass.
- [ ] `cargo audit` and `cargo deny check licenses` pass.
- [ ] A regression test exists for the behavior bug.
- [ ] Contract documentation is updated when needed.
- [ ] No PBF, index, or large data file is included.
- [ ] Commits carry `Signed-off-by` (DCO).
