use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::str;
use tokio::fs;
use tokio::time::{Duration as TokioDuration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetrics {
    pub backup_jobs: Vec<BackupJob>,
    pub storage_locations: Vec<StorageLocation>,
    pub recovery_points: Vec<RecoveryPoint>,
    pub backup_performance: BackupPerformance,
    pub alerts: Vec<BackupAlert>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupJob {
    pub job_id: String,
    pub name: String,
    pub job_type: BackupType,
    pub source_paths: Vec<PathBuf>,
    pub destination: String,
    pub schedule: BackupSchedule,
    pub status: BackupStatus,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub success_rate: f32, // Percentage
    pub data_size: u64,    // Bytes
    pub compression_ratio: f32,
    pub encryption_enabled: bool,
    pub retention_policy: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
    Snapshot,
    Database,
    VMware,
    HyperV,
    FileSync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub frequency: BackupFrequency,
    pub time_of_day: Option<String>, // HH:MM format
    pub days_of_week: Vec<u8>,       // 0-6, Sunday = 0
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    OnDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Scheduled,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub keep_daily: u32,   // Days
    pub keep_weekly: u32,  // Weeks
    pub keep_monthly: u32, // Months
    pub keep_yearly: u32,  // Years
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLocation {
    pub location_id: String,
    pub name: String,
    pub storage_type: StorageType,
    pub path: String,
    pub total_capacity: u64,
    pub used_space: u64,
    pub available_space: u64,
    pub is_accessible: bool,
    pub last_verified: DateTime<Utc>,
    pub read_speed: f64,  // MB/s
    pub write_speed: f64, // MB/s
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    LocalDisk,
    NetworkShare,
    CloudStorage,
    TapeLibrary,
    NAS,
    SAN,
    S3Compatible,
    FTP,
    SFTP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPoint {
    pub recovery_id: String,
    pub job_id: String,
    pub created_at: DateTime<Utc>,
    pub backup_type: BackupType,
    pub data_size: u64,
    pub file_count: u32,
    pub integrity_verified: bool,
    pub recovery_time_estimate: Duration,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPerformance {
    pub average_backup_speed: f64,    // MB/s
    pub average_restore_speed: f64,   // MB/s
    pub average_backup_duration: Duration,
    pub total_data_protected: u64,   // Bytes
    pub deduplication_ratio: f32,
    pub compression_savings: f32,     // Percentage
    pub bandwidth_utilization: f32,  // Percentage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupAlert {
    pub alert_id: String,
    pub alert_type: BackupAlertType,
    pub severity: BackupAlertSeverity,
    pub job_id: Option<String>,
    pub storage_id: Option<String>,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupAlertType {
    JobFailed,
    JobMissed,
    StorageFull,
    StorageOffline,
    IntegrityCheckFailed,
    PerformanceDegraded,
    RetentionViolation,
    EncryptionIssue,
    NetworkIssue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupAlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOperation {
    pub restore_id: String,
    pub recovery_point_id: String,
    pub destination_path: String,
    pub status: RestoreStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_percent: f32,
    pub files_restored: u32,
    pub bytes_restored: u64,
    pub estimated_time_remaining: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestoreStatus {
    Preparing,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub monitor_interval: TokioDuration,
    pub backup_directories: Vec<PathBuf>,
    pub storage_locations: Vec<String>,
    pub log_file_paths: Vec<PathBuf>,
    pub enable_performance_monitoring: bool,
    pub integrity_check_interval: TokioDuration,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            monitor_interval: TokioDuration::from_secs(300), // 5 minutes
            backup_directories: vec![
                PathBuf::from("/var/backups"),
                PathBuf::from("/backup"),
                PathBuf::from("/home/backups"),
            ],
            storage_locations: vec![
                "/mnt/backup".to_string(),
                "/backup".to_string(),
            ],
            log_file_paths: vec![
                PathBuf::from("/var/log/backup.log"),
                PathBuf::from("/var/log/rsync.log"),
            ],
            enable_performance_monitoring: true,
            integrity_check_interval: TokioDuration::from_secs(3600), // 1 hour
        }
    }
}

pub struct BackupMonitor {
    config: BackupConfig,
    last_metrics: Option<BackupMetrics>,
    last_update: Instant,
    backup_jobs: HashMap<String, BackupJob>,
    storage_locations: HashMap<String, StorageLocation>,
    active_restores: Vec<RestoreOperation>,
    alert_history: Vec<BackupAlert>,
}

impl BackupMonitor {
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config,
            last_metrics: None,
            last_update: Instant::now(),
            backup_jobs: HashMap::new(),
            storage_locations: HashMap::new(),
            active_restores: Vec::new(),
            alert_history: Vec::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(BackupConfig::default())
    }

    pub async fn update_metrics(&mut self) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_update) < self.config.monitor_interval {
            return Ok(());
        }

        // Discover backup jobs
        self.discover_backup_jobs().await?;
        
        // Update storage locations
        self.update_storage_locations().await?;
        
        // Discover recovery points
        let recovery_points = self.discover_recovery_points().await?;
        
        // Calculate performance metrics
        let performance = self.calculate_performance_metrics();
        
        // Check for alerts
        let alerts = self.check_for_alerts().await?;

        let metrics = BackupMetrics {
            backup_jobs: self.backup_jobs.values().cloned().collect(),
            storage_locations: self.storage_locations.values().cloned().collect(),
            recovery_points,
            backup_performance: performance,
            alerts,
            last_updated: Utc::now(),
        };

        self.last_metrics = Some(metrics);
        self.last_update = now;
        Ok(())
    }

    pub fn get_metrics(&self) -> Option<&BackupMetrics> {
        self.last_metrics.as_ref()
    }

    pub fn get_active_restores(&self) -> &[RestoreOperation] {
        &self.active_restores
    }

    async fn discover_backup_jobs(&mut self) -> Result<()> {
        // Check for common backup tools and their configurations
        self.discover_rsync_jobs().await?;
        self.discover_cron_backup_jobs().await?;
        self.discover_systemd_backup_services().await?;
        
        Ok(())
    }

    async fn discover_rsync_jobs(&mut self) -> Result<()> {
        // Look for rsync processes and configuration files
        let output = Command::new("ps")
            .args(&["aux"])
            .output()?;

        let stdout = str::from_utf8(&output.stdout)?;
        
        for line in stdout.lines() {
            if line.contains("rsync") && !line.contains("grep") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 10 {
                    let job_id = format!("rsync_{}", parts[1]); // Use PID as identifier
                    
                    if !self.backup_jobs.contains_key(&job_id) {
                        let job = BackupJob {
                            job_id: job_id.clone(),
                            name: "Rsync Backup".to_string(),
                            job_type: BackupType::Incremental,
                            source_paths: vec![PathBuf::from("/home")], // Would parse from command
                            destination: "/backup".to_string(),
                            schedule: BackupSchedule {
                                frequency: BackupFrequency::Daily,
                                time_of_day: Some("02:00".to_string()),
                                days_of_week: vec![1, 2, 3, 4, 5], // Weekdays
                                enabled: true,
                            },
                            status: BackupStatus::Running,
                            last_run: Some(Utc::now()),
                            next_run: Some(Utc::now() + Duration::hours(24)),
                            success_rate: 95.0,
                            data_size: 1024 * 1024 * 1024 * 10, // 10GB
                            compression_ratio: 0.7,
                            encryption_enabled: false,
                            retention_policy: RetentionPolicy {
                                keep_daily: 7,
                                keep_weekly: 4,
                                keep_monthly: 12,
                                keep_yearly: 2,
                            },
                        };
                        
                        self.backup_jobs.insert(job_id, job);
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn discover_cron_backup_jobs(&mut self) -> Result<()> {
        // Check crontab for backup-related entries
        let output = Command::new("crontab")
            .args(&["-l"])
            .output();

        if let Ok(output) = output {
            let stdout = str::from_utf8(&output.stdout)?;
            
            for (i, line) in stdout.lines().enumerate() {
                if line.contains("backup") || line.contains("rsync") || line.contains("tar") {
                    let job_id = format!("cron_backup_{}", i);
                    
                    if !self.backup_jobs.contains_key(&job_id) {
                        let job = BackupJob {
                            job_id: job_id.clone(),
                            name: "Cron Backup Job".to_string(),
                            job_type: BackupType::Full,
                            source_paths: vec![PathBuf::from("/home")],
                            destination: "/backup".to_string(),
                            schedule: self.parse_cron_schedule(line),
                            status: BackupStatus::Scheduled,
                            last_run: None,
                            next_run: self.calculate_next_cron_run(line),
                            success_rate: 90.0,
                            data_size: 1024 * 1024 * 1024 * 5, // 5GB
                            compression_ratio: 0.8,
                            encryption_enabled: true,
                            retention_policy: RetentionPolicy {
                                keep_daily: 14,
                                keep_weekly: 8,
                                keep_monthly: 6,
                                keep_yearly: 1,
                            },
                        };
                        
                        self.backup_jobs.insert(job_id, job);
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn discover_systemd_backup_services(&mut self) -> Result<()> {
        // Check for systemd services related to backups
        let output = Command::new("systemctl")
            .args(&["list-units", "--type=service", "--all"])
            .output()?;

        let stdout = str::from_utf8(&output.stdout)?;
        
        for line in stdout.lines() {
            if line.contains("backup") && line.contains(".service") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let service_name = parts[0];
                    let job_id = format!("systemd_{}", service_name);
                    
                    if !self.backup_jobs.contains_key(&job_id) {
                        let job = BackupJob {
                            job_id: job_id.clone(),
                            name: format!("Systemd Service: {}", service_name),
                            job_type: BackupType::Full,
                            source_paths: vec![PathBuf::from("/")],
                            destination: "/backup".to_string(),
                            schedule: BackupSchedule {
                                frequency: BackupFrequency::Daily,
                                time_of_day: Some("03:00".to_string()),
                                days_of_week: vec![0, 1, 2, 3, 4, 5, 6], // All days
                                enabled: true,
                            },
                            status: if line.contains("active") {
                                BackupStatus::Running
                            } else {
                                BackupStatus::Scheduled
                            },
                            last_run: Some(Utc::now() - Duration::hours(12)),
                            next_run: Some(Utc::now() + Duration::hours(12)),
                            success_rate: 98.0,
                            data_size: 1024 * 1024 * 1024 * 20, // 20GB
                            compression_ratio: 0.6,
                            encryption_enabled: true,
                            retention_policy: RetentionPolicy {
                                keep_daily: 30,
                                keep_weekly: 12,
                                keep_monthly: 12,
                                keep_yearly: 3,
                            },
                        };
                        
                        self.backup_jobs.insert(job_id, job);
                    }
                }
            }
        }
        
        Ok(())
    }

    fn parse_cron_schedule(&self, _cron_line: &str) -> BackupSchedule {
        // Simplified cron parsing
        BackupSchedule {
            frequency: BackupFrequency::Daily,
            time_of_day: Some("02:00".to_string()),
            days_of_week: vec![1, 2, 3, 4, 5],
            enabled: true,
        }
    }

    fn calculate_next_cron_run(&self, _cron_line: &str) -> Option<DateTime<Utc>> {
        // Simplified - would implement proper cron parsing
        Some(Utc::now() + Duration::hours(24))
    }

    async fn update_storage_locations(&mut self) -> Result<()> {
        for location_path in &self.config.storage_locations.clone() {
            let location_id = format!("storage_{}", location_path.replace("/", "_"));
            
            if let Ok(_metadata) = fs::metadata(location_path).await {
                // Get filesystem stats
                let output = Command::new("df")
                    .args(&["-B", "1", location_path])
                    .output();

                if let Ok(output) = output {
                    let stdout = str::from_utf8(&output.stdout)?;
                    if let Some(stats) = self.parse_df_output(stdout) {
                        let storage = StorageLocation {
                            location_id: location_id.clone(),
                            name: format!("Backup Storage: {}", location_path),
                            storage_type: StorageType::LocalDisk,
                            path: location_path.clone(),
                            total_capacity: stats.0,
                            used_space: stats.1,
                            available_space: stats.2,
                            is_accessible: true,
                            last_verified: Utc::now(),
                            read_speed: 100.0,  // Mock data
                            write_speed: 80.0,  // Mock data
                        };
                        
                        self.storage_locations.insert(location_id, storage);
                    }
                }
            }
        }
        
        Ok(())
    }

    fn parse_df_output(&self, output: &str) -> Option<(u64, u64, u64)> {
        // Parse df output to get total, used, available space
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let (Ok(total), Ok(used), Ok(available)) = (
                    parts[1].parse::<u64>(),
                    parts[2].parse::<u64>(),
                    parts[3].parse::<u64>()
                ) {
                    return Some((total, used, available));
                }
            }
        }
        None
    }

    async fn discover_recovery_points(&self) -> Result<Vec<RecoveryPoint>> {
        let mut recovery_points = Vec::new();
        
        // Look for backup files in storage locations
        for storage in self.storage_locations.values() {
            if let Ok(mut entries) = fs::read_dir(&storage.path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_file() {
                            let file_name = entry.file_name().to_string_lossy().to_string();
                            
                            if file_name.contains("backup") || file_name.ends_with(".tar.gz") ||
                               file_name.ends_with(".zip") || file_name.ends_with(".bak") {
                                
                                let recovery_point = RecoveryPoint {
                                    recovery_id: format!("rp_{}_{}", storage.location_id, file_name),
                                    job_id: "unknown".to_string(),
                                    created_at: DateTime::from_timestamp(
                                        metadata.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64, 0
                                    ).unwrap_or_else(Utc::now),
                                    backup_type: BackupType::Full,
                                    data_size: metadata.len(),
                                    file_count: 1, // Would need to inspect archive
                                    integrity_verified: false,
                                    recovery_time_estimate: Duration::minutes(30),
                                    tags: vec!["discovered".to_string()],
                                };
                                
                                recovery_points.push(recovery_point);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(recovery_points)
    }

    fn calculate_performance_metrics(&self) -> BackupPerformance {
        let total_data: u64 = self.backup_jobs.values()
            .map(|job| job.data_size)
            .sum();

        let avg_compression: f32 = self.backup_jobs.values()
            .map(|job| job.compression_ratio)
            .sum::<f32>() / self.backup_jobs.len().max(1) as f32;

        BackupPerformance {
            average_backup_speed: 50.0, // MB/s
            average_restore_speed: 70.0, // MB/s
            average_backup_duration: Duration::hours(2),
            total_data_protected: total_data,
            deduplication_ratio: 0.3,
            compression_savings: (1.0 - avg_compression) * 100.0,
            bandwidth_utilization: 25.0,
        }
    }

    async fn check_for_alerts(&mut self) -> Result<Vec<BackupAlert>> {
        let mut alerts = Vec::new();
        
        // Check for failed jobs
        for job in self.backup_jobs.values() {
            if matches!(job.status, BackupStatus::Failed) {
                alerts.push(BackupAlert {
                    alert_id: format!("alert_job_failed_{}", job.job_id),
                    alert_type: BackupAlertType::JobFailed,
                    severity: BackupAlertSeverity::Error,
                    job_id: Some(job.job_id.clone()),
                    storage_id: None,
                    message: format!("Backup job '{}' failed", job.name),
                    triggered_at: Utc::now(),
                    acknowledged: false,
                });
            }
            
            // Check for missed backups
            if let Some(next_run) = job.next_run {
                if Utc::now() > next_run + Duration::hours(1) {
                    alerts.push(BackupAlert {
                        alert_id: format!("alert_job_missed_{}", job.job_id),
                        alert_type: BackupAlertType::JobMissed,
                        severity: BackupAlertSeverity::Warning,
                        job_id: Some(job.job_id.clone()),
                        storage_id: None,
                        message: format!("Backup job '{}' missed scheduled run", job.name),
                        triggered_at: Utc::now(),
                        acknowledged: false,
                    });
                }
            }
        }
        
        // Check storage capacity
        for storage in self.storage_locations.values() {
            let usage_percent = (storage.used_space as f64 / storage.total_capacity as f64) * 100.0;
            
            if usage_percent > 90.0 {
                alerts.push(BackupAlert {
                    alert_id: format!("alert_storage_full_{}", storage.location_id),
                    alert_type: BackupAlertType::StorageFull,
                    severity: BackupAlertSeverity::Critical,
                    job_id: None,
                    storage_id: Some(storage.location_id.clone()),
                    message: format!("Storage '{}' is {:.1}% full", storage.name, usage_percent),
                    triggered_at: Utc::now(),
                    acknowledged: false,
                });
            }
            
            if !storage.is_accessible {
                alerts.push(BackupAlert {
                    alert_id: format!("alert_storage_offline_{}", storage.location_id),
                    alert_type: BackupAlertType::StorageOffline,
                    severity: BackupAlertSeverity::Critical,
                    job_id: None,
                    storage_id: Some(storage.location_id.clone()),
                    message: format!("Storage '{}' is not accessible", storage.name),
                    triggered_at: Utc::now(),
                    acknowledged: false,
                });
            }
        }
        
        Ok(alerts)
    }

    pub fn get_backup_summary(&self) -> Vec<String> {
        let mut summary = Vec::new();
        
        if let Some(metrics) = &self.last_metrics {
            let total_jobs = metrics.backup_jobs.len();
            let active_jobs = metrics.backup_jobs.iter()
                .filter(|j| matches!(j.status, BackupStatus::Running))
                .count();
            let failed_jobs = metrics.backup_jobs.iter()
                .filter(|j| matches!(j.status, BackupStatus::Failed))
                .count();
                
            summary.push(format!("Backup Jobs: {}/{} active", active_jobs, total_jobs));
            
            if failed_jobs > 0 {
                summary.push(format!("❌ {} Failed Jobs", failed_jobs));
            }
            
            let critical_alerts = metrics.alerts.iter()
                .filter(|a| matches!(a.severity, BackupAlertSeverity::Critical))
                .count();
                
            if critical_alerts > 0 {
                summary.push(format!("🚨 {} Critical Alerts", critical_alerts));
            }
            
            let total_protected = metrics.backup_performance.total_data_protected;
            if total_protected > 0 {
                summary.push(format!("Protected: {:.1} GB", 
                    total_protected as f64 / (1024.0 * 1024.0 * 1024.0)));
            }
        }
        
        summary
    }

    pub async fn start_restore(&mut self, recovery_point_id: &str, destination: &str) -> Result<String> {
        let restore_id = format!("restore_{}", Utc::now().timestamp());
        
        let restore_op = RestoreOperation {
            restore_id: restore_id.clone(),
            recovery_point_id: recovery_point_id.to_string(),
            destination_path: destination.to_string(),
            status: RestoreStatus::Preparing,
            started_at: Utc::now(),
            completed_at: None,
            progress_percent: 0.0,
            files_restored: 0,
            bytes_restored: 0,
            estimated_time_remaining: Some(Duration::hours(1)),
        };
        
        self.active_restores.push(restore_op);
        Ok(restore_id)
    }
}

impl Default for BackupMonitor {
    fn default() -> Self {
        Self::with_default_config()
    }
}