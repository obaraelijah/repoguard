use lazy_static::lazy_static;
use prometheus::{register_int_gauge_vec, IntGaugeVec};

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
    pub static ref CUSTOM: IntGaugeVec = register_int_gauge_vec!(
        "github_custom",
        "Custom metric",
        &[
            "owener",
            "repository",
            "url",
            "query",
            "monitor",
            "prometheus_metric"
        ]
    )
    .unwrap();
}