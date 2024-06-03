use std::rc::Rc;

use serde::Deserialize;
use log::{debug, error, info};
use octocrab::{params::State, Octocrab};

#[derive(Debug, Deserialize)]
pub(crate) struct Repository {
    owner: String,
    repository: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub default_owner: Rc<str>,
    pub default_repo: Rc<str>,
}

#[derive(Debug, Deserialize)]
pub enum Monitoring {
    Job {
        status: Option<String>,
        workflow: String,
        repo: Option<Repository>,
    },
    PullRequests {
        status: Option<PRStatus>,
        labels: Option<Vec<String>>,
        repo: Option<Repository>,
    },
    Custom {
        url: String,
        query: Option<String>,
        repo: Option<Repository>,
    },
}

#[derive(Debug, Deserialize, Clone)]
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
        }
        Monitoring::Custom { .. } => {
            error!("Custom monitoring not implemented");
            panic!("Not implimented");
        }
    }
}
