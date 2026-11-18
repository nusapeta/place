//! HTTP API, jsonv2 response shaping, search, reverse, and lookup.

use crate::format::FORMAT_VERSION;
use crate::manifest::Manifest;
use crate::normalize::{normalize, tokens};
use crate::place_id::{decode, encode};
use crate::reader::Index;
use axum::Json;
use axum::extract::{Query, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use fst::automaton::Str;
use fst::{Automaton, IntoStreamer, Streamer};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const LICENCE: &str = "ODbL 1.0, © OpenStreetMap contributors";
const MAX_CANDIDATES: usize = 5_000;
const MAX_QUERY_BYTES: usize = 256;
const MAX_LIMIT: usize = 20;
const MAX_RADIUS_M: f64 = 10_000.0;

#[derive(Clone, Copy, Debug)]
pub struct RankWeights {
    pub exact: f64,
    pub prefix: f64,
    pub coverage: f64,
    pub importance: f64,
    pub proximity: f64,
}

impl Default for RankWeights {
    fn default() -> Self {
        Self {
            exact: 3.0,
            prefix: 1.5,
            coverage: 1.0,
            importance: 0.5,
            proximity: 0.5,
        }
    }
}

impl RankWeights {
    /// Read weight overrides without touching the index. Invalid values fall
    /// back to the documented default.
    pub fn from_env() -> Self {
        let mut weights = Self::default();
        for (key, value) in [
            ("NUSA_PLACE_WEIGHT_EXACT", &mut weights.exact),
            ("NUSA_PLACE_WEIGHT_PREFIX", &mut weights.prefix),
            ("NUSA_PLACE_WEIGHT_COVERAGE", &mut weights.coverage),
            ("NUSA_PLACE_WEIGHT_IMPORTANCE", &mut weights.importance),
            ("NUSA_PLACE_WEIGHT_PROXIMITY", &mut weights.proximity),
        ] {
            if let Ok(parsed) = std::env::var(key).unwrap_or_default().parse::<f64>()
                && parsed.is_finite()
                && (0.0..=100.0).contains(&parsed)
            {
                *value = parsed;
            }
        }
        weights
    }
}

#[derive(Clone)]
pub struct ApiState {
    pub index: Arc<Index>,
    pub manifest: Option<Arc<Manifest>>,
    pub index_bytes: u64,
    pub metrics: metrics_exporter_prometheus::PrometheusHandle,
    pub weights: RankWeights,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    error: &'static str,
    reason: String,
}

impl ApiError {
    fn invalid(reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_request",
            reason: reason.into(),
        }
    }

    fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "unsupported_parameter",
            reason: reason.into(),
        }
    }

    fn not_found(reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: "not_found",
            reason: reason.into(),
        }
    }

    fn no_admin_polygons() -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            error: "no_admin_polygons",
            reason: "index has no admin polygon data".to_string(),
        }
    }

    fn internal(reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: "internal_error",
            reason: reason.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({"error": self.error, "reason": self.reason});
        (self.status, Json(body)).into_response()
    }
}

pub async fn middleware(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let endpoint = request.uri().path().to_string();
    let method = request.method().as_str().to_string();
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(ToOwned::to_owned)
        .unwrap_or_else(next_request_id);
    let mut response = next.run(request).await;
    let status = response.status().as_u16().to_string();
    let elapsed = started.elapsed().as_secs_f64();
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "application/json")
    {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
    }
    metrics::counter!("nusa_place_requests_total", "endpoint" => endpoint.clone(), "status" => status.clone())
        .increment(1);
    metrics::histogram!("nusa_place_request_duration_seconds", "endpoint" => endpoint.clone())
        .record(elapsed);
    tracing::info!(
        request_id = %request_id,
        method = %method,
        endpoint = %endpoint,
        status = %status,
        duration_ms = elapsed * 1000.0,
        "request"
    );
    response
}

fn next_request_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    format!("{nanos:016x}{counter:08x}")
}

pub async fn not_found() -> ApiError {
    ApiError {
        status: StatusCode::NOT_FOUND,
        error: "not_found",
        reason: "route not found".to_string(),
    }
}

pub async fn method_not_allowed() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "GET")],
        Json(json!({"error": "method_not_allowed", "reason": "only GET is supported"})),
    )
        .into_response()
}

pub async fn healthz(State(state): State<ApiState>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "index": state.index.path().file_name().and_then(|value| value.to_str()).unwrap_or("places.idx"),
        "format_version": FORMAT_VERSION,
        "manifest_sha256": state.manifest.as_ref().map(|manifest| manifest.index.sha256.clone()).unwrap_or_default(),
    }))
}

