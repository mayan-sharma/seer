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

mod config;
mod monitor;
mod ui;

use config::Config;
use monitor::SystemMonitor;
use ui::App;

#[derive(Parser)]
#[command(name = "seer")]
#[command(about = "A comprehensive CLI system monitoring tool")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[arg(short = 'r', long = "refresh-rate", default_value = "2")]
    pub refresh_rate: u64,

    #[arg(long = "show-zombies")]
    pub show_zombies: bool,

    #[arg(short = 'f', long = "filter-process")]
    pub filter_process: Option<String>,

    #[arg(short = 'e', long = "export")]
    pub export: Option<String>,

    #[arg(long = "threshold-cpu", default_value = "80")]
    pub threshold_cpu: f32,

    #[arg(long = "threshold-memory", default_value = "80")]
    pub threshold_memory: f32,
}

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
            system_monitor.update().await?;
            app.update_data(system_monitor);
            *last_update = Instant::now();
        }

        terminal.draw(|f| app.render(f))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('r') => {
                        system_monitor.update().await?;
                        app.update_data(system_monitor);
                        *last_update = Instant::now();
                    }
                    KeyCode::Char('p') => app.toggle_process_view(),
                    KeyCode::Char('n') => app.toggle_network_view(),
                    KeyCode::Char('d') => app.toggle_disk_view(),
                    KeyCode::Char('z') => app.toggle_zombie_highlighting(),
                    KeyCode::Up => app.previous_process(),
                    KeyCode::Down => app.next_process(),
                    KeyCode::Char('c') => app.sort_by_cpu(),
                    KeyCode::Char('m') => app.sort_by_memory(),
                    KeyCode::Char('k') => app.kill_selected_process()?,
                    _ => {}
                }
            }
        }

        sleep(Duration::from_millis(50)).await;
    }
}