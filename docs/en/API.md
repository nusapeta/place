# Nusapeta Place API

| Field | Value |
|---|---|
| Status | Draft |
| Contract version | v1 |
| Format | `jsonv2` |
| Base URL | `http://host:8090` on the internal network |
| Related documents | [SRS](../SRS.md), [TRD](../TRD.md) |

The Indonesian original is [`../API.md`](../API.md). If the two disagree, the
Indonesian file is canonical.

## 1. Principles

This API is a subset of Nominatim `jsonv2`, not a full implementation. Clients
using common fields such as `place_id`, `osm_type`, `osm_id`, `lat`, `lon`,
`display_name`, `address`, and `boundingbox` can migrate with small changes.
Unsupported parameters must be visible, not silently ignored.

Every successful response uses `Content-Type: application/json; charset=utf-8`.
Coordinates are decimal strings. `place_id` and `osm_id` are numbers.

## 2. Endpoints

| Method | Path | Function |
|---|---|---|
| GET | `/search` | Forward search by name |
| GET | `/reverse` | Nearest feature to a coordinate |
| GET | `/lookup` | Fetch features by ID |
| GET | `/healthz` | Index status and server readiness |
| GET | `/version` | Server and manifest versions |
| GET | `/metrics` | Prometheus metrics |

There are no write endpoints. Non-GET methods return 405 with `Allow: GET`.

## 3. Common parameters

| Parameter | Values | Default | Note |
|---|---|---|---|
| `format` | `jsonv2` | `jsonv2` | Other formats return 400 |
| `addressdetails` | `0`, `1` | `0` | `1` adds an `address` object |
| `accept-language` | Header, e.g. `id,en` | `id` | Selects a name variant when present |

Unknown parameters are ignored for Nominatim client tolerance. Known but
unsupported parameters return 400 with `error=unsupported_parameter`. Examples:
`polygon_geojson`, `extratags`, `namedetails`.

## 4. `GET /search`

### 4.1 Parameters

| Parameter | Required | Range | Default | Note |
|---|---|---|---|---|
| `q` | Yes | 1 to 256 bytes | | Search text |
| `limit` | No | 1 to 20 | 5 | Maximum results |
| `lat`, `lon` | No | -90..90, -180..180 | | Must be sent together |
| `viewbox` | No | `min_lon,max_lat,max_lon,min_lat` | | Area bias |
| `bounded` | No | `0`, `1` | `0` | `1` requires `viewbox` |
| `countrycodes` | No | `id` | `id` | Other values return 400 |

`viewbox` uses Nominatim order: left, top, right, bottom. Example:
`110.35,-7.78,110.40,-7.82`.

### 4.2 Behavior

- Empty or whitespace-only `q` returns 400.
- Results are ordered by score. Ties use descending `importance`, then
  ascending `place_id`.
- `limit` truncates after ranking. v1 has no pagination.
- `lat`/`lon` bias distance, not filter. `bounded=1` filters with `viewbox`.
- `display_name` ends with `Indonesia` for the Indonesian index.
- A query with no candidates returns an empty array `[]`.

### 4.3 Example

```http
GET /search?q=malioboro&lat=-7.7956&lon=110.3695&limit=2&addressdetails=1
```

```json
[
  {
    "place_id": 4503599627493952,
    "licence": "ODbL 1.0, © OpenStreetMap contributors",
    "osm_type": "way",
    "osm_id": 123456,
    "lat": "-7.7926000",
    "lon": "110.3658000",
    "class": "highway",
    "type": "pedestrian",
    "name": "Jalan Malioboro",
    "display_name": "Jalan Malioboro, Yogyakarta, Daerah Istimewa Yogyakarta, Indonesia",
    "importance": 0.82,
    "boundingbox": ["-7.8000", "-7.7850", "110.3600", "110.3720"],
    "address": {
      "road": "Jalan Malioboro",
      "city": "Yogyakarta",
      "state": "Daerah Istimewa Yogyakarta",
      "country": "Indonesia",
      "country_code": "id"
    }
  }
]
```

The example is illustrative. `place_id`, `osm_id`, coordinates, and
`importance` are not build output.

## 5. `GET /reverse`

### 5.1 Parameters

| Parameter | Required | Range | Default | Note |
|---|---|---|---|---|
| `lat` | Yes | -90..90 | | Latitude |
| `lon` | Yes | -180..180 | | Longitude |
| `radius` | No | 1 to 10000 meters | 1000 | Search radius |
| `addressdetails` | No | `0`, `1` | `0` | Same as search |

`zoom` and geometry parameters are not supported in v1.

### 5.2 Behavior

- Returns one nearest feature inside the radius.
- If no feature exists, returns 404 with `error=not_found`.
- Distance uses haversine. Feature-type priority applies only when distances
  differ by less than 25 meters.
- There is no house-number interpolation.

### 5.3 Example

```http
GET /reverse?lat=-7.7956&lon=110.3695&radius=500&addressdetails=1
```

