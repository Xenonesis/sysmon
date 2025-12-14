# System Monitor - Rust Edition

A real-time system monitoring application built with Rust that displays CPU, RAM, and GPU usage with a beautiful terminal interface.

## Features

- ✅ Real-time CPU usage monitoring
- ✅ Memory (RAM) usage with detailed breakdown
- ✅ GPU monitoring (NVIDIA GPUs via NVML)
  - GPU utilization percentage
  - GPU memory usage
  - GPU temperature
- ✅ Top 15 processes by memory consumption
- ✅ Color-coded progress bars
- ✅ Auto-refresh every 2 seconds
- ✅ Clean, beautiful terminal UI

## Prerequisites

- Rust (1.70 or later)
- Windows OS (for GPU monitoring)
- NVIDIA GPU with drivers installed (optional, for GPU stats)

## Installation

1. Navigate to the project directory:
```powershell
cd system-monitor-rust
```

2. Build the project:
```powershell
cargo build --release
```

3. Run the monitor:
```powershell
cargo run --release
```

## Usage

Simply run the application and it will display:
- Real-time system statistics
- Memory usage with visual progress bars
- CPU utilization
- GPU stats (if NVIDIA GPU detected)
- Top 15 memory-consuming processes

Press `Ctrl+C` to exit.

## Color Coding

- 🟢 **Green**: < 50% usage (healthy)
- 🟡 **Yellow**: 50-75% usage (moderate)
- 🔴 **Red**: > 75% usage (high)

## Building Standalone Executable

To create a standalone executable:

```powershell
cargo build --release
```

The executable will be at: `target/release/system-monitor.exe`

## Dependencies

- `sysinfo` - Cross-platform system information
- `nvml-wrapper` - NVIDIA GPU monitoring
- `chrono` - Timestamp formatting
- `colored` - Terminal colors
- `crossterm` - Terminal manipulation

## Troubleshooting

**GPU stats not showing?**
- Ensure you have an NVIDIA GPU
- Make sure NVIDIA drivers are installed
- The app will gracefully fall back if GPU monitoring isn't available

**High CPU usage from monitor itself?**
- This is normal for real-time monitoring
- Adjust refresh rate in code if needed (change `Duration::from_secs(2)`)

## Future Enhancements

- [ ] Export logs to file
- [ ] Alert system for high usage
- [ ] Historical graphs
- [ ] Network usage monitoring
- [ ] Disk I/O statistics
- [ ] Process kill functionality
- [ ] Web dashboard interface

## License

MIT License - Feel free to modify and use as needed!
