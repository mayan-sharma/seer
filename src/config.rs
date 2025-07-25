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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.refresh_rate, 2);
        assert!(!config.show_zombies);
        assert_eq!(config.threshold_cpu, 80.0);
        assert_eq!(config.threshold_memory, 80.0);
        assert!(config.filter_process.is_none());
        assert!(config.export_format.is_none());
    }

    #[test]
    fn test_config_from_cli() {
        let cli = crate::Cli {
            refresh_rate: 5,
            show_zombies: true,
            filter_process: Some("test".to_string()),
            export: Some("json".to_string()),
            threshold_cpu: 90.0,
            threshold_memory: 85.0,
        };

        let config = Config::new(cli).expect("Failed to create config");
        assert_eq!(config.refresh_rate, 5);
        assert!(config.show_zombies);
        assert_eq!(config.filter_process, Some("test".to_string()));
        assert_eq!(config.export_format, Some("json".to_string()));
        assert_eq!(config.threshold_cpu, 90.0);
        assert_eq!(config.threshold_memory, 85.0);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = toml::to_string(&config).expect("Failed to serialize config");
        let deserialized: Config = toml::from_str(&serialized).expect("Failed to deserialize config");
        
        assert_eq!(config.refresh_rate, deserialized.refresh_rate);
        assert_eq!(config.show_zombies, deserialized.show_zombies);
        assert_eq!(config.threshold_cpu, deserialized.threshold_cpu);
        assert_eq!(config.threshold_memory, deserialized.threshold_memory);
    }
}