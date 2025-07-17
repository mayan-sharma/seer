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
        let mut disks = Vec::new();
        
        // Get disk information from sysinfo
        for disk in self.disks.list() {
            let total_space = disk.total_space();
            let available_space = disk.available_space();
            let used_space = total_space.saturating_sub(available_space);
            
            let usage_percentage = if total_space > 0 {
                (used_space as f32 / total_space as f32) * 100.0
            } else {
                0.0
            };

            let file_system = disk.file_system()
                .to_string_lossy()
                .to_string();

            let mount_point = disk.mount_point()
                .to_string_lossy()
                .to_string();

            let name = disk.name()
                .to_string_lossy()
                .to_string();

            disks.push(DiskInfo {
                name,
                mount_point,
                file_system,
                total_space,
                available_space,
                used_space,
                usage_percentage,
                is_removable: disk.is_removable(),
            });
        }
        
        disks
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

}