pub async fn version(State(state): State<ApiState>) -> impl IntoResponse {
    let (builder, pbf_sha256, index_sha256, built_at) = match &state.manifest {
        Some(manifest) => (
            manifest.builder.clone(),
            manifest.pbf.sha256.clone(),
            manifest.index.sha256.clone(),
            manifest.built_at.clone(),
        ),
        None => (
            format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            String::new(),
            String::new(),
            String::new(),
        ),
    };
    Json(json!({
        "server": format!("nusa-place {}", env!("CARGO_PKG_VERSION")),
        "api": "v1",
        "format_version": FORMAT_VERSION,
        "builder": builder,
        "pbf_sha256": pbf_sha256,
        "index_sha256": index_sha256,
        "built_at": built_at,
    }))
}

pub async fn metrics_handler(State(state): State<ApiState>) -> Response {
    metrics::gauge!("nusa_place_index_size_bytes").set(state.index_bytes as f64);
    metrics::gauge!("nusa_place_index_features").set(f64::from(state.index.header.feature_count));
    if let Some(manifest) = &state.manifest
        && let Ok(built_at) = time::OffsetDateTime::parse(
            &manifest.built_at,
            &time::format_description::well_known::Rfc3339,
        )
    {
        let age = (time::OffsetDateTime::now_utc() - built_at)
            .whole_seconds()
            .max(0);
        metrics::gauge!("nusa_place_index_age_seconds").set(age as f64);
    }
    let body = state.metrics.render();
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

pub async fn search(
    State(state): State<ApiState>,
    Query(query): Query<HashMap<String, String>>,
    request: Request,
) -> Result<Json<Vec<Value>>, ApiError> {
    reject_unsupported(&query)?;
    validate_format(&query)?;
    validate_addressdetails(&query)?;
    let addressdetails = query.get("addressdetails").map(String::as_str) == Some("1");
    let language = accept_language(&request);
    let q = required(&query, "q")?;
    if q.trim().is_empty() {
        return Err(ApiError::invalid("q is required"));
    }
    if q.len() > MAX_QUERY_BYTES {
        return Err(ApiError::invalid("q exceeds 256 bytes"));
    }
    let limit = parse_limit(&query)?;
    let coordinates = parse_coordinates(&query)?;
    let viewbox = parse_viewbox(&query)?;
    let bounded = query.get("bounded").map(String::as_str) == Some("1");
    if query
        .get("bounded")
        .is_some_and(|value| value != "0" && value != "1")
    {
        return Err(ApiError::invalid("bounded must be 0 or 1"));
    }
    if bounded && viewbox.is_none() {
        return Err(ApiError::invalid("bounded=1 requires viewbox"));
    }
    if let Some(countrycodes) = query.get("countrycodes")
        && countrycodes != "id"
    {
        return Err(ApiError::invalid("countrycodes hanya mendukung id"));
    }

    let rows = select_candidates(
        &state.index,
        q,
        coordinates,
        viewbox,
        bounded,
        limit,
        state.weights,
    )?;
    let results: Vec<Value> = rows
        .into_iter()
        .map(|row| result_json(&state.index, row, addressdetails, &language))
        .collect();
    metrics::counter!("nusa_place_results_total", "endpoint" => "search")
        .increment(results.len() as u64);
    Ok(Json(results))
}

pub async fn reverse(
    State(state): State<ApiState>,
    Query(query): Query<HashMap<String, String>>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    reject_unsupported(&query)?;
    validate_format(&query)?;
    validate_addressdetails(&query)?;
    let addressdetails = query.get("addressdetails").map(String::as_str) == Some("1");
    let language = accept_language(&request);
    let coordinates =
        parse_coordinates(&query)?.ok_or_else(|| ApiError::invalid("lat and lon are required"))?;
    let radius = match query.get("radius") {
        Some(value) => value
            .parse::<f64>()
            .map_err(|_| ApiError::invalid("radius must be a number"))?,
        None => 1000.0,
    };
    if !(1.0..=MAX_RADIUS_M).contains(&radius) {
        return Err(ApiError::invalid(
            "radius must be between 1 and 10000 meters",
        ));
    }
    let (distance, row) = nearest_row(&state.index, coordinates, radius)
        .ok_or_else(|| ApiError::not_found("no feature within radius"))?;
    metrics::counter!("nusa_place_results_total", "endpoint" => "reverse").increment(1);
    let mut value = result_json(&state.index, row, addressdetails, &language);
    value["distance_m"] = json!((distance * 10.0).round() / 10.0);
    Ok(Json(value))
}

/// Reverse-geocode a coordinate to the deepest admin area containing it.
pub async fn admin_reverse(
    State(state): State<ApiState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    reject_unsupported(&query)?;
    validate_format(&query)?;
    let (lat, lon) =
        parse_coordinates(&query)?.ok_or_else(|| ApiError::invalid("lat and lon are required"))?;
    if !state.index.has_admin_polygons() {
        return Err(ApiError::no_admin_polygons());
    }

    let point = (
        (lat * 10_000_000.0).round() as i32,
        (lon * 10_000_000.0).round() as i32,
    );
    let mut containing: Vec<(u32, u8)> = Vec::new();
    for admin_id in 1..=state.index.header.admin_count {
        let Some(polygon) = state.index.admin_polygon(admin_id) else {
            continue;
        };
        if point_in_polygon(point, &polygon.rings) {
            let level = state
                .index
                .admin(admin_id)
                .map(|admin| admin.level())
                .unwrap_or(0);
            containing.push((admin_id, level));
        }
    }
    if containing.is_empty() {
        return Err(ApiError::not_found("no admin area contains the point"));
    }

    let deepest = containing
        .iter()
        .map(|(_, level)| *level)
        .max()
        .unwrap_or(0);
    let selected = containing
        .iter()
        .filter(|(_, level)| *level == deepest)
        .map(|(admin_id, _)| *admin_id)
        .min()
        .unwrap_or(1);
    let ambiguous = containing
        .iter()
        .filter(|(_, level)| *level == deepest)
        .count()
        > 1;

    let mut parts = Vec::new();
    let mut address = Map::new();
    let mut admin_id = selected;
    let mut guard = 0;
    while admin_id != 0 && guard < 16 {
        let Some(admin) = state.index.admin(admin_id) else {
            break;
        };
        let name = state.index.admin_name(&admin).to_string();
        if parts.last().is_none_or(|last| last != &name) {
            parts.push(name.clone());
        }
        if let Some(key) = admin_address_key(admin.level()) {
            address.entry(key).or_insert_with(|| Value::String(name));
        }
        admin_id = admin.parent_id();
        guard += 1;
    }
    parts.push("Indonesia".to_string());
    address.insert(
        "country".to_string(),
        Value::String("Indonesia".to_string()),
    );
    address.insert("country_code".to_string(), Value::String("id".to_string()));

    let place_id = state
        .index
        .admin_polygon(selected)
        .map(|polygon| polygon.place_id)
        .unwrap_or(0);
    let (_, osm_id) = decode(place_id).unwrap_or((3, 0));
    let (lat_text, lon_text) = state
        .index
        .find_row(place_id)
        .map(|row| {
            let place = state.index.place(row);
            (coordinate(place.lat_e7), coordinate(place.lon_e7))
        })
        .unwrap_or((coordinate(point.0), coordinate(point.1)));

    metrics::counter!("nusa_place_results_total", "endpoint" => "admin_reverse").increment(1);
    Ok(Json(json!({
        "place_id": place_id,
        "licence": LICENCE,
        "osm_type": "relation",
        "osm_id": osm_id,
        "lat": lat_text,
        "lon": lon_text,
        "class": "boundary",
        "type": "administrative",
        "name": state.index.admin_name(&state.index.admin(selected).unwrap()),
        "display_name": parts.join(", "),
        "address": Value::Object(address),
        "admin_id": selected,
        "ambiguous": ambiguous,
    })))
}

pub async fn lookup(
    State(state): State<ApiState>,
    Query(query): Query<HashMap<String, String>>,
    request: Request,
) -> Result<Json<Vec<Value>>, ApiError> {
    reject_unsupported(&query)?;
    validate_format(&query)?;
    validate_addressdetails(&query)?;
    let addressdetails = query.get("addressdetails").map(String::as_str) == Some("1");
    let language = accept_language(&request);
    let osm_ids = query.get("osm_ids");
    let place_ids = query.get("place_ids");
    if osm_ids.is_some() == place_ids.is_some() {
        return Err(ApiError::invalid(
            "send exactly one of osm_ids or place_ids",
        ));
    }
    let mut ids = Vec::new();
    if let Some(value) = osm_ids {
        let mut count = 0;
        for part in value.split(',') {
            count += 1;
            if count > 50 {
                return Err(ApiError::invalid("osm_ids accepts at most 50 ids"));
            }
            let part = part.trim();
            let mut chars = part.chars();
            let prefix = chars
                .next()
                .ok_or_else(|| ApiError::invalid("empty osm id"))?;
            let number = chars.as_str();
            if number.is_empty() {
                return Err(ApiError::invalid("invalid osm id"));
            }
            let kind = match prefix.to_ascii_lowercase() {
                'n' => 1,
                'w' => 2,
                'r' => 3,
                _ => return Err(ApiError::invalid("osm_ids prefix must be N, W, or R")),
            };
            let osm_id = number
                .parse::<u64>()
                .map_err(|_| ApiError::invalid("invalid osm id"))?;
            ids.push(encode(kind, osm_id).map_err(|error| ApiError::invalid(error.to_string()))?);
        }
    } else if let Some(value) = place_ids {
        let mut count = 0;
        for part in value.split(',') {
            count += 1;
            if count > 50 {
                return Err(ApiError::invalid("place_ids accepts at most 50 ids"));
            }
            ids.push(
                part.trim()
                    .parse::<u64>()
                    .map_err(|_| ApiError::invalid("invalid place_id"))?,
            );
        }
    }
    let mut results = Vec::new();
    for place_id in ids {
        if let Some(row) = state.index.find_row(place_id) {
            results.push(result_json(&state.index, row, addressdetails, &language));
        }
    }
    metrics::counter!("nusa_place_results_total", "endpoint" => "lookup")
        .increment(results.len() as u64);
    Ok(Json(results))
}

fn reject_unsupported(query: &HashMap<String, String>) -> Result<(), ApiError> {
    const UNSUPPORTED: &[&str] = &[
        "polygon_geojson",
        "polygon_kml",
        "polygon_svg",
        "polygon_text",
        "extratags",
        "namedetails",
        "exclude_place_ids",
        "dedupe",
        "zoom",
        "geojson",
        "polygon_threshold",
    ];
    if let Some(key) = UNSUPPORTED.iter().find(|key| query.contains_key(**key)) {
        return Err(ApiError::unsupported(format!("{key} is not supported")));
    }
    Ok(())
}

fn validate_format(query: &HashMap<String, String>) -> Result<(), ApiError> {
    if let Some(value) = query.get("format")
        && value != "jsonv2"
    {
        return Err(ApiError::invalid("only format=jsonv2 is supported"));
    }
    Ok(())
}

fn validate_addressdetails(query: &HashMap<String, String>) -> Result<(), ApiError> {
    if let Some(value) = query.get("addressdetails")
        && value != "0"
        && value != "1"
    {
        return Err(ApiError::invalid("addressdetails must be 0 or 1"));
    }
    Ok(())
}

fn required<'a>(query: &'a HashMap<String, String>, key: &str) -> Result<&'a str, ApiError> {
    query
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| ApiError::invalid(format!("{key} is required")))
}

