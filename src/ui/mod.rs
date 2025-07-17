pub mod dashboard;
pub mod widgets;

use crate::config::Config;
use crate::monitor::{SystemMonitor, SystemMetrics};
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub use dashboard::*;

#[derive(Debug, Clone)]
pub enum AppView {
    Dashboard,
    ProcessList,
    NetworkView,
    DiskView,
}

#[derive(Debug, Clone)]
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
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            show_zombies_highlighted: config.show_zombies,
            config,
            current_view: AppView::Dashboard,
            selected_process_index: 0,
            sort_by: SortBy::Cpu,
            show_confirmation_dialog: false,
            system_metrics: None,
        }
    }

    pub fn update_data(&mut self, system_monitor: &SystemMonitor) {
        self.system_metrics = Some(system_monitor.get_metrics());
    }

    pub fn render(&mut self, f: &mut Frame) {
        match self.current_view {
            AppView::Dashboard => self.render_dashboard(f),
            AppView::ProcessList => self.render_process_list(f),
            AppView::NetworkView => self.render_network_view(f),
            AppView::DiskView => self.render_disk_view(f),
        }

        if self.show_confirmation_dialog {
            self.render_confirmation_dialog(f);
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

    pub fn next_process(&mut self) {
        if let Some(metrics) = &self.system_metrics {
            if !metrics.processes.is_empty() {
                self.selected_process_index = (self.selected_process_index + 1) % metrics.processes.len();
            }
        }
    }

    pub fn previous_process(&mut self) {
        if let Some(metrics) = &self.system_metrics {
            if !metrics.processes.is_empty() {
                self.selected_process_index = if self.selected_process_index == 0 {
                    metrics.processes.len() - 1
                } else {
                    self.selected_process_index - 1
                };
            }
        }
    }

    pub fn sort_by_cpu(&mut self) {
        self.sort_by = SortBy::Cpu;
    }

    pub fn sort_by_memory(&mut self) {
        self.sort_by = SortBy::Memory;
    }

    pub fn kill_selected_process(&mut self) -> Result<()> {
        self.show_confirmation_dialog = true;
        Ok(())
    }

    fn render_confirmation_dialog(&self, f: &mut Frame) {
        let size = f.size();
        let popup_area = centered_rect(50, 20, size);

        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("Confirm Kill Process")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let text = if let Some(metrics) = &self.system_metrics {
            if let Some(process) = metrics.processes.get(self.selected_process_index) {
                format!("Kill process {} (PID: {})?\nPress 'y' to confirm, any other key to cancel", 
                       process.name, process.pid)
            } else {
                "No process selected".to_string()
            }
        } else {
            "No process data available".to_string()
        };

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, popup_area);
    }

    fn get_threshold_color(&self, percentage: f32, threshold: f32) -> Color {
        if percentage >= threshold {
            Color::Red
        } else if percentage >= threshold * 0.7 {
            Color::Yellow
        } else {
            Color::Green
        }
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
            let header = Paragraph::new("Network Monitoring View")
                .style(Style::default().fg(Color::Cyan))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
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
                            format!("{}: ", iface.name), 
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                        ),
                        ratatui::text::Span::styled(
                            format!("[{}] ", status), 
                            Style::default().fg(if iface.is_up { Color::Green } else { Color::Red })
                        ),
                        ratatui::text::Span::raw(format!("{} {} | Total: ↓ {} ↑ {}", 
                            rx_rate, tx_rate,
                            crate::monitor::SystemMonitor::format_bytes(iface.bytes_received),
                            crate::monitor::SystemMonitor::format_bytes(iface.bytes_transmitted)
                        )),
                    ])
                })
                .collect();

            let interfaces_widget = Paragraph::new(interface_text)
                .block(Block::default().title("Network Interfaces").borders(Borders::ALL))
                .style(Style::default().fg(Color::White));
            f.render_widget(interfaces_widget, chunks[1]);

            // Listening ports placeholder
            let ports_text = "Listening ports feature coming soon...";
            let ports_widget = Paragraph::new(ports_text)
                .block(Block::default().title("Listening Ports").borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(ports_widget, chunks[2]);

            // Footer
            let footer = Paragraph::new("Press 'n' to return to dashboard")
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
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
            let header = Paragraph::new("Disk Usage View")
                .style(Style::default().fg(Color::Cyan))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Disk usage table
            let header_cells = ["Mount Point", "Filesystem", "Size", "Used", "Available", "Use%"]
                .iter()
                .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
            let header_row = ratatui::widgets::Row::new(header_cells).style(Style::default().bg(Color::Blue));

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
                .block(Block::default().borders(Borders::ALL).title("Disk Usage"))
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
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
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