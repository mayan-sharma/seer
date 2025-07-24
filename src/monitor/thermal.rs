use anyhow::Result;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalMetrics {
    pub cpu_temperatures: Vec<CpuTemperature>,
    pub thermal_zones: Vec<ThermalZone>,
    pub cooling_devices: Vec<CoolingDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuTemperature {
    pub core_id: u32,
    pub temperature: f32,
    pub critical_temp: Option<f32>,
    pub max_temp: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalZone {
    pub zone_id: u32,
    pub zone_type: String,
    pub temperature: f32,
    pub critical_temp: Option<f32>,
    pub policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoolingDevice {
    pub device_id: u32,
    pub device_type: String,
    pub current_state: u32,
    pub max_state: u32,
}

pub struct ThermalMonitor {
    thermal_zones_path: String,
    _cooling_devices_path: String,
}

impl ThermalMonitor {
    pub fn new() -> Self {
        Self {
            thermal_zones_path: "/sys/class/thermal".to_string(),
            _cooling_devices_path: "/sys/class/thermal".to_string(),
        }
    }

    pub fn get_thermal_metrics(&self) -> Result<ThermalMetrics> {
        let mut cpu_temperatures = Vec::new();
        let mut thermal_zones = Vec::new();
        let mut cooling_devices = Vec::new();

        // Read thermal zones
        if let Ok(entries) = fs::read_dir(&self.thermal_zones_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if filename.starts_with("thermal_zone") {
                    if let Ok(zone) = self.read_thermal_zone(&path) {
                        thermal_zones.push(zone);
                    }
                } else if filename.starts_with("cooling_device") {
                    if let Ok(device) = self.read_cooling_device(&path) {
                        cooling_devices.push(device);
                    }
                }
            }
        }

        // Try to get CPU core temperatures from different sources
        cpu_temperatures.extend(self.get_cpu_temperatures_from_coretemp()?);
        cpu_temperatures.extend(self.get_cpu_temperatures_from_hwmon()?);

        Ok(ThermalMetrics {
            cpu_temperatures,
            thermal_zones,
            cooling_devices,
        })
    }

    fn read_thermal_zone(&self, zone_path: &Path) -> Result<ThermalZone> {
        let zone_id = zone_path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|s| s.strip_prefix("thermal_zone"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let temp_path = zone_path.join("temp");
        let type_path = zone_path.join("type");
        let policy_path = zone_path.join("policy");
        let crit_path = zone_path.join("trip_point_0_temp");

        let temperature = if temp_path.exists() {
            fs::read_to_string(temp_path)?
                .trim()
                .parse::<i32>()
                .map(|t| t as f32 / 1000.0)
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let zone_type = if type_path.exists() {
            fs::read_to_string(type_path)?.trim().to_string()
        } else {
            "unknown".to_string()
        };

        let policy = if policy_path.exists() {
            fs::read_to_string(policy_path)?.trim().to_string()
        } else {
            "unknown".to_string()
        };

        let critical_temp = if crit_path.exists() {
            fs::read_to_string(crit_path).ok()
                .and_then(|s| s.trim().parse::<i32>().ok())
                .map(|t| t as f32 / 1000.0)
        } else {
            None
        };

        Ok(ThermalZone {
            zone_id,
            zone_type,
            temperature,
            critical_temp,
            policy,
        })
    }

    fn read_cooling_device(&self, device_path: &Path) -> Result<CoolingDevice> {
        let device_id = device_path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|s| s.strip_prefix("cooling_device"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let type_path = device_path.join("type");
        let cur_state_path = device_path.join("cur_state");
        let max_state_path = device_path.join("max_state");

        let device_type = if type_path.exists() {
            fs::read_to_string(type_path)?.trim().to_string()
        } else {
            "unknown".to_string()
        };

        let current_state = if cur_state_path.exists() {
            fs::read_to_string(cur_state_path)?
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        let max_state = if max_state_path.exists() {
            fs::read_to_string(max_state_path)?
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        Ok(CoolingDevice {
            device_id,
            device_type,
            current_state,
            max_state,
        })
    }

    fn get_cpu_temperatures_from_coretemp(&self) -> Result<Vec<CpuTemperature>> {
        let mut temperatures = Vec::new();
        let hwmon_path = "/sys/class/hwmon";

        if let Ok(entries) = fs::read_dir(hwmon_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name_path = path.join("name");
                
                if name_path.exists() {
                    if let Ok(name) = fs::read_to_string(name_path) {
                        if name.trim() == "coretemp" {
                            temperatures.extend(self.read_coretemp_sensors(&path)?);
                        }
                    }
                }
            }
        }

        Ok(temperatures)
    }

    fn get_cpu_temperatures_from_hwmon(&self) -> Result<Vec<CpuTemperature>> {
        let mut temperatures = Vec::new();
        let hwmon_path = "/sys/class/hwmon";

        if let Ok(entries) = fs::read_dir(hwmon_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                // Look for temp*_input files
                if let Ok(temp_entries) = fs::read_dir(&path) {
                    for temp_entry in temp_entries.flatten() {
                        let temp_path = temp_entry.path();
                        let filename = temp_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        if filename.starts_with("temp") && filename.ends_with("_input") {
                            if let Ok(temp_str) = fs::read_to_string(&temp_path) {
                                if let Ok(temp_millis) = temp_str.trim().parse::<i32>() {
                                    let temp = temp_millis as f32 / 1000.0;
                                    
                                    // Extract core ID from filename
                                    let core_id = filename.chars()
                                        .skip(4)
                                        .take_while(|c| c.is_ascii_digit())
                                        .collect::<String>()
                                        .parse()
                                        .unwrap_or(0);

                                    // Try to get critical and max temps
                                    let base_name = filename.replace("_input", "");
                                    let crit_path = path.join(format!("{}_crit", base_name));
                                    let max_path = path.join(format!("{}_max", base_name));

                                    let critical_temp = if crit_path.exists() {
                                        fs::read_to_string(crit_path).ok()
                                            .and_then(|s| s.trim().parse::<i32>().ok())
                                            .map(|t| t as f32 / 1000.0)
                                    } else {
                                        None
                                    };

                                    let max_temp = if max_path.exists() {
                                        fs::read_to_string(max_path).ok()
                                            .and_then(|s| s.trim().parse::<i32>().ok())
                                            .map(|t| t as f32 / 1000.0)
                                    } else {
                                        None
                                    };

                                    temperatures.push(CpuTemperature {
                                        core_id,
                                        temperature: temp,
                                        critical_temp,
                                        max_temp,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(temperatures)
    }

    fn read_coretemp_sensors(&self, hwmon_path: &Path) -> Result<Vec<CpuTemperature>> {
        let mut temperatures = Vec::new();

        if let Ok(entries) = fs::read_dir(hwmon_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Look for temp*_input files
                if filename.starts_with("temp") && filename.ends_with("_input") {
                    if let Ok(temp_str) = fs::read_to_string(&path) {
                        if let Ok(temp_millis) = temp_str.trim().parse::<i32>() {
                            let temp = temp_millis as f32 / 1000.0;
                            
                            // Extract core number from label file if available
                            let label_file = path.parent()
                                .unwrap()
                                .join(filename.replace("_input", "_label"));
                            
                            let core_id = if label_file.exists() {
                                fs::read_to_string(label_file).ok()
                                    .and_then(|label| {
                                        if label.contains("Core") {
                                            label.chars()
                                                .filter(|c| c.is_ascii_digit())
                                                .collect::<String>()
                                                .parse()
                                                .ok()
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or_else(|| {
                                        filename.chars()
                                            .skip(4)
                                            .take_while(|c| c.is_ascii_digit())
                                            .collect::<String>()
                                            .parse()
                                            .unwrap_or(0)
                                    })
                            } else {
                                filename.chars()
                                    .skip(4)
                                    .take_while(|c| c.is_ascii_digit())
                                    .collect::<String>()
                                    .parse()
                                    .unwrap_or(0)
                            };

                            temperatures.push(CpuTemperature {
                                core_id,
                                temperature: temp,
                                critical_temp: None,
                                max_temp: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(temperatures)
    }

    pub fn get_average_cpu_temperature(&self) -> Result<Option<f32>> {
        let metrics = self.get_thermal_metrics()?;
        
        if metrics.cpu_temperatures.is_empty() {
            Ok(None)
        } else {
            let sum: f32 = metrics.cpu_temperatures.iter()
                .map(|t| t.temperature)
                .sum();
            Ok(Some(sum / metrics.cpu_temperatures.len() as f32))
        }
    }
}