use crate::monitor::SystemMonitor;
use chrono::Duration;
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