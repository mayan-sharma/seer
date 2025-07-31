use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::str;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub name: String,
    pub sensor_type: SensorType,
    pub current_value: f64,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub critical_value: Option<f64>,
    pub unit: String,
    pub status: SensorStatus,
    pub chip: String,
    pub label: String,
    pub last_updated: DateTime<Utc>,
    pub history: Vec<(DateTime<Utc>, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SensorType {
    Temperature,
    Fan,
    Voltage,
    Power,
    Current,
    Energy,
    Humidity,
    Intrusion,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SensorStatus {
    Normal,
    Warning,
    Critical,
    #[default]
    Unknown,
    Fault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSensorMetrics {
    pub sensors: Vec<SensorReading>,
    pub total_sensors: usize,
    pub temperature_sensors: Vec<SensorReading>,
    pub fan_sensors: Vec<SensorReading>,
    pub voltage_sensors: Vec<SensorReading>,
    pub power_sensors: Vec<SensorReading>,
    pub critical_alerts: Vec<SensorReading>,
    pub average_cpu_temp: Option<f64>,
    pub average_fan_speed: Option<f64>,
    pub total_power_consumption: Option<f64>,
    pub sensor_backend: SensorBackend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SensorBackend {
    Lmsensors,
    Hwmon,
    Acpi,
    Ipmi,
    Manual,
    Unknown,
}

pub struct HardwareSensorMonitor {
    backend: SensorBackend,
    sensor_cache: HashMap<String, SensorReading>,
    hwmon_paths: Vec<String>,
    last_update: Option<DateTime<Utc>>,
    history_limit: usize,
}


impl From<&str> for SensorType {
    fn from(sensor_type: &str) -> Self {
        match sensor_type.to_lowercase().as_str() {
            "temp" | "temperature" => SensorType::Temperature,
            "fan" => SensorType::Fan,
            "in" | "voltage" => SensorType::Voltage,
            "power" => SensorType::Power,
            "curr" | "current" => SensorType::Current,
            "energy" => SensorType::Energy,
            "humidity" => SensorType::Humidity,
            "intrusion" => SensorType::Intrusion,
            _ => SensorType::Unknown,
        }
    }
}

impl Default for HardwareSensorMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareSensorMonitor {
    pub fn new() -> Self {
        let backend = Self::detect_sensor_backend();
        let hwmon_paths = Self::scan_hwmon_devices();
        
        Self {
            backend,
            sensor_cache: HashMap::new(),
            hwmon_paths,
            last_update: None,
            history_limit: 100, // Keep last 100 readings
        }
    }

    fn detect_sensor_backend() -> SensorBackend {
        // Check for lm-sensors (sensors command)
        if Command::new("sensors").arg("-v").output().is_ok() {
            return SensorBackend::Lmsensors;
        }
        
        // Check for hwmon sysfs interface
        if Path::new("/sys/class/hwmon").exists() {
            return SensorBackend::Hwmon;
        }
        
        // Check for ACPI thermal zones
        if Path::new("/sys/class/thermal").exists() {
            return SensorBackend::Acpi;
        }
        
        // Check for IPMI
        if Command::new("ipmitool").arg("sensor").output().is_ok() {
            return SensorBackend::Ipmi;
        }
        
        SensorBackend::Unknown
    }

    fn scan_hwmon_devices() -> Vec<String> {
        let mut paths = Vec::new();
        
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    paths.push(format!("/sys/class/hwmon/{}", name));
                }
            }
        }
        
        paths.sort();
        paths
    }

    pub fn get_hardware_sensor_metrics(&mut self) -> Result<HardwareSensorMetrics> {
        let sensors = match self.backend {
            SensorBackend::Lmsensors => self.get_lmsensors_readings()?,
            SensorBackend::Hwmon => self.get_hwmon_readings()?,
            SensorBackend::Acpi => self.get_acpi_readings()?,
            SensorBackend::Ipmi => self.get_ipmi_readings()?,
            _ => Vec::new(),
        };

        // Update sensor cache and history
        let now = Utc::now();
        for sensor in &sensors {
            if let Some(cached_sensor) = self.sensor_cache.get_mut(&sensor.name) {
                // Update existing sensor with history
                cached_sensor.current_value = sensor.current_value;
                cached_sensor.status = sensor.status.clone();
                cached_sensor.last_updated = now;
                
                // Add to history, maintaining limit
                cached_sensor.history.push((now, sensor.current_value));
                if cached_sensor.history.len() > self.history_limit {
                    cached_sensor.history.remove(0);
                }
            } else {
                // New sensor
                let mut new_sensor = sensor.clone();
                new_sensor.history = vec![(now, sensor.current_value)];
                self.sensor_cache.insert(sensor.name.clone(), new_sensor);
            }
        }

        // Categorize sensors
        let temperature_sensors = sensors.iter()
            .filter(|s| s.sensor_type == SensorType::Temperature)
            .cloned()
            .collect::<Vec<_>>();

        let fan_sensors = sensors.iter()
            .filter(|s| s.sensor_type == SensorType::Fan)
            .cloned()
            .collect::<Vec<_>>();

        let voltage_sensors = sensors.iter()
            .filter(|s| s.sensor_type == SensorType::Voltage)
            .cloned()
            .collect::<Vec<_>>();

        let power_sensors = sensors.iter()
            .filter(|s| s.sensor_type == SensorType::Power)
            .cloned()
            .collect::<Vec<_>>();

        let critical_alerts = sensors.iter()
            .filter(|s| s.status == SensorStatus::Critical || s.status == SensorStatus::Warning)
            .cloned()
            .collect::<Vec<_>>();

        // Calculate aggregates
        let average_cpu_temp = temperature_sensors.iter()
            .filter(|s| s.name.to_lowercase().contains("cpu") || s.chip.to_lowercase().contains("cpu"))
            .map(|s| s.current_value)
            .collect::<Vec<_>>();
        let average_cpu_temp = if !average_cpu_temp.is_empty() {
            Some(average_cpu_temp.iter().sum::<f64>() / average_cpu_temp.len() as f64)
        } else {
            None
        };

        let average_fan_speed = if !fan_sensors.is_empty() {
            let total: f64 = fan_sensors.iter().map(|s| s.current_value).sum();
            Some(total / fan_sensors.len() as f64)
        } else {
            None
        };

        let total_power_consumption = if !power_sensors.is_empty() {
            Some(power_sensors.iter().map(|s| s.current_value).sum())
        } else {
            None
        };

        self.last_update = Some(now);

        Ok(HardwareSensorMetrics {
            total_sensors: sensors.len(),
            temperature_sensors,
            fan_sensors,
            voltage_sensors,
            power_sensors,
            critical_alerts,
            average_cpu_temp,
            average_fan_speed,
            total_power_consumption,
            sensor_backend: self.backend.clone(),
            sensors,
        })
    }

    fn get_lmsensors_readings(&self) -> Result<Vec<SensorReading>> {
        let mut sensors = Vec::new();
        
        let output = Command::new("sensors")
            .args(["-A", "-u"]) // All sensors, raw output
            .output()?;

        if !output.status.success() {
            return Ok(sensors);
        }

        let stdout = str::from_utf8(&output.stdout)?;
        sensors.extend(self.parse_lmsensors_output(stdout)?);

        Ok(sensors)
    }

    fn parse_lmsensors_output(&self, output: &str) -> Result<Vec<SensorReading>> {
        let mut sensors = Vec::new();
        let mut current_chip = String::new();
        let now = Utc::now();

        for line in output.lines() {
            let line = line.trim();
            
            if line.is_empty() {
                continue;
            }
            
            // Chip identifier line
            if !line.starts_with(' ') && line.contains('-') {
                current_chip = line.to_string();
                continue;
            }
            
            // Sensor reading line
            if line.contains(':') && !line.starts_with(' ') {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let sensor_name = parts[0].trim();
                    let value_part = parts[1].trim();
                    
                    if let Ok(value) = self.parse_sensor_value(value_part) {
                        let sensor_type = self.determine_sensor_type(sensor_name);
                        let unit = self.extract_unit(value_part);
                        let status = self.determine_sensor_status(&sensor_type, value);
                        
                        sensors.push(SensorReading {
                            name: format!("{}:{}", current_chip, sensor_name),
                            sensor_type,
                            current_value: value,
                            min_value: None,
                            max_value: None,
                            critical_value: None,
                            unit,
                            status,
                            chip: current_chip.clone(),
                            label: sensor_name.to_string(),
                            last_updated: now,
                            history: Vec::new(),
                        });
                    }
                }
            }
        }

        Ok(sensors)
    }

    fn get_hwmon_readings(&self) -> Result<Vec<SensorReading>> {
        let mut sensors = Vec::new();
        let now = Utc::now();

        for hwmon_path in &self.hwmon_paths {
            // Get chip name
            let name_path = format!("{}/name", hwmon_path);
            let chip_name = fs::read_to_string(&name_path)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim()
                .to_string();

            // Scan for sensor files
            if let Ok(entries) = fs::read_dir(hwmon_path) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();
                    
                    // Look for input files (actual sensor readings)
                    if file_name_str.ends_with("_input") {
                        let sensor_base = file_name_str.strip_suffix("_input").unwrap();
                        
                        if let Ok(value_str) = fs::read_to_string(entry.path()) {
                            if let Ok(raw_value) = value_str.trim().parse::<f64>() {
                                let sensor_type = self.determine_hwmon_sensor_type(sensor_base);
                                let (value, unit) = self.convert_hwmon_value(raw_value, &sensor_type);
                                
                                // Try to read label
                                let label_path = format!("{}/{}_label", hwmon_path, sensor_base);
                                let label = fs::read_to_string(&label_path)
                                    .unwrap_or_else(|_| sensor_base.to_string())
                                    .trim()
                                    .to_string();

                                // Try to read min/max/crit values
                                let min_value = self.read_hwmon_threshold(hwmon_path, sensor_base, "min", &sensor_type);
                                let max_value = self.read_hwmon_threshold(hwmon_path, sensor_base, "max", &sensor_type);
                                let critical_value = self.read_hwmon_threshold(hwmon_path, sensor_base, "crit", &sensor_type);

                                let status = self.determine_sensor_status_with_thresholds(
                                    &sensor_type, value, min_value, max_value, critical_value
                                );

                                sensors.push(SensorReading {
                                    name: format!("{}:{}", chip_name, sensor_base),
                                    sensor_type,
                                    current_value: value,
                                    min_value,
                                    max_value,
                                    critical_value,
                                    unit,
                                    status,
                                    chip: chip_name.clone(),
                                    label,
                                    last_updated: now,
                                    history: Vec::new(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(sensors)
    }

    fn get_acpi_readings(&self) -> Result<Vec<SensorReading>> {
        let mut sensors = Vec::new();
        let now = Utc::now();

        // Read ACPI thermal zones
        if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let dir_name = entry.file_name();
                let dir_name_str = dir_name.to_string_lossy();
                
                if dir_name_str.starts_with("thermal_zone") {
                    let zone_path = entry.path();
                    
                    // Read temperature
                    let temp_path = zone_path.join("temp");
                    if let Ok(temp_str) = fs::read_to_string(&temp_path) {
                        if let Ok(temp_millidegree) = temp_str.trim().parse::<f64>() {
                            let temp_celsius = temp_millidegree / 1000.0;
                            
                            // Read zone type/name
                            let type_path = zone_path.join("type");
                            let zone_type = fs::read_to_string(&type_path)
                                .unwrap_or_else(|_| dir_name_str.to_string())
                                .trim()
                                .to_string();

                            let status = self.determine_sensor_status(&SensorType::Temperature, temp_celsius);

                            sensors.push(SensorReading {
                                name: format!("acpi:{}", zone_type),
                                sensor_type: SensorType::Temperature,
                                current_value: temp_celsius,
                                min_value: None,
                                max_value: None,
                                critical_value: None,
                                unit: "°C".to_string(),
                                status,
                                chip: "acpi".to_string(),
                                label: zone_type,
                                last_updated: now,
                                history: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        Ok(sensors)
    }

    fn get_ipmi_readings(&self) -> Result<Vec<SensorReading>> {
        let mut sensors = Vec::new();
        
        let output = Command::new("ipmitool")
            .args(["sensor"])
            .output()?;

        if !output.status.success() {
            return Ok(sensors);
        }

        let stdout = str::from_utf8(&output.stdout)?;
        let now = Utc::now();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                let sensor_name = parts[0].trim().to_string();
                let value_str = parts[1].trim();
                let unit = parts[2].trim().to_string();

                if let Ok(value) = value_str.parse::<f64>() {
                    let sensor_type = self.determine_sensor_type(&sensor_name);
                    let status = self.determine_sensor_status(&sensor_type, value);

                    sensors.push(SensorReading {
                        name: format!("ipmi:{}", sensor_name),
                        sensor_type,
                        current_value: value,
                        min_value: None,
                        max_value: None,
                        critical_value: None,
                        unit,
                        status,
                        chip: "ipmi".to_string(),
                        label: sensor_name,
                        last_updated: now,
                        history: Vec::new(),
                    });
                }
            }
        }

        Ok(sensors)
    }

    fn parse_sensor_value(&self, value_str: &str) -> Result<f64> {
        // Extract numeric value from strings like "+45.0°C" or "1234 RPM"
        let mut numeric_str = String::new();
        let mut found_decimal = false;
        
        for ch in value_str.chars() {
            if ch.is_ascii_digit() || (ch == '.' && !found_decimal) || (ch == '-' && numeric_str.is_empty()) || (ch == '+' && numeric_str.is_empty()) {
                if ch == '.' {
                    found_decimal = true;
                }
                numeric_str.push(ch);
            } else if !numeric_str.is_empty() {
                break;
            }
        }
        
        numeric_str.parse::<f64>().map_err(|e| anyhow!("Failed to parse sensor value: {}", e))
    }

    fn extract_unit(&self, value_str: &str) -> String {
        if value_str.contains("°C") {
            "°C".to_string()
        } else if value_str.contains("°F") {
            "°F".to_string()
        } else if value_str.contains("RPM") {
            "RPM".to_string()
        } else if value_str.contains(" V") {
            "V".to_string()
        } else if value_str.contains(" W") {
            "W".to_string()
        } else if value_str.contains(" A") {
            "A".to_string()
        } else if value_str.contains('%') {
            "%".to_string()
        } else {
            "".to_string()
        }
    }

    fn determine_sensor_type(&self, sensor_name: &str) -> SensorType {
        let name_lower = sensor_name.to_lowercase();
        
        if name_lower.contains("temp") || name_lower.contains("thermal") {
            SensorType::Temperature
        } else if name_lower.contains("fan") {
            SensorType::Fan
        } else if name_lower.contains("in") || name_lower.contains("volt") {
            SensorType::Voltage
        } else if name_lower.contains("power") {
            SensorType::Power
        } else if name_lower.contains("curr") {
            SensorType::Current
        } else if name_lower.contains("energy") {
            SensorType::Energy
        } else if name_lower.contains("humidity") {
            SensorType::Humidity
        } else if name_lower.contains("intrusion") {
            SensorType::Intrusion
        } else {
            SensorType::Unknown
        }
    }

    fn determine_hwmon_sensor_type(&self, sensor_base: &str) -> SensorType {
        if sensor_base.starts_with("temp") {
            SensorType::Temperature
        } else if sensor_base.starts_with("fan") {
            SensorType::Fan
        } else if sensor_base.starts_with("in") {
            SensorType::Voltage
        } else if sensor_base.starts_with("power") {
            SensorType::Power
        } else if sensor_base.starts_with("curr") {
            SensorType::Current
        } else if sensor_base.starts_with("energy") {
            SensorType::Energy
        } else if sensor_base.starts_with("humidity") {
            SensorType::Humidity
        } else {
            SensorType::Unknown
        }
    }

    fn convert_hwmon_value(&self, raw_value: f64, sensor_type: &SensorType) -> (f64, String) {
        match sensor_type {
            SensorType::Temperature => (raw_value / 1000.0, "°C".to_string()),
            SensorType::Voltage => (raw_value / 1000.0, "V".to_string()),
            SensorType::Power => (raw_value / 1000000.0, "W".to_string()),
            SensorType::Current => (raw_value / 1000.0, "A".to_string()),
            SensorType::Energy => (raw_value / 1000000.0, "J".to_string()),
            SensorType::Fan => (raw_value, "RPM".to_string()),
            _ => (raw_value, "".to_string()),
        }
    }

    fn read_hwmon_threshold(&self, hwmon_path: &str, sensor_base: &str, threshold_type: &str, sensor_type: &SensorType) -> Option<f64> {
        let threshold_path = format!("{}/_{}_{}", hwmon_path, sensor_base, threshold_type);
        
        if let Ok(value_str) = fs::read_to_string(threshold_path) {
            if let Ok(raw_value) = value_str.trim().parse::<f64>() {
                let (value, _) = self.convert_hwmon_value(raw_value, sensor_type);
                return Some(value);
            }
        }
        
        None
    }

    fn determine_sensor_status(&self, sensor_type: &SensorType, value: f64) -> SensorStatus {
        match sensor_type {
            SensorType::Temperature => {
                if value > 85.0 {
                    SensorStatus::Critical
                } else if value > 70.0 {
                    SensorStatus::Warning
                } else {
                    SensorStatus::Normal
                }
            }
            SensorType::Fan => {
                if value < 500.0 {
                    SensorStatus::Warning
                } else {
                    SensorStatus::Normal
                }
            }
            _ => SensorStatus::Normal,
        }
    }

    fn determine_sensor_status_with_thresholds(
        &self,
        sensor_type: &SensorType,
        value: f64,
        min_value: Option<f64>,
        max_value: Option<f64>,
        critical_value: Option<f64>,
    ) -> SensorStatus {
        if let Some(crit) = critical_value {
            if value >= crit {
                return SensorStatus::Critical;
            }
        }

        if let Some(max) = max_value {
            if value >= max {
                return SensorStatus::Warning;
            }
        }

        if let Some(min) = min_value {
            if value <= min {
                return SensorStatus::Warning;
            }
        }

        // Fallback to type-based status
        self.determine_sensor_status(sensor_type, value)
    }

    pub fn get_sensor_backend(&self) -> &SensorBackend {
        &self.backend
    }

    pub fn is_hardware_monitoring_available(&self) -> bool {
        self.backend != SensorBackend::Unknown
    }

    pub fn get_sensor_history(&self, sensor_name: &str) -> Option<&Vec<(DateTime<Utc>, f64)>> {
        self.sensor_cache.get(sensor_name).map(|s| &s.history)
    }
}