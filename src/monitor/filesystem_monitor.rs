use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: FsEventType,
    pub path: PathBuf,
    pub details: FsEventDetails,
    pub severity: FsEventSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FsEventType {
    FileCreated,
    FileModified,
    FileDeleted,
    FileAccessed,
    PermissionChanged,
    OwnershipChanged,
    DirectoryCreated,
    DirectoryDeleted,
    SymlinkCreated,
    IntegrityViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsEventDetails {
    pub file_size: Option<u64>,
    pub permissions: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub file_type: Option<String>,
    pub hash_changed: bool,
    pub previous_hash: Option<String>,
    pub current_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FsEventSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemAlert {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub alert_type: FsAlertType,
    pub severity: FsEventSeverity,
    pub message: String,
    pub affected_path: PathBuf,
    pub event_details: FsEventDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FsAlertType {
    CriticalSystemFileModified,
    UnauthorizedAccess,
    SuspiciousFileCreation,
    IntegrityBreach,
    PermissionEscalation,
    SystemDirectoryTampering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub permissions: String,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub file_type: String,
    pub hash: Option<String>,
    pub last_checked: DateTime<Utc>,
}

#[derive(Debug)]
pub struct FileSystemMonitorConfig {
    pub watch_paths: Vec<PathBuf>,
    pub critical_paths: Vec<PathBuf>,
    pub excluded_paths: Vec<PathBuf>,
    pub check_integrity: bool,
    pub max_events: usize,
    pub scan_interval_seconds: u64,
}

impl Default for FileSystemMonitorConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![
                PathBuf::from("/etc"),
                PathBuf::from("/bin"),
                PathBuf::from("/sbin"),
                PathBuf::from("/usr/bin"),
                PathBuf::from("/usr/sbin"),
                PathBuf::from("/lib"),
                PathBuf::from("/usr/lib"),
                PathBuf::from("/boot"),
                PathBuf::from("/var/log"),
                PathBuf::from("/home"),
            ],
            critical_paths: vec![
                PathBuf::from("/etc/passwd"),
                PathBuf::from("/etc/shadow"),
                PathBuf::from("/etc/sudoers"),
                PathBuf::from("/etc/ssh/sshd_config"),
                PathBuf::from("/etc/hosts"),
                PathBuf::from("/etc/fstab"),
                PathBuf::from("/etc/crontab"),
                PathBuf::from("/boot/grub/grub.cfg"),
                PathBuf::from("/etc/systemd"),
            ],
            excluded_paths: vec![
                PathBuf::from("/proc"),
                PathBuf::from("/sys"),
                PathBuf::from("/dev"),
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
                PathBuf::from("/var/cache"),
                PathBuf::from("/var/run"),
            ],
            check_integrity: true,
            max_events: 1000,
            scan_interval_seconds: 300, // 5 minutes
        }
    }
}

pub struct FileSystemMonitor {
    config: FileSystemMonitorConfig,
    file_metadata: HashMap<PathBuf, FileMetadata>,
    events: Vec<FileSystemEvent>,
    alerts: Vec<FileSystemAlert>,
    last_scan: DateTime<Utc>,
}

