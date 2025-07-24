# Seer

A comprehensive CLI system monitoring tool built in Rust that provides real-time system information in a terminal user interface.

## Features

- **Real-time System Monitoring**: CPU, memory, disk, and network usage
- **Process Management**: View running processes with sorting and filtering capabilities
- **Process Tree View**: Hierarchical display of process relationships
- **Historical Data Tracking**: Track system metrics over time with 24-hour history
- **Interactive TUI**: Built with Ratatui for a responsive terminal interface
- **Data Export**: Export current metrics and historical data to JSON, CSV, or TOML formats
- **Multiple Views**: Dashboard, Process List, Process Tree, Network, Disk, System Info, and History
- **Customizable Thresholds**: Set CPU and memory usage alerts
- **Zombie Process Detection**: Highlight and filter zombie processes
- **Network Monitoring**: Interface statistics and listening ports display
- **Search Functionality**: Search and filter processes by name
- **Multiple Themes**: 5 built-in color themes (Default, Dark, Gruvbox, Dracula, Solarized)
- **Keyboard Navigation**: Intuitive controls for navigating data

## Installation

### Prerequisites

- Rust 1.70 or later
- Cargo package manager

### Build from Source

```bash
git clone <repository-url>
cd seer
cargo build --release
```

The binary will be available at `target/release/seer`.

## Usage

### Basic Usage

```bash
seer
```

### Command Line Options

- `-r, --refresh-rate <SECONDS>`: Set refresh rate (default: 2 seconds)
- `--show-zombies`: Show zombie processes
- `-f, --filter-process <NAME>`: Filter processes by name
- `-e, --export <FORMAT>`: Export data to specified format (json, csv, toml)
- `--threshold-cpu <PERCENTAGE>`: CPU usage threshold (default: 80%)
- `--threshold-memory <PERCENTAGE>`: Memory usage threshold (default: 80%)

### Examples

```bash
# Set refresh rate to 1 second
seer -r 1

# Show zombie processes with CPU threshold of 90%
seer --show-zombies --threshold-cpu 90

# Filter processes containing "rust"
seer -f rust

# Export system data
seer -e json
```

## Controls

| Key | Action |
|-----|--------|
| `q` | Quit the application |
| `r` | Manual refresh |
| `p` | Toggle process view |
| `T` | Toggle process tree view |
| `n` | Toggle network view |
| `d` | Toggle disk view |
| `i` | Toggle system info |
| `H` | Toggle history view |
| `z` | Toggle zombie highlighting |
| `h` or `?` | Toggle help |
| `t` | Cycle themes |
| `↑/↓` | Navigate process list |
| `c` | Sort by CPU usage |
| `m` | Sort by memory usage |
| `1` | Sort by PID |
| `2` | Sort by Name |
| `k` | Kill selected process |
| `/` | Search processes |
| `e` | Export current data (JSON) |
| `E` | Export historical data (CSV) |

## Configuration

Seer supports configuration through a TOML file located at `~/.config/seer/config.toml`. The configuration file allows you to set default values for various options.

Example configuration:

```toml
refresh_rate = 2
show_zombies = false
threshold_cpu = 80.0
threshold_memory = 80.0
```

## Dependencies

- **sysinfo**: System information gathering
- **ratatui**: Terminal user interface framework
- **crossterm**: Cross-platform terminal manipulation
- **tokio**: Async runtime
- **clap**: Command line argument parsing
- **chrono**: Date and time handling
- **serde**: Serialization framework
- **serde_json**: JSON serialization
- **csv**: CSV file handling
- **toml**: TOML configuration format

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.