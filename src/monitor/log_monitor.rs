use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub level: LogLevel,
    pub message: String,
    pub raw_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLogAlert {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub log_source: String,
    pub alert_type: LogAlertType,
    pub severity: LogAlertSeverity,
    pub message: String,
    pub raw_log_entry: String,
    pub matched_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogAlertType {
    AuthenticationFailure,
    PrivilegeEscalation,
    SuspiciousCommand,
    NetworkIntrusion,
    FileSystemAccess,
    ServiceFailure,
    SystemAnomaly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogAlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug)]
pub struct LogMonitorConfig {
    pub log_files: Vec<String>,
    pub follow_mode: bool,
    pub max_entries: usize,
    pub alert_patterns: HashMap<String, (LogAlertType, LogAlertSeverity)>,
}

impl Default for LogMonitorConfig {
    fn default() -> Self {
        let mut alert_patterns = HashMap::new();
        
        // Authentication patterns
        alert_patterns.insert("authentication failure".to_string(), (LogAlertType::AuthenticationFailure, LogAlertSeverity::Medium));
        alert_patterns.insert("failed login".to_string(), (LogAlertType::AuthenticationFailure, LogAlertSeverity::Medium));
        alert_patterns.insert("invalid user".to_string(), (LogAlertType::AuthenticationFailure, LogAlertSeverity::Medium));
        alert_patterns.insert("connection closed by authenticating user".to_string(), (LogAlertType::AuthenticationFailure, LogAlertSeverity::Low));
        
        // Privilege escalation patterns
        alert_patterns.insert("sudo:".to_string(), (LogAlertType::PrivilegeEscalation, LogAlertSeverity::Medium));
        alert_patterns.insert("su:".to_string(), (LogAlertType::PrivilegeEscalation, LogAlertSeverity::Medium));
        alert_patterns.insert("became user".to_string(), (LogAlertType::PrivilegeEscalation, LogAlertSeverity::High));
        
        // Suspicious commands
        alert_patterns.insert("nc -".to_string(), (LogAlertType::SuspiciousCommand, LogAlertSeverity::High));
        alert_patterns.insert("netcat".to_string(), (LogAlertType::SuspiciousCommand, LogAlertSeverity::High));
        alert_patterns.insert("/bin/sh".to_string(), (LogAlertType::SuspiciousCommand, LogAlertSeverity::Medium));
        alert_patterns.insert("wget http".to_string(), (LogAlertType::SuspiciousCommand, LogAlertSeverity::Medium));
        alert_patterns.insert("curl http".to_string(), (LogAlertType::SuspiciousCommand, LogAlertSeverity::Medium));
        
        // Network intrusion patterns
        alert_patterns.insert("port scan".to_string(), (LogAlertType::NetworkIntrusion, LogAlertSeverity::High));
        alert_patterns.insert("connection reset by peer".to_string(), (LogAlertType::NetworkIntrusion, LogAlertSeverity::Low));
        alert_patterns.insert("refused connect".to_string(), (LogAlertType::NetworkIntrusion, LogAlertSeverity::Low));
        
        // File system access patterns
        alert_patterns.insert("/etc/passwd".to_string(), (LogAlertType::FileSystemAccess, LogAlertSeverity::High));
        alert_patterns.insert("/etc/shadow".to_string(), (LogAlertType::FileSystemAccess, LogAlertSeverity::Critical));
        alert_patterns.insert("permission denied".to_string(), (LogAlertType::FileSystemAccess, LogAlertSeverity::Low));
        
        // Service failure patterns
        alert_patterns.insert("service failed".to_string(), (LogAlertType::ServiceFailure, LogAlertSeverity::Medium));
        alert_patterns.insert("segmentation fault".to_string(), (LogAlertType::ServiceFailure, LogAlertSeverity::High));
        alert_patterns.insert("out of memory".to_string(), (LogAlertType::SystemAnomaly, LogAlertSeverity::High));

        Self {
            log_files: vec![
                "/var/log/syslog".to_string(),
                "/var/log/auth.log".to_string(),
                "/var/log/secure".to_string(),
                "/var/log/messages".to_string(),
                "/var/log/kern.log".to_string(),
            ],
            follow_mode: true,
            max_entries: 1000,
            alert_patterns,
        }
    }
}

pub struct LogMonitor {
    config: LogMonitorConfig,
    log_entries: Vec<LogEntry>,
    alerts: Vec<SecurityLogAlert>,
    file_positions: HashMap<String, u64>,
    last_check: DateTime<Utc>,
}

impl LogMonitor {
    pub fn new(config: LogMonitorConfig) -> Self {
        Self {
            config,
            log_entries: Vec::new(),
            alerts: Vec::new(),
            file_positions: HashMap::new(),
            last_check: Utc::now(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(LogMonitorConfig::default())
    }

    pub async fn update(&mut self) -> Result<()> {
        self.clear_old_entries();
        self.clear_old_alerts();

        for log_file in &self.config.log_files.clone() {
            if Path::new(log_file).exists() {
                if let Err(e) = self.read_log_file(log_file).await {
                    eprintln!("Error reading log file {}: {}", log_file, e);
                }
            }
        }

        self.analyze_logs_for_alerts();
        self.last_check = Utc::now();
        
        Ok(())
    }

    async fn read_log_file(&mut self, file_path: &str) -> Result<()> {
        let file = OpenOptions::new().read(true).open(file_path).await?;
        let mut reader = tokio::io::BufReader::new(file);
        
        // Get current position for this file
        let current_pos = self.file_positions.get(file_path).copied().unwrap_or(0);
        
        // Seek to the last known position if we're following
        if self.config.follow_mode && current_pos > 0 {
            // For simplicity, we'll read from the beginning and skip to new content
            // In a production system, you'd use inotify or similar for real-time monitoring
        }

        let mut line = String::new();
        let mut new_entries = Vec::new();
        let mut lines_read = 0;

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break; // EOF
            }

            lines_read += 1;
            if lines_read <= current_pos as usize {
                continue; // Skip already processed lines
            }

            if let Some(entry) = self.parse_log_line(&line, file_path) {
                new_entries.push(entry);
            }

            // Limit the number of entries we process in one update
            if new_entries.len() > 100 {
                break;
            }
        }

        // Update file position
        self.file_positions.insert(file_path.to_string(), (current_pos + new_entries.len() as u64));

        // Add new entries to our collection
        self.log_entries.extend(new_entries);

        // Keep only the most recent entries
        if self.log_entries.len() > self.config.max_entries {
            let excess = self.log_entries.len() - self.config.max_entries;
            self.log_entries.drain(0..excess);
        }

        Ok(())
    }

