use clap::{Parser, ValueEnum};
use log::Level;

#[derive(Parser, Debug)]
#[command()]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "./config.yaml",
        help = "The monitoring config"
    )]
    pub config: String,
    #[arg(
        long,
        env = "APP_PAT",
        conflicts_with_all = [ "app_id", "app_secret", "help"],
        help = "The personal access token for the GitHub App"
    )]
    pub pat: Option<String>,
    #[arg(
        long,
        env = "APP_ID",
        requires = "app_secret",
        help = "The GitHub App ID"
    )]
    pub app_id: Option<String>,
    #[arg(
        long,
        env = "APP_SECRET",
        requires = "app_id",
        help = "The GitHub App Secret"
    )]
    pub app_secret: Option<String>,
    #[arg(long, short, help = "The log level", default_value = "info")]
    pub log_level: LogLevels,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum LogLevels {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Into<Level> for LogLevels {
    fn into(self) -> Level {
        match self {
            LogLevels::Trace => Level::Trace,
            LogLevels::Debug => Level::Debug,
            LogLevels::Info => Level::Info,
            LogLevels::Warn => Level::Warn,
            LogLevels::Error => Level::Error,
        }
    }
}
