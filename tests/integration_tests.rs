use seer::{config::Config, monitor::SystemMonitor, Cli};
use tokio;

#[tokio::test]
async fn test_system_monitor_creation() {
    let monitor = SystemMonitor::new();
    assert!(monitor.history.get_history().is_empty());
}

#[tokio::test]
async fn test_system_monitor_update() {
    let mut monitor = SystemMonitor::new();
    
    // Test that update doesn't panic
    let result = monitor.update().await;
    assert!(result.is_ok());
    
    // Test that we can get metrics after update
    let metrics = monitor.get_metrics();
    assert!(metrics.cpu.overall_usage >= 0.0);
    assert!(metrics.memory.total_ram > 0);
    assert!(!metrics.processes.is_empty());
}

#[tokio::test]
async fn test_system_metrics_structure() {
    let mut monitor = SystemMonitor::new();
    monitor.update().await.expect("Failed to update monitor");
    
    let metrics = monitor.get_metrics();
    
    // Test CPU metrics
    assert!(metrics.cpu.overall_usage >= 0.0 && metrics.cpu.overall_usage <= 100.0);
    assert!(!metrics.cpu.per_core_usage.is_empty());
    
    // Test memory metrics
    assert!(metrics.memory.total_ram > 0);
    assert!(metrics.memory.used_ram <= metrics.memory.total_ram);
    assert!(metrics.memory.ram_percentage >= 0.0 && metrics.memory.ram_percentage <= 100.0);
    
    // Test process info
    assert!(!metrics.processes.is_empty());
    for process in &metrics.processes[..std::cmp::min(5, metrics.processes.len())] {
        assert!(process.pid > 0);
        assert!(!process.name.is_empty());
        assert!(process.cpu_usage >= 0.0);
    }
}

#[test]
fn test_config_creation_with_defaults() {
    let cli = Cli {
        refresh_rate: 2,
        show_zombies: false,
        filter_process: None,
        export: None,
        threshold_cpu: 80.0,
        threshold_memory: 80.0,
    };
    
    let config = Config::new(cli).expect("Failed to create config");
    
    assert_eq!(config.refresh_rate, 2);
    assert!(!config.show_zombies);
    assert_eq!(config.threshold_cpu, 80.0);
    assert_eq!(config.threshold_memory, 80.0);
}

#[test]
fn test_format_bytes() {
    use seer::monitor::SystemMonitor;
    
    assert_eq!(SystemMonitor::format_bytes(0), "0 B");
    assert_eq!(SystemMonitor::format_bytes(1023), "1023 B");
    assert_eq!(SystemMonitor::format_bytes(1024), "1.0 KB");
    assert_eq!(SystemMonitor::format_bytes(1536), "1.5 KB");
    assert_eq!(SystemMonitor::format_bytes(1048576), "1.0 MB");
    assert_eq!(SystemMonitor::format_bytes(1073741824), "1.0 GB");
}

#[test]
fn test_uptime_formatting() {
    use seer::monitor::SystemMonitor;
    
    assert_eq!(SystemMonitor::format_uptime(30), "0m");
    assert_eq!(SystemMonitor::format_uptime(120), "2m");
    assert_eq!(SystemMonitor::format_uptime(3720), "1h 2m");
    assert_eq!(SystemMonitor::format_uptime(90000), "1d 1h 0m");
}