impl FileSystemMonitor {
    pub fn new(config: FileSystemMonitorConfig) -> Self {
        Self {
            config,
            file_metadata: HashMap::new(),
            events: Vec::new(),
            alerts: Vec::new(),
            last_scan: Utc::now(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(FileSystemMonitorConfig::default())
    }

    pub async fn update(&mut self) -> Result<()> {
        let now = Utc::now();
        let scan_interval = chrono::Duration::seconds(self.config.scan_interval_seconds as i64);

        if now.signed_duration_since(self.last_scan) >= scan_interval {
            self.perform_scan().await?;
            self.last_scan = now;
        }

        self.cleanup_old_events();
        self.cleanup_old_alerts();

        Ok(())
    }

    async fn perform_scan(&mut self) -> Result<()> {
        for watch_path in &self.config.watch_paths.clone() {
            if watch_path.exists() && !self.is_excluded_path(watch_path) {
                if let Err(e) = self.scan_directory(watch_path).await {
                    eprintln!("Error scanning directory {:?}: {}", watch_path, e);
                }
            }
        }

        // Special handling for critical files
        for critical_path in &self.config.critical_paths.clone() {
            if critical_path.exists() {
                if let Err(e) = self.check_critical_file(critical_path).await {
                    eprintln!("Error checking critical file {:?}: {}", critical_path, e);
                }
            }
        }

        Ok(())
    }

    async fn scan_directory(&mut self, dir_path: &Path) -> Result<()> {
        if !dir_path.is_dir() {
            return self.check_file(dir_path).await;
        }

        // Use iterative approach instead of recursive to avoid stack overflow
        let mut dirs_to_scan = vec![dir_path.to_path_buf()];
        let mut depth = 0;
        const MAX_DEPTH: usize = 5; // Limit depth to prevent excessive scanning

        while let Some(current_dir) = dirs_to_scan.pop() {
            if depth > MAX_DEPTH {
                break;
            }

            let entries = match fs::read_dir(&current_dir) {
                Ok(entries) => entries,
                Err(_) => continue, // Skip directories we can't read
            };
            
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let path = entry.path();
                
                if self.is_excluded_path(&path) {
                    continue;
                }

                if path.is_file() {
                    if let Err(e) = self.check_file(&path).await {
                        eprintln!("Error checking file {:?}: {}", path, e);
                    }
                } else if path.is_dir() && self.should_recurse(&path) && depth < MAX_DEPTH {
                    dirs_to_scan.push(path);
                }
            }
            depth += 1;
        }

        Ok(())
    }

    async fn check_file(&mut self, file_path: &Path) -> Result<()> {
        let metadata = fs::metadata(file_path)?;
        let current_metadata = self.extract_file_metadata(file_path, &metadata).await?;

        if let Some(previous_metadata) = self.file_metadata.get(file_path).cloned() {
            self.compare_and_generate_events(&previous_metadata, &current_metadata).await?;
        } else {
            // New file detected
            self.create_event(
                FsEventType::FileCreated,
                file_path.to_path_buf(),
                current_metadata.clone().into(),
                self.determine_severity(file_path, &FsEventType::FileCreated),
            )?;
        }

        self.file_metadata.insert(file_path.to_path_buf(), current_metadata);
        Ok(())
    }

    async fn extract_file_metadata(&self, file_path: &Path, metadata: &fs::Metadata) -> Result<FileMetadata> {
        let permissions = format!("{:o}", self.get_permissions_mode(metadata));
        let file_type = if metadata.is_file() {
            "file".to_string()
        } else if metadata.is_dir() {
            "directory".to_string()
        } else if metadata.is_symlink() {
            "symlink".to_string()
        } else {
            "other".to_string()
        };

        let hash = if self.config.check_integrity && metadata.is_file() && metadata.len() < 10_000_000 {
            // Only calculate hash for files smaller than 10MB
            self.calculate_file_hash(file_path).await.ok()
        } else {
            None
        };

        Ok(FileMetadata {
            path: file_path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified()?,
            permissions,
            owner: self.get_file_owner(file_path),
            group: self.get_file_group(file_path),
            file_type,
            hash,
            last_checked: Utc::now(),
        })
    }

    async fn compare_and_generate_events(&mut self, previous: &FileMetadata, current: &FileMetadata) -> Result<()> {
        // Check for modifications
        if previous.modified != current.modified || previous.size != current.size {
            self.create_event(
                FsEventType::FileModified,
                current.path.clone(),
                current.clone().into(),
                self.determine_severity(&current.path, &FsEventType::FileModified),
            )?;
        }

        // Check for permission changes
        if previous.permissions != current.permissions {
            self.create_event(
                FsEventType::PermissionChanged,
                current.path.clone(),
                current.clone().into(),
                self.determine_severity(&current.path, &FsEventType::PermissionChanged),
            )?;
        }

        // Check for ownership changes
        if previous.owner != current.owner || previous.group != current.group {
            self.create_event(
                FsEventType::OwnershipChanged,
                current.path.clone(),
                current.clone().into(),
                self.determine_severity(&current.path, &FsEventType::OwnershipChanged),
            )?;
        }

        // Check for integrity violations
        if let (Some(prev_hash), Some(curr_hash)) = (&previous.hash, &current.hash) {
            if prev_hash != curr_hash {
                let mut details = current.clone().into();
                let FsEventDetails { ref mut hash_changed, ref mut previous_hash, .. } = details;
                *hash_changed = true;
                *previous_hash = Some(prev_hash.clone());

                self.create_event(
                    FsEventType::IntegrityViolation,
                    current.path.clone(),
                    details,
                    FsEventSeverity::High,
                )?;

                // Create an alert for integrity violations
                self.create_alert(
                    FsAlertType::IntegrityBreach,
                    FsEventSeverity::High,
                    format!("File integrity violation detected: {:?}", current.path),
                    current.path.clone(),
                    current.clone().into(),
                )?;
            }
        }

        Ok(())
    }

    async fn check_critical_file(&mut self, file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            // Critical file deleted
            self.create_alert(
                FsAlertType::CriticalSystemFileModified,
                FsEventSeverity::Critical,
                format!("Critical system file deleted: {:?}", file_path),
                file_path.to_path_buf(),
                FsEventDetails {
                    file_size: None,
                    permissions: None,
                    owner: None,
                    group: None,
                    file_type: None,
                    hash_changed: false,
                    previous_hash: None,
                    current_hash: None,
                },
            )?;
        } else {
            self.check_file(file_path).await?;
        }

        Ok(())
    }

