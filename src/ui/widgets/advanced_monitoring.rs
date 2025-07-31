use crate::monitor::*;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Tabs, 
        BorderType, Wrap,
    },
    Frame,
};

pub struct AdvancedMonitoringView {
    pub current_tab: AdvancedTab,
    pub database_scroll: usize,
    pub apm_scroll: usize,
    pub iot_scroll: usize,
    pub backup_scroll: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdvancedTab {
    Database,
    APM,
    IoT,
    Backup,
}

impl Default for AdvancedMonitoringView {
    fn default() -> Self {
        Self {
            current_tab: AdvancedTab::Database,
            database_scroll: 0,
            apm_scroll: 0,
            iot_scroll: 0,
            backup_scroll: 0,
        }
    }
}

impl AdvancedMonitoringView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            AdvancedTab::Database => AdvancedTab::APM,
            AdvancedTab::APM => AdvancedTab::IoT,
            AdvancedTab::IoT => AdvancedTab::Backup,
            AdvancedTab::Backup => AdvancedTab::Database,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            AdvancedTab::Database => AdvancedTab::Backup,
            AdvancedTab::APM => AdvancedTab::Database,
            AdvancedTab::IoT => AdvancedTab::APM,
            AdvancedTab::Backup => AdvancedTab::IoT,
        };
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, monitor: &SystemMonitor) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Render tabs
        self.render_tabs(f, chunks[0]);

        // Render current tab content
        match self.current_tab {
            AdvancedTab::Database => self.render_database_tab(f, chunks[1], monitor),
            AdvancedTab::APM => self.render_apm_tab(f, chunks[1], monitor),
            AdvancedTab::IoT => self.render_iot_tab(f, chunks[1], monitor),
            AdvancedTab::Backup => self.render_backup_tab(f, chunks[1], monitor),
        }
    }

    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let tab_titles = vec!["Database", "APM", "IoT", "Backup"];
        let selected_tab = match self.current_tab {
            AdvancedTab::Database => 0,
            AdvancedTab::APM => 1,
            AdvancedTab::IoT => 2,
            AdvancedTab::Backup => 3,
        };

        let tabs = Tabs::new(tab_titles)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Advanced Monitoring"))
            .select(selected_tab)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Yellow)
                .fg(Color::Black));

        f.render_widget(tabs, area);
    }

    fn render_database_tab(&mut self, f: &mut Frame, area: Rect, monitor: &SystemMonitor) {
        if let Some(db_metrics) = monitor.database_monitor.get_metrics() {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Left panel - Database summary
            self.render_database_summary(f, chunks[0], db_metrics);
            
            // Right panel - Database details
            self.render_database_details(f, chunks[1], db_metrics);
        } else {
            let no_data = Paragraph::new("No database monitoring data available\n\nTo enable database monitoring:\n1. Configure database connections in settings\n2. Ensure database servers are accessible")
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Database Monitoring"))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);
            f.render_widget(no_data, area);
        }
    }

    fn render_database_summary(&self, f: &mut Frame, area: Rect, metrics: &DatabaseMetrics) {
        let mut items = Vec::new();

        if let Some(mysql) = &metrics.mysql {
            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled("MySQL", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(format!("  Connections: {}/{}", mysql.connections.current, mysql.connections.max)),
                Line::from(format!("  Queries/sec: {:.1}", mysql.queries.queries_per_second)),
                Line::from(format!("  Uptime: {}s", mysql.uptime)),
                Line::from(""),
            ]));
        }

        if let Some(postgres) = &metrics.postgresql {
            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled("PostgreSQL", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(format!("  Connections: {}/{}", postgres.connections.current, postgres.connections.max)),
                Line::from(format!("  Cache Hit: {:.1}%", postgres.cache_hit_ratio * 100.0)),
                Line::from(format!("  Databases: {}", postgres.database_stats.len())),
                Line::from(""),
            ]));
        }

        if let Some(mongodb) = &metrics.mongodb {
            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled("MongoDB", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(format!("  Connections: {}/{}", mongodb.connections.current, mongodb.connections.max)),
                Line::from(format!("  Memory: {}MB", mongodb.memory.resident / (1024 * 1024))),
                Line::from(format!("  Operations: {}", mongodb.operations.queries)),
                Line::from(""),
            ]));
        }

        if let Some(redis) = &metrics.redis {
            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled("Redis", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(format!("  Connections: {}/{}", redis.connections.current, redis.connections.max)),
                Line::from(format!("  Memory: {}MB", redis.memory.used / (1024 * 1024))),
                Line::from(format!("  Keys: {}", redis.keyspace.total_keys)),
                Line::from(""),
            ]));
        }

        if items.is_empty() {
            items.push(ListItem::new("No databases detected"));
        }

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Database Summary"));

        f.render_widget(list, area);
    }

    fn render_database_details(&self, f: &mut Frame, area: Rect, _metrics: &DatabaseMetrics) {
        let details = Paragraph::new("Database performance metrics and connection details would be displayed here.\n\nKey metrics:\n• Query performance\n• Connection pooling\n• Replication status\n• Storage utilization\n• Cache efficiency")
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Database Details"))
            .wrap(Wrap { trim: true });

        f.render_widget(details, area);
    }

    fn render_apm_tab(&mut self, f: &mut Frame, area: Rect, monitor: &SystemMonitor) {
        if let Some(apm_metrics) = monitor.apm_monitor.get_metrics() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);

            // Top panel - Application overview
            self.render_apm_overview(f, chunks[0], apm_metrics);
            
            // Bottom panel - Anomalies and alerts
            self.render_apm_anomalies(f, chunks[1], monitor.apm_monitor.get_anomalies());
        } else {
            let no_data = Paragraph::new("No APM data available\n\nAPM monitoring tracks:\n• JVM applications (Java, Scala, Kotlin)\n• .NET applications\n• Python applications\n• Node.js applications\n• Go applications")
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Application Performance Monitoring"))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);
            f.render_widget(no_data, area);
        }
    }

    fn render_apm_overview(&self, f: &mut Frame, area: Rect, metrics: &APMMetrics) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left - Application counts
        let mut app_items = Vec::new();
        
        if !metrics.jvm_applications.is_empty() {
            app_items.push(ListItem::new(format!("☕ JVM Applications: {}", metrics.jvm_applications.len())));
            for jvm in &metrics.jvm_applications {
                let heap_usage = (jvm.heap_memory.used as f64 / jvm.heap_memory.max as f64) * 100.0;
                app_items.push(ListItem::new(format!("  {} - Heap: {:.1}%", jvm.name, heap_usage)));
            }
        }

        if !metrics.dotnet_applications.is_empty() {
            app_items.push(ListItem::new(format!("🔷 .NET Applications: {}", metrics.dotnet_applications.len())));
            for dotnet in &metrics.dotnet_applications {
                app_items.push(ListItem::new(format!("  {} - GC: {:.1}%", dotnet.name, dotnet.time_in_gc)));
            }
        }

        if !metrics.python_applications.is_empty() {
            app_items.push(ListItem::new(format!("🐍 Python Applications: {}", metrics.python_applications.len())));
        }

        if !metrics.nodejs_applications.is_empty() {
            app_items.push(ListItem::new(format!("🟢 Node.js Applications: {}", metrics.nodejs_applications.len())));
        }

        if !metrics.golang_applications.is_empty() {
            app_items.push(ListItem::new(format!("🔵 Go Applications: {}", metrics.golang_applications.len())));
        }

        if app_items.is_empty() {
            app_items.push(ListItem::new("No applications detected"));
        }

        let app_list = List::new(app_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Applications"));

        f.render_widget(app_list, chunks[0]);

        // Right - Performance summary
        let perf_text = "Application Performance Overview\n\n• Memory usage patterns\n• Garbage collection metrics\n• Thread utilization\n• Exception rates\n• Response times";
        let perf_widget = Paragraph::new(perf_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Performance Metrics"))
            .wrap(Wrap { trim: true });

        f.render_widget(perf_widget, chunks[1]);
    }

    fn render_apm_anomalies(&self, f: &mut Frame, area: Rect, anomalies: &[APMAnomaly]) {
        let mut items = Vec::new();

        for anomaly in anomalies {
            let severity_color = match anomaly.severity {
                APMAnomalySeverity::Critical => Color::Red,
                APMAnomalySeverity::High => Color::LightRed,
                APMAnomalySeverity::Medium => Color::Yellow,
                APMAnomalySeverity::Low => Color::Green,
            };

            let anomaly_type = match anomaly.anomaly_type {
                APMAnomalyType::HighCPUUsage => "🔥 High CPU",
                APMAnomalyType::MemoryLeak => "💧 Memory Leak",
                APMAnomalyType::HighGCPressure => "🗑️ High GC Pressure",
                APMAnomalyType::ThreadContention => "🧵 Thread Contention",
                APMAnomalyType::LongGCPause => "⏸️ Long GC Pause",
                APMAnomalyType::HighExceptionRate => "⚠️ High Exceptions",
                APMAnomalyType::EventLoopBlocked => "🔄 Event Loop Blocked",
            };

            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled(anomaly_type, Style::default().fg(severity_color).add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" - {}", anomaly.application_name)),
                ]),
                Line::from(format!("  {}", anomaly.description)),
            ]));
        }

        if items.is_empty() {
            items.push(ListItem::new("No performance anomalies detected"));
        }

        let anomaly_list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Performance Anomalies"));

        f.render_widget(anomaly_list, area);
    }

    fn render_iot_tab(&mut self, f: &mut Frame, area: Rect, monitor: &SystemMonitor) {
        if let Some(iot_metrics) = monitor.iot_monitor.get_metrics() {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Left panel - Device overview
            self.render_iot_devices(f, chunks[0], &iot_metrics.discovered_devices);
            
            // Right panel - Protocol stats and alerts
            self.render_iot_stats(f, chunks[1], &iot_metrics.protocol_stats, &iot_metrics.device_health);
        } else {
            let no_data = Paragraph::new("No IoT devices detected\n\nIoT monitoring discovers:\n• WiFi devices\n• Bluetooth devices\n• Zigbee devices\n• Smart home devices\n• Network-connected sensors")
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("IoT Device Monitoring"))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);
            f.render_widget(no_data, area);
        }
    }

    fn render_iot_devices(&self, f: &mut Frame, area: Rect, devices: &[IoTDevice]) {
        let mut items = Vec::new();

        for device in devices.iter().take(20) {  // Limit display
            let status_icon = if device.is_online { "🟢" } else { "🔴" };
            let device_icon = match device.device_type {
                DeviceType::SmartPhone => "📱",
                DeviceType::SmartTV => "📺",
                DeviceType::SmartSpeaker => "🔊",
                DeviceType::SecurityCamera => "📹",
                DeviceType::SmartThermostat => "🌡️",
                DeviceType::SmartLight => "💡",
                DeviceType::Router => "📡",
                DeviceType::Printer => "🖨️",
                DeviceType::SmartWatch => "⌚",
                _ => "🔧",
            };

            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::raw(format!("{} {} {}", status_icon, device_icon, device.name)),
                ]),
                Line::from(format!("  IP: {} | Type: {:?}", device.ip_address, device.device_type)),
                if let Some(battery) = device.battery_level {
                    Line::from(format!("  Battery: {}%", battery))
                } else {
                    Line::from("  Battery: N/A")
                },
            ]));
        }

        if items.is_empty() {
            items.push(ListItem::new("No IoT devices discovered"));
        }

        let device_list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!("IoT Devices ({})", devices.len())));

        f.render_widget(device_list, area);
    }

    fn render_iot_stats(&self, f: &mut Frame, area: Rect, stats: &ProtocolStats, health: &std::collections::HashMap<String, DeviceHealth>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Protocol statistics
        let proto_items = vec![
            ListItem::new(format!("📶 WiFi Devices: {}", stats.wifi_devices)),
            ListItem::new(format!("🔵 Bluetooth: {}", stats.bluetooth_connections)),
            ListItem::new(format!("📡 MQTT Messages: {}", stats.mqtt_messages)),
            ListItem::new(format!("🌐 HTTP Requests: {}", stats.http_requests)),
            ListItem::new(format!("📋 CoAP Requests: {}", stats.coap_requests)),
            ListItem::new(format!("⚡ Zigbee Messages: {}", stats.zigbee_messages)),
        ];

        let proto_list = List::new(proto_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Protocol Statistics"));

        f.render_widget(proto_list, chunks[0]);

        // Device health alerts
        let mut alert_items = Vec::new();
        let mut critical_count = 0;

        for device_health in health.values() {
            for alert in &device_health.alerts {
                if matches!(alert.severity, IoTAlertSeverity::Critical) {
                    critical_count += 1;
                }
                
                let severity_icon = match alert.severity {
                    IoTAlertSeverity::Critical => "🚨",
                    IoTAlertSeverity::Warning => "⚠️",
                    IoTAlertSeverity::Info => "ℹ️",
                };

                alert_items.push(ListItem::new(format!("{} {}", severity_icon, alert.message)));
            }
        }

        if alert_items.is_empty() {
            alert_items.push(ListItem::new("No device alerts"));
        }

        let alert_list = List::new(alert_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!("Device Health ({} critical)", critical_count)));

        f.render_widget(alert_list, chunks[1]);
    }

    fn render_backup_tab(&mut self, f: &mut Frame, area: Rect, monitor: &SystemMonitor) {
        if let Some(backup_metrics) = monitor.backup_monitor.get_metrics() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);

            // Top panel - Backup jobs
            self.render_backup_jobs(f, chunks[0], &backup_metrics.backup_jobs);
            
            // Bottom panel - Storage and alerts
            self.render_backup_storage(f, chunks[1], &backup_metrics.storage_locations, &backup_metrics.alerts);
        } else {
            let no_data = Paragraph::new("No backup jobs detected\n\nBackup monitoring tracks:\n• Rsync jobs\n• Cron backup tasks\n• Systemd backup services\n• Storage utilization\n• Backup success rates")
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Backup & Recovery Monitoring"))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);
            f.render_widget(no_data, area);
        }
    }

    fn render_backup_jobs(&self, f: &mut Frame, area: Rect, jobs: &[BackupJob]) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // Job list
        let mut items = Vec::new();
        
        for job in jobs {
            let status_icon = match job.status {
                BackupStatus::Completed => "✅",
                BackupStatus::Running => "🔄",
                BackupStatus::Failed => "❌",
                BackupStatus::Scheduled => "⏰",
                BackupStatus::Cancelled => "🚫",
                BackupStatus::Paused => "⏸️",
            };

            let job_type_icon = match job.job_type {
                BackupType::Full => "📦",
                BackupType::Incremental => "📈",
                BackupType::Differential => "📊",
                BackupType::Snapshot => "📸",
                BackupType::Database => "🗄️",
                _ => "💾",
            };

            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::raw(format!("{} {} {}", status_icon, job_type_icon, job.name)),
                ]),
                Line::from(format!("  Success Rate: {:.1}% | Size: {:.1}GB", 
                    job.success_rate, 
                    job.data_size as f64 / (1024.0 * 1024.0 * 1024.0))),
                Line::from(format!("  Schedule: {:?}", job.schedule.frequency)),
            ]));
        }

        if items.is_empty() {
            items.push(ListItem::new("No backup jobs found"));
        }

        let job_list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!("Backup Jobs ({})", jobs.len())));

        f.render_widget(job_list, chunks[0]);

        // Job statistics
        let running_jobs = jobs.iter().filter(|j| matches!(j.status, BackupStatus::Running)).count();
        let failed_jobs = jobs.iter().filter(|j| matches!(j.status, BackupStatus::Failed)).count();
        let total_data: u64 = jobs.iter().map(|j| j.data_size).sum();

        let stats_text = format!(
            "Backup Statistics\n\nTotal Jobs: {}\nRunning: {}\nFailed: {}\n\nTotal Data: {:.1} GB\nAvg Success: {:.1}%",
            jobs.len(),
            running_jobs,
            failed_jobs,
            total_data as f64 / (1024.0 * 1024.0 * 1024.0),
            jobs.iter().map(|j| j.success_rate).sum::<f32>() / jobs.len().max(1) as f32
        );

        let stats_widget = Paragraph::new(stats_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Statistics"))
            .wrap(Wrap { trim: true });

        f.render_widget(stats_widget, chunks[1]);
    }

    fn render_backup_storage(&self, f: &mut Frame, area: Rect, storage: &[StorageLocation], alerts: &[BackupAlert]) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Storage locations
        let mut storage_items = Vec::new();
        
        for location in storage {
            let usage_percent = (location.used_space as f64 / location.total_capacity as f64) * 100.0;
            let status_icon = if location.is_accessible { "🟢" } else { "🔴" };
            
            storage_items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::raw(format!("{} {}", status_icon, location.name)),
                ]),
                Line::from(format!("  Usage: {:.1}% ({:.1}GB / {:.1}GB)", 
                    usage_percent,
                    location.used_space as f64 / (1024.0 * 1024.0 * 1024.0),
                    location.total_capacity as f64 / (1024.0 * 1024.0 * 1024.0))),
                Line::from(format!("  Type: {:?}", location.storage_type)),
            ]));
        }

        if storage_items.is_empty() {
            storage_items.push(ListItem::new("No storage locations configured"));
        }

        let storage_list = List::new(storage_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Storage Locations"));

        f.render_widget(storage_list, chunks[0]);

        // Backup alerts
        let mut alert_items = Vec::new();
        let mut critical_count = 0;

        for alert in alerts {
            if matches!(alert.severity, BackupAlertSeverity::Critical) {
                critical_count += 1;
            }

            let severity_icon = match alert.severity {
                BackupAlertSeverity::Critical => "🚨",
                BackupAlertSeverity::Error => "❌",
                BackupAlertSeverity::Warning => "⚠️",
                BackupAlertSeverity::Info => "ℹ️",
            };

            alert_items.push(ListItem::new(format!("{} {}", severity_icon, alert.message)));
        }

        if alert_items.is_empty() {
            alert_items.push(ListItem::new("No backup alerts"));
        }

        let alert_list = List::new(alert_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!("Alerts ({} critical)", critical_count)));

        f.render_widget(alert_list, chunks[1]);
    }
}