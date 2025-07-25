use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::str;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub state: String,
    pub created: DateTime<Utc>,
    pub ports: Vec<PortMapping>,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
    pub pids: u32,
    pub labels: HashMap<String, String>,
    pub runtime: ContainerRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerStatus {
    Running,
    Exited,
    Created,
    Restarting,
    Paused,
    Dead,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetrics {
    pub containers: Vec<ContainerInfo>,
    pub total_containers: usize,
    pub running_containers: usize,
    pub stopped_containers: usize,
    pub images_count: usize,
    pub total_cpu_usage: f64,
    pub total_memory_usage: u64,
    pub runtimes_available: Vec<ContainerRuntime>,
}

pub struct ContainerMonitor {
    available_runtimes: Vec<ContainerRuntime>,
}

impl Default for ContainerStatus {
    fn default() -> Self {
        ContainerStatus::Unknown
    }
}

impl From<&str> for ContainerStatus {
    fn from(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "running" | "up" => ContainerStatus::Running,
            "exited" | "stopped" => ContainerStatus::Exited,
            "created" => ContainerStatus::Created,
            "restarting" => ContainerStatus::Restarting,
            "paused" => ContainerStatus::Paused,
            "dead" => ContainerStatus::Dead,
            _ => ContainerStatus::Unknown,
        }
    }
}

impl ContainerMonitor {
    pub fn new() -> Self {
        let mut available_runtimes = Vec::new();
        
        // Check for Docker
        if Command::new("docker").arg("--version").output().is_ok() {
            available_runtimes.push(ContainerRuntime::Docker);
        }
        
        // Check for Podman
        if Command::new("podman").arg("--version").output().is_ok() {
            available_runtimes.push(ContainerRuntime::Podman);
        }

        Self { available_runtimes }
    }

    pub fn get_container_metrics(&self) -> Result<ContainerMetrics> {
        let mut all_containers = Vec::new();
        let mut images_count = 0;

        for runtime in &self.available_runtimes {
            match runtime {
                ContainerRuntime::Docker => {
                    if let Ok(containers) = self.get_docker_containers() {
                        all_containers.extend(containers);
                    }
                    if let Ok(count) = self.get_docker_images_count() {
                        images_count += count;
                    }
                }
                ContainerRuntime::Podman => {
                    if let Ok(containers) = self.get_podman_containers() {
                        all_containers.extend(containers);
                    }
                    if let Ok(count) = self.get_podman_images_count() {
                        images_count += count;
                    }
                }
            }
        }

        let running_containers = all_containers.iter()
            .filter(|c| c.status == ContainerStatus::Running)
            .count();
        
        let stopped_containers = all_containers.len() - running_containers;
        
        let total_cpu_usage = all_containers.iter()
            .map(|c| c.cpu_usage)
            .sum();
        
        let total_memory_usage = all_containers.iter()
            .map(|c| c.memory_usage)
            .sum();

        Ok(ContainerMetrics {
            total_containers: all_containers.len(),
            running_containers,
            stopped_containers,
            images_count,
            total_cpu_usage,
            total_memory_usage,
            containers: all_containers,
            runtimes_available: self.available_runtimes.clone(),
        })
    }

