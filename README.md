# DepMap Downloader (Rust)

A high-performance Rust tool for downloading DepMap (Cancer Dependency Map) data. Supports downloading cancer dependency datasets from the official API with concurrent downloads, progress display, and local caching.

## Overview

The Cancer Dependency Map (DepMap) is a research project led by the Broad Institute that systematically identifies genes and molecular pathways that cancer cells depend on for survival. This Rust implementation provides a high-performance way to programmatically access DepMap data with support for:

- 🚀 **High Performance**: Zero-cost abstractions and efficient memory management from Rust
- 🔄 **Concurrent Downloads**: Multi-threaded concurrent downloads with large file support
- 📊 **Intelligent Caching**: SQLite local caching to avoid duplicate downloads
- 🎯 **Precise Search**: Fuzzy search support for cell lines and datasets
- 🛡️ **Type Safety**: Compile-time type safety guarantees
- 📦 **Single Executable**: No runtime dependencies, easy deployment

## Quick Start

### Prerequisites

- Rust 1.70+ (recommended to use [rustup](https://rustup.rs/))
- Internet connection

### Building the Project

```bash
# Clone the repository
git clone https://github.com/yourusername/depmap-downloader-rs.git
cd depmap-downloader-rs

# Build the project
cargo build --release

# Run the program
./target/release/depmap-downloader --help
```

### Development Mode

```bash
# Development build (faster)
cargo build

# Run the development version
./target/debug/depmap-downloader --help
```

## Usage

### Command Line Interface

#### 1. Update Local Cache

```bash
# Update cache (automatically checks if update is needed)
./target/release/depmap-downloader update

# Force update
./target/release/depmap-downloader update --force

# Update specific data types only
./target/release/depmap-downloader update --data-type CRISPR --data-type Expression
```

#### 2. List Available Data

```bash
# List overview of available data
./target/release/depmap-downloader list

# List all releases
./target/release/depmap-downloader list releases

# List all releases with detailed information
./target/release/depmap-downloader list releases --detailed

# List datasets by type
./target/release/depmap-downloader list datasets --data-type CRISPR

# List datasets with detailed information
./target/release/depmap-downloader list datasets --detailed

# List files in a specific release
./target/release/depmap-downloader list files "DepMap Public 23Q4"
```

#### 3. Download Data

```bash
# Download current release core files
./target/release/depmap-downloader download

# NEW: Download all files from a specific release
./target/release/depmap-downloader download release "DepMap Public 23Q4"

# NEW: Download specific data types from a release
./target/release/depmap-downloader download release "DepMap Public 23Q4" --data-type CRISPR

# NEW: Download all files for a specific dataset
./target/release/depmap-downloader download dataset "CRISPR (DepMap Public 25Q3+Score, Chronos)"

# Legacy: Download specific dataset by ID
./target/release/depmap-downloader download --dataset Chronos_Combined

# Legacy: Download specific file by name
./target/release/depmap-downloader download --file "CRISPRGeneEffect.csv"

# Custom output directory and worker count
./target/release/depmap-downloader download --output ./my_data --workers 8

# Skip existing files
./target/release/depmap-downloader download --skip-existing

# Enable checksum verification
./target/release/depmap-downloader download --verify-checksum
```

#### 4. Search Data

```bash
# Search genes (NEW!)
./target/release/depmap-downloader search "TP53" --gene

# Search by Entrez ID
./target/release/depmap-downloader search "7159" --gene

# Search cell lines
./target/release/depmap-downloader search "A549" --cell-line

# Search datasets
./target/release/depmap-downloader search "CRISPR" --dataset

# Search all content (default: genes, cell lines, and datasets)
./target/release/depmap-downloader search "BRCA1"

# Limit search results
./target/release/depmap-downloader search "cancer" --limit 20
```

#### Gene Search Results
Gene search displays:
- Gene name and Entrez ID
- Dataset source
- Number of dependent cell lines
- Cell lines with data available
- Essentiality status (Common Essential, Strongly Selective, or Non-essential)

#### 5. Cache Statistics

```bash
# Show basic statistics
./target/release/depmap-downloader stats

# Show detailed statistics
./target/release/depmap-downloader stats --detailed
```

#### 6. Clear Cache

```bash
# Clear all cache
./target/release/depmap-downloader clear --all

# Clear specific data type cache
./target/release/depmap-downloader clear --data-type CRISPR
```

### Examples

#### Download CRISPR Data from Latest Release
```bash
# Download only CRISPR files from the latest release
./target/release/depmap-downloader download --skip-existing release "DepMap Public 25Q3" --data-type CRISPR
```

#### Download Multiple Data Types
```bash
# Download Expression and Mutation data from a specific release
./target/release/depmap-downloader download --workers 8 release "DepMap Public 23Q4" --data-type Expression
./target/release/depmap-downloader download --workers 8 release "DepMap Public 23Q4" --data-type Mutations
```

#### Browse and Download
```bash
# List available releases
./target/release/depmap-downloader list releases

# List files in a specific release
./target/release/depmap-downloader list files "DepMap Public 23Q4" --detailed

# Download the release
./target/release/depmap-downloader download --verify-checksum release "DepMap Public 23Q4"
```

#### Search Genes
```bash
# Search for tumor suppressor gene TP53
./target/release/depmap-downloader search TP53 -g

# Search by Entrez ID
./target/release/depmap-downloader search 7159 -g --limit 5

# Find common essential genes (search pattern)
./target/release/depmap-downloader search "RPA" -g

# Search for oncogenes
./target/release/depmap-downloader search MYC -g
```

## Project Structure

```
depmap-downloader-rs/
├── src/                    # Source code directory
│   ├── main.rs             # Main program entry point
│   ├── cli.rs              # CLI argument definitions
│   ├── commands.rs         # Command handling logic
│   ├── cache_manager.rs    # Cache manager
│   ├── downloader.rs        # File downloader
│   ├── models.rs           # Data models
│   └── error.rs            # Error handling
├── Cargo.toml             # Project configuration and dependencies
├── README.md               # This documentation
├── LICENSE                 # License file
└── target/                # Build output directory
```

## Core Components

### CacheManager

Responsible for data caching and database management:

- **Database Migrations**: Automatically creates and updates SQLite database schema
- **API Data Fetching**: Fetches and caches data from DepMap API
- **Query Interface**: Provides data query and search functionality for releases, datasets, cell lines, and genes
- **Cache Management**: Intelligent caching strategies to avoid duplicate downloads

### Downloader

Responsible for high-performance file downloads:

- **Concurrent Downloads**: Multi-threaded concurrent download support
- **Progress Display**: Real-time download progress display
- **Checksum Verification**: MD5 checksum to ensure file integrity
- **Resume Support**: Support for skipping existing files
- **Error Handling**: Comprehensive error recovery mechanisms

### CLI Interface

Provides rich command-line functionality:

- **Interactive Design**: Intuitive command-line interface
- **Parameter Validation**: Complete parameter checking and error prompts
- **Colored Output**: Clear colored terminal output
- **Help System**: Complete help documentation

## Data Types

Supported main data types:

- **CRISPR**: CRISPR gene screening data
- **RNAi**: RNA interference data
- **Expression**: Gene expression data
- **Mutations**: Mutation data
- **CN**: Copy number variation data
- **Drug screen**: Drug screening data
- **Protein Expression**: Protein expression data
- **Metadata**: Metadata

## Configuration Options

### Command Line Options

```bash
--database <PATH>     # Custom database file path (default: depmap_cache.db)
--api-url <URL>        # Custom API base URL (default: https://depmap.org/portal/api)
--verbose             # Enable verbose logging
```

### Download Options

```bash
--output <DIR>         # Output directory (default: depmap_data)
--workers <NUM>        # Number of concurrent downloads (default: 4)
--skip-existing        # Skip files that already exist
--verify-checksum      # Verify file checksums after download
```

## Performance Characteristics

### Memory Efficiency
- Streaming processing of large files to avoid memory overflow
- Intelligent cache management to minimize memory usage
- Async I/O operations for improved concurrent performance

### Download Performance
- Configurable concurrent downloads (default: 4 threads)
- Automatic retry mechanism with error recovery
- Resume support to avoid duplicate downloads

### Database Performance
- SQLite local caching for fast queries
- Indexed optimization for efficient searching
- Batch operations to reduce database calls

## Error Handling

The program includes comprehensive error handling mechanisms:

- **Network Errors**: Automatic retry with exponential backoff
- **Database Errors**: Database migration and recovery mechanisms
- **File Errors**: Checksum verification and file integrity checks
- **Permission Errors**: Clear error messages and solution suggestions

## Development Guide

### Local Development

```bash
# Clone the repository
git clone https://github.com/yourusername/depmap-downloader-rs.git
cd depmap-downloader-rs

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Development build
cargo build

# Run tests
cargo test

# Code linting
cargo clippy

# Format code
cargo fmt
```

### Adding New Features

1. Add new CLI parameters in `src/cli.rs`
2. Implement command handling logic in `src/commands.rs`
3. Add data query methods in `src/cache_manager.rs`
4. Add corresponding test cases

## Troubleshooting

### Compilation Issues

```bash
# Clean build cache
cargo clean

# Rebuild
cargo build

# Check Rust version
rustc --version
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
# Adjust concurrency
./target/release/depmap-downloader download --workers 2

# Limit download count
./target/release/depmap-downloader download --dataset CRISPR

# Clear cache and rebuild
./target/release/depmap-downloader clear --all
```

## Related Resources

- [DepMap Official Website](https://depmap.org)
- [DepMap API Documentation](https://depmap.org/portal/api)
- [DepMap Data Page](https://depmap.org/portal/data_page)
- [Rust Official Documentation](https://doc.rust-lang.org/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Tokio Documentation](https://docs.rs/tokio/)

## Contributing

Issues and Pull Requests are welcome:

1. Fork the project
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License. Please follow the official terms for using DepMap data.

## Acknowledgments

- DepMap project for providing data and research resources
- Rust community for excellent tools and libraries
- All contributors and users for feedback and suggestions

---

**Note**: This tool is developed based on the DepMap experimental API. The API may change, please pay attention to official updates.
