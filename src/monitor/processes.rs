use sysinfo::{ProcessStatus, Pid};
use crate::monitor::SystemMonitor;

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
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
                }
            })
            .collect()
    }

    pub fn get_top_processes_by_cpu(&self, limit: usize) -> Vec<ProcessInfo> {
        let mut processes = self.get_process_info();
        processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
        processes.truncate(limit);
        processes
    }

    pub fn get_top_processes_by_memory(&self, limit: usize) -> Vec<ProcessInfo> {
        let mut processes = self.get_process_info();
        processes.sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage));
        processes.truncate(limit);
        processes
    }

    pub fn get_zombie_processes(&self) -> Vec<ProcessInfo> {
        self.get_process_info()
            .into_iter()
            .filter(|p| p.is_zombie)
            .collect()
    }

    pub fn kill_process(&mut self, pid: u32) -> anyhow::Result<bool> {
        if let Some(process) = self.system.process(Pid::from_u32(pid)) {
            Ok(process.kill())
        } else {
            Err(anyhow::anyhow!("Process with PID {} not found", pid))
        }
    }
}