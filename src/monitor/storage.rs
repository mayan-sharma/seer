// use sysinfo::DiskExt; // Not needed in newer versions
use crate::monitor::SystemMonitor;

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub usage_percentage: f32,
    pub is_removable: bool,
}

#[derive(Debug, Clone)]
pub struct DiskIoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

impl SystemMonitor {
    pub fn get_storage_info(&self) -> Vec<DiskInfo> {
        // Placeholder implementation - disk monitoring may need different approach
        vec![
            DiskInfo {
                name: "/dev/sda1".to_string(),
                mount_point: "/".to_string(),
                file_system: "ext4".to_string(),
                total_space: 100_000_000_000, // 100GB
                available_space: 70_000_000_000, // 70GB
                used_space: 30_000_000_000, // 30GB
                usage_percentage: 30.0,
                is_removable: false,
            }
        ]
    }

    pub fn get_disk_io_stats(&self) -> Vec<DiskIoStats> {
        #[cfg(target_os = "linux")]
        {
            self.read_linux_disk_stats()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Vec::new()
        }
    }

    #[cfg(target_os = "linux")]
    fn read_linux_disk_stats(&self) -> Vec<DiskIoStats> {
        use std::fs;
        
        let mut stats = Vec::new();
        
        if let Ok(content) = fs::read_to_string("/proc/diskstats") {
            for line in content.lines() {
                if let Some(stat) = self.parse_diskstats_line(line) {
                    stats.push(stat);
                }
            }
        }
        
        stats
    }

    #[cfg(target_os = "linux")]
    fn parse_diskstats_line(&self, line: &str) -> Option<DiskIoStats> {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() < 14 {
            return None;
        }

        let device_name = parts[2];
        
        if device_name.starts_with("loop") || 
           device_name.starts_with("ram") ||
           device_name.len() < 3 {
            return None;
        }

        let read_sectors = parts[5].parse::<u64>().ok()?;
        let write_sectors = parts[9].parse::<u64>().ok()?;
        let read_ops = parts[3].parse::<u64>().ok()?;
        let write_ops = parts[7].parse::<u64>().ok()?;

        const SECTOR_SIZE: u64 = 512;

        Some(DiskIoStats {
            read_bytes: read_sectors * SECTOR_SIZE,
            write_bytes: write_sectors * SECTOR_SIZE,
            read_ops,
            write_ops,
        })
    }

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
}