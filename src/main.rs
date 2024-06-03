use std::{fs, sync::Arc};

use args::Args;
use clap::Parser;
use config::Config;
use jsonwebtoken::EncodingKey;
use octocrab::{models::AppId, Octocrab};
use prometheus::serve;

use log::{debug, info, trace};

use crate::config::query;

mod args;
mod config;
mod prometheus;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // setup logging

    // setup octo client
    let args = Args::parse();
    simple_logger::init_with_level(args.log_level.into()).unwrap();

    info!("Reading config from {}", args.config);
    let file = fs::File::open(args.config).expect("Failed to open config file");

    trace!("Parsing config");
    let config: Config = serde_yaml::from_reader(file).expect("Failed to parse config file");
    info!("Config parsed");
    debug!("Config: {:#?}", config);

    info!("Building octocrab");
    let mut octo_builder = Octocrab::builder();
    if let Some(pat) = args.pat {
        trace!("Using personal access token");
        octo_builder = octo_builder.personal_token(pat);
    } else {
        trace!("Using app id and secret");
        let app_id = args.app_id.expect("Either App id or PAT are required");
        let app_secret = args.app_secret.unwrap();
        octo_builder = octo_builder.app(
            AppId(app_id.parse().unwrap()),
            EncodingKey::from_base64_secret(&app_secret).unwrap(),
        );
    }
    let octo = Arc::new(octo_builder.build().expect("Failed to build octocrab"));

    tokio::spawn(async {
        serve().await.expect("Failed to start prometheus server");
    });

    info!("Starting the monitoring loop");
    let period = std::time::Duration::from_secs(config.monitor_period);
    debug!("Monitoring period: {:?}", period);
    let mut interval = tokio::time::interval(period);
    loop {
        interval.tick().await;
        for monitor in &config.monitoring {
            query(
                &octo,
                config.default_owner.clone(),
                config.default_repo.clone(),
                monitor.clone(),
            )
            .await;
        }
    }
}