fn parse_limit(query: &HashMap<String, String>) -> Result<usize, ApiError> {
    match query.get("limit") {
        Some(value) => {
            let limit = value
                .parse::<usize>()
                .map_err(|_| ApiError::invalid("limit must be a number"))?;
            if !(1..=MAX_LIMIT).contains(&limit) {
                return Err(ApiError::invalid("limit must be between 1 and 20"));
            }
            Ok(limit)
        }
        None => Ok(5),
    }
}

fn parse_coordinates(query: &HashMap<String, String>) -> Result<Option<(f64, f64)>, ApiError> {
    match (query.get("lat"), query.get("lon")) {
        (None, None) => Ok(None),
        (Some(lat), Some(lon)) => {
            let lat = lat
                .parse::<f64>()
                .map_err(|_| ApiError::invalid("lat must be a number"))?;
            let lon = lon
                .parse::<f64>()
                .map_err(|_| ApiError::invalid("lon must be a number"))?;
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return Err(ApiError::invalid("lat or lon is out of range"));
            }
            Ok(Some((lat, lon)))
        }
        _ => Err(ApiError::invalid("lat and lon must be sent together")),
    }
}

fn parse_viewbox(query: &HashMap<String, String>) -> Result<Option<[f64; 4]>, ApiError> {
    let Some(value) = query.get("viewbox") else {
        return Ok(None);
    };
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 4 {
        return Err(ApiError::invalid(
            "viewbox needs four comma-separated numbers",
        ));
    }
    let mut values = [0.0; 4];
    for (index, part) in parts.iter().enumerate() {
        values[index] = part
            .trim()
            .parse::<f64>()
            .map_err(|_| ApiError::invalid("viewbox must contain numbers"))?;
    }
    let [min_lon, max_lat, max_lon, min_lat] = values;
    if !(-180.0..=180.0).contains(&min_lon)
        || !(-180.0..=180.0).contains(&max_lon)
        || !(-90.0..=90.0).contains(&min_lat)
        || !(-90.0..=90.0).contains(&max_lat)
        || min_lon >= max_lon
        || min_lat >= max_lat
    {
        return Err(ApiError::invalid("viewbox is out of range or empty"));
    }
    Ok(Some([min_lon, max_lat, max_lon, min_lat]))
}

