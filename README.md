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
- **Enhanced Process Management**: 
  - Process grouping by user, parent process, application, or status
  - CPU affinity viewing and management (Linux)
  - Resource limits monitoring and display
  - Process performance profiling with anomaly detection
- **Security & Monitoring Enhancements**:
  - **Security Dashboard**: Real-time security threat monitoring and suspicious process detection
  - **Log Monitoring**: System log analysis with security pattern detection and alerts
  - **Filesystem Monitoring**: Critical file and directory integrity monitoring with change detection
  - **Behavioral Analysis**: Process behavior profiling and anomaly detection
  - **Privilege Escalation Detection**: Monitor and alert on unauthorized privilege changes
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

### Navigation & General
| Key | Action |
|-----|--------|
| `q` | Quit the application |
| `r` | Manual refresh |
| `h` or `?` | Toggle help |
| `t` | Cycle themes |

### System Views
| Key | Action |
|-----|--------|
| `p` | Toggle process view |
| `T` | Toggle process tree view |
| `n` | Toggle network view |
| `d` | Toggle disk view |
| `i` | Toggle system info |
| `H` | Toggle history view |

### Advanced Monitoring Domains
| Key | Action |
|-----|--------|
| `Tab` | Navigate advanced monitoring tabs |
| `←` / `→` | Switch between Database/APM/IoT/Backup tabs |
| `J` | Toggle database monitoring |
| `Y` | Toggle APM (Application Performance) |
| `O` | Toggle IoT device monitoring |
| `B` | Toggle backup & recovery monitoring |

### System Analysis & Performance
| Key | Action |
|-----|--------|
| `A` | Toggle process affinity view |
| `P` | Toggle performance analysis view |
| `M` | Toggle memory leak detection |
| `I` | Toggle I/O bottleneck analysis |
| `R` | Toggle thermal monitoring |
| `N` | Toggle dependency analysis |
| `U` | Toggle GPU monitoring |

### Security & Infrastructure Monitoring
| Key | Action |
|-----|--------|
| `S` | Toggle security dashboard |
| `L` | Toggle log monitoring view |
| `F` | Toggle filesystem monitoring view |
| `C` | Toggle container monitoring view |
| `V` | Toggle service monitoring view |
| `X` | Toggle user session monitoring view |
| `W` | Toggle hardware sensor monitoring view |

### Process Management
| Key | Action |
|-----|--------|
| `↑/↓` | Navigate process list |
| `c` | Sort by CPU usage |
| `m` | Sort by memory usage |
| `1` | Sort by PID |
| `2` | Sort by Name |
| `k` | Kill selected process |
| `/` | Search processes |
| `z` | Toggle zombie highlighting |
| `G` | Toggle process groups view |
| `D` | Toggle process details view |
| `g` | Cycle process grouping mode |

### Data Export
| Key | Action |
|-----|--------|
| `e` | Export current data (JSON) |
| `E` | Export historical data (CSV) |

## Advanced Monitoring Domains

Seer includes comprehensive monitoring capabilities across multiple domains:

### 📊 Database Monitoring
Monitor popular database systems with detailed performance metrics:
- **MySQL**: Connection pools, query performance, InnoDB statistics, replication status
- **PostgreSQL**: Database statistics, lock metrics, cache hit ratios, transaction throughput
- **MongoDB**: Operations metrics, memory usage, replication lag, sharding status
- **Redis**: Memory utilization, keyspace statistics, persistence info, replication metrics

### 🚀 Application Performance Monitoring (APM)
Real-time application monitoring with automatic language detection:
- **JVM Applications**: Heap/non-heap memory, garbage collection metrics, thread analysis
- **.NET Applications**: Managed memory, GC pressure, exception tracking, performance counters
- **Python Applications**: Memory usage, thread counts, module loading, GC collections
- **Node.js Applications**: V8 heap metrics, event loop lag, active handles/requests
- **Go Applications**: Heap allocation, GC cycles, goroutine tracking, CGO calls
- **Anomaly Detection**: Automatic performance issue detection with severity classification

