pub mod dashboard;
pub mod widgets;

use crate::config::Config;
use crate::monitor::{SystemMonitor, SystemMetrics};
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, BorderType},
    Frame,
};

pub use dashboard::*;

#[derive(Debug, Clone)]
pub enum AppView {
    Dashboard,
    ProcessList,
    NetworkView,
    DiskView,
    SystemInfo,
}

#[derive(Debug, Clone)]
pub enum ColorTheme {
    Default,
    Dark,
    Gruvbox,
    Dracula,
    Solarized,
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub muted: Color,
    pub border: Color,
    pub selection: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

pub struct App {
    pub config: Config,
    pub current_view: AppView,
    pub selected_process_index: usize,
    pub sort_by: SortBy,
    pub show_zombies_highlighted: bool,
    pub show_confirmation_dialog: bool,
    pub system_metrics: Option<SystemMetrics>,
    pub error_message: Option<String>,
    pub theme: ColorTheme,
    pub theme_colors: ThemeColors,
    pub show_help: bool,
    pub search_query: String,
    pub search_mode: bool,
    cached_processes: Vec<crate::monitor::ProcessInfo>,
    cached_sort_by: Option<SortBy>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let theme = ColorTheme::Default;
        let theme_colors = Self::get_theme_colors(&theme);
        
        Self {
            show_zombies_highlighted: config.show_zombies,
            config,
            current_view: AppView::Dashboard,
            selected_process_index: 0,
            sort_by: SortBy::Cpu,
            show_confirmation_dialog: false,
            system_metrics: None,
            error_message: None,
            theme,
            theme_colors,
            show_help: false,
            search_query: String::new(),
            search_mode: false,
            cached_processes: Vec::new(),
            cached_sort_by: None,
        }
    }

    pub fn update_data(&mut self, system_monitor: &SystemMonitor) {
        self.system_metrics = Some(system_monitor.get_metrics());
        self.cached_sort_by = None;
    }

    pub fn set_error_message(&mut self, message: Option<String>) {
        self.error_message = message;
    }

    pub fn render(&mut self, f: &mut Frame) {
        match self.current_view {
            AppView::Dashboard => self.render_dashboard(f),
            AppView::ProcessList => self.render_process_list(f),
            AppView::NetworkView => self.render_network_view(f),
            AppView::DiskView => self.render_disk_view(f),
            AppView::SystemInfo => self.render_system_info(f),
        }

        if self.show_confirmation_dialog {
            self.render_confirmation_dialog(f);
        }

        if let Some(error) = &self.error_message {
            self.render_error_dialog(f, error);
        }

        if self.show_help {
            self.render_help_dialog(f);
        }
    }

    pub fn toggle_process_view(&mut self) {
        self.current_view = match self.current_view {
            AppView::Dashboard => AppView::ProcessList,
            AppView::ProcessList => AppView::Dashboard,
            _ => AppView::Dashboard,
        };
    }

    pub fn toggle_network_view(&mut self) {
        self.current_view = match self.current_view {
            AppView::NetworkView => AppView::Dashboard,
            _ => AppView::NetworkView,
        };
    }

    pub fn toggle_disk_view(&mut self) {
        self.current_view = match self.current_view {
            AppView::DiskView => AppView::Dashboard,
            _ => AppView::DiskView,
        };
    }

    pub fn toggle_zombie_highlighting(&mut self) {
        self.show_zombies_highlighted = !self.show_zombies_highlighted;
    }

