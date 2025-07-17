// use sysinfo::NetworkExt; // Not needed in newer versions
use crate::monitor::SystemMonitor;
// use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    pub interfaces: Vec<NetworkInterface>,
    pub total_bytes_received: u64,
    pub total_bytes_transmitted: u64,
    pub total_packets_received: u64,
    pub total_packets_transmitted: u64,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
    pub bytes_received_per_sec: f64,
    pub bytes_transmitted_per_sec: f64,
    pub is_up: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkConnection {
    pub local_addr: String,
    pub remote_addr: String,
    pub protocol: String,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListeningPort {
    pub port: u16,
    pub protocol: String,
    pub service_name: Option<String>,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

impl SystemMonitor {
    pub fn get_network_metrics(&self) -> NetworkMetrics {
        // Placeholder implementation - sysinfo network monitoring may need different approach
        let interfaces = vec![
            NetworkInterface {
                name: "eth0".to_string(),
                bytes_received: 1024000,
                bytes_transmitted: 512000,
                packets_received: 1000,
                packets_transmitted: 800,
                bytes_received_per_sec: 1024.0,
                bytes_transmitted_per_sec: 512.0,
                is_up: true,
            }
        ];

        NetworkMetrics {
            total_bytes_received: 1024000,
            total_bytes_transmitted: 512000,
            total_packets_received: 1000,
            total_packets_transmitted: 800,
            interfaces,
        }
    }

    fn calculate_network_rates(&self, interface: &str, rx_bytes: u64, tx_bytes: u64) -> (f64, f64) {
        if let Some((prev_rx, prev_tx)) = self.previous_network_data.get(interface) {
            let rx_rate = (rx_bytes.saturating_sub(*prev_rx)) as f64;
            let tx_rate = (tx_bytes.saturating_sub(*prev_tx)) as f64;
            (rx_rate, tx_rate)
        } else {
            (0.0, 0.0)
        }
    }

    pub fn update_network_data(&mut self) {
        // Placeholder - update network data
        self.previous_network_data.insert(
            "eth0".to_string(),
            (1024000, 512000),
        );
    }

    pub fn get_listening_ports(&self) -> Vec<ListeningPort> {
        let mut ports = Vec::new();
        
        #[cfg(target_os = "linux")]
        {
            ports.extend(self.parse_proc_net_tcp());
            ports.extend(self.parse_proc_net_udp());
        }
        
        ports
    }

    #[cfg(target_os = "linux")]
    fn parse_proc_net_tcp(&self) -> Vec<ListeningPort> {
        use std::fs;
        
        let mut ports = Vec::new();
        
        if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
            for line in content.lines().skip(1) {
                if let Some(port) = self.parse_net_line(line, "tcp") {
                    ports.push(port);
                }
            }
        }
        
        ports
    }

    #[cfg(target_os = "linux")]
    fn parse_proc_net_udp(&self) -> Vec<ListeningPort> {
        use std::fs;
        
        let mut ports = Vec::new();
        
        if let Ok(content) = fs::read_to_string("/proc/net/udp") {
            for line in content.lines().skip(1) {
                if let Some(port) = self.parse_net_line(line, "udp") {
                    ports.push(port);
                }
            }
        }
        
        ports
    }

    #[cfg(target_os = "linux")]
    fn parse_net_line(&self, line: &str, protocol: &str) -> Option<ListeningPort> {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() < 10 {
            return None;
        }

        let local_addr = parts[1];
        let state = parts[3];

        if protocol == "tcp" && state != "0A" {
            return None;
        }

        if let Some(colon_pos) = local_addr.rfind(':') {
            if let Ok(port) = u16::from_str_radix(&local_addr[colon_pos + 1..], 16) {
                let service_name = self.get_service_name(port, protocol);
                
                return Some(ListeningPort {
                    port,
                    protocol: protocol.to_uppercase(),
                    service_name,
                    pid: None,
                    process_name: None,
                });
            }
        }

        None
    }

    fn get_service_name(&self, port: u16, protocol: &str) -> Option<String> {
        match (port, protocol) {
            (22, "tcp") => Some("SSH".to_string()),
            (80, "tcp") => Some("HTTP".to_string()),
            (443, "tcp") => Some("HTTPS".to_string()),
            (53, _) => Some("DNS".to_string()),
            (21, "tcp") => Some("FTP".to_string()),
            (25, "tcp") => Some("SMTP".to_string()),
            (110, "tcp") => Some("POP3".to_string()),
            (143, "tcp") => Some("IMAP".to_string()),
            (993, "tcp") => Some("IMAPS".to_string()),
            (995, "tcp") => Some("POP3S".to_string()),
            (3306, "tcp") => Some("MySQL".to_string()),
            (5432, "tcp") => Some("PostgreSQL".to_string()),
            (6379, "tcp") => Some("Redis".to_string()),
            (27017, "tcp") => Some("MongoDB".to_string()),
            _ => None,
        }
    }
}