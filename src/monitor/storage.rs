// use sysinfo::DiskExt; // Not needed in newer versions
use crate::monitor::SystemMonitor;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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


}