pub mod system;
pub mod processes;
pub mod network;
pub mod storage;
pub mod history;
pub mod export;
pub mod process_tree;
pub mod affinity;
pub mod limits;
pub mod performance;
pub mod thermal;
pub mod dependencies;
pub mod memory_leak;
pub mod io_analysis;
pub mod gpu;

use anyhow::Result;
use sysinfo::{System, Networks, Disks};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use history::HistoryManager;

pub use system::*;
pub use processes::*;
pub use network::*;
pub use storage::*;
pub use history::*;
pub use export::*;
pub use process_tree::*;
pub use affinity::*;
pub use limits::*;
pub use performance::*;
pub use thermal::*;
pub use dependencies::*;
pub use memory_leak::*;
pub use io_analysis::*;
pub use gpu::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuMetrics {
    pub overall_usage: f32,
    pub per_core_usage: Vec<f32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryMetrics {
    pub total_ram: u64,
    pub used_ram: u64,
    pub available_ram: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub ram_percentage: f32,
    pub swap_percentage: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

pub struct SystemMonitor {
    system: System,
    networks: Networks,
    disks: Disks,
    previous_network_data: HashMap<String, (u64, u64)>,
    pub history: HistoryManager,
    pub profiler: PerformanceProfiler,
    pub thermal_monitor: ThermalMonitor,
    pub dependency_analyzer: DependencyAnalyzer,
    pub memory_leak_detector: MemoryLeakDetector,
    pub io_bottleneck_analyzer: IOBottleneckAnalyzer,
    pub gpu_monitor: GPUMonitor,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            previous_network_data: HashMap::new(),
            history: HistoryManager::new(1440), // Store 24 hours of data (1 minute intervals)
            profiler: PerformanceProfiler::new(),
            thermal_monitor: ThermalMonitor::new(),
            dependency_analyzer: DependencyAnalyzer::new(),
            memory_leak_detector: MemoryLeakDetector::new(),
            io_bottleneck_analyzer: IOBottleneckAnalyzer::new(),
            gpu_monitor: GPUMonitor::new(),
        }
    }

    pub async fn update(&mut self) -> Result<()> {
        self.system.refresh_all();
        self.networks.refresh_list();
        self.networks.refresh();
        self.disks.refresh_list();
        self.disks.refresh();
        self.update_network_data();
        
        // Update advanced analysis modules
        let processes = self.get_process_info();
        
        // Update memory leak detection
        if let Err(e) = self.memory_leak_detector.update_process_memory(&processes) {
            eprintln!("Memory leak detection error: {}", e);
        }
        
        // Update IO bottleneck analysis
        if let Err(e) = self.io_bottleneck_analyzer.update_io_metrics(&processes) {
            eprintln!("IO bottleneck analysis error: {}", e);
        }
        
        // Update GPU monitoring
        if let Err(e) = self.gpu_monitor.update_gpu_metrics() {
            eprintln!("GPU monitoring error: {}", e);
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    pub fn get_metrics(&mut self) -> SystemMetrics {
        let processes = self.get_process_info();
        
        // Update performance profiler
        for process in &processes {
            self.profiler.update_process(process);
        }
        
        // Clean up old profiles
        let active_pids: Vec<u32> = processes.iter().map(|p| p.pid).collect();
        self.profiler.cleanup_old_profiles(&active_pids);
        
        let metrics = SystemMetrics {
            cpu: self.get_cpu_metrics(),
            memory: self.get_memory_metrics(),
            processes,
            network: self.get_network_metrics(),
            storage: self.get_storage_info(),
            uptime: System::uptime(),
            load_average: self.get_load_average(),
            boot_time: self.get_boot_time(),
        };
        
        // Add to history
        self.history.add_metrics(&metrics);
        
        metrics
    }

    fn get_cpu_metrics(&self) -> CpuMetrics {
        let cpus = self.system.cpus();
        let overall_usage = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32;
        let per_core_usage = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();

        // Get average CPU temperature
        let temperature = self.thermal_monitor.get_average_cpu_temperature().ok().flatten();

        CpuMetrics {
            overall_usage,
            per_core_usage,
            temperature,
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