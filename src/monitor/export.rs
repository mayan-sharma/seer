use crate::monitor::{SystemMetrics, HistoricalMetrics};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub timestamp: DateTime<Utc>,
    pub system_info: SystemInfo,
    pub current_metrics: SystemMetrics,
    pub historical_metrics: Vec<HistoricalMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub kernel_version: String,
    pub os_version: String,
    pub architecture: String,
    pub seer_version: String,
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
    Toml,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "csv" => Ok(ExportFormat::Csv),
            "toml" => Ok(ExportFormat::Toml),
            _ => Err(anyhow::anyhow!("Unsupported export format: {}", s)),
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Toml => "toml",
        }
    }
}

pub struct Exporter;

impl Exporter {
    pub fn export_current_metrics(
        metrics: &SystemMetrics,
        format: ExportFormat,
        output_path: Option<&Path>,
    ) -> Result<String> {
        let system_info = SystemInfo {
            hostname: sysinfo::System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            kernel_version: sysinfo::System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            os_version: sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
            architecture: std::env::consts::ARCH.to_string(),
            seer_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let export_data = ExportData {
            timestamp: Utc::now(),
            system_info,
            current_metrics: metrics.clone(),
            historical_metrics: Vec::new(),
        };

        let content = match format {
            ExportFormat::Json => serde_json::to_string_pretty(&export_data)?,
            ExportFormat::Csv => Self::to_csv(&export_data)?,
            ExportFormat::Toml => toml::to_string_pretty(&export_data)?,
        };

        if let Some(path) = output_path {
            // Security check: validate the output path
            if let Err(e) = Self::validate_export_path(path) {
                return Err(e);
            }
            
            let mut file = File::create(path)?;
            file.write_all(content.as_bytes())?;
            Ok(format!("Exported to: {}", path.display()))
        } else {
            Ok(content)
        }
    }

    pub fn export_historical_metrics(
        historical_metrics: &[HistoricalMetrics],
        format: ExportFormat,
        output_path: Option<&Path>,
    ) -> Result<String> {
        let _system_info = SystemInfo {
            hostname: sysinfo::System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            kernel_version: sysinfo::System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            os_version: sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
            architecture: std::env::consts::ARCH.to_string(),
            seer_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let content = match format {
            ExportFormat::Json => serde_json::to_string_pretty(&historical_metrics)?,
            ExportFormat::Csv => Self::historical_to_csv(historical_metrics)?,
            ExportFormat::Toml => toml::to_string_pretty(&historical_metrics)?,
        };

        if let Some(path) = output_path {
            // Security check: validate the output path
            if let Err(e) = Self::validate_export_path(path) {
                return Err(e);
            }
            
            let mut file = File::create(path)?;
            file.write_all(content.as_bytes())?;
            Ok(format!("Exported {} historical entries to: {}", historical_metrics.len(), path.display()))
        } else {
            Ok(content)
        }
    }

    fn to_csv(export_data: &ExportData) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        
        // Write system info
        wtr.write_record(["Field", "Value"])?;
        wtr.write_record(["Export Time", &export_data.timestamp.to_rfc3339()])?;
        wtr.write_record(["Hostname", &export_data.system_info.hostname])?;
        wtr.write_record(["OS Version", &export_data.system_info.os_version])?;
        wtr.write_record(["Architecture", &export_data.system_info.architecture])?;
        wtr.write_record(["Kernel Version", &export_data.system_info.kernel_version])?;
        wtr.write_record(["Seer Version", &export_data.system_info.seer_version])?;
        wtr.write_record(["", ""])?; // Empty line
        
        // Write current metrics
        wtr.write_record(["Metric", "Value"])?;
        wtr.write_record(["CPU Usage (%)", &format!("{:.2}", export_data.current_metrics.cpu.overall_usage)])?;
        wtr.write_record(["Memory Usage (%)", &format!("{:.2}", export_data.current_metrics.memory.ram_percentage)])?;
        wtr.write_record(["Memory Used (bytes)", &export_data.current_metrics.memory.used_ram.to_string()])?;
        wtr.write_record(["Memory Total (bytes)", &export_data.current_metrics.memory.total_ram.to_string()])?;
        wtr.write_record(["Swap Usage (%)", &format!("{:.2}", export_data.current_metrics.memory.swap_percentage)])?;
        wtr.write_record(["Network RX (bytes)", &export_data.current_metrics.network.total_bytes_received.to_string()])?;
        wtr.write_record(["Network TX (bytes)", &export_data.current_metrics.network.total_bytes_transmitted.to_string()])?;
        wtr.write_record(["Process Count", &export_data.current_metrics.processes.len().to_string()])?;
        wtr.write_record(["Load Average (1m)", &format!("{:.2}", export_data.current_metrics.load_average.one_min)])?;
        wtr.write_record(["Load Average (5m)", &format!("{:.2}", export_data.current_metrics.load_average.five_min)])?;
        wtr.write_record(["Load Average (15m)", &format!("{:.2}", export_data.current_metrics.load_average.fifteen_min)])?;
        wtr.write_record(["Uptime (seconds)", &export_data.current_metrics.uptime.to_string()])?;

        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok(data)
    }

    fn historical_to_csv(historical_metrics: &[HistoricalMetrics]) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        
        // Write header
        wtr.write_record([
            "Timestamp",
            "CPU Usage (%)",
            "Memory Usage (bytes)",
            "Memory Percentage (%)",
            "Network RX (bytes)",
            "Network TX (bytes)",
            "Disk Read (bytes)",
            "Disk Write (bytes)",
            "Load Average (1m)",
            "Process Count",
        ])?;

        // Write data
        for metrics in historical_metrics {
            wtr.write_record(&[
                metrics.timestamp.to_rfc3339(),
                format!("{:.2}", metrics.cpu_usage),
                metrics.memory_usage.to_string(),
                format!("{:.2}", metrics.memory_percentage),
                metrics.network_rx_bytes.to_string(),
                metrics.network_tx_bytes.to_string(),
                metrics.disk_read_bytes.to_string(),
                metrics.disk_write_bytes.to_string(),
                format!("{:.2}", metrics.load_average),
                metrics.process_count.to_string(),
            ])?;
        }

        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok(data)
    }

