use sysinfo::ProcessStatus;
use crate::monitor::SystemMonitor;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub memory_percentage: f32,
    pub user: String,
    pub status: ProcessStatusInfo,
    pub is_zombie: bool,
    pub parent_pid: Option<u32>,
    pub command: String,
    pub start_time: u64,
    pub exe_path: Option<String>,
    pub working_directory: Option<String>,
    pub group_name: Option<String>,
    pub threads_count: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ProcessStatusInfo {
    Running,
    Sleeping,
    Waiting,
    Zombie,
    Stopped,
    Tracing,
    Dead,
    Unknown,
}

impl From<ProcessStatus> for ProcessStatusInfo {
    fn from(status: ProcessStatus) -> Self {
        match status {
            ProcessStatus::Run => ProcessStatusInfo::Running,
            ProcessStatus::Sleep => ProcessStatusInfo::Sleeping,
            ProcessStatus::Stop => ProcessStatusInfo::Stopped,
            ProcessStatus::Zombie => ProcessStatusInfo::Zombie,
            ProcessStatus::Tracing => ProcessStatusInfo::Tracing,
            ProcessStatus::Dead => ProcessStatusInfo::Dead,
            ProcessStatus::Wakekill => ProcessStatusInfo::Waiting,
            ProcessStatus::Waking => ProcessStatusInfo::Waiting,
            ProcessStatus::Parked => ProcessStatusInfo::Waiting,
            ProcessStatus::LockBlocked => ProcessStatusInfo::Waiting,
            ProcessStatus::UninterruptibleDiskSleep => ProcessStatusInfo::Waiting,
            ProcessStatus::Idle => ProcessStatusInfo::Sleeping,
            _ => ProcessStatusInfo::Unknown,
        }
    }
}

