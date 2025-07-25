use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::str;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub user: String,
    pub session_id: String,
    pub session_type: SessionType,
    pub state: SessionState,
    pub tty: Option<String>,
    pub display: Option<String>,
    pub remote_host: Option<String>,
    pub login_time: DateTime<Utc>,
    pub idle_time: Option<u64>, // seconds
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub processes: Vec<u32>, // PIDs of processes owned by this user
    pub uid: u32,
    pub gid: u32,
    pub home_dir: String,
    pub shell: String,
    pub seat: Option<String>,
    pub service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionType {
    TTY,
    X11,
    Wayland,
    SSH,
    Console,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Active,
    Online,
    Closing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub sessions: Vec<UserSession>,
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub unique_users: usize,
    pub ssh_sessions: usize,
    pub local_sessions: usize,
    pub total_memory_usage: u64,
    pub total_cpu_usage: f64,
    pub login_attempts_failed: u32,
    pub session_manager: SessionManager,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionManager {
    Systemd,
    ConsoleKit,
    Traditional,
    Unknown,
}

pub struct SessionMonitor {
    session_manager: SessionManager,
    cached_sessions: HashMap<String, UserSession>,
    last_update: Option<DateTime<Utc>>,
}

impl Default for SessionType {
    fn default() -> Self {
        SessionType::Unknown
    }
}

impl From<&str> for SessionType {
    fn from(session_type: &str) -> Self {
        match session_type.to_lowercase().as_str() {
            "tty" => SessionType::TTY,
            "x11" => SessionType::X11,
            "wayland" => SessionType::Wayland,
            "ssh" => SessionType::SSH,
            "console" => SessionType::Console,
            _ => SessionType::Unknown,
        }
    }
}

impl From<&str> for SessionState {
    fn from(state: &str) -> Self {
        match state.to_lowercase().as_str() {
            "active" => SessionState::Active,
            "online" => SessionState::Online,
            "closing" => SessionState::Closing,
            _ => SessionState::Unknown,
        }
    }
}

impl SessionMonitor {
    pub fn new() -> Self {
        let session_manager = Self::detect_session_manager();
        
        Self {
            session_manager,
            cached_sessions: HashMap::new(),
            last_update: None,
        }
    }

    fn detect_session_manager() -> SessionManager {
        // Check for systemd-logind
        if Command::new("loginctl").arg("--version").output().is_ok() {
            return SessionManager::Systemd;
        }
        
        // Check for ConsoleKit
        if Command::new("ck-list-sessions").output().is_ok() {
            return SessionManager::ConsoleKit;
        }
        
        // Fallback to traditional methods
        SessionManager::Traditional
    }

    pub fn get_session_metrics(&mut self) -> Result<SessionMetrics> {
        let sessions = match self.session_manager {
            SessionManager::Systemd => self.get_systemd_sessions()?,
            SessionManager::ConsoleKit => self.get_consolekit_sessions()?,
            SessionManager::Traditional => self.get_traditional_sessions()?,
            SessionManager::Unknown => Vec::new(),
        };

        let active_sessions = sessions.iter()
            .filter(|s| s.state == SessionState::Active)
            .count();

        let mut unique_users = std::collections::HashSet::new();
        for session in &sessions {
            unique_users.insert(&session.user);
        }

        let ssh_sessions = sessions.iter()
            .filter(|s| s.session_type == SessionType::SSH)
            .count();

        let local_sessions = sessions.len() - ssh_sessions;

        let total_memory_usage = sessions.iter()
            .map(|s| s.memory_usage)
            .sum();

        let total_cpu_usage = sessions.iter()
            .map(|s| s.cpu_usage)
            .sum();

        let login_attempts_failed = self.get_failed_login_attempts()?;

        // Update cache
        self.cached_sessions.clear();
        for session in &sessions {
            self.cached_sessions.insert(session.session_id.clone(), session.clone());
        }
        self.last_update = Some(Utc::now());

        Ok(SessionMetrics {
            total_sessions: sessions.len(),
            active_sessions,
            unique_users: unique_users.len(),
            ssh_sessions,
            local_sessions,
            total_memory_usage,
            total_cpu_usage,
            login_attempts_failed,
            session_manager: self.session_manager.clone(),
            sessions,
        })
    }

    fn get_systemd_sessions(&self) -> Result<Vec<UserSession>> {
        let mut sessions = Vec::new();

        // Get list of sessions from loginctl
        let output = Command::new("loginctl")
            .args(&["list-sessions", "--no-pager", "--no-legend"])
            .output()?;

        if !output.status.success() {
            return Ok(sessions);
        }

        let stdout = str::from_utf8(&output.stdout)?;

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let session_id = parts[0].to_string();
                let uid = parts[1].parse::<u32>().unwrap_or(0);
                let user = parts[2].to_string();

                if let Ok(session_info) = self.get_systemd_session_details(&session_id, uid, &user) {
                    sessions.push(session_info);
                }
            }
        }

        Ok(sessions)
    }

    fn get_systemd_session_details(&self, session_id: &str, uid: u32, user: &str) -> Result<UserSession> {
        // Get session properties
        let properties_output = Command::new("loginctl")
            .args(&["show-session", session_id, "--no-pager", "--property=Type,State,TTY,Display,RemoteHost,Timestamp,IdleHint,IdleSinceHint,Seat,Service"])
            .output()?;

        if !properties_output.status.success() {
            return Err(anyhow!("Failed to get session details for {}", session_id));
        }

        let properties_stdout = str::from_utf8(&properties_output.stdout)?;
        let mut properties = HashMap::new();

        for line in properties_stdout.lines() {
            if let Some(eq_pos) = line.find('=') {
                let key = &line[..eq_pos];
                let value = &line[eq_pos + 1..];
                properties.insert(key.to_string(), value.to_string());
            }
        }

        // Get user information
        let user_info = self.get_user_info(uid)?;

        let session_type = SessionType::from(properties.get("Type").map(|s| s.as_str()).unwrap_or("unknown"));
        let state = SessionState::from(properties.get("State").map(|s| s.as_str()).unwrap_or("unknown"));

        let tty = properties.get("TTY").filter(|s| !s.is_empty()).cloned();
        let display = properties.get("Display").filter(|s| !s.is_empty()).cloned();
        let remote_host = properties.get("RemoteHost").filter(|s| !s.is_empty()).cloned();
        let seat = properties.get("Seat").filter(|s| !s.is_empty()).cloned();
        let service = properties.get("Service").filter(|s| !s.is_empty()).cloned();

        // Parse login time
        let login_time = properties.get("Timestamp")
            .and_then(|ts| {
                if ts.is_empty() || ts == "0" {
                    None
                } else {
                    // Try to parse microseconds timestamp
                    ts.parse::<i64>().ok()
                        .and_then(|micros| chrono::DateTime::from_timestamp_micros(micros))
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                }
            })
            .unwrap_or_else(Utc::now);

        // Get idle time
        let idle_time = properties.get("IdleSinceHint")
            .and_then(|idle_str| {
                if idle_str.is_empty() || idle_str == "0" {
                    None
                } else {
                    idle_str.parse::<i64>().ok()
                        .and_then(|micros| chrono::DateTime::from_timestamp_micros(micros))
                        .map(|idle_dt| {
                            let now = Utc::now();
                            (now - idle_dt).num_seconds() as u64
                        })
                }
            });

        // Get processes owned by this user
        let processes = self.get_user_processes(uid)?;
        
        // Calculate resource usage
        let (cpu_usage, memory_usage) = self.calculate_user_resource_usage(&processes)?;

        Ok(UserSession {
            user: user.to_string(),
            session_id: session_id.to_string(),
            session_type,
            state,
            tty,
            display,
            remote_host,
            login_time,
            idle_time,
            cpu_usage,
            memory_usage,
            processes,
            uid,
            gid: user_info.gid,
            home_dir: user_info.home_dir,
            shell: user_info.shell,
            seat,
            service,
        })
    }

    fn get_consolekit_sessions(&self) -> Result<Vec<UserSession>> {
        let mut sessions = Vec::new();

        // Get sessions from ConsoleKit
        let output = Command::new("ck-list-sessions")
            .output()?;

        if !output.status.success() {
            return Ok(sessions);
        }

        let stdout = str::from_utf8(&output.stdout)?;
        
        // Parse ConsoleKit output (simplified implementation)
        for line in stdout.lines() {
            if line.contains("Session") && line.contains(":") {
                // This is a simplified parser - real implementation would be more complex
                if let Some(session_info) = self.parse_consolekit_session(line)? {
                    sessions.push(session_info);
                }
            }
        }

        Ok(sessions)
    }

    fn parse_consolekit_session(&self, _line: &str) -> Result<Option<UserSession>> {
        // Placeholder implementation for ConsoleKit parsing
        // Real implementation would parse the ConsoleKit session format
        Ok(None)
    }

    fn get_traditional_sessions(&self) -> Result<Vec<UserSession>> {
        let mut sessions = Vec::new();

        // Use 'who' command for traditional session information
        let output = Command::new("who")
            .args(&["-H", "-u"])
            .output()?;

        if !output.status.success() {
            return Ok(sessions);
        }

        let stdout = str::from_utf8(&output.stdout)?;

        for (line_num, line) in stdout.lines().enumerate() {
            if line_num == 0 {
                continue; // Skip header
            }
            
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(session_info) = self.parse_who_output(line) {
                sessions.push(session_info);
            }
        }

        Ok(sessions)
    }

    fn parse_who_output(&self, line: &str) -> Result<UserSession> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(anyhow!("Invalid who output format"));
        }

        let user = parts[0].to_string();
        let tty = Some(parts[1].to_string());
        let login_time_str = format!("{} {}", parts[2], parts[3]);
        
        // Parse login time
        let login_time = chrono::NaiveDateTime::parse_from_str(&login_time_str, "%Y-%m-%d %H:%M")
            .map(|dt| dt.and_utc())
            .unwrap_or_else(|_| Utc::now());

        // Get user info
        let user_info = self.get_user_info_by_name(&user)?;
        let processes = self.get_user_processes(user_info.uid)?;
        let (cpu_usage, memory_usage) = self.calculate_user_resource_usage(&processes)?;

        // Determine session type based on TTY
        let session_type = if let Some(ref tty_name) = tty {
            if tty_name.starts_with("pts/") {
                SessionType::SSH
            } else if tty_name.starts_with("tty") {
                SessionType::TTY
            } else {
                SessionType::Unknown
            }
        } else {
            SessionType::Unknown
        };

        Ok(UserSession {
            user,
            session_id: format!("traditional-{}-{}", user_info.uid, login_time.timestamp()),
            session_type,
            state: SessionState::Active, // Assume active for traditional sessions
            tty,
            display: None,
            remote_host: None, // Could be parsed from who output if available
            login_time,
            idle_time: None,
            cpu_usage,
            memory_usage,
            processes,
            uid: user_info.uid,
            gid: user_info.gid,
            home_dir: user_info.home_dir,
            shell: user_info.shell,
            seat: None,
            service: None,
        })
    }

    fn get_user_info(&self, uid: u32) -> Result<UserInfo> {
        let output = Command::new("getent")
            .args(&["passwd", &uid.to_string()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get user info for UID {}", uid));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        self.parse_passwd_entry(stdout.trim())
    }

    fn get_user_info_by_name(&self, username: &str) -> Result<UserInfo> {
        let output = Command::new("getent")
            .args(&["passwd", username])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get user info for {}", username));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        self.parse_passwd_entry(stdout.trim())
    }

    fn parse_passwd_entry(&self, entry: &str) -> Result<UserInfo> {
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() < 7 {
            return Err(anyhow!("Invalid passwd entry format"));
        }

        Ok(UserInfo {
            username: parts[0].to_string(),
            uid: parts[2].parse::<u32>()?,
            gid: parts[3].parse::<u32>()?,
            home_dir: parts[5].to_string(),
            shell: parts[6].to_string(),
        })
    }

    fn get_user_processes(&self, uid: u32) -> Result<Vec<u32>> {
        let mut processes = Vec::new();

        // Read /proc to find processes owned by this user
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(pid) = name.parse::<u32>() {
                        let status_path = format!("/proc/{}/status", pid);
                        if let Ok(status_content) = std::fs::read_to_string(&status_path) {
                            for line in status_content.lines() {
                                if line.starts_with("Uid:") {
                                    let parts: Vec<&str> = line.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        if let Ok(proc_uid) = parts[1].parse::<u32>() {
                                            if proc_uid == uid {
                                                processes.push(pid);
                                                break;
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(processes)
    }

    fn calculate_user_resource_usage(&self, processes: &[u32]) -> Result<(f64, u64)> {
        let mut total_cpu = 0.0;
        let mut total_memory = 0u64;

        for &pid in processes {
            // Read CPU and memory usage from /proc/pid/stat and /proc/pid/status
            let stat_path = format!("/proc/{}/stat", pid);
            let status_path = format!("/proc/{}/status", pid);

            // Get memory usage
            if let Ok(status_content) = std::fs::read_to_string(&status_path) {
                for line in status_content.lines() {
                    if line.starts_with("VmRSS:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(memory_kb) = parts[1].parse::<u64>() {
                                total_memory += memory_kb * 1024; // Convert to bytes
                            }
                        }
                        break;
                    }
                }
            }

            // Get CPU usage (simplified)
            if let Ok(stat_content) = std::fs::read_to_string(&stat_path) {
                let fields: Vec<&str> = stat_content.split_whitespace().collect();
                if fields.len() >= 17 {
                    let utime: u64 = fields[13].parse().unwrap_or(0);
                    let stime: u64 = fields[14].parse().unwrap_or(0);
                    let total_time = utime + stime;
                    total_cpu += total_time as f64 / 100.0; // Simplified calculation
                }
            }
        }

        Ok((total_cpu, total_memory))
    }

    fn get_failed_login_attempts(&self) -> Result<u32> {
        // Try to read failed login attempts from system logs
        let mut failed_attempts = 0u32;

        // Check /var/log/auth.log or similar
        let log_paths = ["/var/log/auth.log", "/var/log/secure", "/var/log/messages"];
        
        for log_path in &log_paths {
            if let Ok(content) = std::fs::read_to_string(log_path) {
                // Count lines containing failed login attempts
                failed_attempts += content.lines()
                    .filter(|line| {
                        line.contains("Failed password") || 
                        line.contains("authentication failure") ||
                        line.contains("Invalid user")
                    })
                    .count() as u32;
                break; // Use first available log file
            }
        }

        Ok(failed_attempts)
    }

    pub fn get_session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    pub fn is_session_monitoring_available(&self) -> bool {
        self.session_manager != SessionManager::Unknown
    }

    pub fn terminate_session(&self, session_id: &str) -> Result<()> {
        match self.session_manager {
            SessionManager::Systemd => {
                let output = Command::new("loginctl")
                    .args(&["terminate-session", session_id])
                    .output()?;
                
                if !output.status.success() {
                    return Err(anyhow!("Failed to terminate session: {}", session_id));
                }
            }
            _ => return Err(anyhow!("Session termination not implemented for this session manager")),
        }
        
        Ok(())
    }

    pub fn kill_user_processes(&self, username: &str) -> Result<()> {
        match self.session_manager {
            SessionManager::Systemd => {
                let output = Command::new("loginctl")
                    .args(&["kill-user", username])
                    .output()?;
                
                if !output.status.success() {
                    return Err(anyhow!("Failed to kill user processes for: {}", username));
                }
            }
            _ => return Err(anyhow!("Kill user processes not implemented for this session manager")),
        }
        
        Ok(())
    }
}

#[derive(Debug)]
struct UserInfo {
    username: String,
    uid: u32,
    gid: u32,
    home_dir: String,
    shell: String,
}