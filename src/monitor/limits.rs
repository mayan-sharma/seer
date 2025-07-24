use anyhow::Result;
use std::fs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceLimits {
    pub pid: u32,
    pub max_cpu_time: Option<u64>,        // seconds
    pub max_file_size: Option<u64>,       // bytes
    pub max_data_size: Option<u64>,       // bytes
    pub max_stack_size: Option<u64>,      // bytes
    pub max_core_file_size: Option<u64>,  // bytes
    pub max_resident_set: Option<u64>,    // bytes
    pub max_processes: Option<u64>,       // count
    pub max_open_files: Option<u64>,      // count
    pub max_locked_memory: Option<u64>,   // bytes
    pub max_address_space: Option<u64>,   // bytes
    pub max_file_locks: Option<u64>,      // count
    pub max_pending_signals: Option<u64>, // count
    pub max_msgqueue_size: Option<u64>,   // bytes
    pub max_nice_priority: Option<i64>,   // priority
    pub max_realtime_priority: Option<u64>, // priority
    pub max_realtime_timeout: Option<u64>,  // microseconds
}

impl ResourceLimits {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            max_cpu_time: None,
            max_file_size: None,
            max_data_size: None,
            max_stack_size: None,
            max_core_file_size: None,
            max_resident_set: None,
            max_processes: None,
            max_open_files: None,
            max_locked_memory: None,
            max_address_space: None,
            max_file_locks: None,
            max_pending_signals: None,
            max_msgqueue_size: None,
            max_nice_priority: None,
            max_realtime_priority: None,
            max_realtime_timeout: None,
        }
    }
}

pub struct ResourceLimitsManager;

