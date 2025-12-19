# 🧬 DepMap Downloader

> ⚡ High-performance Rust tool for downloading DepMap Cancer Dependency Map data

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/rainoffallingstar/depmap-downloader-rs.svg?style=social&label=Star)](https://github.com/rainoffallingstar/depmap-downloader-rs)

## 🎯 Features

- 🚀 **High Performance** - Zero-cost abstractions and efficient memory management
- ⚡ **Concurrent Downloads** - Multi-threaded parallel downloads for large files
- 💾 **Intelligent Caching** - SQLite local caching to avoid duplicate downloads
- 🔍 **Powerful Search** - Search genes, cell lines, and datasets
- 🛡️ **Type Safe** - Compile-time type safety guarantees
- 📦 **Single Executable** - No runtime dependencies, ready to run

## 🚀 Quick Start

### 📋 Prerequisites
- Rust 1.70+ ([Installation guide](https://rustup.rs/))
- Internet connection

### 🔧 Build Project

```bash
# Clone repository
git clone https://github.com/rainoffallingstar/depmap-downloader-rs.git
cd depmap-downloader-rs

# Build project
cargo build --release

# Run program
./target/release/depmap-downloader --help
```

## 💻 Usage Guide

### 🔄 Update Cache

```bash
# Update cache (automatically checks if needed)
./target/release/depmap-downloader update

# Force update
./target/release/depmap-downloader update --force

# Update specific data types
./target/release/depmap-downloader update --data-type CRISPR --data-type Expression
```

### 📋 List Data

```bash
# View all releases 📦
./target/release/depmap-downloader list releases

# View specific data types 🧬
./target/release/depmap-downloader list datasets --data-type CRISPR

# View detailed release files 📁
./target/release/depmap-downloader list files "DepMap Public 23Q4" --detailed
```

### ⬇️ Download Data

```bash
# 🆕 Download entire release
./target/release/depmap-downloader download release "DepMap Public 23Q4"

# 🆕 Download specific data types
./target/release/depmap-downloader download release "DepMap Public 23Q4" --data-type CRISPR

# 🆕 Download specific dataset
./target/release/depmap-downloader download dataset "CRISPR (DepMap Public 25Q3+Score, Chronos)"

# ⚡ High-speed download (8 workers)
./target/release/depmap-downloader download --workers 8 release "DepMap Public 25Q3"

# Skip existing files
./target/release/depmap-downloader download --skip-existing

# Verify file integrity
./target/release/depmap-downloader download --verify-checksum
```

### 🔍 Search Data

```bash
# 🆕 Search genes (supports gene names and Entrez IDs)
./target/release/depmap-downloader search TP53 -g
./target/release/depmap-downloader search 7159 -g --limit 5

# 🔍 Search cell lines
./target/release/depmap-downloader search "A549" --cell-line

# 📊 Search datasets
./target/release/depmap-downloader search "CRISPR" --dataset

# 🎯 Search all types (default behavior)
./target/release/depmap-downloader search "BRCA1"
```

### 📈 View Statistics

```bash
# Cache statistics
./target/release/depmap-downloader stats

# Detailed statistics
./target/release/depmap-downloader stats --detailed
```

## 💡 Practical Examples

### 🧬 Gene Research
```bash
# Search tumor suppressor gene TP53
./target/release/depmap-downloader search TP53 -g

# Find oncogene MYC
./target/release/depmap-downloader search MYC -g --limit 10

# Search by Entrez ID
./target/release/depmap-downloader search 672 -g  # BRCA1
```

### 📦 Data Downloads
```bash
# Download CRISPR data only (latest version)
./target/release/depmap-downloader download --skip-existing release "DepMap Public 25Q3" --data-type CRISPR

# Download multiple data types
./target/release/depmap-downloader download --workers 8 release "DepMap Public 23Q4" --data-type Expression
./target/release/depmap-downloader download --workers 8 release "DepMap Public 23Q4" --data-type Mutations
```

### 🔍 Exploratory Research
```bash
# View available releases
./target/release/depmap-downloader list releases

# Browse release files
./target/release/depmap-downloader list files "DepMap Public 23Q4" --detailed

# Selective download
./target/release/depmap-downloader download --verify-checksum release "DepMap Public 23Q4"
```

## 📊 Supported Data Types

| Data Type | Description | File Count |
|-----------|-------------|-------------|
| 🧬 **CRISPR** | CRISPR gene screening data | Multiple datasets |
| 🧪 **RNAi** | RNA interference data | Multiple datasets |
| 📈 **Expression** | Gene expression data | Multiple datasets |
| 🧬 **Mutations** | Gene mutation data | Multiple datasets |
| 📊 **CN** | Copy number variation data | Multiple datasets |
| 💊 **Drug screen** | Drug screening data | Multiple datasets |
| 🔬 **Protein** | Protein expression data | Multiple datasets |

## ⚙️ Configuration Options

```bash
# Custom database path
--database <PATH>

# Custom API URL
--api-url <URL>

# Output directory
--output <DIR>

# Worker count (default: 4)
--workers <NUM>

# Enable verbose logging
--verbose
```

## 🏗️ Project Structure

```
depmap-downloader-rs/
├── 📁 src/                    # Source code
│   ├── main.rs                # Program entry point
│   ├── cli.rs                 # CLI definitions
│   ├── commands.rs            # Command handling logic
│   ├── cache_manager.rs       # Cache manager
│   ├── downloader.rs          # File downloader
│   ├── models.rs              # Data models
│   └── error.rs               # Error handling
├── 📄 Cargo.toml               # Project configuration
├── 📝 README.md                # This documentation
└── 📂 target/                  # Build output
```

## 🧪 Performance Features

- **Memory Efficiency** 📉 - Stream large files without memory overflow
- **Download Performance** ⚡ - Configurable concurrent downloads with auto-retry
- **Database Performance** 🔍 - SQLite indexing optimization for fast queries
- **Smart Caching** 🧠 - Avoid duplicate downloads, save bandwidth

## 🔧 Development

```bash
# Clone repository
git clone https://github.com/rainoffallingstar/depmap-downloader-rs.git
cd depmap-downloader-rs

# Development build
cargo build

# Run tests
cargo test

# Code linting
cargo clippy

# Format code
cargo fmt
```

## 🆘 Troubleshooting

### Compilation Issues
```bash
# Clean cache and rebuild
cargo clean && cargo build
```

### Runtime Issues
```bash
# Check database permissions
ls -la depmap_cache.db

# Check network connection
curl -I https://depmap.org/portal/api

# View detailed logs
./target/release/depmap-downloader --verbose update
```

### Performance Issues
```bash
# Adjust worker count
./target/release/depmap-downloader download --workers 2

# Clear cache and rebuild
./target/release/depmap-downloader clear --all
```

## 📚 Related Resources

- 🌐 [DepMap Official Website](https://depmap.org)
- 📖 [DepMap API Documentation](https://depmap.org/portal/api)
- 📊 [DepMap Data Page](https://depmap.org/portal/data_page)
- 🦀 [Rust Documentation](https://doc.rust-lang.org/)

## 🤝 Contributing

Issues and Pull Requests are welcome!

1. Fork the project 🍴
2. Create feature branch (`git checkout -b feature/amazing-feature`) 🌿
3. Commit your changes (`git commit -m 'Add amazing feature'`) ✨
4. Push to branch (`git push origin feature/amazing-feature`) 📤
5. Create Pull Request 🎉

## 📄 License

This project is licensed under the MIT License - see [LICENSE](LICENSE) file for details

## 🙏 Acknowledgments

- 🧬 DepMap project for providing research data
- 🦀 Rust community for excellent tools and libraries
- 💝 All contributors and users for feedback and suggestions

---

> 💡 **Note**: This tool is developed based on the DepMap experimental API. The API may change, please pay attention to official updates
