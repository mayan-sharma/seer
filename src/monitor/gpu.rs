use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::process::Command;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUMonitor {
    gpu_history: VecDeque<GPUSnapshot>,
    nvidia_available: bool,
    amd_available: bool,
    intel_available: bool,
    process_gpu_usage: HashMap<u32, GPUProcessUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUSnapshot {
    pub timestamp: DateTime<Utc>,
    pub gpus: Vec<GPUInfo>,
    pub total_gpu_processes: u32,
    pub total_memory_used: u64,
    pub total_memory_available: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    pub gpu_id: u32,
    pub name: String,
    pub vendor: GPUVendor,
    pub driver_version: String,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_free: u64,
    pub utilization_gpu: f32,
    pub utilization_memory: f32,
    pub temperature: Option<f32>,
    pub power_usage: Option<f32>,
    pub power_limit: Option<f32>,
    pub fan_speed: Option<f32>,
    pub clock_graphics: Option<u32>,
    pub clock_memory: Option<u32>,
    pub processes: Vec<GPUProcess>,
    pub encoder_utilization: Option<f32>,
    pub decoder_utilization: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GPUVendor {
    NVIDIA,
    AMD,
    Intel,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUProcess {
    pub pid: u32,
    pub process_name: String,
    pub memory_usage: u64,
    pub gpu_utilization: Option<f32>,
    pub encode_utilization: Option<f32>,
    pub decode_utilization: Option<f32>,
    pub process_type: GPUProcessType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GPUProcessType {
    Graphics,
    Compute,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUProcessUsage {
    pub pid: u32,
    pub memory_usage_history: VecDeque<u64>,
    pub utilization_history: VecDeque<f32>,
    pub last_seen: DateTime<Utc>,
}

impl GPUMonitor {
    pub fn new() -> Self {
        let mut monitor = Self {
            gpu_history: VecDeque::new(),
            nvidia_available: false,
            amd_available: false,
            intel_available: false,
            process_gpu_usage: HashMap::new(),
        };
        
        monitor.detect_gpu_vendors();
        monitor
    }

    fn detect_gpu_vendors(&mut self) {
        // Check for NVIDIA GPU support
        if Command::new("nvidia-smi").arg("--version").output().is_ok() {
            self.nvidia_available = true;
        }

        // Check for AMD GPU support
        if fs::metadata("/sys/class/drm").is_ok() {
            // Look for AMD GPUs in DRM
            if let Ok(entries) = fs::read_dir("/sys/class/drm") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("card") && !name.contains("-") {
                            let vendor_path = path.join("device/vendor");
                            if let Ok(vendor) = fs::read_to_string(vendor_path) {
                                if vendor.trim() == "0x1002" { // AMD vendor ID
                                    self.amd_available = true;
                                } else if vendor.trim() == "0x8086" { // Intel vendor ID
                                    self.intel_available = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_gpu_metrics(&mut self) -> Result<()> {
        let mut gpus = Vec::new();
        let mut total_processes = 0;
        let mut total_memory_used = 0;
        let mut total_memory_available = 0;

        // Get NVIDIA GPU info
        if self.nvidia_available {
            if let Ok(nvidia_gpus) = self.get_nvidia_info() {
                for gpu in nvidia_gpus {
                    total_processes += gpu.processes.len() as u32;
                    total_memory_used += gpu.memory_used;
                    total_memory_available += gpu.memory_total;
                    gpus.push(gpu);
                }
            }
        }

        // Get AMD GPU info
        if self.amd_available {
            if let Ok(amd_gpus) = self.get_amd_info() {
                for gpu in amd_gpus {
                    total_processes += gpu.processes.len() as u32;
                    total_memory_used += gpu.memory_used;
                    total_memory_available += gpu.memory_total;
                    gpus.push(gpu);
                }
            }
        }

        // Get Intel GPU info
        if self.intel_available {
            if let Ok(intel_gpus) = self.get_intel_info() {
                for gpu in intel_gpus {
                    total_processes += gpu.processes.len() as u32;
                    total_memory_used += gpu.memory_used;
                    total_memory_available += gpu.memory_total;
                    gpus.push(gpu);
                }
            }
        }

        let snapshot = GPUSnapshot {
            timestamp: Utc::now(),
            gpus,
            total_gpu_processes: total_processes,
            total_memory_used,
            total_memory_available,
        };

        // Update process GPU usage tracking
        self.update_process_tracking(&snapshot);

        // Add to history
        self.gpu_history.push_back(snapshot);

        // Limit history size (keep last 1000 entries)
        if self.gpu_history.len() > 1000 {
            self.gpu_history.pop_front();
        }

        Ok(())
    }

    fn get_nvidia_info(&self) -> Result<Vec<GPUInfo>> {
        let mut gpus = Vec::new();

        // Query GPU information using nvidia-ml-py equivalent commands
        let output = Command::new("nvidia-smi")
            .args(&[
                "--query-gpu=index,name,driver_version,memory.total,memory.used,memory.free,utilization.gpu,utilization.memory,temperature.gpu,power.draw,power.limit,fan.speed,clocks.current.graphics,clocks.current.memory,encoder.stats.sessionCount,decoder.stats.sessionCount",
                "--format=csv,noheader,nounits"
            ])
            .output()?;

        let gpu_info = String::from_utf8_lossy(&output.stdout);
        
        for (idx, line) in gpu_info.lines().enumerate() {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if fields.len() >= 16 {
                let processes = self.get_nvidia_processes(idx as u32)?;
                
                let gpu = GPUInfo {
                    gpu_id: idx as u32,
                    name: fields[1].to_string(),
                    vendor: GPUVendor::NVIDIA,
                    driver_version: fields[2].to_string(),
                    memory_total: fields[3].parse::<u64>().unwrap_or(0) * 1024 * 1024, // Convert MB to bytes
                    memory_used: fields[4].parse::<u64>().unwrap_or(0) * 1024 * 1024,
                    memory_free: fields[5].parse::<u64>().unwrap_or(0) * 1024 * 1024,
                    utilization_gpu: fields[6].parse().unwrap_or(0.0),
                    utilization_memory: fields[7].parse().unwrap_or(0.0),
                    temperature: fields[8].parse().ok(),
                    power_usage: fields[9].parse().ok(),
                    power_limit: fields[10].parse().ok(),
                    fan_speed: fields[11].parse().ok(),
                    clock_graphics: fields[12].parse().ok(),
                    clock_memory: fields[13].parse().ok(),
                    processes,
                    encoder_utilization: fields[14].parse().ok(),
                    decoder_utilization: fields[15].parse().ok(),
                };
                
                gpus.push(gpu);
            }
        }

        Ok(gpus)
    }

    fn get_nvidia_processes(&self, _gpu_id: u32) -> Result<Vec<GPUProcess>> {
        let mut processes = Vec::new();

        let output = Command::new("nvidia-smi")
            .args(&[
                "--query-compute-apps=pid,process_name,used_memory",
                "--format=csv,noheader,nounits"
            ])
            .output()?;

        let process_info = String::from_utf8_lossy(&output.stdout);
        
        for line in process_info.lines() {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if fields.len() >= 3 {
                let process = GPUProcess {
                    pid: fields[0].parse().unwrap_or(0),
                    process_name: fields[1].to_string(),
                    memory_usage: fields[2].parse::<u64>().unwrap_or(0) * 1024 * 1024, // Convert MB to bytes
                    gpu_utilization: None, // Would need additional nvidia-ml-py bindings
                    encode_utilization: None,
                    decode_utilization: None,
                    process_type: GPUProcessType::Compute,
                };
                
                processes.push(process);
            }
        }

        // Also check for graphics processes
        let graphics_output = Command::new("nvidia-smi")
            .args(&[
                "--query-apps=pid,process_name,used_memory",
                "--format=csv,noheader,nounits"
            ])
            .output();

        if let Ok(graphics_output) = graphics_output {
            let graphics_info = String::from_utf8_lossy(&graphics_output.stdout);
            
            for line in graphics_info.lines() {
                let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if fields.len() >= 3 {
                    let process = GPUProcess {
                        pid: fields[0].parse().unwrap_or(0),
                        process_name: fields[1].to_string(),
                        memory_usage: fields[2].parse::<u64>().unwrap_or(0) * 1024 * 1024,
                        gpu_utilization: None,
                        encode_utilization: None,
                        decode_utilization: None,
                        process_type: GPUProcessType::Graphics,
                    };
                    
                    processes.push(process);
                }
            }
        }

        Ok(processes)
    }

    fn get_amd_info(&self) -> Result<Vec<GPUInfo>> {
        let mut gpus = Vec::new();

        // Check for ROCm tools
        if Command::new("rocm-smi").arg("--showid").output().is_ok() {
            return self.get_amd_rocm_info();
        }

        // Fallback to sysfs parsing
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("card") && !name.contains("-") {
                        if let Ok(gpu) = self.parse_amd_sysfs(&path) {
                            gpus.push(gpu);
                        }
                    }
                }
            }
        }

        Ok(gpus)
    }

    fn get_amd_rocm_info(&self) -> Result<Vec<GPUInfo>> {
        let mut gpus = Vec::new();

        let output = Command::new("rocm-smi")
            .args(&["--showid", "--showproductname", "--showmeminfo", "--showuse", "--showtemp"])
            .output()?;

        let rocm_info = String::from_utf8_lossy(&output.stdout);
        
        // Parse ROCm output (format may vary)
        // This is a simplified parser - real implementation would need more robust parsing
        for line in rocm_info.lines() {
            if line.contains("GPU") && line.contains(":") {
                // Extract GPU information from ROCm output
                // This would need more sophisticated parsing in real implementation
                let gpu = GPUInfo {
                    gpu_id: 0, // Would extract from output
                    name: "AMD GPU".to_string(), // Would extract from output
                    vendor: GPUVendor::AMD,
                    driver_version: "Unknown".to_string(),
                    memory_total: 0, // Would extract from output
                    memory_used: 0,
                    memory_free: 0,
                    utilization_gpu: 0.0,
                    utilization_memory: 0.0,
                    temperature: None,
                    power_usage: None,
                    power_limit: None,
                    fan_speed: None,
                    clock_graphics: None,
                    clock_memory: None,
                    processes: Vec::new(), // Would need additional parsing
                    encoder_utilization: None,
                    decoder_utilization: None,
                };
                
                gpus.push(gpu);
            }
        }

        Ok(gpus)
    }

    fn parse_amd_sysfs(&self, card_path: &std::path::Path) -> Result<GPUInfo> {
        let device_path = card_path.join("device");
        
        // Read GPU name
        let name = fs::read_to_string(device_path.join("product_name"))
            .or_else(|_| fs::read_to_string(device_path.join("model")))
            .unwrap_or_else(|_| "AMD GPU".to_string())
            .trim()
            .to_string();

        // Read memory information from VRAM
        let memory_total = fs::read_to_string(device_path.join("mem_info_vram_total"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        let memory_used = fs::read_to_string(device_path.join("mem_info_vram_used"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        // Read GPU utilization
        let utilization_gpu = fs::read_to_string(device_path.join("gpu_busy_percent"))
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(0.0);

        // Read temperature
        let temperature = fs::read_to_string(device_path.join("hwmon").join("hwmon0").join("temp1_input"))
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|t| t / 1000.0); // Convert from millidegrees

        let gpu = GPUInfo {
            gpu_id: 0, // Would extract card number
            name,
            vendor: GPUVendor::AMD,
            driver_version: "Unknown".to_string(),
            memory_total,
            memory_used,
            memory_free: memory_total.saturating_sub(memory_used),
            utilization_gpu,
            utilization_memory: 0.0, // Not easily available from sysfs
            temperature,
            power_usage: None, // Would read from hwmon if available
            power_limit: None,
            fan_speed: None,
            clock_graphics: None,
            clock_memory: None,
            processes: Vec::new(), // AMD process tracking is more complex
            encoder_utilization: None,
            decoder_utilization: None,
        };

        Ok(gpu)
    }

    fn get_intel_info(&self) -> Result<Vec<GPUInfo>> {
        let mut gpus = Vec::new();

        // Check for Intel GPU tools
        if Command::new("intel_gpu_top").arg("--help").output().is_ok() {
            return self.get_intel_gpu_top_info();
        }

        // Fallback to sysfs parsing for Intel integrated graphics
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("card") && !name.contains("-") {
                        let vendor_path = path.join("device/vendor");
                        if let Ok(vendor) = fs::read_to_string(vendor_path) {
                            if vendor.trim() == "0x8086" { // Intel vendor ID
                                if let Ok(gpu) = self.parse_intel_sysfs(&path) {
                                    gpus.push(gpu);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(gpus)
    }

    fn get_intel_gpu_top_info(&self) -> Result<Vec<GPUInfo>> {
        // Intel GPU monitoring would use intel_gpu_top or similar tools
        // This is a placeholder implementation
        Ok(vec![GPUInfo {
            gpu_id: 0,
            name: "Intel Integrated Graphics".to_string(),
            vendor: GPUVendor::Intel,
            driver_version: "Unknown".to_string(),
            memory_total: 0, // Shared with system memory
            memory_used: 0,
            memory_free: 0,
            utilization_gpu: 0.0,
            utilization_memory: 0.0,
            temperature: None,
            power_usage: None,
            power_limit: None,
            fan_speed: None,
            clock_graphics: None,
            clock_memory: None,
            processes: Vec::new(),
            encoder_utilization: None,
            decoder_utilization: None,
        }])
    }

    fn parse_intel_sysfs(&self, card_path: &std::path::Path) -> Result<GPUInfo> {
        let _device_path = card_path.join("device");
        
        // Intel integrated graphics information is limited in sysfs
        let gpu = GPUInfo {
            gpu_id: 0,
            name: "Intel Integrated Graphics".to_string(),
            vendor: GPUVendor::Intel,
            driver_version: "Unknown".to_string(),
            memory_total: 0, // Shared memory
            memory_used: 0,
            memory_free: 0,
            utilization_gpu: 0.0,
            utilization_memory: 0.0,
            temperature: None,
            power_usage: None,
            power_limit: None,
            fan_speed: None,
            clock_graphics: None,
            clock_memory: None,
            processes: Vec::new(),
            encoder_utilization: None,
            decoder_utilization: None,
        };

        Ok(gpu)
    }

    fn update_process_tracking(&mut self, snapshot: &GPUSnapshot) {
        let now = Utc::now();
        
        // Update tracking for all GPU processes
        for gpu in &snapshot.gpus {
            for process in &gpu.processes {
                let usage = self.process_gpu_usage
                    .entry(process.pid)
                    .or_insert_with(|| GPUProcessUsage {
                        pid: process.pid,
                        memory_usage_history: VecDeque::new(),
                        utilization_history: VecDeque::new(),
                        last_seen: now,
                    });

                usage.memory_usage_history.push_back(process.memory_usage);
                usage.utilization_history.push_back(process.gpu_utilization.unwrap_or(0.0));
                usage.last_seen = now;

                // Limit history size
                if usage.memory_usage_history.len() > 100 {
                    usage.memory_usage_history.pop_front();
                }
                if usage.utilization_history.len() > 100 {
                    usage.utilization_history.pop_front();
                }
            }
        }

        // Clean up old process data
        let cutoff_time = now - chrono::Duration::minutes(10);
        self.process_gpu_usage.retain(|_, usage| usage.last_seen > cutoff_time);
    }

    pub fn get_latest_snapshot(&self) -> Option<&GPUSnapshot> {
        self.gpu_history.back()
    }

    pub fn get_gpu_history(&self) -> &VecDeque<GPUSnapshot> {
        &self.gpu_history
    }

    pub fn get_process_gpu_usage(&self, pid: u32) -> Option<&GPUProcessUsage> {
        self.process_gpu_usage.get(&pid)
    }

    pub fn get_total_gpu_memory_usage(&self) -> (u64, u64) {
        if let Some(snapshot) = self.get_latest_snapshot() {
            (snapshot.total_memory_used, snapshot.total_memory_available)
        } else {
            (0, 0)
        }
    }

    pub fn get_gpu_count(&self) -> usize {
        if let Some(snapshot) = self.get_latest_snapshot() {
            snapshot.gpus.len()
        } else {
            0
        }
    }

    pub fn has_gpu_support(&self) -> bool {
        self.nvidia_available || self.amd_available || self.intel_available
    }

    pub fn get_supported_vendors(&self) -> Vec<GPUVendor> {
        let mut vendors = Vec::new();
        if self.nvidia_available {
            vendors.push(GPUVendor::NVIDIA);
        }
        if self.amd_available {
            vendors.push(GPUVendor::AMD);
        }
        if self.intel_available {
            vendors.push(GPUVendor::Intel);
        }
        vendors
    }

    pub fn get_gpu_processes(&self) -> Vec<&GPUProcess> {
        if let Some(snapshot) = self.get_latest_snapshot() {
            snapshot.gpus.iter()
                .flat_map(|gpu| gpu.processes.iter())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn cleanup_old_data(&mut self) {
        let cutoff_time = Utc::now() - chrono::Duration::hours(1);
        
        // Clean GPU history
        while let Some(front) = self.gpu_history.front() {
            if front.timestamp < cutoff_time {
                self.gpu_history.pop_front();
            } else {
                break;
            }
        }
        
        // Clean process tracking
        self.process_gpu_usage.retain(|_, usage| usage.last_seen > cutoff_time);
    }
}