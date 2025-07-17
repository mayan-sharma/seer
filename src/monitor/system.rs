use crate::monitor::SystemMonitor;
use chrono::{DateTime, Utc, Duration};
use sysinfo::System;
use std::fmt;

impl SystemMonitor {
    pub fn format_uptime(uptime_seconds: u64) -> String {
        let duration = Duration::seconds(uptime_seconds as i64);
        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        let minutes = duration.num_minutes() % 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    pub fn format_boot_time(boot_time: DateTime<Utc>) -> String {
        boot_time.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }

    pub fn get_cpu_temperature(&self) -> Option<f32> {
        #[cfg(target_os = "linux")]
        {
            self.read_linux_cpu_temperature()
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    #[cfg(target_os = "linux")]
    fn read_linux_cpu_temperature(&self) -> Option<f32> {
        use std::fs;
        use std::path::Path;

        let thermal_zones = [
            "/sys/class/thermal/thermal_zone0/temp",
            "/sys/class/thermal/thermal_zone1/temp",
            "/sys/devices/platform/coretemp.0/hwmon/hwmon*/temp1_input",
        ];

        for zone_path in &thermal_zones {
            if zone_path.contains('*') {
                if let Ok(entries) = glob::glob(zone_path) {
                    for entry in entries.flatten() {
                        if let Some(temp) = self.read_temperature_file(&entry) {
                            return Some(temp);
                        }
                    }
                }
            } else if Path::new(zone_path).exists() {
                if let Some(temp) = self.read_temperature_file(Path::new(zone_path)) {
                    return Some(temp);
                }
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn read_temperature_file(&self, path: &std::path::Path) -> Option<f32> {
        use std::fs;
        
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(temp_millicelsius) = content.trim().parse::<i32>() {
                return Some(temp_millicelsius as f32 / 1000.0);
            }
        }
        None
    }

    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
            architecture: std::env::consts::ARCH.to_string(),
            cpu_count: self.system.cpus().len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub kernel_version: String,
    pub os_version: String,
    pub architecture: String,
    pub cpu_count: usize,
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} | {} | {} | {} cores",
            self.hostname, self.os_version, self.architecture, self.cpu_count
        )
    }
}