use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use seer::{Cli, config::Config, monitor::SystemMonitor, ui::App};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::new(cli)?;
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config.clone());
    let mut system_monitor = SystemMonitor::new();
    
    let refresh_duration = Duration::from_secs(config.refresh_rate);
    let mut last_update = Instant::now();

    let result = run_app(&mut terminal, &mut app, &mut system_monitor, refresh_duration, &mut last_update).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    system_monitor: &mut SystemMonitor,
    refresh_duration: Duration,
    last_update: &mut Instant,
) -> Result<()> {
    loop {
        if last_update.elapsed() >= refresh_duration {
            match system_monitor.update().await {
                Ok(_) => {
                    app.update_data(system_monitor);
                    app.set_error_message(None);
                }
                Err(e) => {
                    app.set_error_message(Some(format!("System update failed: {}", e)));
                }
            }
            *last_update = Instant::now();
        }

        terminal.draw(|f| app.render(f, system_monitor))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Clear error message on any key press
                if app.error_message.is_some() {
                    app.set_error_message(None);
                    continue;
                }
                
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('r') => {
                        match system_monitor.update().await {
                            Ok(_) => {
                                app.update_data(system_monitor);
                                app.set_error_message(None);
                            }
                            Err(e) => {
                                app.set_error_message(Some(format!("Manual refresh failed: {}", e)));
                            }
                        }
                        *last_update = Instant::now();
                    }
                    KeyCode::Char('p') => app.toggle_process_view(),
                    KeyCode::Char('n') => app.toggle_network_view(),
                    KeyCode::Char('d') => app.toggle_disk_view(),
                    KeyCode::Char('i') => app.toggle_system_info(),
                    KeyCode::Char('H') => app.toggle_history_view(),
                    KeyCode::Char('T') => app.toggle_process_tree(),
                    KeyCode::Char('G') => app.toggle_process_groups(),
                    KeyCode::Char('D') => app.toggle_process_details(),
                    KeyCode::Char('A') => app.toggle_process_affinity(),
                    KeyCode::Char('P') => app.toggle_performance_view(),
                    KeyCode::Char('M') => app.toggle_memory_leak_view(),
                    KeyCode::Char('I') => app.toggle_io_analysis_view(),
                    KeyCode::Char('R') => app.toggle_thermal_view(),
                    KeyCode::Char('N') => app.toggle_dependency_view(),
                    KeyCode::Char('U') => app.toggle_gpu_monitor_view(),
                    KeyCode::Char('S') => app.toggle_security_dashboard(),
                    KeyCode::Char('L') => app.toggle_log_monitor_view(),
                    KeyCode::Char('F') => app.toggle_filesystem_monitor_view(),
                    KeyCode::Char('C') => app.toggle_container_view(),
                    KeyCode::Char('V') => app.toggle_service_view(),
                    KeyCode::Char('X') => app.toggle_session_view(),
                    KeyCode::Char('W') => app.toggle_hardware_sensor_view(),
                    KeyCode::Char('g') => app.cycle_group_by(),
                    KeyCode::Char('z') => app.toggle_zombie_highlighting(),
                    KeyCode::Char('h') | KeyCode::Char('?') => app.toggle_help(),
                    KeyCode::Char('t') => app.cycle_theme(),
                    KeyCode::Char('/') => app.toggle_search(),
                    KeyCode::Up => {
                        match app.current_view {
                            seer::ui::AppView::ProcessGroups => app.previous_group(),
                            _ => app.previous_process(),
                        }
                    },
                    KeyCode::Down => {
                        match app.current_view {
                            seer::ui::AppView::ProcessGroups => app.next_group(),
                            _ => app.next_process(),
                        }
                    },
                    KeyCode::Char('c') => app.sort_by_cpu(),
                    KeyCode::Char('m') => app.sort_by_memory(),
                    KeyCode::Char('1') => app.sort_by_pid(),
                    KeyCode::Char('2') => app.sort_by_name(),
                    KeyCode::Char('k') => app.kill_selected_process()?,
                    KeyCode::Char('e') => {
                        if let Err(e) = app.export_current_data("json") {
                            app.set_error_message(Some(format!("Export failed: {}", e)));
                        }
                    }
                    KeyCode::Char('E') => {
                        if let Err(e) = app.export_historical_data("csv", system_monitor) {
                            app.set_error_message(Some(format!("Export failed: {}", e)));
                        }
                    }
                    KeyCode::Esc => {
                        if app.search_mode {
                            app.toggle_search();
                        }
                        if app.export_message.is_some() {
                            app.export_message = None;
                        }
                    }
                    KeyCode::Backspace => app.backspace_search(),
                    KeyCode::Char(c) => {
                        if app.search_mode {
                            app.add_search_char(c);
                        }
                    }
                    _ => {}
                }
            }
        }

        sleep(Duration::from_millis(50)).await;
    }
}