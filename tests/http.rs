//! End-to-end HTTP tests against the real `nusa-place` binary.

use nusa_place::build::build_data;
use nusa_place::manifest::write_manifest;
use nusa_place::writer::write_index;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|error| {
            panic!(
                "response is not JSON: {error}: {}",
                String::from_utf8_lossy(&self.body)
            )
        })
    }
}

fn request(port: u16, method: &str, path: &str) -> Response {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    )
    .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let head = String::from_utf8_lossy(&raw[..split]);
    let body = raw[split + 4..].to_vec();
    let mut lines = head.lines();
    let status = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();
    Response {
        status,
        headers,
        body,
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn build_fixture(dir: &Path) -> (PathBuf, PathBuf) {
    let pbf = Path::new("tests/fixtures/mini.pbf");
    let (data, report) = build_data(pbf).unwrap();
    let index = dir.join("mini.idx");
    let summary = write_index(&data, &index).unwrap();
    let manifest = dir.join("mini.idx.json");
    write_manifest(&manifest, pbf, &summary, &report).unwrap();
    (index, manifest)
}

fn start(index: &Path, manifest: &Path) -> Server {
    let port = free_port();
    let child = Command::new(env!("CARGO_BIN_EXE_nusa-place"))
        .arg("--index")
        .arg(index)
        .arg("--manifest")
        .arg(manifest)
        .arg("--bind")
        .arg(format!("127.0.0.1:{port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let server = Server { child, port };
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return server;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("server did not start");
}

fn fixture_server() -> (Server, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let (index, manifest) = build_fixture(dir.path());
    let server = start(&index, &manifest);
    (server, dir)
}

#[test]
fn all_endpoints_serve_the_fixture() {
    let (server, _dir) = fixture_server();
    let port = server.port;

    let health = request(port, "GET", "/healthz");
    assert_eq!(health.status, 200);
    assert_eq!(
        health.header("content-type"),
        Some("application/json; charset=utf-8")
    );
    assert!(health.header("x-request-id").is_some());
    assert_eq!(health.json()["status"], "ok");
    assert_eq!(health.json()["format_version"], 2);

    let version = request(port, "GET", "/version");
    assert_eq!(version.status, 200);
    assert_eq!(version.json()["api"], "v1");
    assert_eq!(version.json()["format_version"], 2);
    assert!(version.json()["index_sha256"].as_str().unwrap().len() == 64);

    let search = request(port, "GET", "/search?q=indomaret&limit=5");
    assert_eq!(search.status, 200);
    let results = search.json();
    let rows = results.as_array().unwrap();
    assert!(rows.len() >= 2, "duplicate name recall failed: {results}");
    let mut ids: Vec<u64> = rows
        .iter()
        .map(|row| row["place_id"].as_u64().unwrap())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), rows.len(), "duplicate place_id in response");
    assert!(
        rows.iter()
            .all(|row| row["licence"].as_str().unwrap().contains("ODbL 1.0"))
    );

    let first = &rows[0];
    for field in [
        "place_id",
        "licence",
        "osm_type",
        "osm_id",
        "lat",
        "lon",
        "class",
        "type",
        "name",
        "display_name",
        "importance",
        "boundingbox",
    ] {
        assert!(!first[field].is_null(), "missing {field}");
    }
    assert!(first["lat"].is_string());
    assert!(first["osm_id"].is_number());

    let addressed = request(port, "GET", "/search?q=jakarta&addressdetails=1");
    assert_eq!(addressed.status, 200);
    let address = &addressed.json()[0]["address"];
    assert_eq!(address["country"], "Indonesia");
    assert_eq!(address["country_code"], "id");

    let jalan = request(port, "GET", "/search?q=jalan&limit=3");
    assert_eq!(jalan.status, 200);
    assert!(jalan.json().as_array().unwrap().len() >= 2);

    let reverse = request(port, "GET", "/reverse?lat=-6.2&lon=106.816");
    assert_eq!(reverse.status, 200);
    assert_eq!(reverse.json()["name"], "Jalan Panjang");
    assert!(reverse.json()["lat"].is_string());
    assert!(reverse.json()["place_id"].is_number());
    assert!(reverse.json()["distance_m"].as_f64().unwrap() < 10.0);

    let lookup = request(port, "GET", "/lookup?osm_ids=N1,W100");
    assert_eq!(lookup.status, 200);
    let found = lookup.json();
    assert_eq!(found[0]["osm_type"], "node");
    assert_eq!(found[0]["osm_id"], 1);
    assert_eq!(found[1]["osm_type"], "way");
    assert_eq!(found[1]["osm_id"], 100);

    let node_id = nusa_place::place_id::encode(1, 1).unwrap();
    let way_id = nusa_place::place_id::encode(2, 100).unwrap();
    let by_place = request(
        port,
        "GET",
        &format!("/lookup?place_ids={node_id},{way_id},999999999999999999"),
    );
    assert_eq!(by_place.status, 200);
    let ordered = by_place.json();
    assert_eq!(ordered.as_array().unwrap().len(), 2);
    assert_eq!(ordered[0]["place_id"], node_id);
    assert_eq!(ordered[1]["place_id"], way_id);

    let too_many = (0..51)
        .map(|id| format!("N{id}"))
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        request(port, "GET", &format!("/lookup?osm_ids={too_many}")).status,
        400
    );

    let metrics = request(port, "GET", "/metrics");
    assert_eq!(metrics.status, 200);
    assert!(
        metrics
            .header("content-type")
            .unwrap()
            .starts_with("text/plain")
    );
    let text = String::from_utf8_lossy(&metrics.body);
    assert!(text.contains("nusa_place_requests_total"));
    assert!(text.contains("nusa_place_index_features 13"));
}

