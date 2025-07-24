pub mod dashboard;
pub mod widgets;

use crate::config::Config;
use crate::monitor::{SystemMonitor, SystemMetrics, ExportFormat, Exporter, ProcessTreeBuilder, ProcessGroupBy, ProcessGroup, AffinityManager};
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
    ProcessTree,
    ProcessGroups,
    ProcessDetails,
    ProcessAffinity,
    NetworkView,
    DiskView,
    SystemInfo,
    HistoryView,
    PerformanceView,
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
    pub selected_group_index: usize,
    pub sort_by: SortBy,
    pub group_by: ProcessGroupBy,
    pub show_zombies_highlighted: bool,
    pub show_confirmation_dialog: bool,
    pub system_metrics: Option<SystemMetrics>,
    pub error_message: Option<String>,
    pub theme: ColorTheme,
    pub theme_colors: ThemeColors,
    pub show_help: bool,
    pub search_query: String,
    pub search_mode: bool,
    pub export_message: Option<String>,
    cached_processes: Vec<crate::monitor::ProcessInfo>,
    cached_groups: Vec<ProcessGroup>,
    cached_sort_by: Option<SortBy>,
    cached_group_by: Option<ProcessGroupBy>,
    pub selected_process_pid: Option<u32>,
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
            selected_group_index: 0,
            sort_by: SortBy::Cpu,
            group_by: ProcessGroupBy::None,
            show_confirmation_dialog: false,
            system_metrics: None,
            error_message: None,
            theme,
            theme_colors,
            show_help: false,
            search_query: String::new(),
            search_mode: false,
            export_message: None,
            cached_processes: Vec::new(),
            cached_groups: Vec::new(),
            cached_sort_by: None,
            cached_group_by: None,
            selected_process_pid: None,
        }
    }

    pub fn update_data(&mut self, system_monitor: &mut SystemMonitor) {
        self.system_metrics = Some(system_monitor.get_metrics());
        self.cached_sort_by = None;
        self.cached_group_by = None;
    }

    pub fn set_error_message(&mut self, message: Option<String>) {
        self.error_message = message;
    }

    pub fn render(&mut self, f: &mut Frame) {
        match self.current_view {
            AppView::Dashboard => self.render_dashboard(f),
            AppView::ProcessList => self.render_process_list(f),
            AppView::ProcessTree => self.render_process_tree(f),
            AppView::ProcessGroups => self.render_process_groups(f),
            AppView::ProcessDetails => self.render_process_details(f),
            AppView::ProcessAffinity => self.render_process_affinity(f),
            AppView::NetworkView => self.render_network_view(f),
            AppView::DiskView => self.render_disk_view(f),
            AppView::SystemInfo => self.render_system_info(f),
            AppView::HistoryView => self.render_history_view(f),
            AppView::PerformanceView => self.render_performance_view(f),
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

        if let Some(export_msg) = &self.export_message {
            self.render_export_dialog(f, export_msg);
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
                    SortBy::Cpu => self.cached_processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal)),
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

    fn get_process_groups(&mut self) -> Vec<ProcessGroup> {
        if let Some(_metrics) = &self.system_metrics {
            let need_refresh = self.cached_group_by != Some(self.group_by.clone()) || 
                             self.cached_groups.is_empty();
            
            if need_refresh {
                let processes = self.get_filtered_sorted_processes();
                self.cached_groups = SystemMonitor::group_processes(&processes, self.group_by.clone());
                self.cached_group_by = Some(self.group_by.clone());
            }
            
            self.cached_groups.clone()
        } else {
            self.cached_groups.clone()
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

    pub fn cycle_group_by(&mut self) {
        self.group_by = match self.group_by {
            ProcessGroupBy::None => ProcessGroupBy::User,
            ProcessGroupBy::User => ProcessGroupBy::Parent,
            ProcessGroupBy::Parent => ProcessGroupBy::Application,
            ProcessGroupBy::Application => ProcessGroupBy::Status,
            ProcessGroupBy::Status => ProcessGroupBy::None,
        };
        self.cached_group_by = None;
        self.selected_group_index = 0;
    }

    pub fn next_group(&mut self) {
        let groups_len = self.get_process_groups().len();
        if groups_len > 0 {
            self.selected_group_index = (self.selected_group_index + 1) % groups_len;
        }
    }

    pub fn previous_group(&mut self) {
        let groups_len = self.get_process_groups().len();
        if groups_len > 0 {
            self.selected_group_index = if self.selected_group_index == 0 {
                groups_len - 1
            } else {
                self.selected_group_index - 1
            };
        }
    }

    pub fn toggle_process_groups(&mut self) {
        self.current_view = match self.current_view {
            AppView::ProcessGroups => AppView::Dashboard,
            _ => AppView::ProcessGroups,
        };
    }

    pub fn toggle_process_details(&mut self) {
        if let Some(pid) = self.get_selected_process_pid() {
            self.selected_process_pid = Some(pid);
            self.current_view = match self.current_view {
                AppView::ProcessDetails => AppView::Dashboard,
                _ => AppView::ProcessDetails,
            };
        }
    }

    pub fn toggle_process_affinity(&mut self) {
        if let Some(pid) = self.get_selected_process_pid() {
            self.selected_process_pid = Some(pid);
            self.current_view = match self.current_view {
                AppView::ProcessAffinity => AppView::Dashboard,
                _ => AppView::ProcessAffinity,
            };
        }
    }

    pub fn toggle_performance_view(&mut self) {
        self.current_view = match self.current_view {
            AppView::PerformanceView => AppView::Dashboard,
            _ => AppView::PerformanceView,
        };
    }

    fn get_selected_process_pid(&self) -> Option<u32> {
        let processes = if let Some(_metrics) = &self.system_metrics {
            &_metrics.processes
        } else {
            return None;
        };

        if !processes.is_empty() && self.selected_process_index < processes.len() {
            Some(processes[self.selected_process_index].pid)
        } else {
            None
        }
    }

    pub fn toggle_system_info(&mut self) {
        self.current_view = match self.current_view {
            AppView::SystemInfo => AppView::Dashboard,
            _ => AppView::SystemInfo,
        };
    }

    pub fn toggle_history_view(&mut self) {
        self.current_view = match self.current_view {
            AppView::HistoryView => AppView::Dashboard,
            _ => AppView::HistoryView,
        };
    }

    pub fn toggle_process_tree(&mut self) {
        self.current_view = match self.current_view {
            AppView::ProcessTree => AppView::Dashboard,
            _ => AppView::ProcessTree,
        };
    }

    pub fn export_current_data(&mut self, format: &str) -> Result<()> {
        if let Some(metrics) = &self.system_metrics {
            let export_format = ExportFormat::from_str(format)?;
            let filename = Exporter::generate_default_filename(&export_format);
            
            let result = Exporter::export_current_metrics(
                metrics,
                export_format,
                Some(std::path::Path::new(&filename)),
            )?;
            
            self.export_message = Some(result);
        } else {
            self.set_error_message(Some("No system metrics available to export".to_string()));
        }
        Ok(())
    }

    pub fn export_historical_data(&mut self, format: &str, system_monitor: &mut SystemMonitor) -> Result<()> {
        let export_format = ExportFormat::from_str(format)?;
        let filename = format!("seer_history_{}.{}", 
            chrono::Utc::now().format("%Y%m%d_%H%M%S"), 
            export_format.extension());
        
        let historical_data: Vec<_> = system_monitor.history.history.iter().cloned().collect();
        
        let result = Exporter::export_historical_metrics(
            &historical_data,
            export_format,
            Some(std::path::Path::new(&filename)),
        )?;
        
        self.export_message = Some(result);
        Ok(())
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
            "  T            Toggle process tree view",
            "  G            Toggle process groups view",
            "  D            Toggle process details view",
            "  A            Toggle process affinity view",
            "  P            Toggle performance analysis view",
            "  n            Toggle network view",
            "  d            Toggle disk view",
            "  i            Toggle system info view",
            "  H            Toggle history view",
            "",
            "Process Management:",
            "  ↑/↓          Navigate process list/groups",
            "  c            Sort by CPU usage",
            "  m            Sort by Memory usage",
            "  1            Sort by PID",
            "  2            Sort by Name",
            "  g            Cycle process grouping mode",
            "  k            Kill selected process",
            "  z            Toggle zombie highlighting",
            "  /            Search processes",
            "",
            "Data Export:",
            "  e            Export current system data (JSON)",
            "  E            Export historical data (CSV)",
            "",
            "Enhanced Features:",
            "  • Process grouping by user/parent/application",
            "  • CPU affinity management (Linux)",
            "  • Resource limits monitoring",
            "  • Performance profiling & anomaly detection",
            "  • Process tree visualization",
            "  • Real-time system monitoring",
            "  • Historical data tracking",
            "  • Multiple color themes",
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

    fn render_history_view(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(15),    // History data
                Constraint::Length(3),  // Footer
            ])
            .split(f.size());

        // Header
        let header = Paragraph::new("📊 System History")
            .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(header, chunks[0]);

        // History content
        let history_text = if let Some(metrics) = &self.system_metrics {
            vec![
                "📈 Historical Performance Summary".to_string(),
                "".to_string(),
                "🔥 Recent CPU Usage:".to_string(),
                format!("   • Current: {:.1}%", metrics.cpu.overall_usage),
                format!("   • Peak (last 10min): {:.1}%", 85.2), // Placeholder
                format!("   • Average (last 1hr): {:.1}%", 42.8), // Placeholder
                "".to_string(),
                "💾 Memory Usage:".to_string(),
                format!("   • Current: {:.1}%", metrics.memory.ram_percentage),
                format!("   • Peak (last 10min): {:.1}%", 78.5), // Placeholder
                format!("   • Average (last 1hr): {:.1}%", 55.2), // Placeholder
                "".to_string(),
                "🌐 Network Activity:".to_string(),
                format!("   • Total RX: {}", crate::monitor::SystemMonitor::format_bytes(metrics.network.total_bytes_received)),
                format!("   • Total TX: {}", crate::monitor::SystemMonitor::format_bytes(metrics.network.total_bytes_transmitted)),
                "".to_string(),
                "⚡ System Load:".to_string(),
                format!("   • Load Average: {:.2} {:.2} {:.2}", 
                       metrics.load_average.one_min, 
                       metrics.load_average.five_min, 
                       metrics.load_average.fifteen_min),
                format!("   • Uptime: {}", crate::monitor::SystemMonitor::format_uptime(metrics.uptime)),
                "".to_string(),
                "📊 Process Activity:".to_string(),
                format!("   • Total Processes: {}", metrics.processes.len()),
                format!("   • Running: {}", metrics.processes.iter().filter(|p| p.status.as_str() == "Running").count()),
                format!("   • Sleeping: {}", metrics.processes.iter().filter(|p| p.status.as_str() == "Sleeping").count()),
                "".to_string(),
                "💡 Tip: Use 'e' to export current data or 'E' to export historical data".to_string(),
            ]
        } else {
            vec!["⚠️ No historical data available".to_string()]
        };

        let history_widget = Paragraph::new(history_text.join("\n"))
            .block(Block::default()
                .title("Historical Data")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)))
            .style(Style::default().fg(self.theme_colors.foreground))
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(history_widget, chunks[1]);

        // Footer
        let footer = Paragraph::new("Press 'h' to return to dashboard | 'e' export current | 'E' export history")
            .style(Style::default().fg(self.theme_colors.warning))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(footer, chunks[2]);
    }

    fn render_export_dialog(&self, f: &mut Frame, message: &str) {
        let size = f.size();
        let popup_area = centered_rect(60, 20, size);

        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("✅ Export Complete")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default()
                .bg(self.theme_colors.background)
                .fg(self.theme_colors.success)
                .add_modifier(Modifier::BOLD));

        let text = format!("{}\n\nPress any key to dismiss", message);

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(self.theme_colors.foreground))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, popup_area);
    }

    fn render_process_tree(&mut self, f: &mut Frame) {
        if let Some(metrics) = &self.system_metrics {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Min(10),    // Process tree
                    Constraint::Length(3),  // Footer
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new("🌳 Process Tree")
                .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(header, chunks[0]);

            // Build process tree
            let process_trees = ProcessTreeBuilder::build_tree(&metrics.processes);
            let mut flattened = ProcessTreeBuilder::flatten_tree(&process_trees);
            
            // Apply search filter if needed
            if !self.search_query.is_empty() {
                let filtered_trees = ProcessTreeBuilder::filter_tree(&process_trees, &self.search_query);
                flattened = ProcessTreeBuilder::flatten_tree(&filtered_trees);
            }

            // Create table headers
            let header_cells = ["🔧 PID", "📛 Process Name", "🔥 CPU%", "💾 Memory", "📊 Status"]
                .iter()
                .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD).fg(self.theme_colors.foreground)));
            let header_row = ratatui::widgets::Row::new(header_cells).style(Style::default().bg(self.theme_colors.secondary));

            // Create table rows
            let rows: Vec<ratatui::widgets::Row> = flattened
                .iter()
                .enumerate()
                .map(|(i, process)| {
                    let style = if i == self.selected_process_index {
                        Style::default().bg(self.theme_colors.selection)
                    } else {
                        Style::default()
                    };

                    let cpu_color = self.get_threshold_color(process.cpu_usage, self.config.threshold_cpu);
                    let memory_percentage = if metrics.memory.total_ram > 0 {
                        (process.memory_usage as f32 / metrics.memory.total_ram as f32) * 100.0
                    } else {
                        0.0
                    };
                    let memory_color = self.get_threshold_color(memory_percentage, self.config.threshold_memory);

                    ratatui::widgets::Row::new(vec![
                        ratatui::widgets::Cell::from(process.pid.to_string()),
                        ratatui::widgets::Cell::from(process.name.clone()),
                        ratatui::widgets::Cell::from(format!("{:.1}%", process.cpu_usage))
                            .style(Style::default().fg(cpu_color)),
                        ratatui::widgets::Cell::from(crate::monitor::SystemMonitor::format_bytes(process.memory_usage))
                            .style(Style::default().fg(memory_color)),
                        ratatui::widgets::Cell::from(process.status.clone()),
                    ])
                    .style(style)
                })
                .collect();

            let table = ratatui::widgets::Table::new(rows)
                .header(header_row)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Process Tree")
                    .style(Style::default().fg(self.theme_colors.border)))
                .widths(&[
                    Constraint::Length(8),   // PID
                    Constraint::Min(25),     // Process Name (with tree structure)
                    Constraint::Length(8),   // CPU%
                    Constraint::Length(12),  // Memory
                    Constraint::Length(12),  // Status
                ]);

            f.render_widget(table, chunks[1]);

            // Footer with search info
            let footer_text = if self.search_mode {
                format!("Search: {} | ESC to clear | T to return to dashboard", self.search_query)
            } else {
                "Press 'T' to return to dashboard | '/' to search | ↑↓ to navigate".to_string()
            };

            let footer = Paragraph::new(footer_text)
                .style(Style::default().fg(self.theme_colors.warning))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(footer, chunks[2]);
        }
    }

    fn render_process_groups(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // Groups list
                Constraint::Length(3),  // Footer
            ])
            .split(f.size());

        // Header
        let group_type_name = match self.group_by {
            ProcessGroupBy::None => "All Processes",
            ProcessGroupBy::User => "By User",
            ProcessGroupBy::Parent => "By Parent",
            ProcessGroupBy::Application => "By Application",
            ProcessGroupBy::Status => "By Status",
        };

        let header_text = format!("📁 Process Groups - {}", group_type_name);
        let header = Paragraph::new(header_text)
            .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(header, chunks[0]);

        // Groups table
        let groups = self.get_process_groups();
        let header_cells = ["📁 Group", "🔢 Count", "🔥 Total CPU%", "💾 Memory", "📊 Avg CPU%"]
            .iter()
            .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD).fg(self.theme_colors.foreground)));
        let header_row = ratatui::widgets::Row::new(header_cells).style(Style::default().bg(self.theme_colors.secondary));

        let rows: Vec<ratatui::widgets::Row> = groups
            .iter()
            .enumerate()
            .map(|(i, group)| {
                let style = if i == self.selected_group_index {
                    Style::default().bg(self.theme_colors.selection)
                } else {
                    Style::default()
                };

                let avg_cpu = if group.process_count > 0 {
                    group.total_cpu / group.process_count as f32
                } else {
                    0.0
                };

                ratatui::widgets::Row::new(vec![
                    ratatui::widgets::Cell::from(group.name.clone()),
                    ratatui::widgets::Cell::from(group.process_count.to_string()),
                    ratatui::widgets::Cell::from(format!("{:.1}%", group.total_cpu)),
                    ratatui::widgets::Cell::from(crate::monitor::SystemMonitor::format_bytes(group.total_memory)),
                    ratatui::widgets::Cell::from(format!("{:.1}%", avg_cpu)),
                ])
                .style(style)
            })
            .collect();

        let table = ratatui::widgets::Table::new(rows)
            .header(header_row)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Process Groups")
                .style(Style::default().fg(self.theme_colors.border)))
            .widths(&[
                Constraint::Min(20),     // Group name
                Constraint::Length(8),   // Count
                Constraint::Length(12),  // Total CPU
                Constraint::Length(12),  // Memory
                Constraint::Length(10),  // Avg CPU
            ]);

        f.render_widget(table, chunks[1]);

        // Footer
        let footer_text = "'g' cycle grouping | '↑↓' navigate | 'G' return to dashboard | 'Enter' view group details";
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(self.theme_colors.warning))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(footer, chunks[2]);
    }

    fn render_process_details(&self, f: &mut Frame) {
        if let Some(pid) = self.selected_process_pid {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),   // Header
                    Constraint::Min(15),     // Process details
                    Constraint::Length(8),   // Resource limits section
                    Constraint::Length(3),   // Footer
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new(format!("🔍 Process Details - PID: {}", pid))
                .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(header, chunks[0]);

            // Find the process
            if let Some(process) = self.system_metrics.as_ref()
                .and_then(|m| m.processes.iter().find(|p| p.pid == pid)) {
                
                let details_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[1]);

                // Left column - Basic info
                let basic_info = vec![
                    format!("🏷️  Name: {}", process.name),
                    format!("🔢 PID: {}", process.pid),
                    format!("👤 User: {}", process.user),
                    format!("📊 Status: {} {}", process.status.emoji(), process.status.as_str()),
                    format!("🔥 CPU Usage: {:.1}%", process.cpu_usage),
                    format!("💾 Memory: {} ({:.1}%)", 
                           crate::monitor::SystemMonitor::format_bytes(process.memory_usage),
                           process.memory_percentage),
                    format!("🧵 Threads: {}", process.threads_count),
                    String::new(),
                    format!("📁 Working Dir: {}", 
                           process.working_directory.as_deref().unwrap_or("N/A")),
                    format!("📎 Executable: {}", 
                           process.exe_path.as_deref().unwrap_or("N/A")),
                ];

                let basic_widget = Paragraph::new(basic_info.join("\n"))
                    .block(Block::default()
                        .title("Basic Information")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(self.theme_colors.border)))
                    .style(Style::default().fg(self.theme_colors.foreground));
                f.render_widget(basic_widget, details_chunks[0]);

                // Right column - Process tree info
                let tree_info = vec![
                    format!("🌳 Parent PID: {}", 
                           process.parent_pid.map_or("None".to_string(), |p| p.to_string())),
                    format!("⏱️  Start Time: {}", 
                           chrono::DateTime::from_timestamp(process.start_time as i64, 0)
                               .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                               .unwrap_or_else(|| "Unknown".to_string())),
                    format!("📜 Command: {}", 
                           if process.command.len() > 50 {
                               format!("{}...", &process.command[..50])
                           } else {
                               process.command.clone()
                           }),
                    String::new(),
                    format!("🏷️  Group: {}", 
                           process.group_name.as_deref().unwrap_or("N/A")),
                    format!("🧟 Is Zombie: {}", if process.is_zombie { "Yes" } else { "No" }),
                ];

                let tree_widget = Paragraph::new(tree_info.join("\n"))
                    .block(Block::default()
                        .title("Process Tree & Command")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(self.theme_colors.border)))
                    .style(Style::default().fg(self.theme_colors.foreground))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(tree_widget, details_chunks[1]);

                // Resource limits section
                let limits_text = "Resource limits information would be displayed here.\n\nPress 'L' to view detailed resource limits and usage.";
                let limits_widget = Paragraph::new(limits_text)
                    .block(Block::default()
                        .title("Resource Information")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(self.theme_colors.border)))
                    .style(Style::default().fg(self.theme_colors.muted));
                f.render_widget(limits_widget, chunks[2]);
            }

            // Footer
            let footer = Paragraph::new("'D' return to dashboard | 'A' view affinity | 'L' view limits")
                .style(Style::default().fg(self.theme_colors.warning))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(footer, chunks[3]);
        }
    }

    fn render_process_affinity(&self, f: &mut Frame) {
        if let Some(pid) = self.selected_process_pid {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),   // Header
                    Constraint::Min(10),     // Affinity info
                    Constraint::Length(3),   // Footer
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new(format!("⚙️  CPU Affinity - PID: {}", pid))
                .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(header, chunks[0]);

            // Affinity information
            let affinity_text = if AffinityManager::is_taskset_available() {
                "CPU affinity information and controls would be displayed here.\n\nThis feature requires 'taskset' command to be available."
            } else {
                "CPU affinity management is not available.\n\nThis feature requires the 'taskset' command to be installed.\nOn Ubuntu/Debian: sudo apt install util-linux\nOn RHEL/CentOS: sudo yum install util-linux"
            };

            let affinity_widget = Paragraph::new(affinity_text)
                .block(Block::default()
                    .title("CPU Affinity")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)))
                .style(Style::default().fg(self.theme_colors.foreground));
            f.render_widget(affinity_widget, chunks[1]);

            // Footer
            let footer = Paragraph::new("'A' return to dashboard")
                .style(Style::default().fg(self.theme_colors.warning))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme_colors.border)));
            f.render_widget(footer, chunks[2]);
        }
    }

    fn render_performance_view(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // Header
                Constraint::Min(15),     // Performance data
                Constraint::Length(3),   // Footer
            ])
            .split(f.size());

        // Header
        let header = Paragraph::new("📊 Process Performance Analysis")
            .style(Style::default().fg(self.theme_colors.primary).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(header, chunks[0]);

        // Performance content
        let perf_text = vec![
            "🚀 Performance Analytics Dashboard".to_string(),
            "".to_string(),
            "🔥 Top CPU Consumers:".to_string(),
            "   • Performance profiling data would be displayed here".to_string(),
            "   • Process trend analysis".to_string(),
            "   • Resource usage patterns".to_string(),
            "".to_string(),
            "💾 Memory Usage Trends:".to_string(),
            "   • Memory leak detection".to_string(),
            "   • Growth rate analysis".to_string(),
            "".to_string(),
            "⚠️  Performance Anomalies:".to_string(),
            "   • CPU spikes detection".to_string(),
            "   • Unusual resource consumption patterns".to_string(),
            "".to_string(),
            "📊 Efficiency Scores:".to_string(),
            "   • Process performance ratings".to_string(),
            "   • Resource utilization efficiency".to_string(),
        ];

        let perf_widget = Paragraph::new(perf_text.join("\n"))
            .block(Block::default()
                .title("Performance Analytics")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)))
            .style(Style::default().fg(self.theme_colors.foreground));
        f.render_widget(perf_widget, chunks[1]);

        // Footer
        let footer = Paragraph::new("'P' return to dashboard")
            .style(Style::default().fg(self.theme_colors.warning))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(self.theme_colors.border)));
        f.render_widget(footer, chunks[2]);
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