mod args;
mod config;

use std::fs;

use args::Args;
use clap::Parser;
use config::Config;

fn main() {
    let args = Args::parse();
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    let file = fs::File::open(args.config).expect("Failed to open config file");

    let config: Config = serde_yaml::from_reader(file).expect("Failed to parse config file");
}
