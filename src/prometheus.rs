use std::net::SocketAddr;

use anyhow::Result;
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use lazy_static::lazy_static;
use log::{debug, info, trace};
use prometheus::{register_int_gauge_vec, Encoder, IntGaugeVec, TextEncoder};
use tokio::net::TcpListener;

lazy_static! {
    pub static ref PULL_REQUESTS_COUNT: IntGaugeVec = register_int_gauge_vec!(
        "github_pull_requests",
        "Number of pull requests",
        &["owner", "repository", "status", "label"]
    )
    .unwrap();
    pub static ref JOBS_QUEUE_SIZE: IntGaugeVec = register_int_gauge_vec!(
        "github_jobs",
        "Number of jobs",
        &["owner", "repository", "status", "workflow"]
    )
    .unwrap();
    pub static ref JOBS_QUEUE_TIME: IntGaugeVec = register_int_gauge_vec!(
        "github_jobs_queue_time",
        "Queue time of jobs",
        &["owner", "repository", "status", "workflow"]
    )
    .unwrap();
    pub static ref RATE_LIMIT: IntGaugeVec =
        register_int_gauge_vec!("github_rate_limit", "Rate limit", &["username"]).unwrap();
    pub static ref CUSTOM: IntGaugeVec = register_int_gauge_vec!(
        "github_custom",
        "Custom metric",
        &[
            "owner",
            "repository",
            "url",
            "query",
            "monitor",
            "prometheus_metric"
        ]
    )
    .unwrap();
}

// Serve and endpoint for prometheus to read from.
// Should be /metrics
pub async fn serve() -> Result<()> {
    trace!("Initializing prometheus server");
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    debug!("Listening on: {}", addr);

    info!("Starting prometheus server on {}", addr);
    let listener = TcpListener::bind(&addr).await?;

    debug!("Accepting connections");
    loop {
        debug!("Waiting for connection");
        let (stream, _) = listener.accept().await?;
        debug!("Connection accepted");
        let io = TokioIo::new(stream);

        debug!("Handling connection");
        tokio::task::spawn(async {
            if let Err(er) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_connection))
                .await
            {
                eprintln!("Error: {}", er);
            }
            debug!("Handled connection");
        });
    }
}

async fn handle_connection(req: Request<Incoming>) -> Result<Response<String>> {
    info!("Serving metrics");
    if req.uri().path() != "/metrics" {
        debug!("Got request for {}, returning 404", req.uri().path());
        return Ok(Response::builder()
            .status(404)
            .body("Not Found".to_string())
            .unwrap());
    }
    trace!("Getting default registry");
    let registry = prometheus::default_registry();
    trace!("Creating encoder");
    let encoder = TextEncoder::new();
    trace!("Gathering metrics");
    let metrics = registry.gather();
    let mut buffer = vec![];
    trace!("Encoding metrics");
    encoder.encode(&metrics, &mut buffer)?;

    info!("Returning metrics");
    let str_resp = String::from_utf8(buffer)?;
    debug!("Metrics: {}", &str_resp);
    return Ok(Response::builder().status(200).body(str_resp).unwrap());
}