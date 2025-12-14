# Contributing to System Monitor

Thank you for your interest in contributing to System Monitor! This document provides guidelines and instructions for contributing.

## 🤝 How to Contribute

### Reporting Bugs

If you find a bug, please create an issue with:
- **Clear title** describing the problem
- **Steps to reproduce** the issue
- **Expected behavior** vs actual behavior
- **System information** (OS version, Rust version, GPU if applicable)
- **Screenshots** if relevant

### Suggesting Features

Feature requests are welcome! Please:
- **Check existing issues** first to avoid duplicates
- **Describe the feature** clearly and in detail
- **Explain use cases** - why is this feature useful?
- **Consider implementation** - any thoughts on how it could work?

### Pull Requests

1. **Fork the repository**
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Make your changes**
4. **Test thoroughly**
5. **Commit with clear messages** (`git commit -m 'Add amazing feature'`)
6. **Push to your fork** (`git push origin feature/amazing-feature`)
7. **Open a Pull Request**

#### Pull Request Guidelines

- Follow existing code style
- Add tests if applicable
- Update documentation
- Keep commits atomic and well-described
- Reference related issues

## 💻 Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# On Windows, download from:
# https://rustup.rs/
```

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/system-monitor.git
cd system-monitor

# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run the application
cargo run
```

### Project Structure

```
system-monitor/
├── src/
│   └── main.rs          # Main application code
├── Cargo.toml           # Dependencies and metadata
├── build.ps1            # Build script
├── install.ps1          # Installation script
└── docs/                # Documentation files
```

## 🎨 Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Run clippy for lints (`cargo clippy`)
- Keep functions focused and well-named
- Add comments for complex logic
- Use meaningful variable names

## 📝 Commit Messages

Use clear, descriptive commit messages:

```
feat: add network monitoring tab
fix: resolve GPU temperature reading issue
docs: update installation guide
refactor: improve settings persistence
test: add unit tests for disk monitoring
```

Prefixes:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `perf:` - Performance improvements
- `chore:` - Maintenance tasks

## 🧪 Testing

- Test on clean Windows installation if possible
- Verify all tabs work correctly
- Check theme switching
- Test settings persistence
- Monitor for memory leaks
- Verify GPU detection (if available)

## 📚 Documentation

When adding features:
- Update relevant .md files
- Add examples if applicable
- Update screenshots if UI changed
- Keep USER_GUIDE.md current

## 🐛 Debugging

Common issues and solutions:

**Build fails:**
```bash
cargo clean
cargo update
cargo build --release
```

**GPU not detected:**
- Only NVIDIA GPUs supported via NVML
- Ensure NVIDIA drivers are installed
- Check Device Manager

**Settings not saving:**
- Check %LOCALAPPDATA%\SystemMonitor\config\
- Verify write permissions
- Look for error messages in console

## 🔧 Areas for Contribution

### High Priority
- [ ] Export data to JSON/CSV
- [ ] Process management (kill processes)
- [ ] System tray icon
- [ ] Complete notification system

### Medium Priority
- [ ] Network usage graphs
- [ ] Disk I/O monitoring
- [ ] Auto-start with Windows
- [ ] Multiple profiles

### Future Ideas
- [ ] Cross-platform support (Linux, macOS)
- [ ] Web dashboard
- [ ] Remote monitoring
- [ ] Plugin system

## 📞 Getting Help

- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions
- **Documentation**: Check the docs/ folder

## ⚖️ License

By contributing, you agree that your contributions will be licensed under the MIT License.

## 🙏 Acknowledgments

Thanks to all contributors who help make System Monitor better!

---

**Happy Contributing!** 🎉