fn accept_language(request: &Request) -> String {
    request
        .headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "id".to_string())
}

fn select_candidates(
    index: &Index,
    q: &str,
    coordinates: Option<(f64, f64)>,
    viewbox: Option<[f64; 4]>,
    bounded: bool,
    limit: usize,
    weights: RankWeights,
) -> Result<Vec<u32>, ApiError> {
    let normalized = normalize(q);
    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    let effective = administrative_suffix(&normalized).unwrap_or(&normalized);
    let name_fst = index
        .name_fst()
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let token_fst = index
        .token_fst()
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut rows = Vec::new();
    if let Some(row) = name_fst.get(effective) {
        rows.push(row as u32);
    }

    let query_tokens = effective_tokens(effective);
    if !query_tokens.is_empty() {
        let mut intersections: Option<Vec<u32>> = None;
        for token in &query_tokens {
            let Some(value) = token_fst.get(token) else {
                intersections = Some(Vec::new());
                break;
            };
            let Some(postings) = index.postings_for(value) else {
                intersections = Some(Vec::new());
                break;
            };
            let mut current = postings.to_vec();
            current.truncate(MAX_CANDIDATES);
            intersections = Some(match intersections {
                None => current,
                Some(previous) => intersect(&previous, &current),
            });
            if intersections.as_ref().is_some_and(Vec::is_empty) {
                break;
            }
        }
        if let Some(token_rows) = intersections {
            merge_rows(&mut rows, &token_rows);
        }
    }

    let mut prefix_rows = Vec::new();
    let automaton = Str::new(effective).starts_with();
    let mut stream = name_fst.search(automaton).into_stream();
    while let Some((_key, value)) = stream.next() {
        prefix_rows.push(value as u32);
        if prefix_rows.len() >= MAX_CANDIDATES {
            break;
        }
    }
    if let Some(token) = query_tokens.first() {
        let automaton = Str::new(token).starts_with();
        let mut stream = token_fst.search(automaton).into_stream();
        while let Some((_key, value)) = stream.next() {
            if let Some(postings) = index.postings_for(value) {
                let mut values = postings.to_vec();
                values.truncate(MAX_CANDIDATES);
                merge_rows(&mut prefix_rows, &values);
            }
            if prefix_rows.len() >= MAX_CANDIDATES {
                break;
            }
        }
    }
    merge_rows(&mut rows, &prefix_rows);
    metrics::histogram!("nusa_place_search_candidates").record(rows.len() as f64);

    let wants_regency = normalized.starts_with("kabupaten ");
    let total_features = f64::from(index.header.feature_count.max(1));
    let query_idf: Vec<f64> = query_tokens
        .iter()
        .map(|token| {
            let count = token_fst
                .get(token)
                .and_then(|value| index.postings_for(value))
                .map_or(0.0, |rows| rows.len() as f64);
            (1.0 + total_features / (count + 1.0)).ln()
        })
        .collect();
    let total_idf: f64 = query_idf.iter().sum();
    let mut scored: Vec<(f64, f32, u32)> = rows
        .into_iter()
        .filter_map(|row| {
            let place = index.place(row);
            if bounded {
                let viewbox = viewbox?;
                let [min_lon, max_lat, max_lon, min_lat] = viewbox;
                let min_lon = (min_lon * 1e7) as i32;
                let max_lon = (max_lon * 1e7) as i32;
                let min_lat = (min_lat * 1e7) as i32;
                let max_lat = (max_lat * 1e7) as i32;
                if place.bbox_max_lon_e7 < min_lon
                    || place.bbox_min_lon_e7 > max_lon
                    || place.bbox_max_lat_e7 < min_lat
                    || place.bbox_min_lat_e7 > max_lat
                {
                    return None;
                }
            }
            let mut names = vec![index.normalized_name(row)];
            names.extend(
                index
                    .place_variants(row)
                    .into_iter()
                    .map(|(_, value)| normalize(&value)),
            );
            let exact = names.iter().any(|name| name == effective);
            let prefix = names.iter().any(|name| name.starts_with(effective));
            let candidate_tokens: HashSet<String> =
                names.iter().flat_map(|name| tokens(name)).collect();
            let matched_idf: f64 = query_tokens
                .iter()
                .zip(&query_idf)
                .filter(|(token, _)| {
                    candidate_tokens.contains(*token)
                        || candidate_tokens
                            .iter()
                            .any(|value| value.starts_with(*token))
                })
                .map(|(_, idf)| idf)
                .sum();
            let coverage = if total_idf > 0.0 {
                matched_idf / total_idf
            } else {
                0.0
            };
            let mut score =
                f64::from(place.importance) * weights.importance + coverage * weights.coverage;
            if coverage >= 1.0 {
                score += weights.coverage;
            }
            if wants_regency && index.class(row) == "boundary" {
                score += 0.75;
            }
            if exact {
                score += weights.exact;
            }
            if prefix && !exact {
                score += weights.prefix;
            }
            if let Some((lat, lon)) = coordinates {
                let distance = haversine(
                    lat,
                    lon,
                    place.lat_e7 as f64 / 1e7,
                    place.lon_e7 as f64 / 1e7,
                );
                score += weights.proximity * (-distance / 25_000.0).exp();
            }
            Some((score, place.importance, row))
        })
        .collect();
    scored.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .1
                    .partial_cmp(&left.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.2.cmp(&right.2))
    });
    scored.truncate(limit);
    Ok(scored.into_iter().map(|(_, _, row)| row).collect())
}