    fn get_filtered_sorted_processes(&mut self) -> Vec<crate::monitor::ProcessInfo> {
        if let Some(metrics) = &self.system_metrics {
            let need_refresh = self.cached_sort_by != Some(self.sort_by.clone()) || 
                             self.cached_processes.is_empty();
            
            if need_refresh {
                self.cached_processes = metrics.processes.clone();
                
                // Apply filtering
                if let Some(filter) = &self.config.filter_process {
                    self.cached_processes.retain(|p| p.name.contains(filter));
                }
                
                // Apply search filtering
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    self.cached_processes.retain(|p| p.name.to_lowercase().contains(&query));
                }
                
                // Apply sorting
                match self.sort_by {
                    SortBy::Cpu => self.cached_processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap()),
                    SortBy::Memory => self.cached_processes.sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage)),
                    SortBy::Pid => self.cached_processes.sort_by(|a, b| a.pid.cmp(&b.pid)),
                    SortBy::Name => self.cached_processes.sort_by(|a, b| a.name.cmp(&b.name)),
                }
                
                self.cached_sort_by = Some(self.sort_by.clone());
            }
            
            self.cached_processes.clone()
        } else {
            self.cached_processes.clone()
        }
    }

    pub fn next_process(&mut self) {
        let processes_len = self.get_filtered_sorted_processes().len();
        if processes_len > 0 {
            self.selected_process_index = (self.selected_process_index + 1) % processes_len;
        }
    }

    pub fn previous_process(&mut self) {
        let processes_len = self.get_filtered_sorted_processes().len();
        if processes_len > 0 {
            self.selected_process_index = if self.selected_process_index == 0 {
                processes_len - 1
            } else {
                self.selected_process_index - 1
            };
        }
    }

    pub fn sort_by_cpu(&mut self) {
        self.sort_by = SortBy::Cpu;
        self.cached_sort_by = None;
    }

    pub fn sort_by_memory(&mut self) {
        self.sort_by = SortBy::Memory;
        self.cached_sort_by = None;
    }

    pub fn sort_by_pid(&mut self) {
        self.sort_by = SortBy::Pid;
        self.cached_sort_by = None;
    }

    pub fn sort_by_name(&mut self) {
        self.sort_by = SortBy::Name;
        self.cached_sort_by = None;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn toggle_search(&mut self) {
        self.search_mode = !self.search_mode;
        if !self.search_mode {
            self.search_query.clear();
            self.cached_sort_by = None;
        }
    }

    pub fn add_search_char(&mut self, c: char) {
        if self.search_mode {
            self.search_query.push(c);
            self.cached_sort_by = None;
        }
    }

    pub fn backspace_search(&mut self) {
        if self.search_mode && !self.search_query.is_empty() {
            self.search_query.pop();
            self.cached_sort_by = None;
        }
    }

    pub fn cycle_theme(&mut self) {
        self.theme = match self.theme {
            ColorTheme::Default => ColorTheme::Dark,
            ColorTheme::Dark => ColorTheme::Gruvbox,
            ColorTheme::Gruvbox => ColorTheme::Dracula,
            ColorTheme::Dracula => ColorTheme::Solarized,
            ColorTheme::Solarized => ColorTheme::Default,
        };
        self.theme_colors = Self::get_theme_colors(&self.theme);
    }

    pub fn toggle_system_info(&mut self) {
        self.current_view = match self.current_view {
            AppView::SystemInfo => AppView::Dashboard,
            _ => AppView::SystemInfo,
        };
    }

    pub fn get_theme_colors(theme: &ColorTheme) -> ThemeColors {
        match theme {
            ColorTheme::Default => ThemeColors {
                background: Color::Black,
                foreground: Color::White,
                primary: Color::Cyan,
                secondary: Color::Blue,
                accent: Color::Magenta,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                muted: Color::Gray,
                border: Color::White,
                selection: Color::DarkGray,
            },
            ColorTheme::Dark => ThemeColors {
                background: Color::Rgb(40, 44, 52),
                foreground: Color::Rgb(171, 178, 191),
                primary: Color::Rgb(97, 175, 239),
                secondary: Color::Rgb(152, 195, 121),
                accent: Color::Rgb(209, 154, 102),
                success: Color::Rgb(152, 195, 121),
                warning: Color::Rgb(229, 192, 123),
                error: Color::Rgb(224, 108, 117),
                info: Color::Rgb(97, 175, 239),
                muted: Color::Rgb(92, 99, 112),
                border: Color::Rgb(92, 99, 112),
                selection: Color::Rgb(61, 66, 77),
            },
            ColorTheme::Gruvbox => ThemeColors {
                background: Color::Rgb(40, 40, 40),
                foreground: Color::Rgb(235, 219, 178),
                primary: Color::Rgb(131, 165, 152),
                secondary: Color::Rgb(142, 192, 124),
                accent: Color::Rgb(211, 134, 155),
                success: Color::Rgb(142, 192, 124),
                warning: Color::Rgb(250, 189, 47),
                error: Color::Rgb(251, 73, 52),
                info: Color::Rgb(131, 165, 152),
                muted: Color::Rgb(168, 153, 132),
                border: Color::Rgb(80, 73, 69),
                selection: Color::Rgb(60, 56, 54),
            },
            ColorTheme::Dracula => ThemeColors {
                background: Color::Rgb(40, 42, 54),
                foreground: Color::Rgb(248, 248, 242),
                primary: Color::Rgb(139, 233, 253),
                secondary: Color::Rgb(80, 250, 123),
                accent: Color::Rgb(255, 121, 198),
                success: Color::Rgb(80, 250, 123),
                warning: Color::Rgb(241, 250, 140),
                error: Color::Rgb(255, 85, 85),
                info: Color::Rgb(139, 233, 253),
                muted: Color::Rgb(98, 114, 164),
                border: Color::Rgb(68, 71, 90),
                selection: Color::Rgb(68, 71, 90),
            },
            ColorTheme::Solarized => ThemeColors {
                background: Color::Rgb(0, 43, 54),
                foreground: Color::Rgb(131, 148, 150),
                primary: Color::Rgb(42, 161, 152),
                secondary: Color::Rgb(133, 153, 0),
                accent: Color::Rgb(108, 113, 196),
                success: Color::Rgb(133, 153, 0),
                warning: Color::Rgb(181, 137, 0),
                error: Color::Rgb(220, 50, 47),
                info: Color::Rgb(42, 161, 152),
                muted: Color::Rgb(101, 123, 131),
                border: Color::Rgb(7, 54, 66),
                selection: Color::Rgb(7, 54, 66),
            },
        }
    }

    pub fn kill_selected_process(&mut self) -> Result<()> {
        let processes = self.get_filtered_sorted_processes();
        if !processes.is_empty() && self.selected_process_index < processes.len() {
            let process_pid = processes[self.selected_process_index].pid;
            // Check if we can kill the process
            if process_pid == std::process::id() {
                self.set_error_message(Some("Cannot kill the monitoring process itself".to_string()));
                return Ok(());
            }
            
            // Show confirmation dialog
            self.show_confirmation_dialog = true;
        } else {
            self.set_error_message(Some("No process selected".to_string()));
        }
        Ok(())
    }

    fn render_confirmation_dialog(&mut self, f: &mut Frame) {
        let size = f.size();
        let popup_area = centered_rect(50, 20, size);

        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("⚠️  Kill Process")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default()
                .bg(self.theme_colors.background)
                .fg(self.theme_colors.warning)
                .add_modifier(Modifier::BOLD));

        let processes = self.get_filtered_sorted_processes();
        let text = if let Some(process) = processes.get(self.selected_process_index) {
            format!("Kill process {} (PID: {})?\n\nPress 'y' to confirm, any other key to cancel", 
                   process.name, process.pid)
        } else {
            "No process selected".to_string()
        };

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(self.theme_colors.foreground))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, popup_area);
    }

    fn render_error_dialog(&self, f: &mut Frame, error: &str) {
        let size = f.size();
        let popup_area = centered_rect(60, 25, size);

        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("❌ Error")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default()
                .bg(self.theme_colors.background)
                .fg(self.theme_colors.error)
                .add_modifier(Modifier::BOLD));

        let text = format!("{}\n\nPress any key to dismiss", error);

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(self.theme_colors.foreground))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, popup_area);
    }

    fn get_threshold_color(&self, percentage: f32, threshold: f32) -> Color {
        if percentage >= threshold {
            self.theme_colors.error
        } else if percentage >= threshold * 0.7 {
            self.theme_colors.warning
        } else {
            self.theme_colors.success
        }
    }

    fn render_help_dialog(&self, f: &mut Frame) {
        let size = f.size();
        let popup_area = centered_rect(80, 80, size);

        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("💡 Help - Seer System Monitor")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default()
                .bg(self.theme_colors.background)
                .fg(self.theme_colors.primary)
                .add_modifier(Modifier::BOLD));

        let help_text = vec![
            "Navigation:",
            "  q            Quit application",
            "  h/?          Toggle this help screen",
            "  r            Manually refresh data",
            "  t            Cycle through color themes",
            "",
            "Views:",
            "  p            Toggle process list view",
            "  n            Toggle network view",
            "  d            Toggle disk view",
            "  i            Toggle system info view",
            "",
            "Process Management:",
            "  ↑/↓          Navigate process list",
            "  c            Sort by CPU usage",
            "  m            Sort by Memory usage",
            "  1            Sort by PID",
            "  2            Sort by Name",
            "  k            Kill selected process",
            "  z            Toggle zombie highlighting",
            "  /            Search processes",
            "",
            "Features:",
            "  • Real-time system monitoring",
            "  • Multiple color themes",
            "  • Process filtering and sorting",
            "  • Network interface monitoring",
            "  • Disk usage tracking",
            "  • System information display",
            "",
            "Press any key to close this help screen",
        ];

        let paragraph = Paragraph::new(help_text.join("\n"))
            .block(block)
            .style(Style::default().fg(self.theme_colors.foreground))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, popup_area);
    }

    fn render_system_info(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // System details
                Constraint::Length(3),  // Footer
            ])
            .split(f.size());

        // Header
        let header = Paragraph::new("🖥️  System Information")
            .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(header, chunks[0]);

        // System info content
        if let Some(metrics) = &self.system_metrics {
            let system_info = crate::monitor::SystemMonitor::new().get_system_info();
            
            let info_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            // Left column - System details
            let system_details = vec![
                format!("🏷️  Hostname: {}", system_info.hostname),
                format!("🐧 OS: {}", system_info.os_version),
                format!("🔧 Architecture: {}", system_info.architecture),
                format!("⚙️  Kernel: {}", system_info.kernel_version),
                format!("🧠 CPU Cores: {}", system_info.cpu_count),
                format!("⏱️  Uptime: {}", crate::monitor::SystemMonitor::format_uptime(metrics.uptime)),
                format!("📊 Load Average: {:.2} {:.2} {:.2}", 
                       metrics.load_average.one_min, 
                       metrics.load_average.five_min, 
                       metrics.load_average.fifteen_min),
            ];

            let system_widget = Paragraph::new(system_details.join("\n"))
                .block(Block::default()
                    .title("System Details")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .style(Style::default().fg(self.theme_colors.foreground));
            f.render_widget(system_widget, info_chunks[0]);

            // Right column - Memory & Network summary
            let memory_details = vec![
                format!("💾 Total RAM: {}", crate::monitor::SystemMonitor::format_bytes(metrics.memory.total_ram)),
                format!("📈 Used RAM: {} ({:.1}%)", 
                       crate::monitor::SystemMonitor::format_bytes(metrics.memory.used_ram),
                       metrics.memory.ram_percentage),
                format!("📉 Available RAM: {}", crate::monitor::SystemMonitor::format_bytes(metrics.memory.available_ram)),
                String::new(),
                format!("🔄 Total Swap: {}", crate::monitor::SystemMonitor::format_bytes(metrics.memory.total_swap)),
                format!("📊 Used Swap: {} ({:.1}%)", 
                       crate::monitor::SystemMonitor::format_bytes(metrics.memory.used_swap),
                       metrics.memory.swap_percentage),
                String::new(),
                format!("🌐 Active Network Interfaces: {}", 
                       metrics.network.interfaces.iter().filter(|i| i.is_up).count()),
                format!("📤 Total TX: {}", crate::monitor::SystemMonitor::format_bytes(metrics.network.total_bytes_transmitted)),
                format!("📥 Total RX: {}", crate::monitor::SystemMonitor::format_bytes(metrics.network.total_bytes_received)),
            ];

            let memory_widget = Paragraph::new(memory_details.join("\n"))
                .block(Block::default()
                    .title("Memory & Network")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .style(Style::default().fg(self.theme_colors.foreground));
            f.render_widget(memory_widget, info_chunks[1]);
        }

        // Footer
        let footer = Paragraph::new("Press 'i' to return to dashboard")
            .style(Style::default().fg(self.theme_colors.warning))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(footer, chunks[2]);
    }

    pub fn render_process_list(&mut self, f: &mut Frame) {
        // For now, just render the same as dashboard
        self.render_dashboard(f);
    }

    pub fn render_network_view(&mut self, f: &mut Frame) {
        if let Some(metrics) = &self.system_metrics {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Min(10),    // Network details
                    Constraint::Length(8),  // Port list
                    Constraint::Length(3),  // Footer
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new("🌐 Network Monitoring")
                .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(header, chunks[0]);

            // Network interfaces
            let interface_text: Vec<ratatui::text::Line> = metrics.network.interfaces
                .iter()
                .map(|iface| {
                    let status = if iface.is_up { "UP" } else { "DOWN" };
                    let rx_rate = if iface.bytes_received_per_sec > 0.0 {
                        format!("↓ {:.1} KB/s", iface.bytes_received_per_sec / 1024.0)
                    } else {
                        "↓ 0 KB/s".to_string()
                    };
                    let tx_rate = if iface.bytes_transmitted_per_sec > 0.0 {
                        format!("↑ {:.1} KB/s", iface.bytes_transmitted_per_sec / 1024.0)
                    } else {
                        "↑ 0 KB/s".to_string()
                    };
                    
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            format!("📡 {}: ", iface.name), 
                            Style::default().fg(self.theme_colors.foreground).add_modifier(Modifier::BOLD)
                        ),
                        ratatui::text::Span::styled(
                            format!("[{}] ", status), 
                            Style::default().fg(if iface.is_up { self.theme_colors.success } else { self.theme_colors.error })
                        ),
                        ratatui::text::Span::styled(
                            format!("{} {} | Total: ↓ {} ↑ {}", 
                                rx_rate, tx_rate,
                                crate::monitor::SystemMonitor::format_bytes(iface.bytes_received),
                                crate::monitor::SystemMonitor::format_bytes(iface.bytes_transmitted)
                            ),
                            Style::default().fg(self.theme_colors.muted)
                        ),
                    ])
                })
                .collect();

            let interfaces_widget = Paragraph::new(interface_text)
                .block(Block::default()
                    .title("Network Interfaces")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .style(Style::default().fg(self.theme_colors.foreground));
            f.render_widget(interfaces_widget, chunks[1]);

            // Listening ports table
            let system_monitor = crate::monitor::SystemMonitor::new();
            let listening_ports = system_monitor.get_listening_ports();
            
            if !listening_ports.is_empty() {
                let header_cells = ["🔌 Port", "📡 Protocol", "🏷️ Service", "📊 PID", "📛 Process"]
                    .iter()
                    .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD).fg(self.theme_colors.foreground)));
                let header_row = ratatui::widgets::Row::new(header_cells).style(Style::default().bg(self.theme_colors.secondary));

                let port_rows: Vec<ratatui::widgets::Row> = listening_ports
                    .iter()
                    .take(5)
                    .map(|port| {
                        ratatui::widgets::Row::new(vec![
                            ratatui::widgets::Cell::from(port.port.to_string()),
                            ratatui::widgets::Cell::from(port.protocol.clone()),
                            ratatui::widgets::Cell::from(port.service_name.clone().unwrap_or_else(|| "Unknown".to_string())),
                            ratatui::widgets::Cell::from(port.pid.map_or_else(|| "-".to_string(), |p| p.to_string())),
                            ratatui::widgets::Cell::from(port.process_name.clone().unwrap_or_else(|| "-".to_string())),
                        ])
                    })
                    .collect();

                let ports_table = ratatui::widgets::Table::new(port_rows)
                    .header(header_row)
                    .block(Block::default()
                        .title("Listening Ports")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(self.theme_colors.border)))
                    .widths(&[
                        Constraint::Length(8),   // Port
                        Constraint::Length(10),  // Protocol
                        Constraint::Length(12),  // Service
                        Constraint::Length(8),   // PID
                        Constraint::Min(15),     // Process
                    ]);

                f.render_widget(ports_table, chunks[2]);
            } else {
                let ports_text = "🔌 No listening ports found or insufficient permissions";
                let ports_widget = Paragraph::new(ports_text)
                    .block(Block::default()
                        .title("Listening Ports")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(self.theme_colors.border)))
                    .style(Style::default().fg(self.theme_colors.muted));
                f.render_widget(ports_widget, chunks[2]);
            }

            // Footer
            let footer = Paragraph::new("Press 'n' to return to dashboard")
                .style(Style::default().fg(self.theme_colors.warning))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(footer, chunks[3]);
        }
    }

    pub fn render_disk_view(&mut self, f: &mut Frame) {
        if let Some(metrics) = &self.system_metrics {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Min(10),    // Disk list
                    Constraint::Length(3),  // Footer
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new("💾 Disk Usage")
                .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(header, chunks[0]);

            // Disk usage table
            let header_cells = ["📁 Mount Point", "📂 Filesystem", "📏 Size", "📊 Used", "📈 Available", "📉 Use%"]
                .iter()
                .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD).fg(self.theme_colors.foreground)));
            let header_row = ratatui::widgets::Row::new(header_cells).style(Style::default().bg(self.theme_colors.secondary));

            let rows: Vec<ratatui::widgets::Row> = metrics.storage
                .iter()
                .map(|disk| {
                    let usage_color = self.get_threshold_color(disk.usage_percentage, 90.0);
                    ratatui::widgets::Row::new(vec![
                        ratatui::widgets::Cell::from(disk.mount_point.clone()),
                        ratatui::widgets::Cell::from(disk.file_system.clone()),
                        ratatui::widgets::Cell::from(crate::monitor::SystemMonitor::format_bytes(disk.total_space)),
                        ratatui::widgets::Cell::from(crate::monitor::SystemMonitor::format_bytes(disk.used_space)),
                        ratatui::widgets::Cell::from(crate::monitor::SystemMonitor::format_bytes(disk.available_space)),
                        ratatui::widgets::Cell::from(format!("{:.1}%", disk.usage_percentage))
                            .style(Style::default().fg(usage_color)),
                    ])
                })
                .collect();

            let table = ratatui::widgets::Table::new(rows)
                .header(header_row)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("💽 Disk Usage")
                    .style(Style::default().fg(self.theme_colors.border)))
                .widths(&[
                    Constraint::Min(15),     // Mount Point
                    Constraint::Length(10),  // Filesystem
                    Constraint::Length(10),  // Size
                    Constraint::Length(10),  // Used
                    Constraint::Length(12),  // Available
                    Constraint::Length(6),   // Use%
                ]);

            f.render_widget(table, chunks[1]);

            // Footer
            let footer = Paragraph::new("Press 'd' to return to dashboard")
                .style(Style::default().fg(self.theme_colors.warning))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(footer, chunks[2]);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}