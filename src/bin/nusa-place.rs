//! Serve a format v1 index over the read-only HTTP API.

use anyhow::{Context, Result};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::get;
use clap::Parser;
use nusa_place::Index;
use nusa_place::api::{self, ApiState};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "nusa-place", version, about = "Serve a nusapeta-place index")]
struct Args {
    /// Index file to map.
    #[arg(long, short)]
    index: PathBuf,
    /// Optional manifest next to the index.
    #[arg(long)]
    manifest: Option<PathBuf>,
    /// Verify the full index checksum at startup.
    #[arg(long)]
    verify_checksum: bool,
    /// Listen address.
    #[arg(long, default_value = "127.0.0.1:8090")]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if std::env::var("NUSA_PLACE_LOG_FORMAT").as_deref() == Ok("json") {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_target(false)
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_writer(std::io::stderr)
            .init();
    }
    let metrics = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .context("install metrics recorder")?;
    let index = Index::open(&args.index, args.verify_checksum)
        .with_context(|| format!("open index {}", args.index.display()))?;
    let bytes = std::fs::metadata(&args.index)
        .with_context(|| format!("stat {}", args.index.display()))?
        .len();
    let manifest = match args.manifest {
        Some(path) => {
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read manifest {}", path.display()))?;
            Some(Arc::new(serde_json::from_slice(&bytes).with_context(
                || format!("parse manifest {}", path.display()),
            )?))
        }
        None => None,
    };
    let state = ApiState {
        index: Arc::new(index),
        manifest,
        index_bytes: bytes,
        metrics,
        weights: api::RankWeights::from_env(),
    };
    let app = Router::new()
        .route("/search", get(api::search))
        .route("/suggest", get(api::suggest))
        .route("/reverse", get(api::reverse))
        .route("/admin/reverse", get(api::admin_reverse))
        .route("/lookup", get(api::lookup))
        .route("/healthz", get(api::healthz))
        .route("/version", get(api::version))
        .route("/metrics", get(api::metrics_handler))
        .layer(axum::middleware::from_fn(api::middleware))
        .method_not_allowed_fallback(api::method_not_allowed)
        .fallback(api::not_found)
        .layer(DefaultBodyLimit::max(1024))
        .with_state(state);
    let listener = TcpListener::bind(args.bind)
        .await
        .with_context(|| format!("bind {}", args.bind))?;
    // Report the address the kernel gave us, not the one asked for: `:0` is a
    // valid bind, and a caller that needs the port it landed on reads this line.
    let local = listener.local_addr().context("local address")?;
    eprintln!("listening on http://{}", local);
    axum::serve(listener, app).await?;
    Ok(())
}
