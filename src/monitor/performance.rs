use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

const MAX_HISTORY_SIZE: usize = 300; // 5 minutes at 1-second intervals

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPerformanceData {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub memory_percentage: f32,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub threads_count: usize,
    pub context_switches: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPerformanceProfile {
    pub pid: u32,
    pub name: String,
    pub start_time: DateTime<Utc>,
    pub history: VecDeque<ProcessPerformanceData>,
    pub statistics: PerformanceStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStatistics {
    pub avg_cpu_usage: f32,
    pub max_cpu_usage: f32,
    pub min_cpu_usage: f32,
    pub avg_memory_usage: u64,
    pub max_memory_usage: u64,
    pub min_memory_usage: u64,
    pub total_cpu_time: f32,
    pub cpu_usage_variance: f32,
    pub memory_growth_rate: f32, // bytes per second
    pub io_read_total: u64,
    pub io_write_total: u64,
    pub context_switches_total: u64,
    pub uptime_seconds: u64,
}

impl PerformanceStatistics {
    pub fn new() -> Self {
        Self {
            avg_cpu_usage: 0.0,
            max_cpu_usage: 0.0,
            min_cpu_usage: 0.0,
            avg_memory_usage: 0,
            max_memory_usage: 0,
            min_memory_usage: u64::MAX,
            total_cpu_time: 0.0,
            cpu_usage_variance: 0.0,
            memory_growth_rate: 0.0,
            io_read_total: 0,
            io_write_total: 0,
            context_switches_total: 0,
            uptime_seconds: 0,
        }
    }

    pub fn calculate_from_history(history: &VecDeque<ProcessPerformanceData>) -> Self {
        if history.is_empty() {
            return Self::new();
        }

        let mut stats = Self::new();
        let count = history.len() as f32;

        // Calculate basic statistics
        let mut cpu_sum = 0.0;
        let mut memory_sum = 0u64;
        
        for data in history {
            cpu_sum += data.cpu_usage;
            memory_sum += data.memory_usage;
            
            stats.max_cpu_usage = stats.max_cpu_usage.max(data.cpu_usage);
            stats.min_cpu_usage = if stats.min_cpu_usage == 0.0 { data.cpu_usage } else { stats.min_cpu_usage.min(data.cpu_usage) };
            stats.max_memory_usage = stats.max_memory_usage.max(data.memory_usage);
            if stats.min_memory_usage == u64::MAX {
                stats.min_memory_usage = data.memory_usage;
            } else {
                stats.min_memory_usage = stats.min_memory_usage.min(data.memory_usage);
            }
            
            stats.io_read_total = data.io_read_bytes; // Latest values
            stats.io_write_total = data.io_write_bytes;
            stats.context_switches_total = data.context_switches;
        }

        stats.avg_cpu_usage = cpu_sum / count;
        stats.avg_memory_usage = memory_sum / count as u64;
        stats.total_cpu_time = cpu_sum; // Simplified for now

        // Calculate CPU usage variance
        let mut variance_sum = 0.0;
        for data in history {
            let diff = data.cpu_usage - stats.avg_cpu_usage;
            variance_sum += diff * diff;
        }
        stats.cpu_usage_variance = variance_sum / count;

        // Calculate memory growth rate
        if let (Some(first), Some(last)) = (history.front(), history.back()) {
            let time_diff = last.timestamp.timestamp() - first.timestamp.timestamp();
            if time_diff > 0 {
                let memory_diff = last.memory_usage as i64 - first.memory_usage as i64;
                stats.memory_growth_rate = memory_diff as f32 / time_diff as f32;
                stats.uptime_seconds = time_diff as u64;
            }
        }

        if stats.min_memory_usage == u64::MAX {
            stats.min_memory_usage = 0;
        }

        stats
    }
}

impl ProcessPerformanceProfile {
    pub fn new(pid: u32, name: String) -> Self {
        Self {
            pid,
            name,
            start_time: Utc::now(),
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            statistics: PerformanceStatistics::new(),
        }
    }

    pub fn add_data_point(&mut self, data: ProcessPerformanceData) {
        if self.history.len() >= MAX_HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(data);
        self.statistics = PerformanceStatistics::calculate_from_history(&self.history);
    }

    pub fn get_trend(&self, metric: &str) -> ProcessTrend {
        if self.history.len() < 2 {
            return ProcessTrend::Stable;
        }

        let recent_count = (self.history.len() / 4).max(3).min(10);
        let recent_data: Vec<_> = self.history.iter().rev().take(recent_count).collect();
        
        match metric {
            "cpu" => {
                let recent_avg = recent_data.iter().map(|d| d.cpu_usage).sum::<f32>() / recent_data.len() as f32;
                let overall_avg = self.statistics.avg_cpu_usage;
                
                if recent_avg > overall_avg * 1.2 {
                    ProcessTrend::Increasing
                } else if recent_avg < overall_avg * 0.8 {
                    ProcessTrend::Decreasing
                } else {
                    ProcessTrend::Stable
                }
            },
            "memory" => {
                let recent_avg = recent_data.iter().map(|d| d.memory_usage).sum::<u64>() / recent_data.len() as u64;
                let overall_avg = self.statistics.avg_memory_usage;
                
                if recent_avg > overall_avg + (overall_avg / 10) {
                    ProcessTrend::Increasing
                } else if recent_avg < overall_avg.saturating_sub(overall_avg / 10) {
                    ProcessTrend::Decreasing
                } else {
                    ProcessTrend::Stable
                }
            },
            _ => ProcessTrend::Stable,
        }
    }

    pub fn get_anomalies(&self) -> Vec<PerformanceAnomaly> {
        let mut anomalies = Vec::new();
        
        // Check for CPU spikes
        for data in &self.history {
            if data.cpu_usage > self.statistics.avg_cpu_usage + (2.0 * self.statistics.cpu_usage_variance.sqrt()) {
                anomalies.push(PerformanceAnomaly {
                    timestamp: data.timestamp,
                    anomaly_type: AnomalyType::CpuSpike,
                    value: data.cpu_usage as f64,
                    expected_range: (self.statistics.min_cpu_usage as f64, self.statistics.max_cpu_usage as f64),
                    severity: if data.cpu_usage > 90.0 { AnomalySeverity::High } else { AnomalySeverity::Medium },
                });
            }
        }

        // Check for memory leaks (sustained growth)
        if self.statistics.memory_growth_rate > 1024.0 * 1024.0 { // 1MB/sec growth
            anomalies.push(PerformanceAnomaly {
                timestamp: Utc::now(),
                anomaly_type: AnomalyType::MemoryLeak,
                value: self.statistics.memory_growth_rate as f64,
                expected_range: (-1024.0 * 1024.0, 1024.0 * 1024.0), // -1MB to +1MB/sec
                severity: if self.statistics.memory_growth_rate > 10.0 * 1024.0 * 1024.0 { 
                    AnomalySeverity::High 
                } else { 
                    AnomalySeverity::Medium 
                },
            });
        }

        anomalies
    }

    pub fn is_resource_intensive(&self) -> bool {
        self.statistics.avg_cpu_usage > 50.0 || 
        self.statistics.avg_memory_usage > 1024 * 1024 * 1024 // 1GB
    }

    pub fn get_efficiency_score(&self) -> f32 {
        // Simple efficiency score based on resource usage stability
        let cpu_stability = 1.0 - (self.statistics.cpu_usage_variance.sqrt() / 100.0);
        let memory_stability = if self.statistics.memory_growth_rate.abs() < 1024.0 * 1024.0 { 1.0 } else { 0.5 };
        
        (cpu_stability + memory_stability) / 2.0 * 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessTrend {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnomaly {
    pub timestamp: DateTime<Utc>,
    pub anomaly_type: AnomalyType,
    pub value: f64,
    pub expected_range: (f64, f64),
    pub severity: AnomalySeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    CpuSpike,
    MemoryLeak,
    HighIoActivity,
    ThreadExplosion,
    ContextSwitchStorm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
}

pub struct PerformanceProfiler {
    profiles: HashMap<u32, ProcessPerformanceProfile>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    pub fn update_process(&mut self, process_info: &crate::monitor::ProcessInfo) {
        let data = ProcessPerformanceData {
            timestamp: Utc::now(),
            cpu_usage: process_info.cpu_usage,
            memory_usage: process_info.memory_usage,
            memory_percentage: process_info.memory_percentage,
            io_read_bytes: 0,  // Would need additional system calls to get IO stats
            io_write_bytes: 0,
            threads_count: process_info.threads_count,
            context_switches: 0, // Would need additional system calls
        };

        let profile = self.profiles
            .entry(process_info.pid)
            .or_insert_with(|| ProcessPerformanceProfile::new(process_info.pid, process_info.name.clone()));
        
        profile.add_data_point(data);
    }

    pub fn get_profile(&self, pid: u32) -> Option<&ProcessPerformanceProfile> {
        self.profiles.get(&pid)
    }

    pub fn get_all_profiles(&self) -> &HashMap<u32, ProcessPerformanceProfile> {
        &self.profiles
    }

    pub fn get_resource_intensive_processes(&self) -> Vec<&ProcessPerformanceProfile> {
        self.profiles
            .values()
            .filter(|profile| profile.is_resource_intensive())
            .collect()
    }

    pub fn get_anomalous_processes(&self) -> Vec<(&ProcessPerformanceProfile, Vec<PerformanceAnomaly>)> {
        self.profiles
            .values()
            .filter_map(|profile| {
                let anomalies = profile.get_anomalies();
                if anomalies.is_empty() {
                    None
                } else {
                    Some((profile, anomalies))
                }
            })
            .collect()
    }

    pub fn cleanup_old_profiles(&mut self, active_pids: &[u32]) {
        let active_set: std::collections::HashSet<u32> = active_pids.iter().cloned().collect();
        self.profiles.retain(|&pid, _| active_set.contains(&pid));
    }

    pub fn get_top_cpu_consumers(&self, count: usize) -> Vec<&ProcessPerformanceProfile> {
        let mut profiles: Vec<_> = self.profiles.values().collect();
        profiles.sort_by(|a, b| b.statistics.avg_cpu_usage.partial_cmp(&a.statistics.avg_cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
        profiles.into_iter().take(count).collect()
    }

    pub fn get_top_memory_consumers(&self, count: usize) -> Vec<&ProcessPerformanceProfile> {
        let mut profiles: Vec<_> = self.profiles.values().collect();
        profiles.sort_by(|a, b| b.statistics.avg_memory_usage.cmp(&a.statistics.avg_memory_usage));
        profiles.into_iter().take(count).collect()
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}