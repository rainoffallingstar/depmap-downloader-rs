# DepMap Downloader Rust

一个高性能的 Rust 工具，用于下载 DepMap (Cancer Dependency Map) 数据。支持从官方 API 和 Figshare 获取癌症依赖性数据集，具有并发下载、进度显示和本地缓存等功能。

## 项目概述

DepMap (癌症依赖性图谱) 是由博德研究所主导的科学研究项目，旨在系统性地识别癌细胞赖以生存的基因及分子通路。本 Rust 实现提供了程序化访问 DepMap 数据的高性能方式，支持：

- 🚀 **高性能**: Rust 的零成本抽象和高效内存管理
- 🔄 **并发下载**: 多线程并发下载，支持大文件处理
- 📊 **智能缓存**: SQLite 本地缓存，避免重复下载
- 🎯 **精确搜索**: 支持细胞系和数据集的模糊搜索
- 🛡️ **类型安全**: 编译时保证的类型安全
- 📦 **单一可执行文件**: 无需运行时依赖，易于部署

## 快速开始

### 安装要求

- Rust 1.70+ (推荐使用 [rustup](https://rustup.rs/))
- 网络连接

### 构建项目

```bash
# 克隆项目
git clone <repository-url>
cd depmap-downloader-rs

# 构建项目
cargo build --release

# 运行程序
./target/release/depmap-downloader --help
```

### 开发模式

```bash
# 开发构建（更快）
cargo build

# 运行开发版本
./target/debug/depmap-downloader --help
```

## 使用方法

### 命令行界面

#### 1. 更新本地缓存

```bash
# 更新缓存（自动检查是否需要更新）
./target/release/depmap-downloader update

# 强制更新
./target/release/depmap-downloader update --force

# 仅更新特定数据类型
./target/release/depmap-downloader update --data-type CRISPR --data-type Expression
```

#### 2. 列出可用数据

```bash
# 列出所有发布版本
./target/release/depmap-downloader list

# 列出特定数据类型
./target/release/depmap-downloader list --data-type CRISPR

# 显示详细信息
./target/release/depmap-downloader list --detailed

# 列出特定版本的文件
./target/release/depmap-downloader list --release "DepMap Public 25Q3"
```

#### 3. 下载数据

```bash
# 下载当前版本的核心文件
./target/release/depmap-downloader download

# 下载特定数据集
./target/release/depmap-downloader download --dataset Chronos_Combined

# 下载特定文件
./target/release/depmap-downloader download --file "CRISPRGeneEffect.csv"

# 自定义输出目录和并发数
./target/release/depmap-downloader download --output ./my_data --workers 8

# 跳过已存在的文件
./target/release/depmap-downloader download --skip-existing

# 启用校验和验证
./target/release/depmap-downloader download --verify-checksum
```

#### 4. 搜索数据

```bash
# 搜索细胞系
./target/release/depmap-downloader search "A549" --cell-line

# 搜索数据集
./target/release/depmap-downloader search "CRISPR" --dataset

# 搜索所有内容（默认）
./target/release/depmap-downloader search "gene"

# 限制搜索结果数量
./target/release/depmap-downloader search "cancer" --limit 20
```

#### 5. 缓存统计

```bash
# 显示基本统计
./target/release/depmap-downloader stats

# 显示详细统计
./target/release/depmap-downloader stats --detailed
```

#### 6. 清理缓存

```bash
# 清除所有缓存
./target/release/depmap-downloader clear --all

# 清除特定数据类型缓存
./target/release/depmap-downloader clear --data-type CRISPR
```

### 编程接口

```rust
use depmap_downloader::{CacheManager, Downloader};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化缓存管理器
    let cache = CacheManager::new(
        "depmap_cache.db", 
        "https://depmap.org/portal/api", 
        &PathBuf::from(".cache")
    ).await?;
    
    // 更新缓存
    cache.update_cache(false).await?;
    
    // 获取所有发布版本
    let releases = cache.get_releases(None).await?;
    println!("找到 {} 个发布版本", releases.len());
    
    // 获取特定数据集
    let datasets = cache.get_datasets(Some("CRISPR")).await?;
    println!("找到 {} 个 CRISPR 数据集", datasets.len());
    
    // 初始化下载器
    let downloader = Downloader::new(
        4,                              // 并发数
        "downloads".to_string(),     // 输出目录
        true,                           // 跳过已存在文件
        true,                           // 校验和验证
    )?;
    
    // 下载文件
    if !releases.is_empty() {
        let release = &releases[0];
        let download_files = release.files.clone();
        downloader.download_files(download_files).await?;
    }
    
    Ok(())
}
```

## 项目结构

```
depmap-downloader-rs/
├── src/                    # 源代码目录
│   ├── main.rs             # 主程序入口
│   ├── cli.rs              # CLI 参数定义
│   ├── commands.rs         # 命令处理逻辑
│   ├── cache_manager.rs    # 缓存管理器
│   ├── downloader.rs        # 文件下载器
│   ├── models.rs           # 数据模型
│   └── error.rs            # 错误处理
├── Cargo.toml             # 项目配置和依赖
├── README.md               # 本文档
├── LICENSE                 # 许可证文件
└── target/                # 编译输出目录
```

## 核心组件

### CacheManager

负责数据缓存和数据库管理：

- **数据库迁移**: 自动创建和更新 SQLite 数据库结构
- **API 数据获取**: 从 DepMap API 获取数据并缓存
- **查询接口**: 提供数据查询和搜索功能
- **缓存管理**: 智能缓存策略，避免重复下载

### Downloader

负责高性能文件下载：

- **并发下载**: 支持多线程并发下载
- **进度显示**: 实时显示下载进度
- **校验和验证**: MD5 校验确保文件完整性
- **断点续传**: 支持跳过已存在文件
- **错误处理**: 完善的错误恢复机制

### CLI 接口

提供丰富的命令行功能：

- **交互式设计**: 直观的命令行界面
- **参数验证**: 完整的参数检查和错误提示
- **彩色输出**: 清晰的彩色终端输出
- **帮助系统**: 完整的帮助文档

## 数据模型

### Release (发布版本)
- 发行版本信息
- 发布日期
- 包含的文件列表
- 当前版本标识

### Dataset (数据集)
- 数据集 ID 和显示名称
- 数据类型 (CRISPR, RNAi, Expression 等)
- 下载链接
- 关联文件

### DownloadFile (下载文件)
- 文件名和 URL
- MD5 校验和
- 文件大小和类型
- 下载状态

## 数据类型

支持的主要数据类型：

- **CRISPR**: CRISPR 基因筛选数据
- **RNAi**: RNA 干扰数据  
- **Expression**: 基因表达数据
- **Mutations**: 突变数据
- **CN**: 拷贝数变异数据
- **Drug screen**: 药物筛选数据
- **Protein Expression**: 蛋白质表达数据
- **Metadata**: 元数据

## 配置选项

### 环境变量

- `DATABASE_URL`: 数据库连接字符串 (默认: `depmap_cache.db`)
- `SQLX_OFFLINE`: 启用 SQLx 离线模式 (用于编译)

### 命令行选项

```bash
--database <PATH>     # 自定义数据库文件路径
--api-url <URL>        # 自定义 API 基础 URL
--verbose             # 启用详细日志输出
```

## 性能特性

### 内存效率
- 流式处理大文件，避免内存溢出
- 智能缓存管理，最小化内存占用
- 异步 I/O 操作，提高并发性能

### 下载性能
- 可配置的并发下载 (默认: 4 线程)
- 自动重试机制和错误恢复
- 支持断点续传，避免重复下载

### 数据库性能
- SQLite 本地缓存，快速查询
- 索引优化，支持高效搜索
- 批量操作，减少数据库调用

## 错误处理

程序包含完善的错误处理机制：

- **网络错误**: 自动重试，支持指数退避
- **数据库错误**: 数据库迁移和恢复机制
- **文件错误**: 校验和验证和文件完整性检查
- **权限错误**: 清晰的错误信息和解决建议

## 开发指南

### 本地开发

```bash
# 克隆项目
git clone <repository-url>
cd depmap-downloader-rs

# 安装 Rust (如果没有)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 开发构建
cargo build

# 运行测试
cargo test

# 代码检查
cargo clippy

# 格式化代码
cargo fmt
```

### 添加新功能

1. 在 `src/cli.rs` 中添加新的 CLI 参数
2. 在 `src/commands.rs` 中实现命令处理逻辑
3. 在 `src/cache_manager.rs` 中添加数据查询方法
4. 添加相应的测试用例

## 性能对比

与 Python 版本相比：

| 特性 | Python 版本 | Rust 版本 |
|------|------------|----------|
| 内存使用 | 高 | 低 |
| 下载速度 | 中等 | 高 |
| 并发性能 | GIL 限制 | 真正并发 |
| 部署大小 | 需要 Python 环境 | 单一可执行文件 |
| 类型安全 | 运行时错误 | 编译时检查 |
| 错误处理 | 异常捕获 | Result 类型系统 |

## 故障排除

### 编译问题

```bash
# 清理构建缓存
cargo clean

# 重新构建
cargo build

# 检查 Rust 版本
rustc --version
```

### 运行时问题

```bash
# 检查数据库权限
ls -la depmap_cache.db

# 检查网络连接
curl -I https://depmap.org/portal/api

# 查看详细日志
./target/release/depmap-downloader --verbose update
```

### 性能问题

```bash
# 调整并发数
./target/release/depmap-downloader download --workers 2

# 限制下载数量
./target/release/depmap-downloader download --dataset CRISPR

# 清理缓存重建
./target/release/depmap-downloader clear --all
```

## 相关资源

- [DepMap 官方网站](https://depmap.org)
- [DepMap API 文档](https://depmap.org/portal/api)
- [DepMap 数据页面](https://depmap.org/portal/data_page)
- [Rust 官方文档](https://doc.rust-lang.org/)
- [SQLx 文档](https://docs.rs/sqlx/)
- [Tokio 文档](https://docs.rs/tokio/)

## 贡献

欢迎提交 Issue 和 Pull Request：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目采用 MIT 许可证。DepMap 数据的使用请遵循官方条款。

## 致谢

- DepMap 项目提供的数据和研究资源
- Rust 社区的优秀工具和库
- 所有贡献者和用户的反馈和建议

---

**注意**: 本工具基于 DepMap 实验性 API 开发，API 可能会有变更，请关注官方更新。
