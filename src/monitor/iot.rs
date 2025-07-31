use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::process::Command;
use std::str;
use tokio::time::{Duration, Instant};
use tokio::net::UdpSocket;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTMetrics {
    pub discovered_devices: Vec<IoTDevice>,
    pub network_scans: Vec<NetworkScan>,
    pub protocol_stats: ProtocolStats,
    pub device_health: HashMap<String, DeviceHealth>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTDevice {
    pub device_id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub ip_address: IpAddr,
    pub mac_address: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub protocols: Vec<IoTProtocol>,
    pub last_seen: DateTime<Utc>,
    pub is_online: bool,
    pub signal_strength: Option<i32>, // dBm for wireless devices
    pub battery_level: Option<u8>,   // 0-100%
    pub temperature: Option<f32>,    // Celsius
    pub humidity: Option<f32>,       // Percentage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    SmartPhone,
    Tablet,
    SmartTV,
    SmartSpeaker,
    SecurityCamera,
    SmartThermostat,
    SmartLight,
    SmartPlug,
    SmartLock,
    Sensor,
    Router,
    AccessPoint,
    Printer,
    SmartWatch,
    SmartHome,
    IndustrialIoT,
    MedicalDevice,
    VehicleIoT,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IoTProtocol {
    WiFi,
    Bluetooth,
    Zigbee,
    ZWave,
    LoRaWAN,
    Mqtt,
    CoAP,
    HTTP,
    HTTPS,
    Modbus,
    BACnet,
    Thread,
    Matter,
    Cellular,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkScan {
    pub scan_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub network_range: String,
    pub devices_found: u32,
    pub scan_type: ScanType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanType {
    PortScan,
    ArpScan,
    MdnsDiscovery,
    UPnPDiscovery,
    BluetoothScan,
    ZigbeeScan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    pub mqtt_messages: u64,
    pub coap_requests: u64,
    pub http_requests: u64,
    pub bluetooth_connections: u32,
    pub zigbee_messages: u64,
    pub wifi_devices: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHealth {
    pub device_id: String,
    pub health_score: f32, // 0.0 - 1.0
    pub connectivity_status: ConnectivityStatus,
    pub response_time_ms: u64,
    pub packet_loss_percent: f32,
    pub last_health_check: DateTime<Utc>,
    pub alerts: Vec<DeviceAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectivityStatus {
    Online,
    Offline,
    Intermittent,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAlert {
    pub alert_type: IoTAlertType,
    pub severity: IoTAlertSeverity,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IoTAlertType {
    DeviceOffline,
    LowBattery,
    HighTemperature,
    SecurityThreat,
    FirmwareOutdated,
    ConnectivityIssue,
    SensorMalfunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IoTAlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct IoTConfig {
    pub discovery_interval: Duration,
    pub health_check_interval: Duration,
    pub network_ranges: Vec<String>,
    pub enable_bluetooth_scan: bool,
    pub enable_zigbee_scan: bool,
    pub enable_upnp_discovery: bool,
    pub enable_mdns_discovery: bool,
    pub mqtt_broker_url: Option<String>,
    pub device_timeout: Duration,
}

impl Default for IoTConfig {
    fn default() -> Self {
        Self {
            discovery_interval: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(60), // 1 minute
            network_ranges: vec!["192.168.1.0/24".to_string()],
            enable_bluetooth_scan: true,
            enable_zigbee_scan: false,
            enable_upnp_discovery: true,
            enable_mdns_discovery: true,
            mqtt_broker_url: None,
            device_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

pub struct IoTMonitor {
    config: IoTConfig,
    last_metrics: Option<IoTMetrics>,
    last_discovery: Instant,
    last_health_check: Instant,
    known_devices: HashMap<String, IoTDevice>,
    active_scans: Vec<NetworkScan>,
}

impl IoTMonitor {
    pub fn new(config: IoTConfig) -> Self {
        Self {
            config,
            last_metrics: None,
            last_discovery: Instant::now(),
            last_health_check: Instant::now(),
            known_devices: HashMap::new(),
            active_scans: Vec::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(IoTConfig::default())
    }

    pub async fn update_metrics(&mut self) -> Result<()> {
        let now = Instant::now();
        
        // Run device discovery
        if now.duration_since(self.last_discovery) >= self.config.discovery_interval {
            self.discover_devices().await?;
            self.last_discovery = now;
        }

        // Run health checks
        if now.duration_since(self.last_health_check) >= self.config.health_check_interval {
            self.check_device_health().await?;
            self.last_health_check = now;
        }

        // Update metrics
        let device_health = self.calculate_device_health();
        
        let metrics = IoTMetrics {
            discovered_devices: self.known_devices.values().cloned().collect(),
            network_scans: self.active_scans.clone(),
            protocol_stats: self.calculate_protocol_stats(),
            device_health,
            last_updated: Utc::now(),
        };

        self.last_metrics = Some(metrics);
        Ok(())
    }

    pub fn get_metrics(&self) -> Option<&IoTMetrics> {
        self.last_metrics.as_ref()
    }

    async fn discover_devices(&mut self) -> Result<()> {
        for network_range in &self.config.network_ranges.clone() {
            // ARP scan
            self.arp_scan(network_range).await?;
            
            // UPnP discovery
            if self.config.enable_upnp_discovery {
                self.upnp_discovery().await?;
            }

            // mDNS discovery
            if self.config.enable_mdns_discovery {
                self.mdns_discovery().await?;
            }
        }

        // Bluetooth scan
        if self.config.enable_bluetooth_scan {
            self.bluetooth_scan().await?;
        }

        Ok(())
    }

    async fn arp_scan(&mut self, network_range: &str) -> Result<()> {
        let scan_id = format!("arp_{}", Utc::now().timestamp());
        let scan = NetworkScan {
            scan_id: scan_id.clone(),
            started_at: Utc::now(),
            completed_at: None,
            network_range: network_range.to_string(),
            devices_found: 0,
            scan_type: ScanType::ArpScan,
        };
        self.active_scans.push(scan);

        // Run nmap ARP scan
        let output = Command::new("nmap")
            .args(&["-sn", network_range])
            .output();

        if let Ok(output) = output {
            let stdout = str::from_utf8(&output.stdout)?;
            let devices_found = self.parse_nmap_output(stdout);
            
            // Update scan with completion info
            if let Some(scan) = self.active_scans.iter_mut().find(|s| s.scan_id == scan_id) {
                scan.completed_at = Some(Utc::now());
                scan.devices_found = devices_found;
            }
        }

        Ok(())
    }

    fn parse_nmap_output(&mut self, output: &str) -> u32 {
        let mut devices_found = 0;
        
        for line in output.lines() {
            if line.starts_with("Nmap scan report for") {
                devices_found += 1;
                
                // Extract IP address
                if let Some(ip_str) = line.split_whitespace().last() {
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        let device_id = format!("unknown_{}", ip);
                        
                        if !self.known_devices.contains_key(&device_id) {
                            let device = IoTDevice {
                                device_id: device_id.clone(),
                                name: format!("Unknown Device ({})", ip),
                                device_type: DeviceType::Unknown,
                                ip_address: ip,
                                mac_address: None,
                                manufacturer: None,
                                model: None,
                                firmware_version: None,
                                protocols: vec![IoTProtocol::WiFi],
                                last_seen: Utc::now(),
                                is_online: true,
                                signal_strength: None,
                                battery_level: None,
                                temperature: None,
                                humidity: None,
                            };
                            
                            self.known_devices.insert(device_id, device);
                        } else {
                            // Update last seen time
                            if let Some(device) = self.known_devices.get_mut(&device_id) {
                                device.last_seen = Utc::now();
                                device.is_online = true;
                            }
                        }
                    }
                }
            }
        }
        
        devices_found
    }

    async fn upnp_discovery(&mut self) -> Result<()> {
        // UPnP SSDP multicast discovery
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let multicast_addr: SocketAddr = "239.255.255.250:1900".parse()?;
        
        let discovery_msg = "M-SEARCH * HTTP/1.1\r\n\
                            HOST: 239.255.255.250:1900\r\n\
                            MAN: \"ssdp:discover\"\r\n\
                            ST: upnp:rootdevice\r\n\
                            MX: 3\r\n\r\n";

        socket.send_to(discovery_msg.as_bytes(), multicast_addr).await?;
        
        // Listen for responses (simplified - would need proper parsing)
        let mut buf = [0; 1024];
        let timeout = tokio::time::timeout(Duration::from_secs(5), socket.recv_from(&mut buf));
        
        if let Ok(Ok((len, addr))) = timeout.await {
            let response = str::from_utf8(&buf[..len])?;
            self.parse_upnp_response(response, addr.ip());
        }

        Ok(())
    }

    fn parse_upnp_response(&mut self, _response: &str, ip: IpAddr) {
        let device_id = format!("upnp_{}", ip);
        
        if !self.known_devices.contains_key(&device_id) {
            let device = IoTDevice {
                device_id: device_id.clone(),
                name: format!("UPnP Device ({})", ip),
                device_type: DeviceType::SmartHome, // Assume smart home device
                ip_address: ip,
                mac_address: None,
                manufacturer: None,
                model: None,
                firmware_version: None,
                protocols: vec![IoTProtocol::HTTP, IoTProtocol::WiFi],
                last_seen: Utc::now(),
                is_online: true,
                signal_strength: None,
                battery_level: None,
                temperature: None,
                humidity: None,
            };
            
            self.known_devices.insert(device_id, device);
        }
    }

    async fn mdns_discovery(&mut self) -> Result<()> {
        // mDNS discovery using avahi-browse or similar
        let output = Command::new("avahi-browse")
            .args(&["-t", "-r", "_services._dns-sd._udp"])
            .output();

        if let Ok(output) = output {
            let stdout = str::from_utf8(&output.stdout)?;
            self.parse_mdns_output(stdout);
        }

        Ok(())
    }

    fn parse_mdns_output(&mut self, output: &str) {
        // Parse mDNS/Bonjour services (simplified)
        for line in output.lines() {
            if line.contains("IPv4") && line.contains("=") {
                // Extract service information and create device
                let device_id = format!("mdns_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
                
                let device = IoTDevice {
                    device_id: device_id.clone(),
                    name: "mDNS Service Device".to_string(),
                    device_type: DeviceType::SmartHome,
                    ip_address: "192.168.1.100".parse().unwrap(), // Would extract from output
                    mac_address: None,
                    manufacturer: None,
                    model: None,
                    firmware_version: None,
                    protocols: vec![IoTProtocol::HTTP],
                    last_seen: Utc::now(),
                    is_online: true,
                    signal_strength: None,
                    battery_level: None,
                    temperature: None,
                    humidity: None,
                };
                
                self.known_devices.insert(device_id, device);
            }
        }
    }

    async fn bluetooth_scan(&mut self) -> Result<()> {
        // Bluetooth device discovery using bluetoothctl or hcitool
        let output = Command::new("hcitool")
            .args(&["scan"])
            .output();

        if let Ok(output) = output {
            let stdout = str::from_utf8(&output.stdout)?;
            self.parse_bluetooth_output(stdout);
        }

        Ok(())
    }

    fn parse_bluetooth_output(&mut self, output: &str) {
        for line in output.lines() {
            if line.contains(":") && !line.starts_with("Scanning") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let mac = parts[0];
                    let name = parts[1..].join(" ");
                    
                    let device_id = format!("bt_{}", mac.replace(":", ""));
                    
                    if !self.known_devices.contains_key(&device_id) {
                        let device = IoTDevice {
                            device_id: device_id.clone(),
                            name: name.clone(),
                            device_type: self.classify_bluetooth_device(&name),
                            ip_address: "0.0.0.0".parse().unwrap(), // Bluetooth doesn't use IP
                            mac_address: Some(mac.to_string()),
                            manufacturer: None,
                            model: None,
                            firmware_version: None,
                            protocols: vec![IoTProtocol::Bluetooth],
                            last_seen: Utc::now(),
                            is_online: true,
                            signal_strength: None,
                            battery_level: None,
                            temperature: None,
                            humidity: None,
                        };
                        
                        self.known_devices.insert(device_id, device);
                    }
                }
            }
        }
    }

    fn classify_bluetooth_device(&self, name: &str) -> DeviceType {
        let name_lower = name.to_lowercase();
        
        if name_lower.contains("phone") || name_lower.contains("iphone") || name_lower.contains("samsung") {
            DeviceType::SmartPhone
        } else if name_lower.contains("watch") || name_lower.contains("fitbit") || name_lower.contains("garmin") {
            DeviceType::SmartWatch
        } else if name_lower.contains("speaker") || name_lower.contains("echo") || name_lower.contains("google") {
            DeviceType::SmartSpeaker
        } else if name_lower.contains("headset") || name_lower.contains("earbuds") {
            DeviceType::Unknown // Could add headphones type
        } else {
            DeviceType::Unknown
        }
    }

    async fn check_device_health(&mut self) -> Result<()> {
        let now = Utc::now();
        let timeout_duration = chrono::Duration::from_std(self.config.device_timeout).unwrap();
        
        // First, collect device IDs and IP addresses that need ping testing
        let mut devices_to_ping = Vec::new();
        for (device_id, device) in &self.known_devices {
            if device.ip_address.to_string() != "0.0.0.0" {
                devices_to_ping.push((device_id.clone(), device.ip_address));
            }
        }
        
        // Now update device health
        for device in self.known_devices.values_mut() {
            let time_since_seen = now.signed_duration_since(device.last_seen);
            
            if time_since_seen > timeout_duration {
                device.is_online = false;
            }
        }
        
        // Ping devices and update their status
        for (device_id, ip_address) in devices_to_ping {
            let ping_result = self.ping_device(&ip_address).await;
            if let Some(device) = self.known_devices.get_mut(&device_id) {
                device.is_online = ping_result.is_ok();
            }
        }
        
        Ok(())
    }

    async fn ping_device(&self, ip: &IpAddr) -> Result<()> {
        let output = Command::new("ping")
            .args(&["-c", "1", "-W", "1", &ip.to_string()])
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Ping failed"))
        }
    }

    fn calculate_device_health(&self) -> HashMap<String, DeviceHealth> {
        let mut health_map = HashMap::new();
        
        for (device_id, device) in &self.known_devices {
            let mut alerts = Vec::new();
            let mut health_score = 1.0f32;
            
            // Check connectivity
            let connectivity_status = if device.is_online {
                ConnectivityStatus::Online
            } else {
                ConnectivityStatus::Offline
            };
            
            if !device.is_online {
                health_score -= 0.5;
                alerts.push(DeviceAlert {
                    alert_type: IoTAlertType::DeviceOffline,
                    severity: IoTAlertSeverity::Critical,
                    message: "Device is offline".to_string(),
                    triggered_at: Utc::now(),
                });
            }
            
            // Check battery if available
            if let Some(battery) = device.battery_level {
                if battery < 20 {
                    health_score -= 0.2;
                    alerts.push(DeviceAlert {
                        alert_type: IoTAlertType::LowBattery,
                        severity: IoTAlertSeverity::Warning,
                        message: format!("Battery level low: {}%", battery),
                        triggered_at: Utc::now(),
                    });
                }
            }
            
            // Check temperature if available
            if let Some(temp) = device.temperature {
                if temp > 60.0 {
                    health_score -= 0.3;
                    alerts.push(DeviceAlert {
                        alert_type: IoTAlertType::HighTemperature,
                        severity: IoTAlertSeverity::Critical,
                        message: format!("High temperature: {:.1}°C", temp),
                        triggered_at: Utc::now(),
                    });
                }
            }
            
            let health = DeviceHealth {
                device_id: device_id.clone(),
                health_score: health_score.max(0.0),
                connectivity_status,
                response_time_ms: 50, // Mock data
                packet_loss_percent: 0.0,
                last_health_check: Utc::now(),
                alerts,
            };
            
            health_map.insert(device_id.clone(), health);
        }
        
        health_map
    }

    fn calculate_protocol_stats(&self) -> ProtocolStats {
        let wifi_devices = self.known_devices.values()
            .filter(|d| d.protocols.contains(&IoTProtocol::WiFi))
            .count() as u32;
            
        let bluetooth_connections = self.known_devices.values()
            .filter(|d| d.protocols.contains(&IoTProtocol::Bluetooth))
            .count() as u32;

        ProtocolStats {
            mqtt_messages: 1000, // Mock data
            coap_requests: 500,
            http_requests: 2000,
            bluetooth_connections,
            zigbee_messages: 300,
            wifi_devices,
        }
    }

    pub fn get_device_summary(&self) -> Vec<String> {
        let mut summary = Vec::new();
        
        if let Some(metrics) = &self.last_metrics {
            let total_devices = metrics.discovered_devices.len();
            let online_devices = metrics.discovered_devices.iter()
                .filter(|d| d.is_online)
                .count();
                
            summary.push(format!("IoT Devices: {}/{} online", online_devices, total_devices));
            
            let critical_alerts = metrics.device_health.values()
                .flat_map(|h| &h.alerts)
                .filter(|a| matches!(a.severity, IoTAlertSeverity::Critical))
                .count();
                
            if critical_alerts > 0 {
                summary.push(format!("🚨 {} Critical Alerts", critical_alerts));
            }
        }
        
        summary
    }
}

impl Default for IoTMonitor {
    fn default() -> Self {
        Self::with_default_config()
    }
}