    pub fn generate_default_filename(format: &ExportFormat) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("seer_export_{}.{}", timestamp, format.extension())
    }

    /// Validates export path to prevent path traversal attacks
    fn validate_export_path(path: &Path) -> Result<()> {
        // Convert to absolute path for validation
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        // Check for path traversal attempts in the original path
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            return Err(anyhow::anyhow!(
                "Path traversal detected: {}",
                path.display()
            ));
        }

        // Get current working directory
        let current_dir = std::env::current_dir()?;

        // For relative paths, ensure they stay within current directory
        if path.is_relative() {
            // Check that the relative path doesn't escape current directory
            let normalized = current_dir.join(path);
            if !normalized.starts_with(&current_dir) {
                return Err(anyhow::anyhow!(
                    "Export path escapes current directory: {}",
                    path.display()
                ));
            }
        } else {
            // For absolute paths, ensure they're within current directory tree
            if !absolute_path.starts_with(&current_dir) {
                return Err(anyhow::anyhow!(
                    "Export path must be within current directory: {}",
                    absolute_path.display()
                ));
            }
        }

        // Additional check: prevent writing to sensitive system directories
        let path_str = absolute_path.to_string_lossy().to_lowercase();
        let forbidden_paths = ["/etc", "/proc", "/sys", "/dev", "/root", "/boot"];
        
        for forbidden in &forbidden_paths {
            if path_str.starts_with(forbidden) {
                return Err(anyhow::anyhow!(
                    "Cannot export to system directory: {}",
                    absolute_path.display()
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{CpuMetrics, MemoryMetrics, NetworkMetrics, LoadAverage};

    fn create_test_metrics() -> SystemMetrics {
        SystemMetrics {
            cpu: CpuMetrics {
                overall_usage: 25.5,
                per_core_usage: vec![20.0, 30.0],
                temperature: None,
            },
            memory: MemoryMetrics {
                total_ram: 16_000_000_000,
                used_ram: 8_000_000_000,
                available_ram: 8_000_000_000,
                total_swap: 4_000_000_000,
                used_swap: 1_000_000_000,
                ram_percentage: 50.0,
                swap_percentage: 25.0,
            },
            processes: vec![],
            network: NetworkMetrics {
                total_bytes_received: 500000,
                total_bytes_transmitted: 250000,
                total_packets_received: 1000,
                total_packets_transmitted: 500,
                interfaces: vec![],
            },
            storage: vec![],
            uptime: 7200,
            load_average: LoadAverage {
                one_min: 2.5,
                five_min: 2.0,
                fifteen_min: 1.8,
            },
            boot_time: Utc::now(),
        }
    }

    #[test]
    fn test_export_format_from_str() {
        assert!(matches!(ExportFormat::from_str("json").unwrap(), ExportFormat::Json));
        assert!(matches!(ExportFormat::from_str("JSON").unwrap(), ExportFormat::Json));
        assert!(matches!(ExportFormat::from_str("csv").unwrap(), ExportFormat::Csv));
        assert!(matches!(ExportFormat::from_str("toml").unwrap(), ExportFormat::Toml));
        
        assert!(ExportFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Toml.extension(), "toml");
    }

    #[test]
    fn test_export_current_metrics_json() {
        let metrics = create_test_metrics();
        let result = Exporter::export_current_metrics(&metrics, ExportFormat::Json, None);
        
        assert!(result.is_ok());
        let json_content = result.unwrap();
        
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_content).expect("Invalid JSON");
        
        // Check some key fields exist
        assert!(parsed.get("timestamp").is_some());
        assert!(parsed.get("current_metrics").is_some());
        assert!(parsed.get("system_info").is_some());
    }

    #[test]
    fn test_export_current_metrics_csv() {
        let metrics = create_test_metrics();
        let result = Exporter::export_current_metrics(&metrics, ExportFormat::Csv, None);
        
        assert!(result.is_ok());
        let csv_content = result.unwrap();
        
        // Should contain expected headers and values
        assert!(csv_content.contains("Field,Value"));
        assert!(csv_content.contains("CPU Usage (%)"));
        assert!(csv_content.contains("Memory Usage (%)"));
        assert!(csv_content.contains("25.50")); // CPU usage
        assert!(csv_content.contains("50.00")); // Memory percentage
    }

    #[test]
    fn test_export_current_metrics_toml() {
        let metrics = create_test_metrics();
        let result = Exporter::export_current_metrics(&metrics, ExportFormat::Toml, None);
        
        assert!(result.is_ok());
        let toml_content = result.unwrap();
        
        // Should be valid TOML
        let parsed: toml::Value = toml::from_str(&toml_content).expect("Invalid TOML");
        
        // Check some key fields exist
        assert!(parsed.get("timestamp").is_some());
        assert!(parsed.get("current_metrics").is_some());
        assert!(parsed.get("system_info").is_some());
    }

    #[test]
    fn test_generate_default_filename() {
        let json_filename = Exporter::generate_default_filename(&ExportFormat::Json);
        assert!(json_filename.starts_with("seer_export_"));
        assert!(json_filename.ends_with(".json"));
        
        let csv_filename = Exporter::generate_default_filename(&ExportFormat::Csv);
        assert!(csv_filename.ends_with(".csv"));
        
        let toml_filename = Exporter::generate_default_filename(&ExportFormat::Toml);
        assert!(toml_filename.ends_with(".toml"));
    }

    #[test]
    fn test_export_historical_metrics() {
        let historical_metrics = vec![
            crate::monitor::HistoricalMetrics {
                timestamp: Utc::now(),
                cpu_usage: 30.0,
                memory_usage: 5_000_000_000.0,
                memory_percentage: 62.5,
                network_rx_bytes: 1000,
                network_tx_bytes: 2000,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
                load_average: 1.5,
                process_count: 150,
            },
        ];

        let result = Exporter::export_historical_metrics(&historical_metrics, ExportFormat::Csv, None);
        assert!(result.is_ok());
        
        let csv_content = result.unwrap();
        assert!(csv_content.contains("Timestamp,CPU Usage (%)"));
        assert!(csv_content.contains("30.00")); // CPU usage
        assert!(csv_content.contains("62.50")); // Memory percentage
    }

    #[test]
    fn test_path_validation_security() {
        use std::path::Path;
        
        // Test valid current directory paths
        assert!(Exporter::validate_export_path(Path::new("./test.json")).is_ok());
        assert!(Exporter::validate_export_path(Path::new("subdir/test.json")).is_ok());
        
        // Test path traversal attacks should be rejected
        assert!(Exporter::validate_export_path(Path::new("../../../etc/passwd")).is_err());
        assert!(Exporter::validate_export_path(Path::new("../../sensitive.txt")).is_err());
        
        // Test absolute system paths should be rejected
        assert!(Exporter::validate_export_path(Path::new("/etc/passwd")).is_err());
        assert!(Exporter::validate_export_path(Path::new("/proc/version")).is_err());
        assert!(Exporter::validate_export_path(Path::new("/sys/kernel/version")).is_err());
    }
}