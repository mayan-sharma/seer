use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::fs;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::monitor::ProcessInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOBottleneckAnalyzer {
    process_io_history: HashMap<u32, VecDeque<IOSnapshot>>,
    system_io_history: VecDeque<SystemIOSnapshot>,
    bottleneck_alerts: Vec<IOBottleneckAlert>,
    analysis_settings: IOAnalysisSettings,
    last_system_stats: Option<SystemIOStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOSnapshot {
    pub timestamp: DateTime<Utc>,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_syscalls: u64,
    pub write_syscalls: u64,
    pub read_rate: f64,  // bytes/sec
    pub write_rate: f64, // bytes/sec
    pub io_wait_time: Option<f64>, // milliseconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIOSnapshot {
    pub timestamp: DateTime<Utc>,
    pub total_read_rate: f64,
    pub total_write_rate: f64,
    pub disk_utilization: HashMap<String, f64>,
    pub average_wait_time: f64,
    pub queue_depth: HashMap<String, f64>,
    pub io_operations_per_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIOStats {
    pub sectors_read: u64,
    pub sectors_written: u64,
    pub reads_completed: u64,
    pub writes_completed: u64,
    pub time_spent_reading: u64,
    pub time_spent_writing: u64,
    pub io_in_progress: u64,
    pub time_spent_io: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOBottleneckAlert {
    pub alert_id: String,
    pub alert_type: BottleneckType,
    pub severity: BottleneckSeverity,
    pub detected_at: DateTime<Utc>,
    pub affected_processes: Vec<u32>,
    pub bottleneck_source: String,
    pub metrics: BottleneckMetrics,
    pub suggested_actions: Vec<String>,
    pub trend_data: Vec<IOTrendPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    HighIOWait,
    DiskSaturation,
    ExcessiveRandomIO,
    SingleProcessDominance,
    IOStorm,
    SlowDisk,
    IOContentionDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckMetrics {
    pub io_wait_percentage: f64,
    pub disk_utilization: f64,
    pub avg_response_time_ms: f64,
    pub iops: f64,
    pub throughput_mbps: f64,
    pub queue_depth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOTrendPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub metric_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOAnalysisSettings {
    pub history_duration_minutes: u32,
    pub high_io_wait_threshold: f64,
    pub disk_utilization_threshold: f64,
    pub dominant_process_threshold: f64,
    pub io_storm_iops_threshold: f64,
    pub slow_disk_response_threshold_ms: f64,
    pub min_samples_for_analysis: usize,
}

impl Default for IOAnalysisSettings {
    fn default() -> Self {
        Self {
            history_duration_minutes: 30,
            high_io_wait_threshold: 20.0, // 20% IO wait
            disk_utilization_threshold: 90.0, // 90% disk utilization
            dominant_process_threshold: 80.0, // 80% of total IO
            io_storm_iops_threshold: 10000.0, // 10k IOPS threshold
            slow_disk_response_threshold_ms: 50.0, // 50ms response time
            min_samples_for_analysis: 5,
        }
    }
}

impl IOBottleneckAnalyzer {
    pub fn new() -> Self {
        Self {
            process_io_history: HashMap::new(),
            system_io_history: VecDeque::new(),
            bottleneck_alerts: Vec::new(),
            analysis_settings: IOAnalysisSettings::default(),
            last_system_stats: None,
        }
    }

    pub fn with_settings(settings: IOAnalysisSettings) -> Self {
        Self {
            process_io_history: HashMap::new(),
            system_io_history: VecDeque::new(),
            bottleneck_alerts: Vec::new(),
            analysis_settings: settings,
            last_system_stats: None,
        }
    }

    pub fn update_io_metrics(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        let now = Utc::now();
        
        // Update process IO metrics
        for process in processes {
            if let Ok(io_stats) = self.get_process_io_stats(process.pid) {
                // Get I/O wait time before mutable borrow
                let io_wait_time = self.get_process_io_wait(process.pid).ok();
                
                let history = self.process_io_history
                    .entry(process.pid)
                    .or_insert_with(VecDeque::new);

                // Calculate rates if we have previous data
                let (read_rate, write_rate) = if let Some(last_snapshot) = history.back() {
                    let time_diff = now.signed_duration_since(last_snapshot.timestamp);
                    let time_secs = time_diff.num_seconds() as f64;
                    
                    if time_secs > 0.0 {
                        let read_rate = (io_stats.read_bytes.saturating_sub(last_snapshot.read_bytes)) as f64 / time_secs;
                        let write_rate = (io_stats.write_bytes.saturating_sub(last_snapshot.write_bytes)) as f64 / time_secs;
                        (read_rate, write_rate)
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                };

                let snapshot = IOSnapshot {
                    timestamp: now,
                    read_bytes: io_stats.read_bytes,
                    write_bytes: io_stats.write_bytes,
                    read_syscalls: io_stats.read_syscalls,
                    write_syscalls: io_stats.write_syscalls,
                    read_rate,
                    write_rate,
                    io_wait_time,
                };

                history.push_back(snapshot);
                
                // Clean up old history
                let cutoff_time = now - chrono::Duration::minutes(self.analysis_settings.history_duration_minutes as i64);
                while let Some(front) = history.front() {
                    if front.timestamp < cutoff_time {
                        history.pop_front();
                    } else {
                        break;
                    }
                }
            }
        }

        // Update system IO metrics
        if let Ok(system_snapshot) = self.get_system_io_snapshot(now) {
            self.system_io_history.push_back(system_snapshot);
            self.cleanup_old_system_history(now);
        }

        // Analyze for bottlenecks
        self.analyze_io_bottlenecks(processes)?;

        // Clean up data for dead processes
        let active_pids: std::collections::HashSet<u32> = processes.iter().map(|p| p.pid).collect();
        self.process_io_history.retain(|pid, _| active_pids.contains(pid));

        Ok(())
    }

    fn get_process_io_stats(&self, pid: u32) -> Result<ProcessIOStats> {
        let io_path = format!("/proc/{}/io", pid);
        let content = fs::read_to_string(io_path)?;
        
        let mut stats = ProcessIOStats::default();
        
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value = parts[1].parse::<u64>().unwrap_or(0);
                match parts[0] {
                    "rchar:" => stats.read_bytes = value,
                    "wchar:" => stats.write_bytes = value,
                    "syscr:" => stats.read_syscalls = value,
                    "syscw:" => stats.write_syscalls = value,
                    "read_bytes:" => stats.actual_read_bytes = value,
                    "write_bytes:" => stats.actual_write_bytes = value,
                    _ => {}
                }
            }
        }
        
        Ok(stats)
    }

    fn get_process_io_wait(&self, pid: u32) -> Result<f64> {
        let stat_path = format!("/proc/{}/stat", pid);
        let content = fs::read_to_string(stat_path)?;
        
        let fields: Vec<&str> = content.split_whitespace().collect();
        if fields.len() > 41 {
            // Field 42 is delayacct_blkio_ticks (IO wait time in ticks)
            let io_wait_ticks = fields[41].parse::<u64>()?;
            let ticks_per_second = 100; // Typical value, should read from sysconf
            Ok(io_wait_ticks as f64 / ticks_per_second as f64 * 1000.0) // Convert to milliseconds
        } else {
            Err(anyhow::anyhow!("Invalid stat format"))
        }
    }

    fn get_system_io_snapshot(&mut self, timestamp: DateTime<Utc>) -> Result<SystemIOSnapshot> {
        let diskstats_content = fs::read_to_string("/proc/diskstats")?;
        let mut total_read_rate = 0.0;
        let mut total_write_rate = 0.0;
        let mut disk_utilization = HashMap::new();
        let mut queue_depth = HashMap::new();
        let mut total_operations = 0.0;
        let mut total_wait_time = 0.0;
        let mut _disk_count = 0;

        for line in diskstats_content.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 14 {
                let device_name = fields[2].to_string();
                
                // Skip loop devices and other virtual devices
                if device_name.starts_with("loop") || device_name.starts_with("ram") {
                    continue;
                }

                let sectors_read = fields[5].parse::<u64>().unwrap_or(0);
                let sectors_written = fields[9].parse::<u64>().unwrap_or(0);
                let reads_completed = fields[3].parse::<u64>().unwrap_or(0);
                let writes_completed = fields[7].parse::<u64>().unwrap_or(0);
                let time_spent_io = fields[12].parse::<u64>().unwrap_or(0);
                let weighted_time_io = fields[13].parse::<u64>().unwrap_or(0);

                // Calculate rates if we have previous system stats
                if let Some(ref last_stats) = self.last_system_stats {
                    let time_diff = 1.0; // Assume 1 second for now
                    let read_rate = (sectors_read.saturating_sub(last_stats.sectors_read)) as f64 * 512.0 / time_diff;
                    let write_rate = (sectors_written.saturating_sub(last_stats.sectors_written)) as f64 * 512.0 / time_diff;
                    
                    total_read_rate += read_rate;
                    total_write_rate += write_rate;
                    
                    // Calculate utilization (percentage of time spent doing IO)
                    let io_time_diff = time_spent_io.saturating_sub(last_stats.time_spent_io) as f64;
                    let utilization = (io_time_diff / 10.0).min(100.0); // Convert from milliseconds to percentage
                    disk_utilization.insert(device_name.clone(), utilization);
                    
                    // Estimate queue depth
                    let ops_diff = (reads_completed + writes_completed).saturating_sub(last_stats.reads_completed + last_stats.writes_completed) as f64;
                    let avg_queue_depth = if ops_diff > 0.0 {
                        weighted_time_io.saturating_sub(last_stats.time_spent_io) as f64 / ops_diff / 1000.0
                    } else {
                        0.0
                    };
                    queue_depth.insert(device_name, avg_queue_depth);
                    
                    total_operations += ops_diff;
                    total_wait_time += weighted_time_io.saturating_sub(last_stats.time_spent_io) as f64;
                }
                
                _disk_count += 1;
            }
        }

        // Update last system stats for next calculation
        self.last_system_stats = Some(SystemIOStats {
            sectors_read: 0, // Would need to store per-device stats for accuracy
            sectors_written: 0,
            reads_completed: 0,
            writes_completed: 0,
            time_spent_reading: 0,
            time_spent_writing: 0,
            io_in_progress: 0,
            time_spent_io: 0,
        });

        let average_wait_time = if total_operations > 0.0 {
            total_wait_time / total_operations
        } else {
            0.0
        };

        Ok(SystemIOSnapshot {
            timestamp,
            total_read_rate,
            total_write_rate,
            disk_utilization,
            average_wait_time,
            queue_depth,
            io_operations_per_sec: total_operations,
        })
    }

    fn analyze_io_bottlenecks(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        if self.system_io_history.len() < self.analysis_settings.min_samples_for_analysis {
            return Ok(());
        }

        // Clear old alerts
        self.bottleneck_alerts.clear();

        // Analyze different types of bottlenecks
        self.detect_high_io_wait()?;
        self.detect_disk_saturation()?;
        self.detect_dominant_process(processes)?;
        self.detect_io_storm()?;
        self.detect_slow_disk_response()?;
        self.detect_io_contention(processes)?;

        Ok(())
    }

    fn detect_high_io_wait(&mut self) -> Result<()> {
        if let Some(latest_snapshot) = self.system_io_history.back() {
            let avg_utilization: f64 = latest_snapshot.disk_utilization.values().sum::<f64>() 
                / latest_snapshot.disk_utilization.len().max(1) as f64;
            
            if avg_utilization > self.analysis_settings.high_io_wait_threshold {
                let trend_data = self.build_system_trend_data("io_wait");
                
                let alert = IOBottleneckAlert {
                    alert_id: format!("io_wait_{}", Utc::now().timestamp()),
                    alert_type: BottleneckType::HighIOWait,
                    severity: self.calculate_io_severity(avg_utilization, 50.0),
                    detected_at: Utc::now(),
                    affected_processes: Vec::new(),
                    bottleneck_source: "System-wide high IO wait".to_string(),
                    metrics: BottleneckMetrics {
                        io_wait_percentage: avg_utilization,
                        disk_utilization: avg_utilization,
                        avg_response_time_ms: latest_snapshot.average_wait_time,
                        iops: latest_snapshot.io_operations_per_sec,
                        throughput_mbps: (latest_snapshot.total_read_rate + latest_snapshot.total_write_rate) / (1024.0 * 1024.0),
                        queue_depth: latest_snapshot.queue_depth.values().sum::<f64>() / latest_snapshot.queue_depth.len().max(1) as f64,
                    },
                    suggested_actions: vec![
                        "Check disk health and performance".to_string(),
                        "Consider IO scheduling optimization".to_string(),
                        "Monitor for disk errors".to_string(),
                    ],
                    trend_data,
                };
                
                self.bottleneck_alerts.push(alert);
            }
        }
        Ok(())
    }

    fn detect_disk_saturation(&mut self) -> Result<()> {
        if let Some(latest_snapshot) = self.system_io_history.back() {
            for (disk, utilization) in &latest_snapshot.disk_utilization {
                if *utilization > self.analysis_settings.disk_utilization_threshold {
                    let trend_data = self.build_disk_trend_data(disk);
                    
                    let alert = IOBottleneckAlert {
                        alert_id: format!("disk_sat_{}_{}", disk, Utc::now().timestamp()),
                        alert_type: BottleneckType::DiskSaturation,
                        severity: self.calculate_io_severity(*utilization, 95.0),
                        detected_at: Utc::now(),
                        affected_processes: Vec::new(),
                        bottleneck_source: format!("Disk {} saturated", disk),
                        metrics: BottleneckMetrics {
                            io_wait_percentage: *utilization,
                            disk_utilization: *utilization,
                            avg_response_time_ms: latest_snapshot.average_wait_time,
                            iops: latest_snapshot.io_operations_per_sec,
                            throughput_mbps: (latest_snapshot.total_read_rate + latest_snapshot.total_write_rate) / (1024.0 * 1024.0),
                            queue_depth: latest_snapshot.queue_depth.get(disk).copied().unwrap_or(0.0),
                        },
                        suggested_actions: vec![
                            format!("Check {} disk performance", disk),
                            "Consider workload balancing".to_string(),
                            "Evaluate storage upgrade".to_string(),
                        ],
                        trend_data,
                    };
                    
                    self.bottleneck_alerts.push(alert);
                }
            }
        }
        Ok(())
    }

    fn detect_dominant_process(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        if let Some(latest_system) = self.system_io_history.back() {
            let total_io_rate = latest_system.total_read_rate + latest_system.total_write_rate;
            
            if total_io_rate > 0.0 {
                for process in processes {
                    if let Some(history) = self.process_io_history.get(&process.pid) {
                        if let Some(latest_process) = history.back() {
                            let process_io_rate = latest_process.read_rate + latest_process.write_rate;
                            let dominance_percentage = (process_io_rate / total_io_rate) * 100.0;
                            
                            if dominance_percentage > self.analysis_settings.dominant_process_threshold {
                                let trend_data = self.build_process_trend_data(process.pid);
                                
                                let alert = IOBottleneckAlert {
                                    alert_id: format!("dominant_{}_{}", process.pid, Utc::now().timestamp()),
                                    alert_type: BottleneckType::SingleProcessDominance,
                                    severity: self.calculate_io_severity(dominance_percentage, 90.0),
                                    detected_at: Utc::now(),
                                    affected_processes: vec![process.pid],
                                    bottleneck_source: format!("Process {} ({}) dominating IO", process.pid, process.name),
                                    metrics: BottleneckMetrics {
                                        io_wait_percentage: dominance_percentage,
                                        disk_utilization: 0.0,
                                        avg_response_time_ms: latest_process.io_wait_time.unwrap_or(0.0),
                                        iops: (latest_process.read_syscalls + latest_process.write_syscalls) as f64,
                                        throughput_mbps: process_io_rate / (1024.0 * 1024.0),
                                        queue_depth: 0.0,
                                    },
                                    suggested_actions: vec![
                                        format!("Investigate process {} IO patterns", process.name),
                                        "Consider process IO throttling".to_string(),
                                        "Check if process behavior is expected".to_string(),
                                    ],
                                    trend_data,
                                };
                                
                                self.bottleneck_alerts.push(alert);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn detect_io_storm(&mut self) -> Result<()> {
        if let Some(latest_snapshot) = self.system_io_history.back() {
            if latest_snapshot.io_operations_per_sec > self.analysis_settings.io_storm_iops_threshold {
                let trend_data = self.build_system_trend_data("iops");
                
                let alert = IOBottleneckAlert {
                    alert_id: format!("io_storm_{}", Utc::now().timestamp()),
                    alert_type: BottleneckType::IOStorm,
                    severity: self.calculate_io_severity(latest_snapshot.io_operations_per_sec, 50000.0),
                    detected_at: Utc::now(),
                    affected_processes: Vec::new(),
                    bottleneck_source: "System experiencing IO storm".to_string(),
                    metrics: BottleneckMetrics {
                        io_wait_percentage: 0.0,
                        disk_utilization: latest_snapshot.disk_utilization.values().sum::<f64>() / latest_snapshot.disk_utilization.len().max(1) as f64,
                        avg_response_time_ms: latest_snapshot.average_wait_time,
                        iops: latest_snapshot.io_operations_per_sec,
                        throughput_mbps: (latest_snapshot.total_read_rate + latest_snapshot.total_write_rate) / (1024.0 * 1024.0),
                        queue_depth: latest_snapshot.queue_depth.values().sum::<f64>() / latest_snapshot.queue_depth.len().max(1) as f64,
                    },
                    suggested_actions: vec![
                        "Identify processes causing high IOPS".to_string(),
                        "Consider IO rate limiting".to_string(),
                        "Check for runaway processes".to_string(),
                    ],
                    trend_data,
                };
                
                self.bottleneck_alerts.push(alert);
            }
        }
        Ok(())
    }

    fn detect_slow_disk_response(&mut self) -> Result<()> {
        if let Some(latest_snapshot) = self.system_io_history.back() {
            if latest_snapshot.average_wait_time > self.analysis_settings.slow_disk_response_threshold_ms {
                let trend_data = self.build_system_trend_data("response_time");
                
                let alert = IOBottleneckAlert {
                    alert_id: format!("slow_disk_{}", Utc::now().timestamp()),
                    alert_type: BottleneckType::SlowDisk,
                    severity: self.calculate_io_severity(latest_snapshot.average_wait_time, 100.0),
                    detected_at: Utc::now(),
                    affected_processes: Vec::new(),
                    bottleneck_source: "Slow disk response times detected".to_string(),
                    metrics: BottleneckMetrics {
                        io_wait_percentage: 0.0,
                        disk_utilization: latest_snapshot.disk_utilization.values().sum::<f64>() / latest_snapshot.disk_utilization.len().max(1) as f64,
                        avg_response_time_ms: latest_snapshot.average_wait_time,
                        iops: latest_snapshot.io_operations_per_sec,
                        throughput_mbps: (latest_snapshot.total_read_rate + latest_snapshot.total_write_rate) / (1024.0 * 1024.0),
                        queue_depth: latest_snapshot.queue_depth.values().sum::<f64>() / latest_snapshot.queue_depth.len().max(1) as f64,
                    },
                    suggested_actions: vec![
                        "Check disk health with SMART tools".to_string(),
                        "Monitor disk temperatures".to_string(),
                        "Consider disk replacement if failing".to_string(),
                    ],
                    trend_data,
                };
                
                self.bottleneck_alerts.push(alert);
            }
        }
        Ok(())
    }

    fn detect_io_contention(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        // Look for multiple processes with high IO wait times
        let mut high_wait_processes = Vec::new();
        
        for process in processes {
            if let Some(history) = self.process_io_history.get(&process.pid) {
                if let Some(latest) = history.back() {
                    if let Some(wait_time) = latest.io_wait_time {
                        if wait_time > 10.0 { // 10ms threshold
                            high_wait_processes.push((process.pid, wait_time));
                        }
                    }
                }
            }
        }
        
        if high_wait_processes.len() >= 3 {
            let avg_wait: f64 = high_wait_processes.iter().map(|(_, wait)| wait).sum::<f64>() / high_wait_processes.len() as f64;
            let affected_pids: Vec<u32> = high_wait_processes.iter().map(|(pid, _)| *pid).collect();
            
            let alert = IOBottleneckAlert {
                alert_id: format!("io_contention_{}", Utc::now().timestamp()),
                alert_type: BottleneckType::IOContentionDetected,
                severity: self.calculate_io_severity(avg_wait, 50.0),
                detected_at: Utc::now(),
                affected_processes: affected_pids,
                bottleneck_source: format!("{} processes experiencing IO contention", high_wait_processes.len()),
                metrics: BottleneckMetrics {
                    io_wait_percentage: 0.0,
                    disk_utilization: 0.0,
                    avg_response_time_ms: avg_wait,
                    iops: 0.0,
                    throughput_mbps: 0.0,
                    queue_depth: 0.0,
                },
                suggested_actions: vec![
                    "Analyze IO access patterns".to_string(),
                    "Consider workload scheduling".to_string(),
                    "Evaluate storage architecture".to_string(),
                ],
                trend_data: Vec::new(),
            };
            
            self.bottleneck_alerts.push(alert);
        }
        
        Ok(())
    }

    fn calculate_io_severity(&self, value: f64, critical_threshold: f64) -> BottleneckSeverity {
        let ratio = value / critical_threshold;
        
        if ratio >= 2.0 {
            BottleneckSeverity::Critical
        } else if ratio >= 1.5 {
            BottleneckSeverity::High
        } else if ratio >= 1.0 {
            BottleneckSeverity::Medium
        } else {
            BottleneckSeverity::Low
        }
    }

    fn build_system_trend_data(&self, metric_type: &str) -> Vec<IOTrendPoint> {
        self.system_io_history.iter().map(|snapshot| {
            let value = match metric_type {
                "io_wait" => snapshot.disk_utilization.values().sum::<f64>() / snapshot.disk_utilization.len().max(1) as f64,
                "iops" => snapshot.io_operations_per_sec,
                "response_time" => snapshot.average_wait_time,
                _ => 0.0,
            };
            
            IOTrendPoint {
                timestamp: snapshot.timestamp,
                value,
                metric_type: metric_type.to_string(),
            }
        }).collect()
    }

    fn build_disk_trend_data(&self, disk_name: &str) -> Vec<IOTrendPoint> {
        self.system_io_history.iter().filter_map(|snapshot| {
            snapshot.disk_utilization.get(disk_name).map(|&utilization| {
                IOTrendPoint {
                    timestamp: snapshot.timestamp,
                    value: utilization,
                    metric_type: "disk_utilization".to_string(),
                }
            })
        }).collect()
    }

    fn build_process_trend_data(&self, pid: u32) -> Vec<IOTrendPoint> {
        if let Some(history) = self.process_io_history.get(&pid) {
            history.iter().map(|snapshot| {
                IOTrendPoint {
                    timestamp: snapshot.timestamp,
                    value: snapshot.read_rate + snapshot.write_rate,
                    metric_type: "io_rate".to_string(),
                }
            }).collect()
        } else {
            Vec::new()
        }
    }


    fn cleanup_old_system_history(&mut self, now: DateTime<Utc>) {
        let cutoff_time = now - chrono::Duration::minutes(self.analysis_settings.history_duration_minutes as i64);
        while let Some(front) = self.system_io_history.front() {
            if front.timestamp < cutoff_time {
                self.system_io_history.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_bottleneck_alerts(&self) -> &[IOBottleneckAlert] {
        &self.bottleneck_alerts
    }

    pub fn get_process_io_history(&self, pid: u32) -> Option<&VecDeque<IOSnapshot>> {
        self.process_io_history.get(&pid)
    }

    pub fn get_system_io_history(&self) -> &VecDeque<SystemIOSnapshot> {
        &self.system_io_history
    }
}

#[derive(Debug, Clone, Default)]
struct ProcessIOStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_syscalls: u64,
    pub write_syscalls: u64,
    pub actual_read_bytes: u64,
    pub actual_write_bytes: u64,
}