impl ResourceLimitsManager {
    /// Get resource limits for a process from /proc/pid/limits (Linux only)
    pub fn get_process_limits(pid: u32) -> Result<ResourceLimits> {
        #[cfg(target_os = "linux")]
        {
            let limits_path = format!("/proc/{}/limits", pid);
            let limits_content = fs::read_to_string(&limits_path)
                .map_err(|e| anyhow::anyhow!("Failed to read process limits: {}", e))?;

            let mut limits = ResourceLimits::new(pid);
            
            for line in limits_content.lines().skip(1) { // Skip header line
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let limit_name = parts[0];
                    let soft_limit = parts[1];
                    let _hard_limit = parts[2]; // We'll focus on soft limits for now
                    
                    let parsed_limit = if soft_limit == "unlimited" {
                        None
                    } else {
                        soft_limit.parse::<u64>().ok()
                    };
                    
                    match limit_name {
                        "Max" if parts.len() > 3 && parts[1] == "cpu" => {
                            limits.max_cpu_time = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "fsize" => {
                            limits.max_file_size = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "data" => {
                            limits.max_data_size = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "stack" => {
                            limits.max_stack_size = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "core" => {
                            limits.max_core_file_size = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "rss" => {
                            limits.max_resident_set = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "nproc" => {
                            limits.max_processes = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "nofile" => {
                            limits.max_open_files = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "memlock" => {
                            limits.max_locked_memory = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "as" => {
                            limits.max_address_space = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "locks" => {
                            limits.max_file_locks = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "sigpending" => {
                            limits.max_pending_signals = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "msgqueue" => {
                            limits.max_msgqueue_size = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "nice" => {
                            limits.max_nice_priority = soft_limit.parse::<i64>().ok();
                        },
                        "Max" if parts.len() > 3 && parts[1] == "rtprio" => {
                            limits.max_realtime_priority = parsed_limit;
                        },
                        "Max" if parts.len() > 3 && parts[1] == "rttime" => {
                            limits.max_realtime_timeout = parsed_limit;
                        },
                        _ => {}
                    }
                }
            }
            
            Ok(limits)
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Return empty limits for non-Linux systems
            Ok(ResourceLimits::new(pid))
        }
    }

    /// Get resource usage statistics for a process
    pub fn get_process_resource_usage(pid: u32) -> Result<ResourceUsage> {
        #[cfg(target_os = "linux")]
        {
            let stat_path = format!("/proc/{}/stat", pid);
            let stat_content = fs::read_to_string(&stat_path)
                .map_err(|e| anyhow::anyhow!("Failed to read process stat: {}", e))?;
            
            let stats: Vec<&str> = stat_content.split_whitespace().collect();
            
            let mut usage = ResourceUsage::new(pid);
            
            if stats.len() >= 52 {
                // Parse relevant fields from /proc/pid/stat
                usage.user_time = stats[13].parse().unwrap_or(0);
                usage.system_time = stats[14].parse().unwrap_or(0);
                usage.num_threads = stats[19].parse().unwrap_or(0);
                usage.virtual_memory = stats[22].parse().unwrap_or(0);
                usage.resident_memory = stats[23].parse::<u64>().unwrap_or(0) * 4096; // Convert pages to bytes
                usage.minor_faults = stats[9].parse().unwrap_or(0);
                usage.major_faults = stats[11].parse().unwrap_or(0);
            }
            
            // Get file descriptor count
            if let Ok(fd_dir) = fs::read_dir(format!("/proc/{}/fd", pid)) {
                usage.open_files = fd_dir.count() as u64;
            }
            
            Ok(usage)
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok(ResourceUsage::new(pid))
        }
    }

    /// Get a summary of resource usage vs limits
    pub fn get_resource_summary(pid: u32) -> Result<ResourceSummary> {
        let limits = Self::get_process_limits(pid)?;
        let usage = Self::get_process_resource_usage(pid)?;
        
        Ok(ResourceSummary {
            pid,
            limits,
            usage,
        })
    }

    /// Format bytes in human-readable format
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Check if a process is approaching its resource limits
    pub fn check_resource_warnings(summary: &ResourceSummary) -> Vec<ResourceWarning> {
        let mut warnings = Vec::new();
        
        // Check memory usage vs limit
        if let Some(limit) = summary.limits.max_resident_set {
            let usage_percent = (summary.usage.resident_memory as f64 / limit as f64) * 100.0;
            if usage_percent > 80.0 {
                warnings.push(ResourceWarning {
                    resource: "Memory".to_string(),
                    current: summary.usage.resident_memory,
                    limit,
                    percentage: usage_percent,
                    severity: if usage_percent > 95.0 { WarningLevel::Critical } else { WarningLevel::Warning },
                });
            }
        }
        
        // Check file descriptor usage vs limit
        if let Some(limit) = summary.limits.max_open_files {
            let usage_percent = (summary.usage.open_files as f64 / limit as f64) * 100.0;
            if usage_percent > 80.0 {
                warnings.push(ResourceWarning {
                    resource: "File Descriptors".to_string(),
                    current: summary.usage.open_files,
                    limit,
                    percentage: usage_percent,
                    severity: if usage_percent > 95.0 { WarningLevel::Critical } else { WarningLevel::Warning },
                });
            }
        }
        
        warnings
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceUsage {
    pub pid: u32,
    pub user_time: u64,          // CPU time in user mode (clock ticks)
    pub system_time: u64,        // CPU time in kernel mode (clock ticks)
    pub virtual_memory: u64,     // Virtual memory size in bytes
    pub resident_memory: u64,    // Resident set size in bytes
    pub num_threads: u64,        // Number of threads
    pub minor_faults: u64,       // Minor page faults
    pub major_faults: u64,       // Major page faults
    pub open_files: u64,         // Number of open file descriptors
}

impl ResourceUsage {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            user_time: 0,
            system_time: 0,
            virtual_memory: 0,
            resident_memory: 0,
            num_threads: 0,
            minor_faults: 0,
            major_faults: 0,
            open_files: 0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceSummary {
    pub pid: u32,
    pub limits: ResourceLimits,
    pub usage: ResourceUsage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceWarning {
    pub resource: String,
    pub current: u64,
    pub limit: u64,
    pub percentage: f64,
    pub severity: WarningLevel,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WarningLevel {
    Warning,
    Critical,
}