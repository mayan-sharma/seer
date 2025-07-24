use anyhow::Result;
use std::fs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessAffinity {
    pub pid: u32,
    pub cpu_mask: Vec<bool>,
    pub allowed_cpus: Vec<usize>,
    pub cpu_count: usize,
}

impl ProcessAffinity {
    pub fn new(pid: u32, cpu_count: usize) -> Self {
        Self {
            pid,
            cpu_mask: vec![true; cpu_count],
            allowed_cpus: (0..cpu_count).collect(),
            cpu_count,
        }
    }

    pub fn from_mask_string(pid: u32, mask_str: &str, cpu_count: usize) -> Result<Self> {
        let mut cpu_mask = vec![false; cpu_count];
        let mut allowed_cpus = Vec::new();

        // Parse CPU mask from string (e.g., "ff" for 8 CPUs all enabled)
        if let Ok(mask_value) = u64::from_str_radix(mask_str.trim_start_matches("0x"), 16) {
            for i in 0..cpu_count {
                if mask_value & (1 << i) != 0 {
                    cpu_mask[i] = true;
                    allowed_cpus.push(i);
                }
            }
        }

        Ok(Self {
            pid,
            cpu_mask,
            allowed_cpus,
            cpu_count,
        })
    }

    pub fn to_mask_string(&self) -> String {
        let mut mask_value = 0u64;
        for (i, &enabled) in self.cpu_mask.iter().enumerate() {
            if enabled {
                mask_value |= 1 << i;
            }
        }
        format!("0x{:x}", mask_value)
    }

    pub fn set_cpu(&mut self, cpu_index: usize, enabled: bool) -> Result<()> {
        if cpu_index >= self.cpu_count {
            return Err(anyhow::anyhow!("CPU index {} out of range (max: {})", cpu_index, self.cpu_count - 1));
        }

        self.cpu_mask[cpu_index] = enabled;
        self.allowed_cpus.clear();
        
        for (i, &enabled) in self.cpu_mask.iter().enumerate() {
            if enabled {
                self.allowed_cpus.push(i);
            }
        }

        Ok(())
    }
}

pub struct AffinityManager;

impl AffinityManager {
    /// Get current CPU affinity for a process (Linux only)
    pub fn get_process_affinity(pid: u32) -> Result<ProcessAffinity> {
        #[cfg(target_os = "linux")]
        {
            let status_path = format!("/proc/{}/status", pid);
            let status_content = fs::read_to_string(&status_path)
                .map_err(|e| anyhow::anyhow!("Failed to read process status: {}", e))?;

            let cpu_count = num_cpus::get();
            let mut affinity = ProcessAffinity::new(pid, cpu_count);

            // Look for Cpus_allowed field in /proc/pid/status
            for line in status_content.lines() {
                if line.starts_with("Cpus_allowed:") {
                    if let Some(mask_str) = line.split_whitespace().nth(1) {
                        affinity = ProcessAffinity::from_mask_string(pid, mask_str, cpu_count)?;
                    }
                    break;
                }
            }

            Ok(affinity)
        }

        #[cfg(not(target_os = "linux"))]
        {
            // For non-Linux systems, return a default affinity with all CPUs enabled
            let cpu_count = num_cpus::get();
            Ok(ProcessAffinity::new(pid, cpu_count))
        }
    }

    /// Set CPU affinity for a process using taskset command
    pub fn set_process_affinity(pid: u32, affinity: &ProcessAffinity) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            let mask_str = affinity.to_mask_string();
            let output = Command::new("taskset")
                .args(&["-p", &mask_str, &pid.to_string()])
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to execute taskset: {}", e))?;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("taskset failed: {}", error_msg));
            }

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(anyhow::anyhow!("CPU affinity modification is only supported on Linux"))
        }
    }

    /// Get list of processes with their current CPU affinity
    pub fn get_all_process_affinities(pids: &[u32]) -> Vec<ProcessAffinity> {
        let mut affinities = Vec::new();
        
        for &pid in pids {
            if let Ok(affinity) = Self::get_process_affinity(pid) {
                affinities.push(affinity);
            }
        }
        
        affinities
    }

    /// Check if taskset command is available
    pub fn is_taskset_available() -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            Command::new("taskset")
                .arg("--help")
                .output()
                .is_ok()
        }

        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    /// Get CPU topology information
    pub fn get_cpu_topology() -> Result<CpuTopology> {
        let cpu_count = num_cpus::get();
        let physical_count = num_cpus::get_physical();
        
        Ok(CpuTopology {
            logical_cpus: cpu_count,
            physical_cpus: physical_count,
            threads_per_core: cpu_count / physical_count,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuTopology {
    pub logical_cpus: usize,
    pub physical_cpus: usize,
    pub threads_per_core: usize,
}

impl CpuTopology {
    pub fn get_core_siblings(&self, cpu_index: usize) -> Vec<usize> {
        let mut siblings = Vec::new();
        let core_id = cpu_index / self.threads_per_core;
        
        for i in 0..self.threads_per_core {
            let sibling = core_id * self.threads_per_core + i;
            if sibling < self.logical_cpus {
                siblings.push(sibling);
            }
        }
        
        siblings
    }
}