    fn parse_log_line(&self, line: &str, source: &str) -> Option<LogEntry> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Try to extract timestamp and log level
        let (timestamp, level) = self.extract_timestamp_and_level(line);
        
        Some(LogEntry {
            timestamp,
            source: source.to_string(),
            level,
            message: self.extract_message(line),
            raw_line: line.to_string(),
        })
    }

    fn extract_timestamp_and_level(&self, line: &str) -> (DateTime<Utc>, LogLevel) {
        // Simple timestamp extraction - this would need to be more robust for production
        let timestamp = if line.len() > 15 {
            // Try to parse common syslog timestamp format
            let timestamp_str = &line[0..15];
            chrono::NaiveDateTime::parse_from_str(&format!("2024 {}", timestamp_str), "%Y %b %d %H:%M:%S")
                .map(|dt| dt.and_utc())
                .unwrap_or_else(|_| Utc::now())
        } else {
            Utc::now()
        };

        // Extract log level
        let level = if line.contains("EMERGENCY") || line.contains("PANIC") {
            LogLevel::Emergency
        } else if line.contains("ALERT") {
            LogLevel::Alert
        } else if line.contains("CRIT") || line.contains("CRITICAL") {
            LogLevel::Critical
        } else if line.contains("ERR") || line.contains("ERROR") {
            LogLevel::Error
        } else if line.contains("WARN") || line.contains("WARNING") {
            LogLevel::Warning
        } else if line.contains("NOTICE") {
            LogLevel::Notice
        } else if line.contains("INFO") {
            LogLevel::Info
        } else if line.contains("DEBUG") {
            LogLevel::Debug
        } else {
            LogLevel::Info
        };

        (timestamp, level)
    }

    fn extract_message(&self, line: &str) -> String {
        // Extract the message part after timestamp and hostname
        // This is a simplified extraction - production would need more sophisticated parsing
        if let Some(colon_pos) = line.find(':') {
            if let Some(second_colon) = line[colon_pos + 1..].find(':') {
                return line[colon_pos + second_colon + 2..].trim().to_string();
            }
        }
        line.to_string()
    }

    fn analyze_logs_for_alerts(&mut self) {
        let recent_entries: Vec<&LogEntry> = self.log_entries.iter()
            .filter(|entry| {
                Utc::now().signed_duration_since(entry.timestamp) < chrono::Duration::minutes(5)
            })
            .collect();

        for entry in recent_entries {
            for (pattern, (alert_type, severity)) in &self.config.alert_patterns {
                if entry.message.to_lowercase().contains(&pattern.to_lowercase()) ||
                   entry.raw_line.to_lowercase().contains(&pattern.to_lowercase()) {
                    
                    // Check if we already have a similar recent alert
                    let similar_recent = self.alerts.iter().any(|alert| {
                        alert.matched_pattern == *pattern &&
                        Utc::now().signed_duration_since(alert.timestamp) < chrono::Duration::minutes(1)
                    });

                    if !similar_recent {
                        let alert = SecurityLogAlert {
                            id: format!("log-alert-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                            timestamp: entry.timestamp,
                            log_source: entry.source.clone(),
                            alert_type: alert_type.clone(),
                            severity: severity.clone(),
                            message: format!("Security pattern detected: {}", pattern),
                            raw_log_entry: entry.raw_line.clone(),
                            matched_pattern: pattern.clone(),
                        };

                        self.alerts.push(alert);
                    }
                }
            }
        }
    }

    fn clear_old_entries(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        self.log_entries.retain(|entry| entry.timestamp > cutoff);
    }

    fn clear_old_alerts(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::hours(6);
        self.alerts.retain(|alert| alert.timestamp > cutoff);
    }

    pub fn get_recent_entries(&self, limit: usize) -> Vec<&LogEntry> {
        self.log_entries.iter()
            .rev()
            .take(limit)
            .collect()
    }

    pub fn get_entries_by_level(&self, level: LogLevel) -> Vec<&LogEntry> {
        self.log_entries.iter()
            .filter(|entry| entry.level == level)
            .collect()
    }

    pub fn get_alerts(&self) -> &[SecurityLogAlert] {
        &self.alerts
    }

    pub fn get_alerts_by_severity(&self, severity: LogAlertSeverity) -> Vec<&SecurityLogAlert> {
        self.alerts.iter()
            .filter(|alert| alert.severity == severity)
            .collect()
    }

    pub fn get_alert_count_by_type(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for alert in &self.alerts {
            let type_name = format!("{:?}", alert.alert_type);
            *counts.entry(type_name).or_insert(0) += 1;
        }
        counts
    }

    pub fn search_logs(&self, query: &str) -> Vec<&LogEntry> {
        let query_lower = query.to_lowercase();
        self.log_entries.iter()
            .filter(|entry| {
                entry.message.to_lowercase().contains(&query_lower) ||
                entry.raw_line.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}