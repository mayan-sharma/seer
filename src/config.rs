use crate::Cli;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub refresh_rate: u64,
    pub show_zombies: bool,
    pub filter_process: Option<String>,
    pub export_format: Option<String>,
    pub threshold_cpu: f32,
    pub threshold_memory: f32,
}

impl Config {
    pub fn new(cli: Cli) -> Result<Self> {
        let mut config = Self::load_from_file().unwrap_or_default();
        
        config.refresh_rate = cli.refresh_rate;
        config.show_zombies = cli.show_zombies;
        config.filter_process = cli.filter_process;
        config.export_format = cli.export;
        config.threshold_cpu = cli.threshold_cpu;
        config.threshold_memory = cli.threshold_memory;
        
        Ok(config)
    }

    fn load_from_file() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let config: Self = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    fn get_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(".config").join("seer").join("config.toml"))
    }

    pub fn save_to_file(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_rate: 2,
            show_zombies: false,
            filter_process: None,
            export_format: None,
            threshold_cpu: 80.0,
            threshold_memory: 80.0,
        }
    }
}