impl ProcessStatusInfo {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProcessStatusInfo::Running => "Running",
            ProcessStatusInfo::Sleeping => "Sleeping",
            ProcessStatusInfo::Waiting => "Waiting",
            ProcessStatusInfo::Zombie => "Zombie",
            ProcessStatusInfo::Stopped => "Stopped",
            ProcessStatusInfo::Tracing => "Tracing",
            ProcessStatusInfo::Dead => "Dead",
            ProcessStatusInfo::Unknown => "Unknown",
        }
    }
    
    pub fn emoji(&self) -> &'static str {
        match self {
            ProcessStatusInfo::Running => "🏃",
            ProcessStatusInfo::Sleeping => "😴",
            ProcessStatusInfo::Waiting => "⏳",
            ProcessStatusInfo::Zombie => "🧟",
            ProcessStatusInfo::Stopped => "⏹️",
            ProcessStatusInfo::Tracing => "🔍",
            ProcessStatusInfo::Dead => "💀",
            ProcessStatusInfo::Unknown => "❓",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessGroupBy {
    User,
    Parent,
    Application,
    Status,
    None,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessGroup {
    pub name: String,
    pub processes: Vec<ProcessInfo>,
    pub total_cpu: f32,
    pub total_memory: u64,
    pub process_count: usize,
}

impl SystemMonitor {
    pub fn get_process_info(&self) -> Vec<ProcessInfo> {
        let total_memory = self.system.total_memory() as f32;
        
        self.system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let status = ProcessStatusInfo::from(process.status());
                let is_zombie = matches!(status, ProcessStatusInfo::Zombie);
                
                let memory_percentage = if total_memory > 0.0 {
                    (process.memory() as f32 / total_memory) * 100.0
                } else {
                    0.0
                };

                // Extract application name from exe path
                let group_name = process.exe().and_then(|path| {
                    path.file_stem()
                        .and_then(|name| name.to_str())
                        .map(|s| s.to_string())
                });

                ProcessInfo {
                    pid: pid.as_u32(),
                    name: process.name().to_string(),
                    cpu_usage: process.cpu_usage(),
                    memory_usage: process.memory(),
                    memory_percentage,
                    user: process.user_id().map(|uid| uid.to_string()).unwrap_or_else(|| "unknown".to_string()),
                    status,
                    is_zombie,
                    parent_pid: process.parent().map(|p| p.as_u32()),
                    command: process.cmd().join(" "),
                    start_time: process.start_time(),
                    exe_path: process.exe().map(|p| p.to_string_lossy().to_string()),
                    working_directory: process.cwd().map(|p| p.to_string_lossy().to_string()),
                    group_name,
                    threads_count: process.tasks().map(|tasks| tasks.len()).unwrap_or(1),
                }
            })
            .collect()
    }

    pub fn group_processes(processes: &[ProcessInfo], group_by: ProcessGroupBy) -> Vec<ProcessGroup> {
        match group_by {
            ProcessGroupBy::None => vec![ProcessGroup {
                name: "All Processes".to_string(),
                processes: processes.to_vec(),
                total_cpu: processes.iter().map(|p| p.cpu_usage).sum(),
                total_memory: processes.iter().map(|p| p.memory_usage).sum(),
                process_count: processes.len(),
            }],
            ProcessGroupBy::User => {
                let mut groups: HashMap<&str, Vec<&ProcessInfo>> = HashMap::new();
                for process in processes {
                    groups.entry(&process.user).or_default().push(process);
                }
                Self::create_process_groups_ref(groups)
            },
            ProcessGroupBy::Parent => {
                let mut groups: HashMap<String, Vec<ProcessInfo>> = HashMap::new();
                for process in processes {
                    let parent_key = process.parent_pid
                        .map(|pid| format!("Parent PID: {}", pid))
                        .unwrap_or_else(|| "Root Processes".to_string());
                    groups.entry(parent_key).or_default().push(process.clone());
                }
                Self::create_process_groups(groups)
            },
            ProcessGroupBy::Application => {
                let mut groups: HashMap<String, Vec<ProcessInfo>> = HashMap::new();
                for process in processes {
                    let app_name = process.group_name.clone()
                        .unwrap_or_else(|| process.name.clone());
                    groups.entry(app_name).or_default().push(process.clone());
                }
                Self::create_process_groups(groups)
            },
            ProcessGroupBy::Status => {
                let mut groups: HashMap<String, Vec<ProcessInfo>> = HashMap::new();
                for process in processes {
                    groups.entry(process.status.as_str().to_string()).or_default().push(process.clone());
                }
                Self::create_process_groups(groups)
            },
        }
    }

    fn create_process_groups(groups: HashMap<String, Vec<ProcessInfo>>) -> Vec<ProcessGroup> {
        let mut result: Vec<ProcessGroup> = groups
            .into_iter()
            .map(|(name, processes)| {
                let total_cpu = processes.iter().map(|p| p.cpu_usage).sum();
                let total_memory = processes.iter().map(|p| p.memory_usage).sum();
                let process_count = processes.len();
                
                ProcessGroup {
                    name,
                    processes,
                    total_cpu,
                    total_memory,
                    process_count,
                }
            })
            .collect();
        
        // Sort groups by total CPU usage (descending)
        result.sort_by(|a, b| b.total_cpu.partial_cmp(&a.total_cpu).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    fn create_process_groups_ref(groups: HashMap<&str, Vec<&ProcessInfo>>) -> Vec<ProcessGroup> {
        let mut result: Vec<ProcessGroup> = groups
            .into_iter()
            .map(|(name, processes)| {
                let total_cpu = processes.iter().map(|p| p.cpu_usage).sum();
                let total_memory = processes.iter().map(|p| p.memory_usage).sum();
                let process_count = processes.len();
                
                ProcessGroup {
                    name: name.to_owned(),
                    processes: processes.into_iter().cloned().collect(),
                    total_cpu,
                    total_memory,
                    process_count,
                }
            })
            .collect();
        
        // Sort groups by total CPU usage (descending)
        result.sort_by(|a, b| b.total_cpu.partial_cmp(&a.total_cpu).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

}