fn merge_rows(target: &mut Vec<u32>, source: &[u32]) {
    let mut seen: HashSet<u32> = target.iter().copied().collect();
    for row in source {
        if seen.insert(*row) {
            target.push(*row);
        }
    }
}

fn administrative_suffix(input: &str) -> Option<&str> {
    let (prefix, rest) = input.split_once(' ')?;
    matches!(
        prefix,
        "kabupaten" | "kecamatan" | "kelurahan" | "provinsi" | "kec" | "kel" | "kab"
    )
    .then_some(rest)
    .filter(|rest| !rest.is_empty())
}

fn effective_tokens(input: &str) -> Vec<String> {
    tokens(input)
        .into_iter()
        .map(|token| match token.as_str() {
            "jogja" | "jogjakarta" => "yogyakarta".to_string(),
            "jkt" => "jakarta".to_string(),
            "sby" => "surabaya".to_string(),
            "jl" | "jln" => "jalan".to_string(),
            _ => token,
        })
        .collect()
}

fn intersect(left: &[u32], right: &[u32]) -> Vec<u32> {
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        match left[i].cmp(&right[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                result.push(left[i]);
                i += 1;
                j += 1;
            }
        }
    }
    result
}

fn nearest_row(index: &Index, (lat, lon): (f64, f64), radius_m: f64) -> Option<(f64, u32)> {
    let lat_e7 = (lat * 1e7).round() as i32;
    let lon_e7 = (lon * 1e7).round() as i32;
    let grid = index.grid_info();
    let radius_deg = radius_m / 111_320.0;
    let lon_deg = radius_m / (111_320.0 * lat.to_radians().cos().max(0.01));
    let min_row = ((lat_e7 as f64 - radius_deg * 1e7 - f64::from(grid.min_lat_e7))
        / f64::from(grid.cell_size_e7))
    .floor() as i64;
    let max_row = ((lat_e7 as f64 + radius_deg * 1e7 - f64::from(grid.min_lat_e7))
        / f64::from(grid.cell_size_e7))
    .ceil() as i64;
    let min_col = ((lon_e7 as f64 - lon_deg * 1e7 - f64::from(grid.min_lon_e7))
        / f64::from(grid.cell_size_e7))
    .floor() as i64;
    let max_col = ((lon_e7 as f64 + lon_deg * 1e7 - f64::from(grid.min_lon_e7))
        / f64::from(grid.cell_size_e7))
    .ceil() as i64;
    let mut best: Option<(f64, u32)> = None;
    let mut seen = HashSet::new();
    for row in min_row.max(0)..=max_row.min(i64::from(grid.rows) - 1) {
        for col in min_col.max(0)..=max_col.min(i64::from(grid.cols) - 1) {
            let key = (row as u32) * (grid.cols as u32) + col as u32;
            let Some(postings) = index.grid_postings(key) else {
                continue;
            };
            for row in postings {
                if !seen.insert(*row) {
                    continue;
                }

                let distance = place_distance_m(index, *row, lat, lon);
                if distance > radius_m {
                    continue;
                }
                let replace = match best {
                    None => true,
                    Some((best_distance, best_row)) => {
                        if distance + 25.0 < best_distance {
                            true
                        } else if (distance - best_distance).abs() <= 25.0 {
                            priority(index, *row) > priority(index, best_row)
                        } else {
                            false
                        }
                    }
                };
                if replace {
                    best = Some((distance, *row));
                }
            }
        }
    }
    best
}

