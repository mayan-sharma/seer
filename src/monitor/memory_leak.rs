use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use crate::monitor::ProcessInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeakDetector {
    process_memory_history: HashMap<u32, VecDeque<MemorySnapshot>>,
    leak_alerts: Vec<MemoryLeakAlert>,
    detection_settings: LeakDetectionSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub timestamp: DateTime<Utc>,
    pub virtual_memory: u64,
    pub resident_memory: u64,
    pub shared_memory: u64,
    pub heap_size: Option<u64>,
    pub stack_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeakAlert {
    pub pid: u32,
    pub process_name: String,
    pub alert_type: LeakAlertType,
    pub severity: LeakSeverity,
    pub detected_at: DateTime<Utc>,
    pub growth_rate: f64, // MB per minute
    pub current_memory: u64,
    pub baseline_memory: u64,
    pub trend_data: Vec<MemoryTrendPoint>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeakAlertType {
    SteadyGrowth,
    RapidIncrease,
    MemorySpike,
    SuspiciousPattern,
    FragmentationIncrease,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeakSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTrendPoint {
    pub timestamp: DateTime<Utc>,
    pub memory_usage: u64,
    pub growth_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakDetectionSettings {
    pub history_duration_minutes: u32,
    pub min_growth_threshold_mb: f64,
    pub rapid_growth_threshold_mb_per_min: f64,
    pub spike_threshold_multiplier: f64,
    pub min_samples_for_detection: usize,
    pub fragmentation_threshold: f64,
}

impl Default for LeakDetectionSettings {
    fn default() -> Self {
        Self {
            history_duration_minutes: 60, // 1 hour of history
            min_growth_threshold_mb: 10.0, // 10MB minimum growth to consider
            rapid_growth_threshold_mb_per_min: 5.0, // 5MB/min is rapid growth
            spike_threshold_multiplier: 2.0, // 2x sudden increase
            min_samples_for_detection: 10, // Need at least 10 samples
            fragmentation_threshold: 0.3, // 30% fragmentation threshold
        }
    }
}

impl MemoryLeakDetector {
    pub fn new() -> Self {
        Self {
            process_memory_history: HashMap::new(),
            leak_alerts: Vec::new(),
            detection_settings: LeakDetectionSettings::default(),
        }
    }

    pub fn with_settings(settings: LeakDetectionSettings) -> Self {
        Self {
            process_memory_history: HashMap::new(),
            leak_alerts: Vec::new(),
            detection_settings: settings,
        }
    }

    pub fn update_process_memory(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        let now = Utc::now();
        
        for process in processes {
            let snapshot = MemorySnapshot {
                timestamp: now,
                virtual_memory: process.memory_usage, // Using memory_usage as approximation
                resident_memory: process.memory_usage,
                shared_memory: 0, // TODO: Implement shared memory reading
                heap_size: self.get_heap_size(process.pid).ok(),
                stack_size: self.get_stack_size(process.pid).ok(),
            };

            let history = self.process_memory_history
                .entry(process.pid)
                .or_insert_with(VecDeque::new);

            history.push_back(snapshot);

            // Limit history size based on duration
            let cutoff_time = now - Duration::minutes(self.detection_settings.history_duration_minutes as i64);
            while let Some(front) = history.front() {
                if front.timestamp < cutoff_time {
                    history.pop_front();
                } else {
                    break;
                }
            }

            // Analyze for memory leaks
            if history.len() >= self.detection_settings.min_samples_for_detection {
                self.analyze_process_for_leaks(process)?;
            }
        }

        // Clean up history for dead processes
        let active_pids: std::collections::HashSet<u32> = processes.iter().map(|p| p.pid).collect();
        self.process_memory_history.retain(|pid, _| active_pids.contains(pid));

        Ok(())
    }

    fn analyze_process_for_leaks(&mut self, process: &ProcessInfo) -> Result<()> {
        // Clone the history to avoid borrow checker issues
        let history = if let Some(history) = self.process_memory_history.get(&process.pid) {
            if history.len() < self.detection_settings.min_samples_for_detection {
                return Ok(());
            }
            history.clone()
        } else {
            return Ok(());
        };

        let mut alerts = Vec::new();

        // Check for steady growth
        if let Some(alert) = self.detect_steady_growth(process, &history)? {
            alerts.push(alert);
        }

        // Check for rapid increase
        if let Some(alert) = self.detect_rapid_increase(process, &history)? {
            alerts.push(alert);
        }

        // Check for memory spikes
        if let Some(alert) = self.detect_memory_spikes(process, &history)? {
            alerts.push(alert);
        }

        // Check for suspicious patterns
        if let Some(alert) = self.detect_suspicious_patterns(process, &history)? {
            alerts.push(alert);
        }

        // Add all alerts at once
        for alert in alerts {
            self.add_or_update_alert(alert);
        }

        Ok(())
    }

    fn detect_steady_growth(&self, process: &ProcessInfo, history: &VecDeque<MemorySnapshot>) -> Result<Option<MemoryLeakAlert>> {
        if history.len() < 5 {
            return Ok(None);
        }

        let recent_samples = history.iter().rev().take(10).collect::<Vec<_>>();
        let baseline = recent_samples.last().unwrap().resident_memory;
        let current = recent_samples.first().unwrap().resident_memory;
        
        let growth_mb = (current as f64 - baseline as f64) / (1024.0 * 1024.0);
        
        if growth_mb < self.detection_settings.min_growth_threshold_mb {
            return Ok(None);
        }

        // Calculate growth rate
        let time_diff = recent_samples.first().unwrap().timestamp
            .signed_duration_since(recent_samples.last().unwrap().timestamp);
        let growth_rate_mb_per_min = growth_mb / (time_diff.num_minutes() as f64).max(1.0);

        // Check if growth is consistent (linear regression)
        let is_steady = self.is_steady_growth(history);
        
        if is_steady && growth_rate_mb_per_min > 0.1 {
            let severity = self.calculate_severity(growth_rate_mb_per_min, current);
            let trend_data = self.build_trend_data(history);
            
            return Ok(Some(MemoryLeakAlert {
                pid: process.pid,
                process_name: process.name.clone(),
                alert_type: LeakAlertType::SteadyGrowth,
                severity,
                detected_at: Utc::now(),
                growth_rate: growth_rate_mb_per_min,
                current_memory: current,
                baseline_memory: baseline,
                trend_data,
                suggested_action: self.suggest_action(&LeakAlertType::SteadyGrowth, growth_rate_mb_per_min),
            }));
        }

        Ok(None)
    }

    fn detect_rapid_increase(&self, process: &ProcessInfo, history: &VecDeque<MemorySnapshot>) -> Result<Option<MemoryLeakAlert>> {
        if history.len() < 3 {
            return Ok(None);
        }

        let recent = history.iter().rev().take(3).collect::<Vec<_>>();
        let current = recent[0].resident_memory;
        let previous = recent[1].resident_memory;
        let baseline = recent[2].resident_memory;

        let recent_growth = (current as f64 - previous as f64) / (1024.0 * 1024.0);
        let time_diff = recent[0].timestamp.signed_duration_since(recent[1].timestamp);
        let growth_rate = recent_growth / (time_diff.num_minutes() as f64).max(1.0);

        if growth_rate > self.detection_settings.rapid_growth_threshold_mb_per_min {
            let severity = self.calculate_severity(growth_rate, current);
            let trend_data = self.build_trend_data(history);

            return Ok(Some(MemoryLeakAlert {
                pid: process.pid,
                process_name: process.name.clone(),
                alert_type: LeakAlertType::RapidIncrease,
                severity,
                detected_at: Utc::now(),
                growth_rate,
                current_memory: current,
                baseline_memory: baseline,
                trend_data,
                suggested_action: self.suggest_action(&LeakAlertType::RapidIncrease, growth_rate),
            }));
        }

        Ok(None)
    }

    fn detect_memory_spikes(&self, process: &ProcessInfo, history: &VecDeque<MemorySnapshot>) -> Result<Option<MemoryLeakAlert>> {
        if history.len() < 5 {
            return Ok(None);
        }

        let samples: Vec<_> = history.iter().collect();
        let current = samples.last().unwrap().resident_memory;
        
        // Calculate moving average
        let _window_size = (samples.len() / 2).max(5);
        let baseline_samples = &samples[..samples.len() - 2];
        let baseline_avg = baseline_samples.iter()
            .map(|s| s.resident_memory)
            .sum::<u64>() as f64 / baseline_samples.len() as f64;

        let spike_ratio = current as f64 / baseline_avg;
        
        if spike_ratio > self.detection_settings.spike_threshold_multiplier {
            let growth_rate = (current as f64 - baseline_avg) / (1024.0 * 1024.0);
            let severity = if spike_ratio > 5.0 {
                LeakSeverity::Critical
            } else if spike_ratio > 3.0 {
                LeakSeverity::High
            } else {
                LeakSeverity::Medium
            };

            let trend_data = self.build_trend_data(history);

            return Ok(Some(MemoryLeakAlert {
                pid: process.pid,
                process_name: process.name.clone(),
                alert_type: LeakAlertType::MemorySpike,
                severity,
                detected_at: Utc::now(),
                growth_rate,
                current_memory: current,
                baseline_memory: baseline_avg as u64,
                trend_data,
                suggested_action: self.suggest_action(&LeakAlertType::MemorySpike, growth_rate),
            }));
        }

        Ok(None)
    }

    fn detect_suspicious_patterns(&self, process: &ProcessInfo, history: &VecDeque<MemorySnapshot>) -> Result<Option<MemoryLeakAlert>> {
        if history.len() < 10 {
            return Ok(None);
        }

        // Check for sawtooth pattern (allocate, free, allocate more, repeat)
        let samples: Vec<_> = history.iter().collect();
        let mut peaks = Vec::new();
        let mut valleys = Vec::new();

        for i in 1..samples.len() - 1 {
            let prev = samples[i - 1].resident_memory;
            let curr = samples[i].resident_memory;
            let next = samples[i + 1].resident_memory;

            if curr > prev && curr > next {
                peaks.push((i, curr));
            } else if curr < prev && curr < next {
                valleys.push((i, curr));
            }
        }

        // Check if peaks are increasing over time (classic leak pattern)
        if peaks.len() >= 3 {
            let peak_values: Vec<u64> = peaks.iter().map(|(_, val)| *val).collect();
            let is_increasing = peak_values.windows(2).all(|w| w[1] > w[0]);
            
            if is_increasing {
                let first_peak = peak_values[0];
                let last_peak = *peak_values.last().unwrap();
                let growth_rate = (last_peak as f64 - first_peak as f64) / (1024.0 * 1024.0);
                
                if growth_rate > self.detection_settings.min_growth_threshold_mb {
                    let severity = self.calculate_severity(growth_rate / peaks.len() as f64, last_peak);
                    let trend_data = self.build_trend_data(history);

                    return Ok(Some(MemoryLeakAlert {
                        pid: process.pid,
                        process_name: process.name.clone(),
                        alert_type: LeakAlertType::SuspiciousPattern,
                        severity,
                        detected_at: Utc::now(),
                        growth_rate,
                        current_memory: last_peak,
                        baseline_memory: first_peak,
                        trend_data,
                        suggested_action: self.suggest_action(&LeakAlertType::SuspiciousPattern, growth_rate),
                    }));
                }
            }
        }

        Ok(None)
    }

    fn is_steady_growth(&self, history: &VecDeque<MemorySnapshot>) -> bool {
        if history.len() < 5 {
            return false;
        }

        let samples: Vec<_> = history.iter().collect();
        let mut increasing_count = 0;
        let mut total_comparisons = 0;

        for window in samples.windows(3) {
            let trend = window[2].resident_memory as f64 - window[0].resident_memory as f64;
            if trend > 0.0 {
                increasing_count += 1;
            }
            total_comparisons += 1;
        }

        // Consider it steady growth if 70% of samples show increase
        (increasing_count as f64 / total_comparisons as f64) > 0.7
    }

    fn calculate_severity(&self, growth_rate_mb_per_min: f64, current_memory: u64) -> LeakSeverity {
        let current_memory_mb = current_memory as f64 / (1024.0 * 1024.0);
        
        if growth_rate_mb_per_min > 20.0 || current_memory_mb > 2048.0 {
            LeakSeverity::Critical
        } else if growth_rate_mb_per_min > 10.0 || current_memory_mb > 1024.0 {
            LeakSeverity::High
        } else if growth_rate_mb_per_min > 2.0 || current_memory_mb > 512.0 {
            LeakSeverity::Medium
        } else {
            LeakSeverity::Low
        }
    }

    fn build_trend_data(&self, history: &VecDeque<MemorySnapshot>) -> Vec<MemoryTrendPoint> {
        let samples: Vec<_> = history.iter().collect();
        let mut trend_data = Vec::new();

        for window in samples.windows(2) {
            let time_diff = window[1].timestamp.signed_duration_since(window[0].timestamp);
            let memory_diff = window[1].resident_memory as f64 - window[0].resident_memory as f64;
            let growth_rate = memory_diff / (1024.0 * 1024.0) / (time_diff.num_minutes() as f64).max(1.0);

            trend_data.push(MemoryTrendPoint {
                timestamp: window[1].timestamp,
                memory_usage: window[1].resident_memory,
                growth_rate,
            });
        }

        trend_data
    }

    fn suggest_action(&self, alert_type: &LeakAlertType, growth_rate: f64) -> String {
        match alert_type {
            LeakAlertType::SteadyGrowth => {
                if growth_rate > 10.0 {
                    "Investigate process for memory leaks. Consider restarting if critical.".to_string()
                } else {
                    "Monitor process memory usage. May need optimization.".to_string()
                }
            },
            LeakAlertType::RapidIncrease => {
                "Immediate investigation required. Process may be malfunctioning.".to_string()
            },
            LeakAlertType::MemorySpike => {
                "Check for temporary high memory usage. Monitor for pattern.".to_string()
            },
            LeakAlertType::SuspiciousPattern => {
                "Analyze process allocation patterns. Likely memory leak.".to_string()
            },
            LeakAlertType::FragmentationIncrease => {
                "Check for memory fragmentation issues.".to_string()
            },
        }
    }

    fn add_or_update_alert(&mut self, new_alert: MemoryLeakAlert) {
        // Remove existing alert for same process and type
        self.leak_alerts.retain(|alert| 
            !(alert.pid == new_alert.pid && 
              std::mem::discriminant(&alert.alert_type) == std::mem::discriminant(&new_alert.alert_type))
        );
        
        self.leak_alerts.push(new_alert);
        
        // Limit alerts to most recent 100
        if self.leak_alerts.len() > 100 {
            self.leak_alerts.drain(0..self.leak_alerts.len() - 100);
        }
    }

    fn get_heap_size(&self, pid: u32) -> Result<u64> {
        // Read from /proc/pid/smaps or /proc/pid/status
        let status_path = format!("/proc/{}/status", pid);
        if let Ok(content) = std::fs::read_to_string(status_path) {
            for line in content.lines() {
                if line.starts_with("VmData:") {
                    if let Some(size_str) = line.split_whitespace().nth(1) {
                        if let Ok(size_kb) = size_str.parse::<u64>() {
                            return Ok(size_kb * 1024);
                        }
                    }
                }
            }
        }
        Err(anyhow::anyhow!("Could not read heap size"))
    }

    fn get_stack_size(&self, pid: u32) -> Result<u64> {
        let status_path = format!("/proc/{}/status", pid);
        if let Ok(content) = std::fs::read_to_string(status_path) {
            for line in content.lines() {
                if line.starts_with("VmStk:") {
                    if let Some(size_str) = line.split_whitespace().nth(1) {
                        if let Ok(size_kb) = size_str.parse::<u64>() {
                            return Ok(size_kb * 1024);
                        }
                    }
                }
            }
        }
        Err(anyhow::anyhow!("Could not read stack size"))
    }

    pub fn get_alerts(&self) -> &[MemoryLeakAlert] {
        &self.leak_alerts
    }

    pub fn get_alerts_for_process(&self, pid: u32) -> Vec<&MemoryLeakAlert> {
        self.leak_alerts.iter().filter(|alert| alert.pid == pid).collect()
    }

    pub fn clear_alerts_for_process(&mut self, pid: u32) {
        self.leak_alerts.retain(|alert| alert.pid != pid);
    }

    pub fn get_memory_history(&self, pid: u32) -> Option<&VecDeque<MemorySnapshot>> {
        self.process_memory_history.get(&pid)
    }

    pub fn cleanup_old_data(&mut self) {
        let cutoff_time = Utc::now() - Duration::hours(24);
        
        // Clean old alerts
        self.leak_alerts.retain(|alert| alert.detected_at > cutoff_time);
        
        // Clean old history entries
        for history in self.process_memory_history.values_mut() {
            while let Some(front) = history.front() {
                if front.timestamp < cutoff_time {
                    history.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}