# 🧬 DepMap 下载工具 (Rust 版)

> ⚡ 高性能 Rust 工具，用于下载 DepMap 癌症依赖性图谱数据

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/rainoffallingstar/depmap-downloader-rs.svg?style=social&label=Star)](https://github.com/rainoffallingstar/depmap-downloader-rs)

## 🎯 特性

- 🚀 **高性能** - Rust 零成本抽象和高效内存管理
- ⚡ **并发下载** - 多线程并行下载，轻松处理大文件
- 💾 **智能缓存** - SQLite 本地缓存，避免重复下载
- 🔍 **强大搜索** - 支持基因、细胞系和数据集搜索
- 🛡️ **类型安全** - 编译时类型安全保证
- 📦 **单一可执行** - 无需运行时依赖，开箱即用

## 🚀 快速开始

### 📋 系统要求
- Rust 1.70+ ([安装指南](https://rustup.rs/))
- 网络连接

### 🔧 构建项目

```bash
# 克隆仓库
git clone https://github.com/rainoffallingstar/depmap-downloader-rs.git
cd depmap-downloader-rs

# 构建项目
cargo build --release

# 运行程序
./target/release/depdown --help
```

### 🎮 开发模式（更快构建）

```bash
# 开发构建
cargo build

# 运行开发版本
./target/debug/depdown --help
```

## 💻 使用指南

### 🔄 更新缓存

```bash
# 更新缓存（自动检查是否需要）
./target/release/depdown update

# 强制更新
./target/release/depdown update --force

# 更新特定数据类型
./target/release/depdown update --data-type CRISPR --data-type Expression

# 支持的数据类型
# CRISPR, Expression, Mutations, CN, RNAi, Drug screen, Protein
```

### 📋 列出数据

```bash
# 查看所有发布版本 📦
./target/release/depdown list releases

# 详细版本信息
./target/release/depdown list releases --detailed

# 查看特定数据类型 🧬
./target/release/depdown list datasets --data-type CRISPR

# 查看版本文件详情 📁
./target/release/depdown list files "DepMap Public 23Q4" --detailed
```

### ⬇️ 下载数据

```bash
# 🆕 下载整个发布版本
./target/release/depdown download release "DepMap Public 23Q4"

# 🆕 下载特定数据类型
./target/release/depdown download release "DepMap Public 23Q4" --data-type CRISPR

# 🆕 下载特定数据集
./target/release/depdown download dataset "CRISPR (DepMap Public 25Q3+Score, Chronos)"

# ⚡ 高速下载（8个并发）
./target/release/depdown download --workers 8 release "DepMap Public 25Q3"

# 跳过已存在文件
./target/release/depdown download --skip-existing

# 验证文件完整性
./target/release/depdown download --verify-checksum

# 自定义选项组合
./target/release/depdown download \
  --output ./my_data \
  --workers 8 \
  --skip-existing \
  --verify-checksum \
  release "DepMap Public 25Q3" \
  --data-type CRISPR
```

### 🔍 搜索数据

```bash
# 🆕 搜索基因（支持基因名和 Entrez ID）
./target/release/depdown search TP53 -g
./target/release/depdown search 7159 -g --limit 5

# 🔍 搜索细胞系
./target/release/depdown search "A549" --cell-line

# 📊 搜索数据集
./target/release/depdown search "CRISPR" --dataset

# 🎯 搜索所有类型（默认行为）
./target/release/depdown search "BRCA1"
```

### 📈 查看统计

```bash
# 缓存统计信息
./target/release/depdown stats

# 详细统计信息
./target/release/depdown stats --detailed
```

### 🗑️ 清理缓存

```bash
# 清理所有缓存
./target/release/depdown clear --all

# 清理特定数据类型缓存
./target/release/depdown clear --data-type CRISPR
```

## 💡 实用示例

### 🧬 基因研究

```bash
# 搜索肿瘤抑制基因 TP53
./target/release/depdown search TP53 -g

# 搜索癌基因 MYC
./target/release/depdown search MYC -g --limit 10

# 按 Entrez ID 精确查找
./target/release/depdown search 672 -g  # BRCA1
```

### 📦 数据下载

```bash
# 仅下载 CRISPR 数据（最新版本）
./target/release/depdown download --skip-existing release "DepMap Public 25Q3" --data-type CRISPR

# 下载多种数据类型
./target/release/depdown download --workers 8 release "DepMap Public 23Q4" --data-type Expression
./target/release/depdown download --workers 8 release "DepMap Public 23Q4" --data-type Mutations
```

### 🔍 探索式研究

```bash
# 查看可用版本
./target/release/depdown list releases

# 浏览版本文件
./target/release/depdown list files "DepMap Public 23Q4" --detailed

# 选择性下载
./target/release/depdown download --verify-checksum release "DepMap Public 23Q4"
```

## 📊 支持的数据类型

| 数据类型 | 描述 | 文件数量 |
|---------|------|---------|
| 🧬 **CRISPR** | CRISPR 基因筛选数据 | 多个数据集 |
| 🧪 **RNAi** | RNA 干扰数据 | 多个数据集 |
| 📈 **Expression** | 基因表达数据 | 多个数据集 |
| 🧬 **Mutations** | 基因突变数据 | 多个数据集 |
| 📊 **CN** | 拷贝数变异数据 | 多个数据集 |
| 💊 **Drug screen** | 药物筛选数据 | 多个数据集 |
| 🔬 **Protein** | 蛋白质表达数据 | 多个数据集 |

## ⚙️ 配置选项

```bash
# 自定义数据库路径
--database <PATH>

# 自定义 API 地址
--api-url <URL>

# 输出目录
--output <DIR>

# 并发数量（默认: 4）
--workers <NUM>

# 启用详细日志
--verbose
```

## 🏗️ 项目结构

```
depmap-downloader-rs/
├── 📁 src/                    # 源代码
│   ├── main.rs                # 程序入口
│   ├── cli.rs                 # 命令行定义
│   ├── commands.rs            # 命令处理逻辑
│   ├── cache_manager.rs       # 缓存管理器
│   ├── downloader.rs          # 文件下载器
│   ├── models.rs              # 数据模型
│   └── error.rs               # 错误处理
├── 📄 Cargo.toml               # 项目配置
├── 📝 README.md                # 英文文档
├── 📜 readme_zh.md              # 中文文档
├── 📖 usage.md               # 使用指南
├── 📂 target/                  # 构建输出
```

## 🧪 性能特点

- **内存效率** 📉 - 流式处理大文件，避免内存溢出
- **下载性能** ⚡ - 可配置并发下载，自动重试
- **数据库性能** 🔍 - SQLite 索引优化，快速查询
- **缓存智能** 🧠 - 避免重复下载，节省带宽

## 🔧 开发

```bash
# 克隆仓库
git clone https://github.com/rainoffallingstar/depmap-downloader-rs.git
cd depmap-downloader-rs

# 开发构建
cargo build

# 运行测试
cargo test

# 代码检查
cargo clippy

# 格式化代码
cargo fmt
```

## 🆘 故障排除

### 编译问题
```bash
# 清理缓存并重新构建
cargo clean && cargo build
```

### 运行时问题
```bash
# 检查数据库权限
ls -la depmap_cache.db

# 检查网络连接
curl -I https://depmap.org/portal/api

# 查看详细日志
./target/release/depdown --verbose update
```

### 性能问题
```bash
# 调整并发数
./target/release/depdown download --workers 2

# 清理缓存重建
./target/release/depdown clear --all
```

## 📚 相关资源

- 🌐 [DepMap 官网](https://depmap.org)
- 📖 [DepMap API 文档](https://depmap.org/portal/api)
- 📊 [DepMap 数据页面](https://depmap.org/portal/data_page)
- 🦀 [Rust 文档](https://doc.rust-lang.org/)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

1. Fork 项目 🍴
2. 创建功能分支 (`git checkout -b feature/amazing-feature`) 🌿
3. 提交更改 (`git commit -m 'Add amazing feature'`) ✨
4. 推送到分支 (`git push origin feature/amazing-feature`) 📤
5. 创建 Pull Request 🎉

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 🙏 致谢

- 🧬 DepMap 项目提供的研究数据
- 🦀 Rust 社区优秀的工具和库
- 💝 所有贡献者和用户的反馈与建议

---

> 💡 **提示**: 本工具基于 DepMap 实验性 API 开发，API 可能会有变更，请关注官方更新。建议定期运行 `update` 命令以获取最新数据。

## 🌐 国际化支持

- 📖 [English README (README.md)](./README.md) - 国际用户文档
- 📖 [中文使用指南 (usage.md)](./usage.md) - 详细中文使用说明
- 📖 [中文 README (readme_zh.md)](./readme_zh.md) - 本文档

## 🚀 下载统计

- 总文件数：4635+
- 基因依赖数据：67,402 条记录
- 支持版本：64+ 个发布版本
- 数据类型：8 种主要类型
