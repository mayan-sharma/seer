use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::str;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub status: ServiceStatus,
    pub state: ServiceState,
    pub enabled: bool,
    pub active_since: Option<DateTime<Utc>>,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub restart_count: u32,
    pub main_pid: Option<u32>,
    pub unit_file_path: String,
    pub service_type: ServiceType,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Active,
    Inactive,
    Failed,
    Activating,
    Deactivating,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceState {
    Enabled,
    Disabled,
    Static,
    Masked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    Simple,
    Forking,
    Oneshot,
    Notify,
    Idle,
    DBus,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub services: Vec<ServiceInfo>,
    pub total_services: usize,
    pub active_services: usize,
    pub failed_services: usize,
    pub enabled_services: usize,
    pub disabled_services: usize,
    pub total_memory_usage: u64,
    pub system_service_manager: ServiceManager,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceManager {
    Systemd,
    InitD,
    Runit,
    OpenRC,
    Unknown,
}

pub struct ServiceMonitor {
    service_manager: ServiceManager,
    cached_services: HashMap<String, ServiceInfo>,
    last_update: Option<DateTime<Utc>>,
}

impl Default for ServiceStatus {
    fn default() -> Self {
        ServiceStatus::Unknown
    }
}

impl From<&str> for ServiceStatus {
    fn from(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "active" => ServiceStatus::Active,
            "inactive" | "dead" => ServiceStatus::Inactive,
            "failed" => ServiceStatus::Failed,
            "activating" => ServiceStatus::Activating,
            "deactivating" => ServiceStatus::Deactivating,
            _ => ServiceStatus::Unknown,
        }
    }
}

impl From<&str> for ServiceState {
    fn from(state: &str) -> Self {
        match state.to_lowercase().as_str() {
            "enabled" => ServiceState::Enabled,
            "disabled" => ServiceState::Disabled,
            "static" => ServiceState::Static,
            "masked" => ServiceState::Masked,
            _ => ServiceState::Unknown,
        }
    }
}

impl From<&str> for ServiceType {
    fn from(service_type: &str) -> Self {
        match service_type.to_lowercase().as_str() {
            "simple" => ServiceType::Simple,
            "forking" => ServiceType::Forking,
            "oneshot" => ServiceType::Oneshot,
            "notify" => ServiceType::Notify,
            "idle" => ServiceType::Idle,
            "dbus" => ServiceType::DBus,
            _ => ServiceType::Unknown,
        }
    }
}

impl ServiceMonitor {
    pub fn new() -> Self {
        let service_manager = Self::detect_service_manager();
        
        Self {
            service_manager,
            cached_services: HashMap::new(),
            last_update: None,
        }
    }

    fn detect_service_manager() -> ServiceManager {
        // Check for systemd
        if Command::new("systemctl").arg("--version").output().is_ok() {
            return ServiceManager::Systemd;
        }
        
        // Check for OpenRC
        if Command::new("rc-service").arg("--version").output().is_ok() {
            return ServiceManager::OpenRC;
        }
        
        // Check for runit
        if std::path::Path::new("/etc/runit").exists() {
            return ServiceManager::Runit;
        }
        
        // Check for init.d
        if std::path::Path::new("/etc/init.d").exists() {
            return ServiceManager::InitD;
        }
        
        ServiceManager::Unknown
    }

    pub fn get_service_metrics(&mut self) -> Result<ServiceMetrics> {
        let services = match self.service_manager {
            ServiceManager::Systemd => self.get_systemd_services()?,
            ServiceManager::OpenRC => self.get_openrc_services()?,
            ServiceManager::InitD => self.get_initd_services()?,
            ServiceManager::Runit => self.get_runit_services()?,
            ServiceManager::Unknown => Vec::new(),
        };

        let active_services = services.iter()
            .filter(|s| s.status == ServiceStatus::Active)
            .count();
        
        let failed_services = services.iter()
            .filter(|s| s.status == ServiceStatus::Failed)
            .count();
        
        let enabled_services = services.iter()
            .filter(|s| s.state == ServiceState::Enabled)
            .count();
        
        let disabled_services = services.iter()
            .filter(|s| s.state == ServiceState::Disabled)
            .count();
        
        let total_memory_usage = services.iter()
            .map(|s| s.memory_usage)
            .sum();

        // Update cache
        self.cached_services.clear();
        for service in &services {
            self.cached_services.insert(service.name.clone(), service.clone());
        }
        self.last_update = Some(Utc::now());

        Ok(ServiceMetrics {
            total_services: services.len(),
            active_services,
            failed_services,
            enabled_services,
            disabled_services,
            total_memory_usage,
            system_service_manager: self.service_manager.clone(),
            services,
        })
    }