    fn create_event(
        &mut self,
        event_type: FsEventType,
        path: PathBuf,
        details: FsEventDetails,
        severity: FsEventSeverity,
    ) -> Result<()> {
        let event = FileSystemEvent {
            id: format!("fs-event-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            timestamp: Utc::now(),
            event_type: event_type.clone(),
            path: path.clone(),
            details: details.clone(),
            severity: severity.clone(),
        };

        self.events.push(event);

        // Create alerts for high-severity events
        if severity == FsEventSeverity::High || severity == FsEventSeverity::Critical {
            let alert_type = match path.to_str() {
                Some(p) if self.config.critical_paths.iter().any(|cp| cp.to_str() == Some(p)) => {
                    FsAlertType::CriticalSystemFileModified
                }
                Some(p) if p.contains("/etc") || p.contains("/bin") || p.contains("/sbin") => {
                    FsAlertType::SystemDirectoryTampering
                }
                _ => FsAlertType::SuspiciousFileCreation,
            };

            self.create_alert(
                alert_type,
                severity,
                format!("High-severity filesystem event: {:?} on {:?}", event_type, path),
                path,
                details,
            )?;
        }

        // Limit events size
        if self.events.len() > self.config.max_events {
            self.events.remove(0);
        }

        Ok(())
    }

    fn create_alert(
        &mut self,
        alert_type: FsAlertType,
        severity: FsEventSeverity,
        message: String,
        affected_path: PathBuf,
        event_details: FsEventDetails,
    ) -> Result<()> {
        let alert = FileSystemAlert {
            id: format!("fs-alert-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            timestamp: Utc::now(),
            alert_type,
            severity,
            message,
            affected_path,
            event_details,
        };

        self.alerts.push(alert);
        Ok(())
    }

    fn determine_severity(&self, path: &Path, event_type: &FsEventType) -> FsEventSeverity {
        // Critical paths always get high severity
        if self.config.critical_paths.iter().any(|cp| path.starts_with(cp) || path == cp) {
            return match event_type {
                FsEventType::FileDeleted | FsEventType::IntegrityViolation => FsEventSeverity::Critical,
                FsEventType::FileModified | FsEventType::PermissionChanged => FsEventSeverity::High,
                _ => FsEventSeverity::Medium,
            };
        }

        // System directories
        if let Some(path_str) = path.to_str() {
            if path_str.starts_with("/etc") || path_str.starts_with("/bin") || path_str.starts_with("/sbin") {
                return match event_type {
                    FsEventType::FileDeleted | FsEventType::IntegrityViolation => FsEventSeverity::High,
                    FsEventType::FileModified | FsEventType::PermissionChanged => FsEventSeverity::Medium,
                    _ => FsEventSeverity::Low,
                };
            }
        }

        FsEventSeverity::Low
    }

    fn is_excluded_path(&self, path: &Path) -> bool {
        self.config.excluded_paths.iter().any(|excluded| path.starts_with(excluded))
    }

    fn should_recurse(&self, path: &Path) -> bool {
        // Limit recursion to avoid performance issues
        let depth = path.components().count();
        depth < 10 && !self.is_excluded_path(path)
    }

    async fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        // Simple hash calculation - in production, use a proper hashing library
        let contents = tokio::fs::read(file_path).await?;
        Ok(format!("{:x}", md5::compute(&contents)))
    }

    fn get_permissions_mode(&self, metadata: &fs::Metadata) -> u32 {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        }
        #[cfg(not(unix))]
        {
            0o644 // Default for non-Unix systems
        }
    }

    fn get_file_owner(&self, _file_path: &Path) -> Option<String> {
        // Platform-specific implementation needed
        // For now, return None - in production, use nix crate or similar
        None
    }

    fn get_file_group(&self, _file_path: &Path) -> Option<String> {
        // Platform-specific implementation needed
        // For now, return None - in production, use nix crate or similar
        None
    }

    fn cleanup_old_events(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        self.events.retain(|event| event.timestamp > cutoff);
    }

    fn cleanup_old_alerts(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::hours(6);
        self.alerts.retain(|alert| alert.timestamp > cutoff);
    }

    pub fn get_events(&self) -> &[FileSystemEvent] {
        &self.events
    }

    pub fn get_alerts(&self) -> &[FileSystemAlert] {
        &self.alerts
    }

    pub fn get_events_by_severity(&self, severity: FsEventSeverity) -> Vec<&FileSystemEvent> {
        self.events.iter().filter(|event| event.severity == severity).collect()
    }

    pub fn get_alerts_by_severity(&self, severity: FsEventSeverity) -> Vec<&FileSystemAlert> {
        self.alerts.iter().filter(|alert| alert.severity == severity).collect()
    }

    pub fn get_recent_events(&self, limit: usize) -> Vec<&FileSystemEvent> {
        self.events.iter().rev().take(limit).collect()
    }

    pub fn search_events(&self, query: &str) -> Vec<&FileSystemEvent> {
        let query_lower = query.to_lowercase();
        self.events.iter()
            .filter(|event| {
                event.path.to_string_lossy().to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

impl From<FileMetadata> for FsEventDetails {
    fn from(metadata: FileMetadata) -> Self {
        FsEventDetails {
            file_size: Some(metadata.size),
            permissions: Some(metadata.permissions),
            owner: metadata.owner,
            group: metadata.group,
            file_type: Some(metadata.file_type),
            hash_changed: false,
            previous_hash: None,
            current_hash: metadata.hash,
        }
    }
}