#[test]
fn invalid_inputs_return_json_errors_and_allow_header() {
    let (server, _dir) = fixture_server();
    let port = server.port;

    for path in [
        "/search",
        "/search?q=",
        "/search?q=jakarta&limit=21",
        "/reverse?lat=91&lon=0",
        "/reverse?lat=-6.2&lon=106.8&radius=0",
        "/reverse?lat=-6.2&lon=106.8&radius=10001",
        "/lookup?osm_ids=N1&place_ids=N2",
        "/lookup?osm_ids=",
    ] {
        let response = request(port, "GET", path);
        assert_eq!(response.status, 400, "{path}");
        let body = response.json();
        assert!(body["error"].is_string(), "{path}: {body}");
        assert!(body["reason"].is_string(), "{path}: {body}");
        assert!(response.header("x-request-id").is_some(), "{path}");
    }

    let long_road = request(port, "GET", "/reverse?lat=-6.2&lon=106.875&radius=1000");
    assert_eq!(long_road.status, 200);
    assert_eq!(long_road.json()["name"], "Jalan Panjang");
    assert_eq!(long_road.json()["osm_id"], 101);
    let long_road_end = request(port, "GET", "/reverse?lat=-6.2&lon=106.94&radius=1000");
    assert_eq!(long_road_end.status, 200);
    assert_eq!(long_road_end.json()["name"], "Jalan Panjang");
    assert!(long_road_end.json()["distance_m"].as_f64().unwrap() < 10.0);

    let missing = request(port, "GET", "/reverse?lat=-7.0&lon=107.0&radius=1000");
    assert_eq!(missing.status, 404);
    assert_eq!(missing.json()["error"], "not_found");

    let method = request(port, "POST", "/search?q=jakarta");
    assert_eq!(method.status, 405);
    assert_eq!(method.header("allow"), Some("GET"));

    let unknown = request(port, "GET", "/nope");
    assert_eq!(unknown.status, 404);
    assert_eq!(unknown.json()["error"], "not_found");
}

#[test]
fn admin_reverse_returns_hierarchy_and_errors() {
    let (server, _dir) = fixture_server();
    let port = server.port;

    let response = request(port, "GET", "/place/admin/reverse?lat=-6.195&lon=106.815");
    assert_eq!(response.status, 200);
    let body = response.json();
    assert_eq!(body["name"], "Gambir Village");
    assert_eq!(body["osm_type"], "relation");
    assert_eq!(body["osm_id"], 1003);
    assert_eq!(body["admin_id"], 4);
    assert_eq!(body["ambiguous"], false);
    assert_eq!(body["address"]["village"], "Gambir Village");
    assert_eq!(body["address"]["county"], "Gambir");
    assert_eq!(body["address"]["city"], "Jakarta Pusat");
    assert_eq!(body["address"]["state"], "Daerah Khusus Jakarta");
    assert_eq!(body["address"]["country"], "Indonesia");
    assert_eq!(
        body["display_name"],
        "Gambir Village, Gambir, Jakarta Pusat, Daerah Khusus Jakarta, Indonesia"
    );

    let missing = request(
        port,
        "GET",
        "/place/admin/reverse?lat=-7.0&lon=107.0",
    );
    assert_eq!(missing.status, 404);
    assert_eq!(missing.json()["error"], "not_found");

    let invalid = request(port, "GET", "/place/admin/reverse?lat=91&lon=0");
    assert_eq!(invalid.status, 400);
    assert_eq!(invalid.json()["error"], "invalid_request");
}
