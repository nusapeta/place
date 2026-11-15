# Security

## Reporting a problem

Do not open a public issue for an exploitable vulnerability. Use the GitHub
private security advisory at
<https://github.com/nusapeta/place/security/advisories/new>. Include:

- the commit version;
- reproduction steps;
- possible impact;
- whether the problem is reachable over HTTP or only through a corrupt index.

An acknowledgement follows within 7 days, then a fix and release note once the
problem is handled. There is no fix-date promise or SLA.

## Threat model

| Asset | Threat | Mitigation |
|---|---|---|
| Server process | Hostile HTTP input | Length, range, and type validation |
| Index | Corrupt or modified file | Magic, version, section bounds, checksum |
| Parser | Out-of-bounds offset | Strict validation, limited `unsafe`, fuzzing |
| Dependencies | Hostile or vulnerable crate | `cargo audit`, dependency review |
| Container | Write access or root | Read-only, non-root, no shell |
| Data | OSM data misuse | ODbL attribution, no query storage |

## Boundaries

`nusapeta-place` does not handle authentication, TLS, or rate limiting. The
gateway owns those. Do not expose the `nusapeta-place` port directly to the
internet.

## What counts as a vulnerability

- A remote crash or panic from a valid HTTP request.
- An out-of-bounds read from an index opened with checksum verification.
- Code execution from PBF or index input.
- Filesystem path or stack trace leakage in a response.
- Denial of service that request limits cannot bound.

## What does not count

- Wrong search results caused by incomplete OSM data.
- Latency outside target on hardware that does not meet the specification.
- A client sending an unsupported `format` and receiving 400.
- A `place_id` different from Nominatim. That is a documented design
  difference.

## Dependencies

- Pin versions in `Cargo.lock`.
- Run `cargo audit` before release.
- Avoid unmaintained crates or large `unsafe` surfaces without a reason.
- Record new dependencies in the pull request.

## Data and privacy

- The service does not store queries, coordinates, or results.
- Default logs do not contain query text or coordinates.
- Metrics do not use queries as labels.
- PBFs and indexes never enter git.
