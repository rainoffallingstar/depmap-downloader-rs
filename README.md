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
./target/release/depdown --help
```

### 🛠️ Installation

You can run the application in any directory without pre-existing setup:

```bash
# Download pre-built binary (when available) or build from source
./depdown update  # Automatically creates database on first run
```

The application automatically handles:
- ✅ Database creation and initialization
- ✅ WAL mode error recovery  
- ✅ Directory creation for cache files
- ✅ Migration and schema updates

## 💻 Usage Guide

### 🔄 Update Cache

```bash
# Update cache (automatically checks if needed)
./target/release/depdown update

# Force update
./target/release/depdown update --force

# Update specific data types
./target/release/depdown update --data-type CRISPR --data-type Expression
```

### 📋 List Data

```bash
# View all releases 📦
./target/release/depdown list releases

# View specific data types 🧬
./target/release/depdown list datasets --data-type CRISPR

# View detailed release files 📁
./target/release/depdown list files "DepMap Public 23Q4" --detailed
```

### ⬇️ Download Data

```bash
# 🆕 Download entire release
./target/release/depdown download release "DepMap Public 23Q4"

# 🆕 Download specific data types
./target/release/depdown download release "DepMap Public 23Q4" --data-type CRISPR

# 🆕 Download specific dataset
./target/release/depdown download dataset "CRISPR (DepMap Public 25Q3+Score, Chronos)"

# ⚡ High-speed download (8 workers)
./target/release/depdown download --workers 8 release "DepMap Public 25Q3"

# Skip existing files
./target/release/depdown download --skip-existing

# Verify file integrity
./target/release/depdown download --verify-checksum
```

### 🔍 Search Data

```bash
# 🆕 Search genes (supports gene names and Entrez IDs)
./target/release/depdown search TP53 -g
./target/release/depdown search 7159 -g --limit 5

# 🔍 Search cell lines
./target/release/depdown search "A549" --cell-line

# 📊 Search datasets
./target/release/depdown search "CRISPR" --dataset

# 🎯 Search all types (default behavior)
./target/release/depdown search "BRCA1"
```

### 📈 View Statistics

```bash
# Cache statistics
./target/release/depdown stats

# Detailed statistics
./target/release/depdown stats --detailed
```

## 💡 Practical Examples

### 🧬 Gene Research
```bash
# Search tumor suppressor gene TP53
./target/release/depdown search TP53 -g

# Find oncogene MYC
./target/release/depdown search MYC -g --limit 10

# Search by Entrez ID
./target/release/depdown search 672 -g  # BRCA1
```

### 📦 Data Downloads
```bash
# Download CRISPR data only (latest version)
./target/release/depdown download --skip-existing release "DepMap Public 25Q3" --data-type CRISPR

# Download multiple data types
./target/release/depdown download --workers 8 release "DepMap Public 23Q4" --data-type Expression
./target/release/depdown download --workers 8 release "DepMap Public 23Q4" --data-type Mutations
```

### 🔍 Exploratory Research
```bash
# View available releases
./target/release/depdown list releases

# Browse release files
./target/release/depdown list files "DepMap Public 23Q4" --detailed

# Selective download
./target/release/depdown download --verify-checksum release "DepMap Public 23Q4"
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

## 🔧 Troubleshooting

### Database Connection Issues

If you encounter database connection errors, the application will automatically attempt recovery. Common issues:

#### **"unable to open database file" Error**
This occurs when SQLite WAL files are missing or corrupted. The application automatically:

1. ✅ Detects WAL file corruption
2. ✅ Switches to DELETE mode to recover
3. ✅ Re-establishes database connection
4. ✅ Restores normal operation

#### **"Database file does not exist" in New Directories**
The application automatically creates databases in new directories:

```bash
# Works in any directory - no setup required!
mkdir -p /tmp/depmap-workspace && cd /tmp/depmap-workspace
./depdown update  # Creates database automatically
```

#### Manual Recovery (If Needed)

If automatic recovery fails:

```bash
# Switch to DELETE mode manually
sqlite3 depmap_cache.db "PRAGMA journal_mode = DELETE;"

# Verify database integrity
sqlite3 depmap_cache.db ".tables"
```

### Performance Tips

- Use `--data-type` filters to update specific data types only
- Increase `--workers` for faster downloads on good connections  
- Use `--force` only when needed to avoid unnecessary API calls

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
./target/release/depdown --verbose update
```

### Performance Issues
```bash
# Adjust worker count
./target/release/depdown download --workers 2

# Clear cache and rebuild
./target/release/depdown clear --all
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