    fn get_systemd_services(&self) -> Result<Vec<ServiceInfo>> {
        let mut services = Vec::new();
        
        // Get list of all services
        let output = Command::new("systemctl")
            .args(&["list-units", "--type=service", "--all", "--no-pager", "--plain"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get systemd services"));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        
        for line in stdout.lines().skip(1) { // Skip header
            let line = line.trim();
            if line.is_empty() || line.starts_with("LOAD") {
                continue;
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }
            
            let service_name = parts[0].to_string();
            if !service_name.ends_with(".service") {
                continue;
            }
            
            let service_name = service_name.strip_suffix(".service").unwrap_or(&service_name).to_string();
            
            if let Ok(service_info) = self.get_systemd_service_details(&service_name) {
                services.push(service_info);
            }
        }

        Ok(services)
    }

    fn get_systemd_service_details(&self, service_name: &str) -> Result<ServiceInfo> {
        let service_unit = format!("{}.service", service_name);
        
        // Get service status
        let status_output = Command::new("systemctl")
            .args(&["show", &service_unit, "--no-pager", "--property=Id,Description,ActiveState,UnitFileState,ActiveEnterTimestamp,MainPID,Type,MemoryCurrent,Requires,WantedBy,NRestarts"])
            .output()?;

        if !status_output.status.success() {
            return Err(anyhow!("Failed to get service details for {}", service_name));
        }

        let status_stdout = str::from_utf8(&status_output.stdout)?;
        let mut properties = HashMap::new();
        
        for line in status_stdout.lines() {
            if let Some(eq_pos) = line.find('=') {
                let key = &line[..eq_pos];
                let value = &line[eq_pos + 1..];
                properties.insert(key.to_string(), value.to_string());
            }
        }

        let description = properties.get("Description").cloned().unwrap_or_default();
        let status = ServiceStatus::from(properties.get("ActiveState").map(|s| s.as_str()).unwrap_or("unknown"));
        let state = ServiceState::from(properties.get("UnitFileState").map(|s| s.as_str()).unwrap_or("unknown"));
        
        let active_since = properties.get("ActiveEnterTimestamp")
            .and_then(|ts| {
                if ts.is_empty() || ts == "0" {
                    None
                } else {
                    chrono::DateTime::parse_from_rfc3339(ts).ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                }
            });

        let main_pid = properties.get("MainPID")
            .and_then(|pid_str| {
                if pid_str == "0" {
                    None
                } else {
                    pid_str.parse::<u32>().ok()
                }
            });

        let service_type = ServiceType::from(properties.get("Type").map(|s| s.as_str()).unwrap_or("simple"));
        
        let memory_usage = properties.get("MemoryCurrent")
            .and_then(|mem_str| mem_str.parse::<u64>().ok())
            .unwrap_or(0);

        let restart_count = properties.get("NRestarts")
            .and_then(|count_str| count_str.parse::<u32>().ok())
            .unwrap_or(0);

        // Get CPU usage if service has a main PID
        let cpu_usage = if let Some(pid) = main_pid {
            self.get_process_cpu_usage(pid).unwrap_or(0.0)
        } else {
            0.0
        };

        // Get dependencies
        let dependencies = properties.get("Requires")
            .map(|deps| deps.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let dependents = properties.get("WantedBy")
            .map(|deps| deps.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        // Get unit file path
        let unit_file_path = self.get_systemd_unit_file_path(&service_unit)?;

        Ok(ServiceInfo {
            name: service_name.to_string(),
            description,
            status,
            state: state.clone(),
            enabled: matches!(state, ServiceState::Enabled),
            active_since,
            memory_usage,
            cpu_usage,
            restart_count,
            main_pid,
            unit_file_path,
            service_type,
            dependencies,
            dependents,
        })
    }

    fn get_systemd_unit_file_path(&self, service_unit: &str) -> Result<String> {
        let output = Command::new("systemctl")
            .args(&["show", service_unit, "--property=FragmentPath", "--no-pager"])
            .output()?;

        if output.status.success() {
            let stdout = str::from_utf8(&output.stdout)?;
            for line in stdout.lines() {
                if line.starts_with("FragmentPath=") {
                    return Ok(line[13..].to_string());
                }
            }
        }

        Ok(String::new())
    }

    fn get_process_cpu_usage(&self, pid: u32) -> Result<f64> {
        // Read /proc/pid/stat to get CPU usage
        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = std::fs::read_to_string(&stat_path)?;
        
        let fields: Vec<&str> = stat_content.split_whitespace().collect();
        if fields.len() >= 17 {
            let utime: u64 = fields[13].parse().unwrap_or(0);
            let stime: u64 = fields[14].parse().unwrap_or(0);
            let total_time = utime + stime;
            
            // Simple CPU usage calculation (would need historical data for accuracy)
            let cpu_usage = total_time as f64 / 100.0; // Simplified calculation
            Ok(cpu_usage)
        } else {
            Ok(0.0)
        }
    }

    fn get_openrc_services(&self) -> Result<Vec<ServiceInfo>> {
        // Placeholder implementation for OpenRC
        let mut services = Vec::new();
        
        let output = Command::new("rc-status")
            .args(&["-a"])
            .output()?;

        if !output.status.success() {
            return Ok(services);
        }

        let stdout = str::from_utf8(&output.stdout)?;
        
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("Runlevel:") {
                continue;
            }
            
            // Parse OpenRC service format
            if let Some(service_name) = line.split_whitespace().next() {
                let status = if line.contains("started") {
                    ServiceStatus::Active
                } else {
                    ServiceStatus::Inactive
                };
                
                let service_info = ServiceInfo {
                    name: service_name.to_string(),
                    description: String::new(),
                    status,
                    state: ServiceState::Unknown,
                    enabled: true, // Simplified
                    active_since: None,
                    memory_usage: 0,
                    cpu_usage: 0.0,
                    restart_count: 0,
                    main_pid: None,
                    unit_file_path: format!("/etc/init.d/{}", service_name),
                    service_type: ServiceType::Unknown,
                    dependencies: Vec::new(),
                    dependents: Vec::new(),
                };
                
                services.push(service_info);
            }
        }

        Ok(services)
    }

    fn get_initd_services(&self) -> Result<Vec<ServiceInfo>> {
        // Placeholder implementation for init.d
        let mut services = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir("/etc/init.d") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with('.') {
                        continue;
                    }
                    
                    let service_info = ServiceInfo {
                        name: name.to_string(),
                        description: String::new(),
                        status: ServiceStatus::Unknown,
                        state: ServiceState::Unknown,
                        enabled: false,
                        active_since: None,
                        memory_usage: 0,
                        cpu_usage: 0.0,
                        restart_count: 0,
                        main_pid: None,
                        unit_file_path: format!("/etc/init.d/{}", name),
                        service_type: ServiceType::Unknown,
                        dependencies: Vec::new(),
                        dependents: Vec::new(),
                    };
                    
                    services.push(service_info);
                }
            }
        }

        Ok(services)
    }

