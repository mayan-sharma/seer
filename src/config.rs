use crate::Cli;
use crate::monitor::{DatabaseConfig, IoTConfig, BackupConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub refresh_rate: u64,
    pub show_zombies: bool,
    pub filter_process: Option<String>,
    pub export_format: Option<String>,
    pub threshold_cpu: f32,
    pub threshold_memory: f32,
    pub advanced_monitoring: AdvancedMonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedMonitoringConfig {
    pub database: DatabaseConfigWrapper,
    pub iot: IoTConfigWrapper,
    pub backup: BackupConfigWrapper,
    pub apm: APMConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfigWrapper {
    pub enabled: bool,
    pub mysql_enabled: bool,
    pub mysql_host: String,
    pub mysql_port: u16,
    pub mysql_user: String,
    pub postgresql_enabled: bool,
    pub postgresql_host: String,
    pub postgresql_port: u16,
    pub postgresql_user: String,
    pub postgresql_database: String,
    pub mongodb_enabled: bool,
    pub mongodb_host: String,
    pub mongodb_port: u16,
    pub redis_enabled: bool,
    pub redis_host: String,
    pub redis_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTConfigWrapper {
    pub enabled: bool,
    pub discovery_interval_seconds: u64,
    pub health_check_interval_seconds: u64,
    pub network_ranges: Vec<String>,
    pub enable_bluetooth_scan: bool,
    pub enable_zigbee_scan: bool,
    pub enable_upnp_discovery: bool,
    pub enable_mdns_discovery: bool,
    pub device_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfigWrapper {
    pub enabled: bool,
    pub monitor_interval_seconds: u64,
    pub backup_directories: Vec<String>,
    pub storage_locations: Vec<String>,
    pub log_file_paths: Vec<String>,
    pub enable_performance_monitoring: bool,
    pub integrity_check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APMConfig {
    pub enabled: bool,
    pub monitor_jvm: bool,
    pub monitor_dotnet: bool,
    pub monitor_python: bool,
    pub monitor_nodejs: bool,
    pub monitor_golang: bool,
    pub anomaly_detection_enabled: bool,
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
            advanced_monitoring: AdvancedMonitoringConfig::default(),
        }
    }
}

impl Default for AdvancedMonitoringConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfigWrapper::default(),
            iot: IoTConfigWrapper::default(),
            backup: BackupConfigWrapper::default(),
            apm: APMConfig::default(),
        }
    }
}

impl Default for DatabaseConfigWrapper {
    fn default() -> Self {
        Self {
            enabled: false,
            mysql_enabled: false,
            mysql_host: "localhost".to_string(),
            mysql_port: 3306,
            mysql_user: "root".to_string(),
            postgresql_enabled: false,
            postgresql_host: "localhost".to_string(),
            postgresql_port: 5432,
            postgresql_user: "postgres".to_string(),
            postgresql_database: "postgres".to_string(),
            mongodb_enabled: false,
            mongodb_host: "localhost".to_string(),
            mongodb_port: 27017,
            redis_enabled: false,
            redis_host: "localhost".to_string(),
            redis_port: 6379,
        }
    }
}

impl Default for IoTConfigWrapper {
    fn default() -> Self {
        Self {
            enabled: false,
            discovery_interval_seconds: 300,
            health_check_interval_seconds: 60,
            network_ranges: vec!["192.168.1.0/24".to_string()],
            enable_bluetooth_scan: true,
            enable_zigbee_scan: false,
            enable_upnp_discovery: true,
            enable_mdns_discovery: true,
            device_timeout_seconds: 300,
        }
    }
}

impl Default for BackupConfigWrapper {
    fn default() -> Self {
        Self {
            enabled: false,
            monitor_interval_seconds: 300,
            backup_directories: vec![
                "/var/backups".to_string(),
                "/backup".to_string(),
                "/home/backups".to_string(),
            ],
            storage_locations: vec![
                "/mnt/backup".to_string(),
                "/backup".to_string(),
            ],
            log_file_paths: vec![
                "/var/log/backup.log".to_string(),
                "/var/log/rsync.log".to_string(),
            ],
            enable_performance_monitoring: true,
            integrity_check_interval_seconds: 3600,
        }
    }
}

impl Default for APMConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitor_jvm: true,
            monitor_dotnet: true,
            monitor_python: true,
            monitor_nodejs: true,
            monitor_golang: true,
            anomaly_detection_enabled: true,
        }
    }
}

impl Config {
    pub fn get_database_config(&self) -> DatabaseConfig {
        DatabaseConfig {
            mysql_enabled: self.advanced_monitoring.database.mysql_enabled,
            mysql_host: self.advanced_monitoring.database.mysql_host.clone(),
            mysql_port: self.advanced_monitoring.database.mysql_port,
            mysql_user: self.advanced_monitoring.database.mysql_user.clone(),
            mysql_password: String::new(), // Don't store passwords in config
            postgresql_enabled: self.advanced_monitoring.database.postgresql_enabled,
            postgresql_host: self.advanced_monitoring.database.postgresql_host.clone(),
            postgresql_port: self.advanced_monitoring.database.postgresql_port,
            postgresql_user: self.advanced_monitoring.database.postgresql_user.clone(),
            postgresql_password: String::new(),
            postgresql_database: self.advanced_monitoring.database.postgresql_database.clone(),
            mongodb_enabled: self.advanced_monitoring.database.mongodb_enabled,
            mongodb_host: self.advanced_monitoring.database.mongodb_host.clone(),
            mongodb_port: self.advanced_monitoring.database.mongodb_port,
            redis_enabled: self.advanced_monitoring.database.redis_enabled,
            redis_host: self.advanced_monitoring.database.redis_host.clone(),
            redis_port: self.advanced_monitoring.database.redis_port,
        }
    }

    pub fn get_iot_config(&self) -> IoTConfig {
        IoTConfig {
            discovery_interval: Duration::from_secs(self.advanced_monitoring.iot.discovery_interval_seconds),
            health_check_interval: Duration::from_secs(self.advanced_monitoring.iot.health_check_interval_seconds),
            network_ranges: self.advanced_monitoring.iot.network_ranges.clone(),
            enable_bluetooth_scan: self.advanced_monitoring.iot.enable_bluetooth_scan,
            enable_zigbee_scan: self.advanced_monitoring.iot.enable_zigbee_scan,
            enable_upnp_discovery: self.advanced_monitoring.iot.enable_upnp_discovery,
            enable_mdns_discovery: self.advanced_monitoring.iot.enable_mdns_discovery,
            mqtt_broker_url: None,
            device_timeout: Duration::from_secs(self.advanced_monitoring.iot.device_timeout_seconds),
        }
    }

    pub fn get_backup_config(&self) -> BackupConfig {
        BackupConfig {
            monitor_interval: Duration::from_secs(self.advanced_monitoring.backup.monitor_interval_seconds),
            backup_directories: self.advanced_monitoring.backup.backup_directories
                .iter()
                .map(|s| PathBuf::from(s))
                .collect(),
            storage_locations: self.advanced_monitoring.backup.storage_locations.clone(),
            log_file_paths: self.advanced_monitoring.backup.log_file_paths
                .iter()
                .map(|s| PathBuf::from(s))
                .collect(),
            enable_performance_monitoring: self.advanced_monitoring.backup.enable_performance_monitoring,
            integrity_check_interval: Duration::from_secs(self.advanced_monitoring.backup.integrity_check_interval_seconds),
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