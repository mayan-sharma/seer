use clap::Parser;

pub mod config;
pub mod monitor;
pub mod ui;

#[derive(Parser)]
#[command(name = "seer")]
#[command(about = "A comprehensive CLI system monitoring tool")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[arg(short = 'r', long = "refresh-rate", default_value = "2")]
    pub refresh_rate: u64,

    #[arg(long = "show-zombies")]
    pub show_zombies: bool,

    #[arg(short = 'f', long = "filter-process")]
    pub filter_process: Option<String>,

    #[arg(short = 'e', long = "export")]
    pub export: Option<String>,

    #[arg(long = "threshold-cpu", default_value = "80")]
    pub threshold_cpu: f32,

    #[arg(long = "threshold-memory", default_value = "80")]
    pub threshold_memory: f32,
}