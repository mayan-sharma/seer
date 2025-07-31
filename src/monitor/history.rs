use crate::monitor::SystemMetrics;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub memory_percentage: f32,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub load_average: f64,
    pub process_count: usize,
}

impl From<&SystemMetrics> for HistoricalMetrics {
    fn from(metrics: &SystemMetrics) -> Self {
        Self {
            timestamp: Utc::now(),
            cpu_usage: metrics.cpu.overall_usage,
            memory_usage: metrics.memory.used_ram as f32,
            memory_percentage: metrics.memory.ram_percentage,
            network_rx_bytes: metrics.network.total_bytes_received,
            network_tx_bytes: metrics.network.total_bytes_transmitted,
            disk_read_bytes: 0, // TODO: implement disk I/O metrics
            disk_write_bytes: 0, // TODO: implement disk I/O metrics
            load_average: metrics.load_average.one_min,
            process_count: metrics.processes.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HistoryManager {
    pub history: VecDeque<HistoricalMetrics>,
    pub max_entries: usize,
}

impl HistoryManager {
    pub fn new(max_entries: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn add_metrics(&mut self, metrics: &SystemMetrics) {
        let historical_metrics = HistoricalMetrics::from(metrics);
        
        if self.history.len() >= self.max_entries {
            self.history.pop_front();
        }
        
        self.history.push_back(historical_metrics);
    }

    pub fn get_history(&self) -> &VecDeque<HistoricalMetrics> {
        &self.history
    }

    pub fn get_history_vec(&self) -> Vec<HistoricalMetrics> {
        self.history.iter().cloned().collect()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::monitor::{CpuMetrics, MemoryMetrics, NetworkMetrics, LoadAverage};

    fn create_test_metrics() -> SystemMetrics {
        SystemMetrics {
            cpu: CpuMetrics {
                overall_usage: 50.0,
                per_core_usage: vec![45.0, 55.0],
                temperature: None,
            },
            memory: MemoryMetrics {
                total_ram: 8_000_000_000,
                used_ram: 4_000_000_000,
                available_ram: 4_000_000_000,
                total_swap: 2_000_000_000,
                used_swap: 500_000_000,
                ram_percentage: 50.0,
                swap_percentage: 25.0,
            },
            processes: vec![],
            network: NetworkMetrics {
                total_bytes_received: 1000,
                total_bytes_transmitted: 2000,
                total_packets_received: 10,
                total_packets_transmitted: 20,
                interfaces: vec![],
            },
            storage: vec![],
            uptime: 3600,
            load_average: LoadAverage {
                one_min: 1.5,
                five_min: 1.2,
                fifteen_min: 1.0,
            },
            boot_time: Utc::now(),
        }
    }

    #[test]
    fn test_history_manager_creation() {
        let history = HistoryManager::new(100);
        assert_eq!(history.max_entries, 100);
        assert!(history.get_history().is_empty());
    }

    #[test]
    fn test_add_metrics() {
        let mut history = HistoryManager::new(100);
        let metrics = create_test_metrics();
        
        history.add_metrics(&metrics);
        
        let historical_data = history.get_history();
        assert_eq!(historical_data.len(), 1);
        
        let first_entry = &historical_data[0];
        assert_eq!(first_entry.cpu_usage, 50.0);
        assert_eq!(first_entry.memory_usage, 4_000_000_000.0);
        assert_eq!(first_entry.memory_percentage, 50.0);
        assert_eq!(first_entry.network_rx_bytes, 1000);
        assert_eq!(first_entry.network_tx_bytes, 2000);
    }

    #[test]
    fn test_history_size_limit() {
        let mut history = HistoryManager::new(2);
        let metrics = create_test_metrics();
        
        // Add 3 entries to test size limiting
        history.add_metrics(&metrics);
        history.add_metrics(&metrics);
        history.add_metrics(&metrics);
        
        let historical_data = history.get_history();
        assert_eq!(historical_data.len(), 2); // Should not exceed max_entries
    }

    #[test]
    fn test_historical_metrics_fields() {
        let mut history = HistoryManager::new(100);
        let metrics = create_test_metrics();
        
        history.add_metrics(&metrics);
        let historical_data = history.get_history();
        let entry = &historical_data[0];
        
        assert!(entry.timestamp <= Utc::now());
        assert_eq!(entry.load_average, 1.5);
        assert_eq!(entry.process_count, 0); // Empty processes vec
        assert_eq!(entry.disk_read_bytes, 0);
        assert_eq!(entry.disk_write_bytes, 0);
    }
}