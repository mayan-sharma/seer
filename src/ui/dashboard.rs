use crate::ui::App;
use crate::monitor::SystemMonitor;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Gauge, Paragraph, Row, Table, BorderType,
    },
    Frame,
};

impl App {
    pub fn render_dashboard(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // Main content
                Constraint::Length(3),  // Footer
            ])
            .split(f.size());

        self.render_header(f, chunks[0]);
        self.render_main_content(f, chunks[1]);
        self.render_footer(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
            .split(area);

        if let Some(metrics) = &self.system_metrics {
            let uptime = SystemMonitor::format_uptime(metrics.uptime);
            
            // Left section - App name and version
            let app_info = Paragraph::new("👁️  Seer v0.1.0")
                .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Left)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(app_info, chunks[0]);

            // Center section - System status
            let system_status = format!("⏱️  Uptime: {}", uptime);
            let status_widget = Paragraph::new(system_status)
                .style(Style::default().fg(self.theme_colors.info))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(status_widget, chunks[1]);

            // Right section - Load average
            let load_avg = format!("📊 Load: {:.2} {:.2} {:.2}", 
                                 metrics.load_average.one_min, 
                                 metrics.load_average.five_min, 
                                 metrics.load_average.fifteen_min);
            let load_widget = Paragraph::new(load_avg)
                .style(Style::default().fg(self.theme_colors.accent))
                .alignment(Alignment::Right)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(load_widget, chunks[2]);
        } else {
            let loading = Paragraph::new("👁️  Seer - System Monitor | Loading...")
                .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(loading, area);
        }
    }

    fn render_main_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // CPU and Memory
                Constraint::Min(8),     // Processes
                Constraint::Length(4),  // Network summary
            ])
            .split(area);

        self.render_system_metrics(f, chunks[0]);
        self.render_process_table(f, chunks[1]);
        self.render_network_summary(f, chunks[2]);
    }

    fn render_system_metrics(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_cpu_widget(f, chunks[0]);
        self.render_memory_widget(f, chunks[1]);
    }

    fn render_cpu_widget(&self, f: &mut Frame, area: Rect) {
        if let Some(metrics) = &self.system_metrics {
            let cpu_usage = metrics.cpu.overall_usage;
            let color = self.get_threshold_color(cpu_usage, self.config.threshold_cpu);

            let gauge = Gauge::default()
                .block(Block::default()
                    .title("🖥️  CPU Usage")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .gauge_style(Style::default().fg(color))
                .percent(cpu_usage as u16)
                .label(format!("{:.1}%", cpu_usage));

            f.render_widget(gauge, area);

            if area.height > 4 {
                let core_area = Rect {
                    x: area.x + 1,
                    y: area.y + 3,
                    width: area.width - 2,
                    height: area.height - 4,
                };

                let core_text: Vec<Line> = metrics.cpu.per_core_usage
                    .iter()
                    .enumerate()
                    .take((core_area.height as usize).saturating_sub(1))
                    .map(|(i, usage)| {
                        let bar_length = (usage / 100.0 * 12.0) as usize;
                        let bar = "▰".repeat(bar_length);
                        let empty_bar = "▱".repeat(12 - bar_length);
                        Line::from(vec![
                            Span::styled(format!("Core {:2}: ", i), Style::default().fg(self.theme_colors.muted)),
                            Span::styled(bar, Style::default().fg(color)),
                            Span::styled(empty_bar, Style::default().fg(self.theme_colors.muted)),
                            Span::styled(format!(" {:.1}%", usage), Style::default().fg(self.theme_colors.foreground)),
                        ])
                    })
                    .collect();

                let paragraph = Paragraph::new(core_text);
                f.render_widget(paragraph, core_area);
            }
        } else {
            let gauge = Gauge::default()
                .block(Block::default()
                    .title("🖥️  CPU Usage")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .gauge_style(Style::default().fg(self.theme_colors.muted))
                .percent(0)
                .label("Loading...");

            f.render_widget(gauge, area);
        }
    }

    fn render_memory_widget(&self, f: &mut Frame, area: Rect) {
        if let Some(metrics) = &self.system_metrics {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3)])
                .split(area);

            let ram_color = self.get_threshold_color(metrics.memory.ram_percentage, self.config.threshold_memory);
            let ram_gauge = Gauge::default()
                .block(Block::default()
                    .title("🧠 RAM Usage")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .gauge_style(Style::default().fg(ram_color))
                .percent(metrics.memory.ram_percentage as u16)
                .label(format!(
                    "{:.1}% ({}/{})",
                    metrics.memory.ram_percentage,
                    SystemMonitor::format_bytes(metrics.memory.used_ram),
                    SystemMonitor::format_bytes(metrics.memory.total_ram)
                ));

            f.render_widget(ram_gauge, chunks[0]);

            if metrics.memory.total_swap > 0 {
                let swap_color = self.get_threshold_color(metrics.memory.swap_percentage, self.config.threshold_memory);
                let swap_gauge = Gauge::default()
                    .block(Block::default()
                        .title("💾 Swap Usage")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(self.theme_colors.border)))
                    .gauge_style(Style::default().fg(swap_color))
                    .percent(metrics.memory.swap_percentage as u16)
                    .label(format!(
                        "{:.1}% ({}/{})",
                        metrics.memory.swap_percentage,
                        SystemMonitor::format_bytes(metrics.memory.used_swap),
                        SystemMonitor::format_bytes(metrics.memory.total_swap)
                    ));

                f.render_widget(swap_gauge, chunks[1]);
            } else {
                let no_swap = Paragraph::new("💾 No swap configured")
                    .block(Block::default()
                        .title("💾 Swap Usage")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(self.theme_colors.border)))
                    .style(Style::default().fg(self.theme_colors.muted))
                    .alignment(Alignment::Center);

                f.render_widget(no_swap, chunks[1]);
            }
        } else {
            let gauge = Gauge::default()
                .block(Block::default()
                    .title("🧠 Memory Usage")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .gauge_style(Style::default().fg(self.theme_colors.muted))
                .percent(0)
                .label("Loading...");

            f.render_widget(gauge, area);
        }
    }

    fn render_process_table(&mut self, f: &mut Frame, area: Rect) {
        if let Some(metrics) = &self.system_metrics {
            let mut processes = metrics.processes.clone();
            
            if let Some(filter) = &self.config.filter_process {
                processes.retain(|p| p.name.contains(filter));
            }

            match self.sort_by {
                crate::ui::SortBy::Cpu => processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap()),
                crate::ui::SortBy::Memory => processes.sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage)),
                crate::ui::SortBy::Pid => processes.sort_by(|a, b| a.pid.cmp(&b.pid)),
                crate::ui::SortBy::Name => processes.sort_by(|a, b| a.name.cmp(&b.name)),
            }

            let zombie_count = processes.iter().filter(|p| p.is_zombie).count();
            
            let sort_indicator = match self.sort_by {
                crate::ui::SortBy::Cpu => "🔥 CPU",
                crate::ui::SortBy::Memory => "🧠 Memory",
                crate::ui::SortBy::Pid => "🔢 PID",
                crate::ui::SortBy::Name => "📛 Name",
            };

            let title = format!(
                "🔍 Top Processes {} - {} total, {} zombies {} | Sort: {}",
                if self.show_zombies_highlighted { "(⚠️ Zombies highlighted)" } else { "" },
                processes.len(),
                zombie_count,
                if zombie_count > 0 { "⚠️" } else { "" },
                sort_indicator
            );

            let header_cells = ["🆔 PID", "📛 Name", "🔥 CPU%", "🧠 MEM%", "💾 Memory", "👤 User", "📊 Status"]
                .iter()
                .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD).fg(self.theme_colors.foreground)));
            let header = Row::new(header_cells).style(Style::default().bg(self.theme_colors.secondary));

            let rows: Vec<Row> = processes
                .iter()
                .take(area.height.saturating_sub(3) as usize)
                .enumerate()
                .map(|(i, process)| {
                    let style = if i == self.selected_process_index {
                        Style::default().bg(self.theme_colors.selection).add_modifier(Modifier::BOLD)
                    } else if process.is_zombie && self.show_zombies_highlighted {
                        Style::default().fg(self.theme_colors.error).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(self.theme_colors.foreground)
                    };

                    let status_display = if process.is_zombie {
                        format!("{} ⚠️", process.status.as_str())
                    } else {
                        process.status.as_str().to_string()
                    };

                    let cpu_color = if process.cpu_usage > 80.0 {
                        self.theme_colors.error
                    } else if process.cpu_usage > 50.0 {
                        self.theme_colors.warning
                    } else {
                        self.theme_colors.foreground
                    };

                    let memory_color = if process.memory_percentage > 80.0 {
                        self.theme_colors.error
                    } else if process.memory_percentage > 50.0 {
                        self.theme_colors.warning
                    } else {
                        self.theme_colors.foreground
                    };

                    Row::new(vec![
                        Cell::from(process.pid.to_string()),
                        Cell::from(process.name.clone()),
                        Cell::from(format!("{:.1}", process.cpu_usage)).style(Style::default().fg(cpu_color)),
                        Cell::from(format!("{:.1}", process.memory_percentage)).style(Style::default().fg(memory_color)),
                        Cell::from(SystemMonitor::format_bytes(process.memory_usage)),
                        Cell::from(process.user.clone()),
                        Cell::from(status_display),
                    ]).style(style)
                })
                .collect();

            let table = Table::new(rows)
                .header(header)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(title)
                    .style(Style::default().fg(self.theme_colors.border)))
                .widths(&[
                    Constraint::Length(8),   // PID
                    Constraint::Min(15),     // Name
                    Constraint::Length(8),   // CPU%
                    Constraint::Length(8),   // MEM%
                    Constraint::Length(10),  // Memory
                    Constraint::Length(10),  // User
                    Constraint::Min(10),     // Status
                ]);

            f.render_widget(table, area);
        } else {
            let placeholder = Paragraph::new("🔄 Loading process information...")
                .block(Block::default()
                    .title("🔍 Processes")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .style(Style::default().fg(self.theme_colors.muted))
                .alignment(Alignment::Center);

            f.render_widget(placeholder, area);
        }
    }

    fn render_network_summary(&self, f: &mut Frame, area: Rect) {
        if let Some(metrics) = &self.system_metrics {
            let active_interfaces: Vec<&str> = metrics.network.interfaces
                .iter()
                .filter(|iface| iface.is_up && iface.name != "lo")
                .map(|iface| iface.name.as_str())
                .collect();

            let summary_text = if active_interfaces.is_empty() {
                "🔌 No active network interfaces".to_string()
            } else {
                let total_rx = SystemMonitor::format_bytes(metrics.network.total_bytes_received);
                let total_tx = SystemMonitor::format_bytes(metrics.network.total_bytes_transmitted);
                format!(
                    "🌐 Active: {} | Total: 📥 {} 📤 {} | Press 'n' for details",
                    active_interfaces.join(", "),
                    total_rx,
                    total_tx
                )
            };

            let paragraph = Paragraph::new(summary_text)
                .block(Block::default()
                    .title("🌐 Network Summary")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .style(Style::default().fg(self.theme_colors.info))
                .alignment(Alignment::Center);

            f.render_widget(paragraph, area);
        } else {
            let placeholder = Paragraph::new("🔄 Loading network information...")
                .block(Block::default()
                    .title("🌐 Network Summary")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .style(Style::default().fg(self.theme_colors.muted))
                .alignment(Alignment::Center);

            f.render_widget(placeholder, area);
        }
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let theme_name = match self.theme {
            crate::ui::ColorTheme::Default => "Default",
            crate::ui::ColorTheme::Dark => "Dark",
            crate::ui::ColorTheme::Gruvbox => "Gruvbox",
            crate::ui::ColorTheme::Dracula => "Dracula",
            crate::ui::ColorTheme::Solarized => "Solarized",
        };

        let footer_text = format!(
            "🎯 q:Quit | 🔄 r:Refresh | 📊 p:Processes | 🌐 n:Network | 💾 d:Disk | 🖥️ i:SysInfo | 🎨 t:Theme({}) | 💡 h:Help | ⚠️ z:Zombies | ⬆️⬇️:Navigate | 🔥 c:CPU | 🧠 m:Memory | ⚡ k:Kill",
            theme_name
        );
        
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(self.theme_colors.warning))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));

        f.render_widget(footer, area);
    }
}