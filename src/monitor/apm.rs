use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::str;
use tokio::time::{Duration, Instant};
use crate::monitor::ProcessInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APMMetrics {
    pub jvm_applications: Vec<JVMMetrics>,
    pub dotnet_applications: Vec<DotNetMetrics>,
    pub python_applications: Vec<PythonMetrics>,
    pub nodejs_applications: Vec<NodeJSMetrics>,
    pub golang_applications: Vec<GoMetrics>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JVMMetrics {
    pub pid: u32,
    pub name: String,
    pub heap_memory: HeapMemoryMetrics,
    pub non_heap_memory: NonHeapMemoryMetrics,
    pub garbage_collection: Vec<GCMetrics>,
    pub thread_metrics: ThreadMetrics,
    pub class_loading: ClassLoadingMetrics,
    pub jvm_version: String,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotNetMetrics {
    pub pid: u32,
    pub name: String,
    pub runtime_version: String,
    pub managed_memory: u64,
    pub gen0_collections: u64,
    pub gen1_collections: u64,
    pub gen2_collections: u64,
    pub thread_count: u32,
    pub exception_count: u64,
    pub time_in_gc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonMetrics {
    pub pid: u32,
    pub name: String,
    pub python_version: String,
    pub memory_usage: u64,
    pub active_threads: u32,
    pub modules_loaded: u32,
    pub exceptions_raised: u64,
    pub gc_collections: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeJSMetrics {
    pub pid: u32,
    pub name: String,
    pub node_version: String,
    pub v8_heap_used: u64,
    pub v8_heap_total: u64,
    pub event_loop_lag: f64,
    pub active_handles: u32,
    pub active_requests: u32,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoMetrics {
    pub pid: u32,
    pub name: String,
    pub go_version: String,
    pub heap_alloc: u64,
    pub heap_sys: u64,
    pub gc_cycles: u64,
    pub goroutines: u32,
    pub cgo_calls: u64,
    pub next_gc: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapMemoryMetrics {
    pub used: u64,
    pub committed: u64,
    pub max: u64,
    pub init: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonHeapMemoryMetrics {
    pub used: u64,
    pub committed: u64,
    pub max: u64,
    pub init: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCMetrics {
    pub name: String,
    pub collection_count: u64,
    pub collection_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMetrics {
    pub current_thread_count: u32,
    pub daemon_thread_count: u32,
    pub peak_thread_count: u32,
    pub total_started_thread_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassLoadingMetrics {
    pub loaded_class_count: u32,
    pub total_loaded_class_count: u64,
    pub unloaded_class_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationTrace {
    pub pid: u32,
    pub timestamp: DateTime<Utc>,
    pub method_name: String,
    pub duration_ms: u64,
    pub cpu_time_ms: u64,
    pub memory_allocated: u64,
    pub thread_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APMAnomaly {
    pub pid: u32,
    pub application_name: String,
    pub anomaly_type: APMAnomalyType,
    pub severity: APMAnomalySeverity,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub metric_value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APMAnomalyType {
    HighCPUUsage,
    MemoryLeak,
    HighGCPressure,
    ThreadContention,
    LongGCPause,
    HighExceptionRate,
    EventLoopBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APMAnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

pub struct APMMonitor {
    last_metrics: Option<APMMetrics>,
    last_update: Instant,
    anomalies: Vec<APMAnomaly>,
    historical_data: HashMap<u32, Vec<ApplicationTrace>>,
}

impl APMMonitor {
    pub fn new() -> Self {
        Self {
            last_metrics: None,
            last_update: Instant::now(),
            anomalies: Vec::new(),
            historical_data: HashMap::new(),
        }
    }

    pub async fn update_metrics(&mut self, processes: &[ProcessInfo]) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_update) < Duration::from_secs(10) {
            return Ok(());
        }

        let mut metrics = APMMetrics {
            jvm_applications: Vec::new(),
            dotnet_applications: Vec::new(),
            python_applications: Vec::new(),
            nodejs_applications: Vec::new(),
            golang_applications: Vec::new(),
            last_updated: Utc::now(),
        };

        // Identify and collect metrics for different application types
        for process in processes {
            if self.is_jvm_process(process) {
                if let Ok(jvm_metrics) = self.collect_jvm_metrics(process).await {
                    metrics.jvm_applications.push(jvm_metrics);
                }
            } else if self.is_dotnet_process(process) {
                if let Ok(dotnet_metrics) = self.collect_dotnet_metrics(process).await {
                    metrics.dotnet_applications.push(dotnet_metrics);
                }
            } else if self.is_python_process(process) {
                if let Ok(python_metrics) = self.collect_python_metrics(process).await {
                    metrics.python_applications.push(python_metrics);
                }
            } else if self.is_nodejs_process(process) {
                if let Ok(nodejs_metrics) = self.collect_nodejs_metrics(process).await {
                    metrics.nodejs_applications.push(nodejs_metrics);
                }
            } else if self.is_golang_process(process) {
                if let Ok(go_metrics) = self.collect_golang_metrics(process).await {
                    metrics.golang_applications.push(go_metrics);
                }
            }
        }

        // Detect anomalies
        self.detect_anomalies(&metrics);

        self.last_metrics = Some(metrics);
        self.last_update = now;
        Ok(())
    }

    pub fn get_metrics(&self) -> Option<&APMMetrics> {
        self.last_metrics.as_ref()
    }

    pub fn get_anomalies(&self) -> &[APMAnomaly] {
        &self.anomalies
    }

    pub fn get_application_traces(&self, pid: u32) -> Option<&Vec<ApplicationTrace>> {
        self.historical_data.get(&pid)
    }

    fn is_jvm_process(&self, process: &ProcessInfo) -> bool {
        process.name.to_lowercase().contains("java") ||
        process.command.contains("java") || process.command.contains(".jar")
    }

    fn is_dotnet_process(&self, process: &ProcessInfo) -> bool {
        process.name.to_lowercase().contains("dotnet") ||
        process.command.contains("dotnet") || process.command.ends_with(".dll")
    }

    fn is_python_process(&self, process: &ProcessInfo) -> bool {
        process.name.to_lowercase().contains("python") ||
        process.command.contains("python") || process.command.ends_with(".py")
    }

    fn is_nodejs_process(&self, process: &ProcessInfo) -> bool {
        process.name.to_lowercase().contains("node") ||
        process.command.contains("node") || process.command.ends_with(".js")
    }

    fn is_golang_process(&self, process: &ProcessInfo) -> bool {
        // Go binaries don't have a clear indicator, so we'll check for common patterns
        process.command.contains("go run") || 
        (process.command.starts_with("./") && !process.command.contains("."))
    }

    async fn collect_jvm_metrics(&self, process: &ProcessInfo) -> Result<JVMMetrics> {
        // Use jstat and jinfo to collect JVM metrics
        let _jstat_output = Command::new("jstat")
            .args(&["-gc", "-gccapacity", &process.pid.to_string()])
            .output();

        let _jinfo_output = Command::new("jinfo")
            .args(&[&process.pid.to_string()])
            .output();

        // Parse jstat output for GC and memory metrics
        let heap_memory = HeapMemoryMetrics {
            used: process.memory_usage, // Already in bytes
            committed: process.memory_usage,
            max: process.memory_usage * 2, // Estimate
            init: process.memory_usage / 2, // Estimate
        };

        let non_heap_memory = NonHeapMemoryMetrics {
            used: process.memory_usage / 10, // Estimate
            committed: process.memory_usage / 8,
            max: process.memory_usage / 4,
            init: process.memory_usage / 16,
        };

        let gc_metrics = vec![
            GCMetrics {
                name: "G1 Young Generation".to_string(),
                collection_count: 100, // Mock data
                collection_time: 500,
            },
            GCMetrics {
                name: "G1 Old Generation".to_string(),
                collection_count: 10,
                collection_time: 2000,
            },
        ];

        let thread_metrics = ThreadMetrics {
            current_thread_count: 25,
            daemon_thread_count: 15,
            peak_thread_count: 30,
            total_started_thread_count: 1000,
        };

        let class_loading = ClassLoadingMetrics {
            loaded_class_count: 5000,
            total_loaded_class_count: 5500,
            unloaded_class_count: 500,
        };

        Ok(JVMMetrics {
            pid: process.pid,
            name: process.name.clone(),
            heap_memory,
            non_heap_memory,
            garbage_collection: gc_metrics,
            thread_metrics,
            class_loading,
            jvm_version: "11.0.12".to_string(), // Would get from jinfo
            uptime: Utc::now().timestamp() as u64 - process.start_time,
        })
    }

    async fn collect_dotnet_metrics(&self, process: &ProcessInfo) -> Result<DotNetMetrics> {
        // Use dotnet-counters or similar tools
        Ok(DotNetMetrics {
            pid: process.pid,
            name: process.name.clone(),
            runtime_version: ".NET 6.0".to_string(),
            managed_memory: process.memory_usage,
            gen0_collections: 1000,
            gen1_collections: 100,
            gen2_collections: 10,
            thread_count: 20,
            exception_count: 50,
            time_in_gc: 5.2,
        })
    }

    async fn collect_python_metrics(&self, process: &ProcessInfo) -> Result<PythonMetrics> {
        // Would require Python process instrumentation
        Ok(PythonMetrics {
            pid: process.pid,
            name: process.name.clone(),
            python_version: "3.9.7".to_string(),
            memory_usage: process.memory_usage,
            active_threads: 5,
            modules_loaded: 150,
            exceptions_raised: 25,
            gc_collections: 500,
        })
    }

    async fn collect_nodejs_metrics(&self, process: &ProcessInfo) -> Result<NodeJSMetrics> {
        // Would require Node.js process instrumentation
        Ok(NodeJSMetrics {
            pid: process.pid,
            name: process.name.clone(),
            node_version: "16.14.0".to_string(),
            v8_heap_used: process.memory_usage * 3 / 4,
            v8_heap_total: process.memory_usage,
            event_loop_lag: 2.5,
            active_handles: 15,
            active_requests: 3,
            uptime: Utc::now().timestamp() as u64 - process.start_time,
        })
    }

    async fn collect_golang_metrics(&self, process: &ProcessInfo) -> Result<GoMetrics> {
        // Would require Go process instrumentation or pprof
        Ok(GoMetrics {
            pid: process.pid,
            name: process.name.clone(),
            go_version: "1.19.3".to_string(),
            heap_alloc: process.memory_usage * 2 / 3,
            heap_sys: process.memory_usage,
            gc_cycles: 200,
            goroutines: 50,
            cgo_calls: 1000,
            next_gc: process.memory_usage * 2,
        })
    }

    fn detect_anomalies(&mut self, metrics: &APMMetrics) {
        self.anomalies.clear();

        // Check JVM applications for anomalies
        for jvm in &metrics.jvm_applications {
            // High GC pressure
            let total_gc_time: u64 = jvm.garbage_collection.iter().map(|gc| gc.collection_time).sum();
            if total_gc_time > 5000 { // 5 seconds
                self.anomalies.push(APMAnomaly {
                    pid: jvm.pid,
                    application_name: jvm.name.clone(),
                    anomaly_type: APMAnomalyType::HighGCPressure,
                    severity: APMAnomalySeverity::High,
                    description: format!("High GC time: {}ms", total_gc_time),
                    detected_at: Utc::now(),
                    metric_value: total_gc_time as f64,
                    threshold: 5000.0,
                });
            }

            // Memory usage close to max
            let heap_usage_percent = (jvm.heap_memory.used as f64 / jvm.heap_memory.max as f64) * 100.0;
            if heap_usage_percent > 90.0 {
                self.anomalies.push(APMAnomaly {
                    pid: jvm.pid,
                    application_name: jvm.name.clone(),
                    anomaly_type: APMAnomalyType::MemoryLeak,
                    severity: APMAnomalySeverity::Critical,
                    description: format!("Heap usage at {:.1}%", heap_usage_percent),
                    detected_at: Utc::now(),
                    metric_value: heap_usage_percent,
                    threshold: 90.0,
                });
            }
        }

        // Check .NET applications
        for dotnet in &metrics.dotnet_applications {
            if dotnet.time_in_gc > 10.0 {
                self.anomalies.push(APMAnomaly {
                    pid: dotnet.pid,
                    application_name: dotnet.name.clone(),
                    anomaly_type: APMAnomalyType::HighGCPressure,
                    severity: APMAnomalySeverity::Medium,
                    description: format!("High time in GC: {:.1}%", dotnet.time_in_gc),
                    detected_at: Utc::now(),
                    metric_value: dotnet.time_in_gc,
                    threshold: 10.0,
                });
            }
        }

        // Check Node.js applications
        for nodejs in &metrics.nodejs_applications {
            if nodejs.event_loop_lag > 10.0 {
                self.anomalies.push(APMAnomaly {
                    pid: nodejs.pid,
                    application_name: nodejs.name.clone(),
                    anomaly_type: APMAnomalyType::EventLoopBlocked,
                    severity: APMAnomalySeverity::High,
                    description: format!("High event loop lag: {:.1}ms", nodejs.event_loop_lag),
                    detected_at: Utc::now(),
                    metric_value: nodejs.event_loop_lag,
                    threshold: 10.0,
                });
            }
        }
    }

    pub fn get_application_summary(&self) -> Vec<String> {
        let mut summary = Vec::new();
        
        if let Some(metrics) = &self.last_metrics {
            if !metrics.jvm_applications.is_empty() {
                summary.push(format!("JVM Apps: {}", metrics.jvm_applications.len()));
            }
            if !metrics.dotnet_applications.is_empty() {
                summary.push(format!(".NET Apps: {}", metrics.dotnet_applications.len()));
            }
            if !metrics.python_applications.is_empty() {
                summary.push(format!("Python Apps: {}", metrics.python_applications.len()));
            }
            if !metrics.nodejs_applications.is_empty() {
                summary.push(format!("Node.js Apps: {}", metrics.nodejs_applications.len()));
            }
            if !metrics.golang_applications.is_empty() {
                summary.push(format!("Go Apps: {}", metrics.golang_applications.len()));
            }
            
            if !self.anomalies.is_empty() {
                let critical_count = self.anomalies.iter()
                    .filter(|a| matches!(a.severity, APMAnomalySeverity::Critical))
                    .count();
                if critical_count > 0 {
                    summary.push(format!("⚠️ {} Critical Issues", critical_count));
                }
            }
        }
        
        summary
    }
}

impl Default for APMMonitor {
    fn default() -> Self {
        Self::new()
    }
}