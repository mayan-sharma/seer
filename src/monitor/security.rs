use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::monitor::ProcessInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlert {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub severity: AlertSeverity,
    pub alert_type: AlertType,
    pub message: String,
    pub process_pid: Option<u32>,
    pub process_name: Option<String>,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    SuspiciousProcess,
    PrivilegeEscalation,
    UnusualNetworkActivity,
    HighResourceUsage,
    ProcessAnomalyDetection,
    UnauthorizedFileAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessBehaviorProfile {
    pub pid: u32,
    pub name: String,
    pub cpu_usage_history: Vec<f32>,
    pub memory_usage_history: Vec<u64>,
    pub network_connections: u32,
    pub file_operations: u32,
    pub privilege_level: String,
    pub parent_pid: Option<u32>,
    pub start_time: DateTime<Utc>,
    pub suspicious_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub total_processes: usize,
    pub suspicious_processes: usize,
    pub privilege_escalations: usize,
    pub network_anomalies: usize,
    pub file_system_alerts: usize,
    pub active_alerts: Vec<SecurityAlert>,
    pub process_profiles: Vec<ProcessBehaviorProfile>,
}

pub struct SecurityDashboard {
    alerts: Vec<SecurityAlert>,
    process_profiles: HashMap<u32, ProcessBehaviorProfile>,
    baseline_cpu_threshold: f32,
    baseline_memory_threshold: u64,
    suspicious_process_names: Vec<String>,
    privileged_processes: HashMap<u32, String>,
    alert_history: Vec<SecurityAlert>,
    max_history_size: usize,
}

impl SecurityDashboard {
    pub fn new() -> Self {
        Self {
            alerts: Vec::new(),
            process_profiles: HashMap::new(),
            baseline_cpu_threshold: 80.0,
            baseline_memory_threshold: 1024 * 1024 * 1024, // 1GB
            suspicious_process_names: vec![
                "nc".to_string(),
                "netcat".to_string(),
                "nmap".to_string(),
                "wget".to_string(),
                "curl".to_string(),
                "python".to_string(),
                "perl".to_string(),
                "bash".to_string(),
                "sh".to_string(),
                "powershell".to_string(),
                "cmd".to_string(),
            ],
            privileged_processes: HashMap::new(),
            alert_history: Vec::new(),
            max_history_size: 1000,
        }
    }

    pub fn update_security_analysis(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        self.clear_expired_alerts();
        
        for process in processes {
            self.update_process_profile(process);
            self.analyze_process_behavior(process)?;
        }

        self.detect_privilege_escalations(processes)?;
        self.cleanup_old_profiles(processes);
        
        Ok(())
    }

    fn update_process_profile(&mut self, process: &ProcessInfo) {
        // First, ensure the profile exists
        if !self.process_profiles.contains_key(&process.pid) {
            let new_profile = ProcessBehaviorProfile {
                pid: process.pid,
                name: process.name.clone(),
                cpu_usage_history: Vec::new(),
                memory_usage_history: Vec::new(),
                network_connections: 0,
                file_operations: 0,
                privilege_level: process.user.clone(),
                parent_pid: process.parent_pid,
                start_time: Utc::now(),
                suspicious_score: 0.0,
            };
            self.process_profiles.insert(process.pid, new_profile);
        }

        // Now update the profile - first update the data
        if let Some(profile) = self.process_profiles.get_mut(&process.pid) {
            // Update CPU and memory history (keep last 60 readings)
            profile.cpu_usage_history.push(process.cpu_usage);
            if profile.cpu_usage_history.len() > 60 {
                profile.cpu_usage_history.remove(0);
            }

            profile.memory_usage_history.push(process.memory_usage);
            if profile.memory_usage_history.len() > 60 {
                profile.memory_usage_history.remove(0);
            }
        }

        // Calculate and update suspicious score separately
        if let Some(profile) = self.process_profiles.get(&process.pid) {
            let score = self.calculate_suspicious_score(profile, process);
            if let Some(profile_mut) = self.process_profiles.get_mut(&process.pid) {
                profile_mut.suspicious_score = score;
            }
        }
    }

    fn calculate_suspicious_score(&self, profile: &ProcessBehaviorProfile, process: &ProcessInfo) -> f32 {
        let mut score: f32 = 0.0;

        // Check for suspicious process names
        if self.suspicious_process_names.iter().any(|name| process.name.contains(name)) {
            score += 20.0;
        }

        // Check for high CPU usage spikes
        if let Some(&max_cpu) = profile.cpu_usage_history.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
            if max_cpu > self.baseline_cpu_threshold {
                score += 15.0;
            }
        }

        // Check for rapid memory growth
        if profile.memory_usage_history.len() >= 10 {
            let recent_memory: u64 = profile.memory_usage_history.iter().rev().take(5).sum::<u64>() / 5;
            let older_memory: u64 = profile.memory_usage_history.iter().rev().skip(5).take(5).sum::<u64>() / 5;
            
            if recent_memory > older_memory * 2 {
                score += 25.0;
            }
        }

        // Check for unusual command line arguments
        if process.command.contains("--") || process.command.contains("-") {
            let arg_count = process.command.matches('-').count();
            if arg_count > 5 {
                score += 10.0;
            }
        }

        // Check for processes running as root/admin
        if process.user == "root" || process.user == "administrator" {
            score += 10.0;
        }

        // Check for processes with no parent (potential orphans)
        if process.parent_pid.is_none() && process.pid != 1 {
            score += 15.0;
        }

        score.min(100.0) // Cap at 100
    }

    fn analyze_process_behavior(&mut self, process: &ProcessInfo) -> Result<()> {
        if let Some(profile) = self.process_profiles.get(&process.pid) {
            // Generate alerts based on suspicious score
            if profile.suspicious_score > 70.0 {
                self.create_alert(
                    AlertType::SuspiciousProcess,
                    AlertSeverity::High,
                    format!("Process '{}' has high suspicious score: {:.1}", process.name, profile.suspicious_score),
                    Some(process.pid),
                    Some(process.name.clone()),
                )?;
            } else if profile.suspicious_score > 50.0 {
                self.create_alert(
                    AlertType::ProcessAnomalyDetection,
                    AlertSeverity::Medium,
                    format!("Process '{}' showing anomalous behavior: {:.1}", process.name, profile.suspicious_score),
                    Some(process.pid),
                    Some(process.name.clone()),
                )?;
            }

            // Check for resource abuse
            if process.cpu_usage > self.baseline_cpu_threshold {
                self.create_alert(
                    AlertType::HighResourceUsage,
                    AlertSeverity::Medium,
                    format!("Process '{}' using high CPU: {:.1}%", process.name, process.cpu_usage),
                    Some(process.pid),
                    Some(process.name.clone()),
                )?;
            }

            if process.memory_usage > self.baseline_memory_threshold {
                self.create_alert(
                    AlertType::HighResourceUsage,
                    AlertSeverity::Medium,
                    format!("Process '{}' using high memory: {} MB", process.name, process.memory_usage / 1024 / 1024),
                    Some(process.pid),
                    Some(process.name.clone()),
                )?;
            }
        }

        Ok(())
    }

    fn detect_privilege_escalations(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        for process in processes {
            // Check if this process has elevated privileges compared to its parent
            if let Some(parent_pid) = process.parent_pid {
                if let Some(parent_process) = processes.iter().find(|p| p.pid == parent_pid) {
                    if self.is_privilege_escalation(&parent_process.user, &process.user) {
                        self.create_alert(
                            AlertType::PrivilegeEscalation,
                            AlertSeverity::High,
                            format!("Privilege escalation detected: '{}' (user: {}) spawned from '{}' (user: {})", 
                                    process.name, process.user, parent_process.name, parent_process.user),
                            Some(process.pid),
                            Some(process.name.clone()),
                        )?;
                    }
                }
            }

            // Track privileged processes
            if process.user == "root" || process.user == "administrator" {
                self.privileged_processes.insert(process.pid, process.name.clone());
            }
        }

        Ok(())
    }

    fn is_privilege_escalation(&self, parent_user: &str, child_user: &str) -> bool {
        // Simple privilege escalation detection
        match (parent_user, child_user) {
            ("root", _) => false, // Root can spawn anything
            (_, "root") => true,  // Non-root spawning root is escalation
            ("administrator", _) => false, // Admin can spawn anything
            (_, "administrator") => true, // Non-admin spawning admin is escalation
            _ => false,
        }
    }

    fn create_alert(
        &mut self,
        alert_type: AlertType,
        severity: AlertSeverity,
        message: String,
        process_pid: Option<u32>,
        process_name: Option<String>,
    ) -> Result<()> {
        // Check if we already have a similar recent alert to avoid spam
        let recent_threshold = chrono::Duration::minutes(5);
        let now = Utc::now();
        
        let similar_recent_alert = self.alerts.iter().any(|alert| {
            alert.alert_type.discriminant() == alert_type.discriminant() &&
            alert.process_pid == process_pid &&
            now.signed_duration_since(alert.timestamp) < recent_threshold
        });

        if !similar_recent_alert {
            let alert = SecurityAlert {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: now,
                severity,
                alert_type,
                message,
                process_pid,
                process_name,
                details: HashMap::new(),
            };

            self.alerts.push(alert.clone());
            self.alert_history.push(alert);

            // Limit alert history size
            if self.alert_history.len() > self.max_history_size {
                self.alert_history.remove(0);
            }
        }

        Ok(())
    }

    fn clear_expired_alerts(&mut self) {
        let threshold = chrono::Duration::hours(1);
        let now = Utc::now();
        
        self.alerts.retain(|alert| {
            now.signed_duration_since(alert.timestamp) < threshold
        });
    }

    fn cleanup_old_profiles(&mut self, current_processes: &[ProcessInfo]) {
        let current_pids: std::collections::HashSet<u32> = current_processes.iter().map(|p| p.pid).collect();
        self.process_profiles.retain(|&pid, _| current_pids.contains(&pid));
        self.privileged_processes.retain(|&pid, _| current_pids.contains(&pid));
    }

    pub fn get_security_metrics(&self) -> SecurityMetrics {
        let suspicious_processes = self.process_profiles.values()
            .filter(|profile| profile.suspicious_score > 50.0)
            .count();

        let privilege_escalations = self.alerts.iter()
            .filter(|alert| matches!(alert.alert_type, AlertType::PrivilegeEscalation))
            .count();

        let network_anomalies = self.alerts.iter()
            .filter(|alert| matches!(alert.alert_type, AlertType::UnusualNetworkActivity))
            .count();

        let file_system_alerts = self.alerts.iter()
            .filter(|alert| matches!(alert.alert_type, AlertType::UnauthorizedFileAccess))
            .count();

        SecurityMetrics {
            total_processes: self.process_profiles.len(),
            suspicious_processes,
            privilege_escalations,
            network_anomalies,
            file_system_alerts,
            active_alerts: self.alerts.clone(),
            process_profiles: self.process_profiles.values().cloned().collect(),
        }
    }

    pub fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<&SecurityAlert> {
        self.alerts.iter().filter(|alert| alert.severity == severity).collect()
    }

    pub fn get_alert_history(&self) -> &[SecurityAlert] {
        &self.alert_history
    }
}

// Helper trait for alert type discrimination
impl AlertType {
    fn discriminant(&self) -> std::mem::Discriminant<AlertType> {
        std::mem::discriminant(self)
    }
}

// Add uuid dependency support (would need to add to Cargo.toml)
mod uuid {
    pub struct Uuid;
    impl Uuid {
        pub fn new_v4() -> Self { Self }
        pub fn to_string(&self) -> String {
            format!("alert-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
        }
    }
}