fn priority(index: &Index, row: u32) -> u8 {
    match index.class(row) {
        "place" | "boundary" => 3,
        "highway" => 2,
        _ => 1,
    }
}

fn place_distance_m(index: &Index, row: u32, lat: f64, lon: f64) -> f64 {
    let place = index.place(row);
    let Some(points) = index.way_points(row) else {
        return haversine(
            lat,
            lon,
            place.lat_e7 as f64 / 1e7,
            place.lon_e7 as f64 / 1e7,
        );
    };
    let mut best = f64::INFINITY;
    for segment in points.chunks_exact(4) {
        let start = (segment[0], segment[1]);
        let end = (segment[2], segment[3]);
        best = best.min(segment_distance_m(lat, lon, start, end));
    }
    best
}

fn segment_distance_m(lat: f64, lon: f64, start: (i32, i32), end: (i32, i32)) -> f64 {
    let scale = 111_320.0 * lat.to_radians().cos().max(0.01);
    let (ax, ay) = (
        (start.1 as f64 / 1e7 - lon) * scale,
        (start.0 as f64 / 1e7 - lat) * 111_320.0,
    );
    let (bx, by) = (
        (end.1 as f64 / 1e7 - lon) * scale,
        (end.0 as f64 / 1e7 - lat) * 111_320.0,
    );
    let dx = bx - ax;
    let dy = by - ay;
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 {
        return ax.hypot(ay);
    }
    let t = ((-ax * dx - ay * dy) / length_squared).clamp(0.0, 1.0);
    (ax + t * dx).hypot(ay + t * dy)
}