### 🌐 IoT Device Monitoring
Comprehensive IoT and smart device discovery and management:
- **Device Discovery**: ARP scanning, UPnP discovery, mDNS/Bonjour, Bluetooth scanning
- **Protocol Support**: WiFi, Bluetooth, Zigbee, Z-Wave, LoRaWAN, MQTT, CoAP, HTTP/HTTPS
- **Device Classification**: Smart phones, TVs, speakers, cameras, thermostats, sensors
- **Health Monitoring**: Connectivity status, battery levels, temperature, signal strength
- **Network Analysis**: Protocol statistics, device communication patterns

### 💾 Backup & Recovery Monitoring
Monitor backup systems and storage infrastructure:
- **Job Discovery**: Automatic detection of rsync processes, cron backup jobs, systemd services
- **Backup Types**: Full, incremental, differential, snapshot, database, VM backups
- **Storage Monitoring**: Capacity tracking, accessibility checks, read/write performance
- **Success Tracking**: Success rates, retention policy compliance, recovery time estimates
- **Alert System**: Failed jobs, storage capacity warnings, integrity check failures

## Configuration

Seer supports configuration through a TOML file located at `~/.config/seer/config.toml`. The configuration file allows you to set default values for various options.

### Basic Configuration

```toml
refresh_rate = 2
show_zombies = false
threshold_cpu = 80.0
threshold_memory = 80.0
```

### Advanced Monitoring Configuration

```toml
[advanced_monitoring]

[advanced_monitoring.database]
enabled = true
mysql_enabled = true
mysql_host = "localhost"
mysql_port = 3306
mysql_user = "monitoring_user"
postgresql_enabled = true
postgresql_host = "localhost"
postgresql_port = 5432
postgresql_user = "postgres"
postgresql_database = "postgres"
mongodb_enabled = false
redis_enabled = true
redis_host = "localhost"
redis_port = 6379

[advanced_monitoring.iot]
enabled = true
discovery_interval_seconds = 300
health_check_interval_seconds = 60
network_ranges = ["192.168.1.0/24", "10.0.0.0/8"]
enable_bluetooth_scan = true
enable_zigbee_scan = false
enable_upnp_discovery = true
enable_mdns_discovery = true
device_timeout_seconds = 300

[advanced_monitoring.backup]
enabled = true
monitor_interval_seconds = 300
backup_directories = ["/var/backups", "/backup", "/home/backups"]
storage_locations = ["/mnt/backup", "/backup"]
log_file_paths = ["/var/log/backup.log", "/var/log/rsync.log"]
enable_performance_monitoring = true
integrity_check_interval_seconds = 3600

[advanced_monitoring.apm]
enabled = true
monitor_jvm = true
monitor_dotnet = true
monitor_python = true
monitor_nodejs = true
monitor_golang = true
anomaly_detection_enabled = true
```

## Enhanced Process Management Features

### Process Grouping
View and analyze processes organized by:
- **User**: Group processes by the user running them
- **Parent Process**: Group processes by their parent PID
- **Application**: Group processes by application/executable name
- **Status**: Group processes by their current status (Running, Sleeping, etc.)

### CPU Affinity Management (Linux)
- View current CPU affinity settings for processes
- Modify CPU affinity using taskset integration
- Display CPU topology information
- Manage which CPU cores processes can run on

### Resource Limits Monitoring
- Display process resource limits (ulimits)
- Monitor resource usage against limits
- Show warnings when processes approach limits
- Track memory, file descriptors, CPU time, and other resources

### Performance Profiling
- Real-time performance tracking for all processes
- CPU and memory usage trend analysis
- Anomaly detection (CPU spikes, memory leaks)
- Process efficiency scoring
- Historical performance data with statistics

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
- **num_cpus**: CPU topology detection

## Platform Support

- **Linux**: Full feature support including CPU affinity management and resource limits
- **macOS/Windows**: Core monitoring features supported, some advanced features may be limited

### Linux-Specific Features
- CPU affinity viewing and modification (requires `taskset` utility)
- Resource limits monitoring via `/proc` filesystem
- Enhanced process information from `/proc/pid/` files

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.