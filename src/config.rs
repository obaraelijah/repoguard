use std::rc::Rc;

use anyhow::Result;
use serde::Deserialize;
use log::{debug, error, info};
use octocrab::{params::State, Octocrab};

use crate::prometheus::{self, JOBS_QUEUE_SIZE, PULL_REQUESTS_COUNT};

#[derive(Debug, Deserialize)]
pub(crate) struct Repository {
    owner: String,
    repository: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "owner")]
    pub default_owner: Rc<str>,
    #[serde(rename = "repository")]
    pub default_repo: Rc<str>,
    #[serde(default = "default_monitor_period")]
    pub monitor_period: u64,
    pub monitoring: Vec<Rc<Monitoring>>,
}

fn default_monitor_period() -> u64 {
    30
}

#[derive(Debug, Deserialize)]
#[serde(tag = "name")]
pub enum Monitoring {
    #[serde(rename = "job")]
    Job {
        status: Option<String>,
        workflow: String,
        #[serde(flatten)]
        repo: Option<Repository>,
    },
    #[serde(rename = "pull_requests")]
    PullRequests {
        status: Option<PRStatus>,
        labels: Option<Vec<String>>,
        #[serde(flatten)]
        repo: Option<Repository>,
    },
    #[serde(rename = "rate_limit")]
    RateLimit { pat_env: Option<String> },
    Custom {
        url: String,
        query: Option<String>,
        prometheus_metric: PrometheusMetric,
        #[serde(flatten)]
        repo: Option<Repository>,
    },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PRStatus {
    Open,
    Closed,
    All,
}

impl Into<String> for &PRStatus {
    fn into(self) -> String {
        match self {
            PRStatus::Open => "open".to_string(),
            PRStatus::Closed => "closed".to_string(),
            PRStatus::All => "all".to_string(),
        }
    }
}

impl Into<State> for PRStatus {
    fn into(self) -> State {
        match self {
            PRStatus::All => State::All,
            PRStatus::Closed => State::Closed,
            PRStatus::Open => State::Open,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) enum PrometheusMetric {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

pub async fn query(
    octo: &Octocrab,
    def_owner: Rc<str>, 
    defo_repo: Rc<str>,
    monitor: Rc<Monitoring>,
) {
    info!("Querying {:?}", monitor);
    let default = Repository {
        owner: def_owner.to_string(),
        repository: defo_repo.to_string(),
    };
    debug!("Default repo settings: {:?}", default);
    match monitor.as_ref() {
        Monitoring::Job { 
            status, 
            workflow, 
            repo 
        } => {
            let repo_def = repo.as_ref().unwrap_or(&default);
            debug!("Using repo settings: {:?}", repo_def);

            // Builder for GitHub workflows
            let builder = octo.workflows(&repo_def.owner, &repo_def.repository);
            let mut run_builder = builder.list_runs(workflow);

            // Apply status filter if specified
            if let Some(status) = &status {
                debug!("Filtering job status to {:?}", status);
                run_builder = run_builder.status(status);
            }
            info!("Querying repo runs");
            let runs = run_builder.send().await.unwrap();
            debug!("Got runs: {:?}", runs);

            info!("Pushing metrics to prometheus");
            JOBS_QUEUE_SIZE
                .with_label_values(&[
                    &repo_def.owner,
                    &repo_def.repository,
                    &status.as_ref().unwrap_or(&"".to_string()),
                    &workflow,
                ])
                .set(runs.total_count.unwrap_or(0) as i64);
        }
        Monitoring::PullRequests { 
            status, 
            labels, 
            repo 
        } => {
            let repo_def = repo.as_ref().unwrap_or(&default);
            debug!("Using repo settings: {:?}", repo_def);
            
            let builder = octo.issues(&repo_def.owner, &repo_def.repository);
            let mut pull_builder = builder.list();
            if let Some(status) = &status {
                debug!("Filtering pull request status to {:?}", status);
                pull_builder = pull_builder.state(status.clone().into());
            }
            if let Some(labels) = &labels {
                debug!("Filtering pull request labels to {:?}", labels);
                pull_builder = pull_builder.labels(labels)
            }
            let pulls = pull_builder.send().await.unwrap();
            // NOTE: This probably downloads all issue, then gets the count. Should look into a
            // better solution
            // NOTE: Using the `total_count` does not return the correct count. It could be
            // possible that this workaround is needed any
            let count = pulls
                .into_iter()
                .filter(|issue| issue.pull_request.is_some())
                .count();
            let tmp: String = status.as_ref().unwrap_or(&PRStatus::All).into();

            info!("Pushing metrics to prometheus");
            PULL_REQUESTS_COUNT
                .with_label_values(&[
                    &repo_def.owner,
                    &repo_def.repository,
                    &tmp,
                    &labels.as_ref().unwrap_or(&vec![]).join(","),
                ])
                .set(count as i64);
        }
        Monitoring::RateLimit { pat_env: bot } => {
            let user_name: String;
            let rate_remaining: i64;
            match bot {
                Option::None => {
                    (user_name, rate_remaining) = get_rate_limit(octo)
                        .await
                        .expect(&format!("Failed to get rate limit for self"));
                }
                Option::Some(pat_env) => {
                    info!("Querying rate limit for {}", &pat_env);
                    let pat = std::env::var(&pat_env).unwrap();
                    let local_octo = Octocrab::builder().personal_token(pat).build().unwrap();
                    (user_name, rate_remaining) = get_rate_limit(&local_octo)
                        .await
                        .expect(&format!("Failed to get rate limit for {}", &pat_env));
                }
            };

            info!("Pushing metrics to prometheus");
            prometheus::RATE_LIMIT
                .with_label_values(&[&user_name])
                .set(rate_remaining);
        }
        Monitoring::Custom { .. } => {
            error!("Custom monitoring not implemented");
            panic!("Not implimented");
        }
    }
}

async fn get_rate_limit(octo: &Octocrab) -> Result<(String, i64)> {
    let user = octo.current().user().await?;
    debug!("Got user: {:?}", user);
    let rate = octo.ratelimit().get().await?;
    debug!("Got rate limit: {:?}", rate);
    return Ok((user.login, rate.rate.remaining as i64));
}