fn point_in_ring(point: (i32, i32), ring: &[i32]) -> bool {
    let count = ring.len() / 2;
    if count < 3 {
        return false;
    }
    let (lat, lon) = (f64::from(point.0), f64::from(point.1));
    let mut inside = false;
    let mut previous = count - 1;
    for current in 0..count {
        let (current_lat, current_lon) = (
            f64::from(ring[current * 2]),
            f64::from(ring[current * 2 + 1]),
        );
        let (previous_lat, previous_lon) = (
            f64::from(ring[previous * 2]),
            f64::from(ring[previous * 2 + 1]),
        );
        if (current_lat > lat) != (previous_lat > lat)
            && lon
                < previous_lon
                    + (lat - current_lat) * (previous_lon - current_lon)
                        / (previous_lat - current_lat)
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn point_in_polygon(point: (i32, i32), rings: &[&[i32]]) -> bool {
    let Some(outer) = rings.first() else {
        return false;
    };
    point_in_ring(point, outer) && !rings.iter().skip(1).any(|hole| point_in_ring(point, hole))
}

fn admin_address_key(level: u8) -> Option<&'static str> {
    match level {
        4 => Some("state"),
        5 => Some("city"),
        6 => Some("county"),
        7 => Some("village"),
        _ => None,
    }
}

fn result_json(index: &Index, row: u32, addressdetails: bool, language: &str) -> Value {
    let place = index.place(row);
    let name = localized_name(index, row, language);
    let (_, osm_id) = decode(place.place_id)
        .map_err(|_| ())
        .unwrap_or((place.kind, 0));
    let (osm_type, osm_id) = kind(place.kind, osm_id);
    let mut value = json!({
        "place_id": place.place_id,
        "licence": LICENCE,
        "osm_type": osm_type,
        "osm_id": osm_id,
        "lat": coordinate(place.lat_e7),
        "lon": coordinate(place.lon_e7),
        "class": index.class(row),
        "type": index.place_type(row),
        "name": name,
        "display_name": display_name(index, row, &name),
        "importance": place.importance,
        "boundingbox": [
            coordinate(place.bbox_min_lat_e7),
            coordinate(place.bbox_max_lat_e7),
            coordinate(place.bbox_min_lon_e7),
            coordinate(place.bbox_max_lon_e7),
        ],
    });
    if addressdetails {
        value["address"] = address(index, row, &name);
    }
    value
}

fn localized_name(index: &Index, row: u32, language: &str) -> String {
    let place = index.place(row);
    let main = index.str_at(place.name_off, place.name_len).to_string();
    let language = language.split('-').next().unwrap_or(language);
    for (lang, value) in index.place_variants(row) {
        if lang.eq_ignore_ascii_case(language) {
            return value;
        }
    }
    main
}

fn display_name(index: &Index, row: u32, name: &str) -> String {
    let place = index.place(row);
    let mut parts = vec![name.to_string()];
    let mut admin_id = place.admin_id;
    let mut guard = 0;
    while admin_id != 0 && guard < 16 {
        if let Some(admin) = index.admin(admin_id) {
            let admin_name = index.admin_name(&admin);
            if parts
                .last()
                .is_none_or(|last| !last.eq_ignore_ascii_case(admin_name))
            {
                parts.push(admin_name.to_string());
            }
            admin_id = admin.parent_id();
        } else {
            break;
        }
        guard += 1;
    }
    if parts.last().is_none_or(|last| last != "Indonesia") {
        parts.push("Indonesia".to_string());
    }
    parts.join(", ")
}

fn address(index: &Index, row: u32, name: &str) -> Value {
    let place = index.place(row);
    let mut map = Map::new();
    if index.class(row) == "highway" {
        map.insert("road".to_string(), Value::String(name.to_string()));
    } else if index.class(row) == "place" {
        match index.place_type(row) {
            "city" => {
                map.insert("city".to_string(), Value::String(name.to_string()));
            }
            "town" | "village" | "hamlet" => {
                map.insert("city".to_string(), Value::String(name.to_string()));
            }
            "suburb" | "neighbourhood" | "quarter" => {
                map.insert("suburb".to_string(), Value::String(name.to_string()));
            }
            _ => {}
        }
    }
    let mut admin_id = place.admin_id;
    let mut guard = 0;
    while admin_id != 0 && guard < 16 {
        if let Some(admin) = index.admin(admin_id) {
            let admin_name = index.admin_name(&admin);
            match admin.level() {
                4 => {
                    map.entry("state".to_string())
                        .or_insert_with(|| Value::String(admin_name.to_string()));
                }
                5..=7 => {
                    map.entry("city".to_string())
                        .or_insert_with(|| Value::String(admin_name.to_string()));
                }
                _ => {}
            }
            admin_id = admin.parent_id();
        } else {
            break;
        }
        guard += 1;
    }
    map.insert(
        "country".to_string(),
        Value::String("Indonesia".to_string()),
    );
    map.insert("country_code".to_string(), Value::String("id".to_string()));
    Value::Object(map)
}

fn kind(kind: u8, osm_id: u64) -> (&'static str, u64) {
    let name = match kind {
        1 => "node",
        2 => "way",
        3 => "relation",
        _ => "unknown",
    };
    (name, osm_id)
}

fn coordinate(value: i32) -> String {
    format!("{:.7}", value as f64 / 1e7)
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_008.8;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().clamp(0.0, 1.0).asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersects_sorted_rows() {
        assert_eq!(intersect(&[1, 3, 5], &[2, 3, 5, 7]), vec![3, 5]);
    }

    fn fixture_index() -> (tempfile::TempDir, Index) {
        let dir = tempfile::TempDir::new().unwrap();
        let (data, _report) =
            crate::build::build_data(std::path::Path::new("tests/fixtures/mini.pbf")).unwrap();
        let path = dir.path().join("mini.idx");
        crate::writer::write_index(&data, &path).unwrap();
        let index = Index::open(&path, false).unwrap();
        (dir, index)
    }

    fn rows(index: &Index, q: &str) -> Vec<String> {
        select_candidates(index, q, None, None, false, 10, RankWeights::default())
            .unwrap()
            .into_iter()
            .map(|row| index.normalized_name(row).to_string())
            .collect()
    }

    #[test]
    fn ranks_exact_then_prefix_and_keeps_duplicates() {
        let (_dir, index) = fixture_index();
        assert_eq!(rows(&index, "indomaret"), vec!["indomaret", "indomaret"]);
        assert_eq!(rows(&index, "indom"), vec!["indomaret", "indomaret"]);
        let jalan = rows(&index, "jalan merdeka");
        assert_eq!(jalan.first().map(String::as_str), Some("jalan merdeka"));
        assert_eq!(jalan.len(), 2);
        assert!(jalan.contains(&"jalan panjang".to_string()));
        assert_eq!(
            rows(&index, "jakarta").first().map(String::as_str),
            Some("jakarta")
        );
    }

    #[test]
    fn candidate_selection_is_deterministic() {
        let (_dir, index) = fixture_index();
        for q in ["jalan", "indomaret", "jak", "kota contoh"] {
            assert_eq!(rows(&index, q), rows(&index, q));
        }
    }

    #[test]
    fn haversine_known_distance() {
        assert!((haversine(-6.2, 106.8, -6.2, 106.9) - 11_090.0).abs() < 200.0);
    }

    #[test]
    fn segment_distance_measures_perpendicular_offset() {
        let distance = segment_distance_m(
            -6.201,
            106.9,
            (-62_000_000, 1_068_000_000),
            (-62_000_000, 1_069_500_000),
        );
        assert!((distance - 111.3).abs() < 5.0);
    }

    #[test]
    fn reverse_uses_polyline_distance_for_long_road() {
        let (_dir, index) = fixture_index();
        let (distance, row) = nearest_row(&index, (-6.2, 106.94), 1_000.0).unwrap();
        assert_eq!(index.normalized_name(row), "jalan panjang");
        assert!(distance < 50.0, "unexpected distance {distance}");
    }

    #[test]
    fn admin_polygons_round_trip() {
        let (_dir, index) = fixture_index();
        assert!(index.has_admin_polygons());
        let polygons: Vec<_> = (1..=index.header.admin_count)
            .filter_map(|admin_id| index.admin_polygon(admin_id))
            .collect();
        assert_eq!(polygons.len(), index.header.admin_count as usize);
        assert!(
            polygons
                .iter()
                .all(|polygon| !polygon.rings.is_empty() && polygon.rings[0].len() >= 6)
        );
    }

    #[test]
    fn no_admin_polygons_error_is_not_implemented() {
        let error = ApiError::no_admin_polygons();
        assert_eq!(error.status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(error.error, "no_admin_polygons");
    }
}