    fn get_docker_containers(&self) -> Result<Vec<ContainerInfo>> {
        let output = Command::new("docker")
            .args(&["ps", "-a", "--format", "json"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = str::from_utf8(&output.stdout)?;
        let mut containers = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            if let Ok(container) = self.parse_docker_container(line) {
                containers.push(container);
            }
        }

        // Get detailed stats for running containers
        for container in &mut containers {
            if container.status == ContainerStatus::Running {
                if let Ok(stats) = self.get_docker_container_stats(&container.id) {
                    container.cpu_usage = stats.0;
                    container.memory_usage = stats.1;
                    container.memory_limit = stats.2;
                    container.network_rx = stats.3;
                    container.network_tx = stats.4;
                    container.block_read = stats.5;
                    container.block_write = stats.6;
                    container.pids = stats.7;
                }
            }
        }

        Ok(containers)
    }

    fn get_podman_containers(&self) -> Result<Vec<ContainerInfo>> {
        let output = Command::new("podman")
            .args(&["ps", "-a", "--format", "json"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = str::from_utf8(&output.stdout)?;
        let mut containers = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            if let Ok(container) = self.parse_podman_container(line) {
                containers.push(container);
            }
        }

        // Get detailed stats for running containers
        for container in &mut containers {
            if container.status == ContainerStatus::Running {
                if let Ok(stats) = self.get_podman_container_stats(&container.id) {
                    container.cpu_usage = stats.0;
                    container.memory_usage = stats.1;
                    container.memory_limit = stats.2;
                    container.network_rx = stats.3;
                    container.network_tx = stats.4;
                    container.block_read = stats.5;
                    container.block_write = stats.6;
                    container.pids = stats.7;
                }
            }
        }

        Ok(containers)
    }

    fn parse_docker_container(&self, json_line: &str) -> Result<ContainerInfo> {
        #[derive(Deserialize)]
        struct DockerContainer {
            #[serde(rename = "ID")]
            id: String,
            #[serde(rename = "Names")]
            names: String,
            #[serde(rename = "Image")]
            image: String,
            #[serde(rename = "State")]
            state: String,
            #[serde(rename = "Status")]
            status: String,
            #[serde(rename = "CreatedAt")]
            created_at: String,
            #[serde(rename = "Ports")]
            ports: String,
            #[serde(rename = "Labels")]
            labels: String,
        }

        let docker_container: DockerContainer = serde_json::from_str(json_line)?;
        
        let created = chrono::DateTime::parse_from_rfc3339(&docker_container.created_at)
            .unwrap_or_else(|_| chrono::Utc::now().into())
            .with_timezone(&chrono::Utc);

        let ports = self.parse_ports(&docker_container.ports);
        let labels = self.parse_labels(&docker_container.labels);

        Ok(ContainerInfo {
            id: docker_container.id,
            name: docker_container.names.trim_start_matches('/').to_string(),
            image: docker_container.image,
            status: ContainerStatus::from(docker_container.state.as_str()),
            state: docker_container.status,
            created,
            ports,
            labels,
            runtime: ContainerRuntime::Docker,
            cpu_usage: 0.0,
            memory_usage: 0,
            memory_limit: 0,
            network_rx: 0,
            network_tx: 0,
            block_read: 0,
            block_write: 0,
            pids: 0,
        })
    }

    fn parse_podman_container(&self, json_line: &str) -> Result<ContainerInfo> {
        #[derive(Deserialize)]
        struct PodmanContainer {
            #[serde(rename = "Id")]
            id: String,
            #[serde(rename = "Names")]
            names: Vec<String>,
            #[serde(rename = "Image")]
            image: String,
            #[serde(rename = "State")]
            state: String,
            #[serde(rename = "Status")]
            status: String,
            #[serde(rename = "Created")]
            created: i64,
            #[serde(rename = "Ports")]
            ports: Option<Vec<serde_json::Value>>,
            #[serde(rename = "Labels")]
            labels: Option<HashMap<String, String>>,
        }

        let podman_container: PodmanContainer = serde_json::from_str(json_line)?;
        
        let created = chrono::DateTime::from_timestamp(podman_container.created, 0)
            .unwrap_or_else(|| chrono::Utc::now());

        let name = podman_container.names.first()
            .cloned()
            .unwrap_or_else(|| podman_container.id[..12].to_string());

        let ports = Vec::new(); // TODO: Parse Podman ports format
        let labels = podman_container.labels.unwrap_or_default();

        Ok(ContainerInfo {
            id: podman_container.id,
            name,
            image: podman_container.image,
            status: ContainerStatus::from(podman_container.state.as_str()),
            state: podman_container.status,
            created,
            ports,
            labels,
            runtime: ContainerRuntime::Podman,
            cpu_usage: 0.0,
            memory_usage: 0,
            memory_limit: 0,
            network_rx: 0,
            network_tx: 0,
            block_read: 0,
            block_write: 0,
            pids: 0,
        })
    }

    fn get_docker_container_stats(&self, container_id: &str) -> Result<(f64, u64, u64, u64, u64, u64, u64, u32)> {
        let output = Command::new("docker")
            .args(&["stats", "--no-stream", "--format", "json", container_id])
            .output()?;

        if !output.status.success() {
            return Ok((0.0, 0, 0, 0, 0, 0, 0, 0));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        
        #[derive(Deserialize)]
        struct DockerStats {
            #[serde(rename = "CPUPerc")]
            cpu_perc: String,
            #[serde(rename = "MemUsage")]
            mem_usage: String,
            #[serde(rename = "NetIO")]
            net_io: String,
            #[serde(rename = "BlockIO")]
            block_io: String,
            #[serde(rename = "PIDs")]
            pids: String,
        }

        let stats: DockerStats = serde_json::from_str(stdout.trim())?;
        
        let cpu_usage = stats.cpu_perc.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
        
        let (memory_usage, memory_limit) = self.parse_memory_usage(&stats.mem_usage);
        let (network_rx, network_tx) = self.parse_network_io(&stats.net_io);
        let (block_read, block_write) = self.parse_block_io(&stats.block_io);
        let pids = stats.pids.parse::<u32>().unwrap_or(0);

        Ok((cpu_usage, memory_usage, memory_limit, network_rx, network_tx, block_read, block_write, pids))
    }

    fn get_podman_container_stats(&self, container_id: &str) -> Result<(f64, u64, u64, u64, u64, u64, u64, u32)> {
        let output = Command::new("podman")
            .args(&["stats", "--no-stream", "--format", "json", container_id])
            .output()?;

        if !output.status.success() {
            return Ok((0.0, 0, 0, 0, 0, 0, 0, 0));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        
        #[derive(Deserialize)]
        struct PodmanStats {
            #[serde(rename = "CPU")]
            cpu: String,
            #[serde(rename = "MemUsage")]
            mem_usage: String,
            #[serde(rename = "NetIO")]
            net_io: String,
            #[serde(rename = "BlockIO")]
            block_io: String,
            #[serde(rename = "PIDS")]
            pids: String,
        }

        let stats: PodmanStats = serde_json::from_str(stdout.trim())?;
        
        let cpu_usage = stats.cpu.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
        
        let (memory_usage, memory_limit) = self.parse_memory_usage(&stats.mem_usage);
        let (network_rx, network_tx) = self.parse_network_io(&stats.net_io);
        let (block_read, block_write) = self.parse_block_io(&stats.block_io);
        let pids = stats.pids.parse::<u32>().unwrap_or(0);

        Ok((cpu_usage, memory_usage, memory_limit, network_rx, network_tx, block_read, block_write, pids))
    }

    fn get_docker_images_count(&self) -> Result<usize> {
        let output = Command::new("docker")
            .args(&["images", "-q"])
            .output()?;

        if !output.status.success() {
            return Ok(0);
        }

        let stdout = str::from_utf8(&output.stdout)?;
        Ok(stdout.lines().filter(|line| !line.trim().is_empty()).count())
    }

    fn get_podman_images_count(&self) -> Result<usize> {
        let output = Command::new("podman")
            .args(&["images", "-q"])
            .output()?;

        if !output.status.success() {
            return Ok(0);
        }

        let stdout = str::from_utf8(&output.stdout)?;
        Ok(stdout.lines().filter(|line| !line.trim().is_empty()).count())
    }

    fn parse_ports(&self, ports_str: &str) -> Vec<PortMapping> {
        let mut ports = Vec::new();
        
        for port_mapping in ports_str.split(',') {
            let port_mapping = port_mapping.trim();
            if port_mapping.is_empty() {
                continue;
            }
            
            // Parse formats like "0.0.0.0:8080->80/tcp" or "80/tcp"
            if let Some(arrow_pos) = port_mapping.find("->") {
                let host_part = &port_mapping[..arrow_pos];
                let container_part = &port_mapping[arrow_pos + 2..];
                
                let host_port = host_part.split(':').last()
                    .and_then(|p| p.parse::<u16>().ok());
                
                if let Some(slash_pos) = container_part.find('/') {
                    let container_port = container_part[..slash_pos].parse::<u16>().unwrap_or(0);
                    let protocol = container_part[slash_pos + 1..].to_string();
                    
                    ports.push(PortMapping {
                        container_port,
                        host_port,
                        protocol,
                    });
                }
            }
        }
        
        ports
    }

    fn parse_labels(&self, labels_str: &str) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        
        // Labels in Docker format are comma-separated key=value pairs
        for label in labels_str.split(',') {
            let label = label.trim();
            if let Some(eq_pos) = label.find('=') {
                let key = label[..eq_pos].to_string();
                let value = label[eq_pos + 1..].to_string();
                labels.insert(key, value);
            }
        }
        
        labels
    }

    fn parse_memory_usage(&self, mem_usage: &str) -> (u64, u64) {
        // Parse format like "1.2GiB / 4GiB"
        let parts: Vec<&str> = mem_usage.split(" / ").collect();
        if parts.len() != 2 {
            return (0, 0);
        }
        
        let usage = self.parse_memory_size(parts[0]).unwrap_or(0);
        let limit = self.parse_memory_size(parts[1]).unwrap_or(0);
        
        (usage, limit)
    }

    fn parse_memory_size(&self, size_str: &str) -> Result<u64> {
        let size_str = size_str.trim();
        let (number_part, unit_part) = if let Some(pos) = size_str.find(|c: char| c.is_alphabetic()) {
            (&size_str[..pos], &size_str[pos..])
        } else {
            (size_str, "")
        };
        
        let number: f64 = number_part.parse()?;
        let multiplier = match unit_part.to_uppercase().as_str() {
            "B" => 1,
            "KB" | "KIB" => 1024,
            "MB" | "MIB" => 1024 * 1024,
            "GB" | "GIB" => 1024 * 1024 * 1024,
            "TB" | "TIB" => 1024_u64.pow(4),
            _ => 1,
        };
        
        Ok((number * multiplier as f64) as u64)
    }

    fn parse_network_io(&self, net_io: &str) -> (u64, u64) {
        // Parse format like "1.2kB / 3.4MB"
        let parts: Vec<&str> = net_io.split(" / ").collect();
        if parts.len() != 2 {
            return (0, 0);
        }
        
        let rx = self.parse_memory_size(parts[0]).unwrap_or(0);
        let tx = self.parse_memory_size(parts[1]).unwrap_or(0);
        
        (rx, tx)
    }

    fn parse_block_io(&self, block_io: &str) -> (u64, u64) {
        // Parse format like "1.2MB / 3.4MB"
        let parts: Vec<&str> = block_io.split(" / ").collect();
        if parts.len() != 2 {
            return (0, 0);
        }
        
        let read = self.parse_memory_size(parts[0]).unwrap_or(0);
        let write = self.parse_memory_size(parts[1]).unwrap_or(0);
        
        (read, write)
    }

    pub fn has_runtime(&self, runtime: &ContainerRuntime) -> bool {
        self.available_runtimes.contains(runtime)
    }

    pub fn get_available_runtimes(&self) -> &[ContainerRuntime] {
        &self.available_runtimes
    }
}