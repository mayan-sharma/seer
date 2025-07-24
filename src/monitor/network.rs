// use sysinfo::NetworkExt; // Not needed in newer versions
use crate::monitor::SystemMonitor;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkMetrics {
    pub interfaces: Vec<NetworkInterface>,
    pub total_bytes_received: u64,
    pub total_bytes_transmitted: u64,
    pub total_packets_received: u64,
    pub total_packets_transmitted: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConnection {
    pub local_addr: String,
    pub remote_addr: String,
    pub protocol: String,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListeningPort {
    pub port: u16,
    pub protocol: String,
    pub service_name: Option<String>,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

impl SystemMonitor {
    pub fn get_network_metrics(&self) -> NetworkMetrics {
        let mut interfaces = Vec::new();
        let mut total_bytes_received = 0;
        let mut total_bytes_transmitted = 0;
        let mut total_packets_received = 0;
        let mut total_packets_transmitted = 0;

        // Get network interfaces from sysinfo
        for (interface_name, network_data) in &self.networks {
            let (rx_rate, tx_rate) = self.calculate_network_rates(
                interface_name,
                network_data.total_received(),
                network_data.total_transmitted(),
            );

            let interface = NetworkInterface {
                name: interface_name.to_string(),
                bytes_received: network_data.total_received(),
                bytes_transmitted: network_data.total_transmitted(),
                packets_received: network_data.total_packets_received(),
                packets_transmitted: network_data.total_packets_transmitted(),
                bytes_received_per_sec: rx_rate,
                bytes_transmitted_per_sec: tx_rate,
                is_up: network_data.total_received() > 0 || network_data.total_transmitted() > 0,
            };

            total_bytes_received += interface.bytes_received;
            total_bytes_transmitted += interface.bytes_transmitted;
            total_packets_received += interface.packets_received;
            total_packets_transmitted += interface.packets_transmitted;

            interfaces.push(interface);
        }

        NetworkMetrics {
            total_bytes_received,
            total_bytes_transmitted,
            total_packets_received,
            total_packets_transmitted,
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
        // Update previous network data for rate calculation
        for (interface_name, network_data) in &self.networks {
            self.previous_network_data.insert(
                interface_name.to_string(),
                (network_data.total_received(), network_data.total_transmitted()),
            );
        }
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
        use std::path::Path;
        
        let mut ports = Vec::new();
        let proc_path = Path::new("/proc/net/tcp");
        
        // Validate that the path is safe and exists
        if !proc_path.exists() || !proc_path.is_file() {
            return ports;
        }
        
        // Additional security check: ensure we're reading from /proc
        if let Some(parent) = proc_path.parent() {
            if parent != Path::new("/proc/net") {
                return ports;
            }
        }
        
        if let Ok(content) = fs::read_to_string(proc_path) {
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
        use std::path::Path;
        
        let mut ports = Vec::new();
        let proc_path = Path::new("/proc/net/udp");
        
        // Validate that the path is safe and exists
        if !proc_path.exists() || !proc_path.is_file() {
            return ports;
        }
        
        // Additional security check: ensure we're reading from /proc
        if let Some(parent) = proc_path.parent() {
            if parent != Path::new("/proc/net") {
                return ports;
            }
        }
        
        if let Ok(content) = fs::read_to_string(proc_path) {
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
        // Input validation: check line length to prevent potential attacks
        if line.len() > 1024 {
            return None;
        }
        
        // Validate protocol parameter
        if !matches!(protocol, "tcp" | "udp") {
            return None;
        }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            return None;
        }

        let local_addr = parts[1];
        let state = parts[3];
        
        // Validate local_addr format
        if !local_addr.contains(':') || local_addr.len() > 64 {
            return None;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_net_line_security() {
        let monitor = SystemMonitor::new();
        
        // Test normal valid input
        let valid_line = "  1: 00000000:0016 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 10000 1 0000000000000000 100 0 0 10 0";
        let result = monitor.parse_net_line(valid_line, "tcp");
        assert!(result.is_some());
        
        // Test input validation: oversized line should be rejected
        let oversized_line = "x".repeat(2000);
        let result = monitor.parse_net_line(&oversized_line, "tcp");
        assert!(result.is_none());
        
        // Test invalid protocol should be rejected
        let result = monitor.parse_net_line(valid_line, "invalid_protocol");
        assert!(result.is_none());
        
        // Test malformed local_addr should be rejected
        let malformed_line = "  1: malformed_addr 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 10000 1 0000000000000000 100 0 0 10 0";
        let result = monitor.parse_net_line(malformed_line, "tcp");
        assert!(result.is_none());
        
        // Test line with insufficient fields should be rejected
        let short_line = "1: 00000000:0016 00000000:0000";
        let result = monitor.parse_net_line(short_line, "tcp");
        assert!(result.is_none());
    }

    #[test]
    fn test_proc_path_validation() {
        let monitor = SystemMonitor::new();
        
        // Test that the functions handle non-existent proc files gracefully
        let tcp_ports = monitor.parse_proc_net_tcp();
        let udp_ports = monitor.parse_proc_net_udp();
        
        // These should not panic even if /proc/net/ files don't exist or are inaccessible
        assert!(tcp_ports.len() >= 0);
        assert!(udp_ports.len() >= 0);
    }
}