    fn get_runit_services(&self) -> Result<Vec<ServiceInfo>> {
        // Placeholder implementation for runit
        let mut services = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir("/etc/sv") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    let service_info = ServiceInfo {
                        name: name.to_string(),
                        description: String::new(),
                        status: ServiceStatus::Unknown,
                        state: ServiceState::Unknown,
                        enabled: false,
                        active_since: None,
                        memory_usage: 0,
                        cpu_usage: 0.0,
                        restart_count: 0,
                        main_pid: None,
                        unit_file_path: format!("/etc/sv/{}", name),
                        service_type: ServiceType::Unknown,
                        dependencies: Vec::new(),
                        dependents: Vec::new(),
                    };
                    
                    services.push(service_info);
                }
            }
        }

        Ok(services)
    }

    pub fn get_service_manager(&self) -> &ServiceManager {
        &self.service_manager
    }

    pub fn is_service_available(&self) -> bool {
        self.service_manager != ServiceManager::Unknown
    }

    pub fn restart_service(&self, service_name: &str) -> Result<()> {
        match self.service_manager {
            ServiceManager::Systemd => {
                let output = Command::new("systemctl")
                    .args(&["restart", &format!("{}.service", service_name)])
                    .output()?;
                
                if !output.status.success() {
                    return Err(anyhow!("Failed to restart service: {}", service_name));
                }
            }
            _ => return Err(anyhow!("Service restart not implemented for this service manager")),
        }
        
        Ok(())
    }

    pub fn stop_service(&self, service_name: &str) -> Result<()> {
        match self.service_manager {
            ServiceManager::Systemd => {
                let output = Command::new("systemctl")
                    .args(&["stop", &format!("{}.service", service_name)])
                    .output()?;
                
                if !output.status.success() {
                    return Err(anyhow!("Failed to stop service: {}", service_name));
                }
            }
            _ => return Err(anyhow!("Service stop not implemented for this service manager")),
        }
        
        Ok(())
    }

    pub fn start_service(&self, service_name: &str) -> Result<()> {
        match self.service_manager {
            ServiceManager::Systemd => {
                let output = Command::new("systemctl")
                    .args(&["start", &format!("{}.service", service_name)])
                    .output()?;
                
                if !output.status.success() {
                    return Err(anyhow!("Failed to start service: {}", service_name));
                }
            }
            _ => return Err(anyhow!("Service start not implemented for this service manager")),
        }
        
        Ok(())
    }
}