```json
{
  "place_id": 4503599627493952,
  "licence": "ODbL 1.0, © OpenStreetMap contributors",
  "osm_type": "way",
  "osm_id": 123456,
  "lat": "-7.7926000",
  "lon": "110.3658000",
  "class": "highway",
  "type": "pedestrian",
  "name": "Jalan Malioboro",
  "display_name": "Jalan Malioboro, Yogyakarta, Daerah Istimewa Yogyakarta, Indonesia",
  "importance": 0.82,
  "boundingbox": ["-7.8000", "-7.7850", "110.3600", "110.3720"],
  "address": {
    "road": "Jalan Malioboro",
    "city": "Yogyakarta",
    "state": "Daerah Istimewa Yogyakarta",
    "country": "Indonesia",
    "country_code": "id"
  }
}
```

## 6. `GET /lookup`

### 6.1 Parameters

| Parameter | Required | Range | Note |
|---|---|---|---|
| `osm_ids` | One of | At most 50 IDs | Example `N123,W456,R789` |
| `place_ids` | One of | At most 50 IDs | Extension, numeric list |
| `addressdetails` | No | `0`, `1` | Same as search |

Only one of `osm_ids` or `place_ids` may be sent. If both are sent, the server
returns 400.

### 6.2 Behavior

- The result is an array containing only found IDs.
- Result order follows request order.
- Missing IDs are omitted, not treated as errors.
- Prefixes `N`, `W`, `R` are case-insensitive.

### 6.3 Example

```http
GET /lookup?osm_ids=N123,W456&addressdetails=0
```

```json
[
  {
    "place_id": 2251799813685371,
    "licence": "ODbL 1.0, © OpenStreetMap contributors",
    "osm_type": "node",
    "osm_id": 123,
    "lat": "-7.8000000",
    "lon": "110.3700000",
    "class": "place",
    "type": "village",
    "name": "Contoh",
    "display_name": "Contoh, Yogyakarta, Indonesia",
    "importance": 0.40,
    "boundingbox": ["-7.8100", "-7.7900", "110.3600", "110.3800"]
  }
]
```

## 7. Operational endpoints

### 7.1 `/healthz`

```json
{
  "status": "ok",
  "index": "places.idx",
  "format_version": 1,
  "manifest_sha256": "abc123..."
}
```

Status is 200 only when the index is mapped and the header is valid. Status is
503 when the index is not ready. This endpoint does not search.

### 7.2 `/version`

```json
{
  "server": "nusa-place 0.1.0",
  "api": "v1",
  "format_version": 1,
  "builder": "nusa-place-build 0.1.0",
  "pbf_sha256": "def456...",
  "index_sha256": "abc123...",
  "built_at": "2026-11-13T00:00:00Z"
}
```

### 7.3 `/metrics`

Prometheus text format. Minimum metrics:

| Metric | Type | Labels |
|---|---|---|
| `nusa_place_requests_total` | counter | `endpoint`, `status` |
| `nusa_place_request_duration_seconds` | histogram | `endpoint` |
| `nusa_place_results_total` | counter | `endpoint` |
| `nusa_place_index_size_bytes` | gauge | |
| `nusa_place_index_features` | gauge | |

There are no `q`, `lat`, `lon`, or `place_id` labels.

## 8. Errors

Shape:

```json
{
  "error": "invalid_request",
  "reason": "q is required"
}
```

| Status | `error` | When |
|---|---|---|
| 400 | `invalid_request` | Missing parameter, wrong type, out of range |
| 400 | `unsupported_parameter` | Known but unsupported parameter |
| 404 | `not_found` | Reverse found no feature |
| 405 | `method_not_allowed` | Method other than GET |
| 413 | `payload_too_large` | Body exceeds the limit |
| 429 | `rate_limited` | Limited by the gateway |
| 503 | `index_unavailable` | Index not ready |
| 500 | `internal_error` | Unexpected error, no internal details |

`X-Request-ID` is always returned. For 429 and 503, include `Retry-After` in
seconds.

## 9. Nominatim compatibility

| Nominatim feature | Nusapeta Place v1 |
|---|---|
| `format=jsonv2` | Supported |
| `q`, `limit`, `lat`, `lon`, `viewbox`, `bounded` | Supported with limits |
| `addressdetails` | Supported |
| `accept-language` | Supported for `id` and `en` |
| `polygon_geojson` | Not supported, 400 |
| `extratags`, `namedetails` | Not supported, 400 |
| `exclude_place_ids` | Not supported, 400 |
| `countrycodes` | Only `id` |
| `dedupe` | Not supported, 400 |
| `place_id` | Stable from `(osm_type, osm_id)`, not Nominatim's `place_id` |
| `osm_ids` | Can be used to migrate from Nominatim |
| `/status` | Absent; use `/healthz` and `/version` |
| `/deletable` | Absent |

Response fields absent from `nusapeta-place` must not be assumed empty. Clients
must check the fields they use.

## 10. Attribution and license

Every result contains:

```text
licence: "ODbL 1.0, © OpenStreetMap contributors"
```

Attribution must also be visible in documentation and in any UI that displays
results. OSM data is ODbL 1.0. `nusapeta-place` code is MIT.

## 11. Limits and versioning

- `q` is at most 256 bytes.
- `limit` is at most 20.
- `radius` is at most 10 km.
- `osm_ids` or `place_ids` is at most 50.
- There is no pagination.
- Additive response changes do not bump the API version.
- Breaking changes require a new path and a deprecation period.
- `/version` reports the API version and index format.
