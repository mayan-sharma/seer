# Seer

A comprehensive CLI system monitoring tool built in Rust that provides real-time system information in a terminal user interface.

## Features

- **Real-time System Monitoring**: CPU, memory, disk, and network usage
- **Process Management**: View running processes with sorting and filtering capabilities
- **Interactive TUI**: Built with Ratatui for a responsive terminal interface
- **Customizable Thresholds**: Set CPU and memory usage alerts
- **Zombie Process Detection**: Highlight and filter zombie processes
- **Export Functionality**: Export system data to various formats
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
- `-e, --export <FORMAT>`: Export data to specified format
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
| `n` | Toggle network view |
| `d` | Toggle disk view |
| `i` | Toggle system info |
| `z` | Toggle zombie highlighting |
| `h` or `?` | Toggle help |
| `t` | Cycle themes |
| `↑/↓` | Navigate process list |
| `c` | Sort by CPU usage |
| `m` | Sort by memory usage |
| `k` | Kill selected process |

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

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.