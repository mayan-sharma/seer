pub mod system;
pub mod processes;
pub mod network;
pub mod storage;

use anyhow::Result;
use sysinfo::System;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

pub use system::*;
pub use processes::*;
pub use network::*;
pub use storage::*;

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub processes: Vec<ProcessInfo>,
    pub network: NetworkMetrics,
    pub storage: Vec<DiskInfo>,
    pub uptime: u64,
    pub load_average: LoadAverage,
    pub boot_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CpuMetrics {
    pub overall_usage: f32,
    pub per_core_usage: Vec<f32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub total_ram: u64,
    pub used_ram: u64,
    pub available_ram: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub ram_percentage: f32,
    pub swap_percentage: f32,
}

#[derive(Debug, Clone)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

pub struct SystemMonitor {
    system: System,
    previous_network_data: HashMap<String, (u64, u64)>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system,
            previous_network_data: HashMap::new(),
        }
    }

    pub async fn update(&mut self) -> Result<()> {
        self.system.refresh_all();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    pub fn get_metrics(&self) -> SystemMetrics {
        SystemMetrics {
            cpu: self.get_cpu_metrics(),
            memory: self.get_memory_metrics(),
            processes: self.get_process_info(),
            network: self.get_network_metrics(),
            storage: self.get_storage_info(),
            uptime: System::uptime(),
            load_average: self.get_load_average(),
            boot_time: self.get_boot_time(),
        }
    }

    fn get_cpu_metrics(&self) -> CpuMetrics {
        let cpus = self.system.cpus();
        let overall_usage = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32;
        let per_core_usage = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();

        CpuMetrics {
            overall_usage,
            per_core_usage,
            temperature: None, // TODO: Implement temperature reading
        }
    }

    fn get_memory_metrics(&self) -> MemoryMetrics {
        let total_ram = self.system.total_memory();
        let used_ram = self.system.used_memory();
        let available_ram = self.system.available_memory();
        let total_swap = self.system.total_swap();
        let used_swap = self.system.used_swap();

        let ram_percentage = if total_ram > 0 {
            (used_ram as f32 / total_ram as f32) * 100.0
        } else {
            0.0
        };

        let swap_percentage = if total_swap > 0 {
            (used_swap as f32 / total_swap as f32) * 100.0
        } else {
            0.0
        };

        MemoryMetrics {
            total_ram,
            used_ram,
            available_ram,
            total_swap,
            used_swap,
            ram_percentage,
            swap_percentage,
        }
    }

    fn get_load_average(&self) -> LoadAverage {
        let load_avg = System::load_average();
        LoadAverage {
            one_min: load_avg.one,
            five_min: load_avg.five,
            fifteen_min: load_avg.fifteen,
        }
    }

    fn get_boot_time(&self) -> DateTime<Utc> {
        let boot_time = System::boot_time();
        DateTime::from_timestamp(boot_time as i64, 0).unwrap_or_else(Utc::now)
    }
}