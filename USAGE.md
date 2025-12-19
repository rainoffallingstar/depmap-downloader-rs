# DepMap Downloader Rust 使用指南

本文档提供了 DepMap Downloader Rust 版本的详细使用说明和实际示例。

## 安装和设置

### 1. 安装 Rust

首先安装 Rust 工具链：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. 克隆项目

```bash
git clone <repository-url>
cd depmap-downloader-rs
```

### 3. 构建项目

```bash
# 发布版本（优化性能）
cargo build --release

# 或者开发版本（编译更快）
cargo build
```

## 基本使用

### 首次使用

1. **更新缓存**：获取最新的 DepMap 数据信息
2. **浏览数据**：查看可用的数据集和版本
3. **下载数据**：选择需要的数据进行下载

```bash
# 1. 更新缓存
./target/release/depmap-downloader update

# 2. 查看可用的数据类型
./target/release/depmap-downloader list --data-type CRISPR

# 3. 下载 CRISPR 数据
./target/release/depmap-downloader download --data-type CRISPR
```

## 详细命令说明

### Update 命令

更新本地缓存，从 DepMap API 获取最新数据：

```bash
# 常规更新
./target/release/depmap-downloader update

# 强制更新（忽略缓存时间）
./target/release/depmap-downloader update --force

# 仅更新特定数据类型
./target/release/depmap-downloader update --data-type CRISPR --data-type Expression
```

### List 命令

浏览和列出可用的数据：

```bash
# 列出所有发布版本
./target/release/depmap-downloader list

# 列出特定数据类型的数据集
./target/release/depmap-downloader list --data-type CRISPR

# 显示详细信息（包含文件数量、大小等）
./target/release/depmap-downloader list --detailed

# 列出特定发布版本的文件
./target/release/depmap-downloader list --release "DepMap Public 25Q3"
```

### Download 命令

下载 DepMap 数据文件：

```bash
# 下载当前版本的所有核心文件
./target/release/depmap-downloader download

# 下载特定的数据集（所有相关文件）
./target/release/depmap-downloader download --dataset Chronos_Combined

# 下载单个特定文件
./target/release/depmap-downloader download --file "CRISPRGeneEffect.csv"

# 自定义设置
./target/release/depmap-downloader download \
  --output ./my_depmap_data \
  --workers 8 \
  --skip-existing \
  --verify-checksum
```

### Search 命令

搜索特定的细胞系或数据集：

```bash
# 搜索细胞系（支持模糊匹配）
./target/release/depmap-downloader search "A549" --cell-line

# 搜索数据集
./target/release/depmap-downloader search "CRISPR" --dataset

# 搜索所有内容（默认行为）
./target/release/depmap-downloader search "gene dependency"

# 限制搜索结果数量
./target/release/depmap-downloader search "cancer" --limit 10
```

### Stats 命令

查看缓存统计信息：

```bash
# 基本统计
./target/release/depmap-downloader stats

# 详细统计（包含各类型数据量）
./target/release/depmap-downloader stats --detailed
```

### Clear 命令

清理本地缓存：

```bash
# 清除所有缓存
./target/release/depmap-downloader clear --all

# 清除特定数据类型的缓存
./target/release/depmap-downloader clear --data-type CRISPR
```

## 实际使用场景

### 场景 1：研究者需要 CRISPR 基因筛选数据

```bash
# 1. 更新缓存
./target/release/depmap-downloader update

# 2. 查看 CRISPR 数据集
./target/release/depmap-downloader list --data-type CRISPR

# 3. 下载 CRISPR 数据
./target/release/depmap-downloader download --dataset Chronos_Combined

# 4. 搜索特定基因
./target/release/depmap-downloader search "TP53" --dataset --limit 5
```

### 场景 2：批量下载特定细胞系相关数据

```bash
# 1. 搜索细胞系
./target/release/depmap-downloader search "MCF7" --cell-line

# 2. 查看数据集
./target/release/depmap-downloader list --detailed

# 3. 下载基因表达数据（通常包含细胞系信息）
./target/release/depmap-downloader download --dataset expression

# 4. 下载突变数据
./target/release/depmap-downloader download --dataset mutations_damaging
```

### 场景 3：获取最新发布版本的所有数据

```bash
# 1. 更新到最新版本
./target/release/depmap-downloader update --force

# 2. 查看最新版本
./target/release/depmap-downloader list

# 3. 下载核心数据文件（不包含所有大文件）
./target/release/depmap-downloader download

# 4. 检查下载统计
./target/release/depmap-downloader stats --detailed
```

### 场景 4：特定研究目的下载

```bash
# 药物敏感性研究
./target/release/depmap-downloader download --dataset GDSC2_AUC

# 蛋白质表达数据
./target/release/depmap-downloader download --dataset "Harmonized MS CCLE Gygi"

# 代谢组学数据
./target/release/depmap-downloader download --dataset metabolomics
```

## 高级配置

### 自定义数据库路径

```bash
# 使用自定义数据库路径
./target/release/depmap-downloader \
  --database /path/to/my_cache.db \
  update
```

### 自定义 API 端点

```bash
# 使用自定义 API（测试或开发环境）
./target/release/depmap-downloader \
  --api-url "https://test-api.depmap.org" \
  update
```

### 启用详细日志

```bash
# 查看详细的操作日志
./target/release/depmap-downloader --verbose update
```

## 编程接口使用

### Rust 项目集成

在你的 Rust 项目中添加依赖：

```toml
[dependencies]
depmap-downloader = { path = "/path/to/depmap-downloader-rs" }
tokio = { version = "1.0", features = ["full"] }
```

### 基本使用示例

```rust
use depmap_downloader::{CacheManager, Downloader};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化缓存管理器
    let cache = CacheManager::new(
        "my_cache.db",
        "https://depmap.org/portal/api",
        &PathBuf::from("./cache")
    ).await?;
    
    // 更新缓存
    println!("正在更新缓存...");
    cache.update_cache(false).await?;
    println!("缓存更新完成！");
    
    // 搜索数据集
    println!("搜索 CRISPR 数据集...");
    let crispr_datasets = cache.get_datasets(Some("CRISPR")).await?;
    
    for dataset in crispr_datasets {
        println!("找到数据集: {}", dataset.display_name);
    }
    
    // 下载特定数据集
    if let Some(dataset) = crispr_datasets.first() {
        println!("下载数据集: {}", dataset.display_name);
        let downloader = Downloader::new(4, "downloads".to_string(), false, true)?;
        
        // 获取数据集关联文件
        let files = cache.get_dataset_files(&dataset.id).await?;
        downloader.download_files(files).await?;
        println!("下载完成！");
    }
    
    Ok(())
}
```

### 批量操作示例

```rust
use depmap_downloader::CacheManager;

async fn batch_download() -> Result<(), Box<dyn std::error::Error>> {
    let cache = CacheManager::new("batch_cache.db", 
                                   "https://depmap.org/portal/api", 
                                   &PathBuf::from("./batch_cache")).await?;
    
    // 预定义要下载的数据集
    let target_datasets = vec![
        "Chronos_Combined",
        "mutations_damaging", 
        "expression",
        "copy_number_relative"
    ];
    
    for dataset_id in target_datasets {
        println!("处理数据集: {}", dataset_id);
        
        // 搜索数据集
        let datasets = cache.get_datasets(None).await?;
        if let Some(dataset) = datasets.iter().find(|d| d.id.contains(dataset_id)) {
            println!("找到匹配数据集: {}", dataset.display_name);
            
            // 下载相关文件
            let files = cache.get_dataset_files(&dataset.id).await?;
            if !files.is_empty() {
                let downloader = Downloader::new(2, format!("downloads_{}", dataset_id), true, false)?;
                downloader.download_files(files).await?;
                println!("✅ {} 下载完成", dataset.display_name);
            }
        }
    }
    
    Ok(())
}
```

## 性能优化建议

### 1. 并发设置

根据网络带宽调整并发数：

```bash
# 高速网络（100+ Mbps）
./target/release/depmap-downloader download --workers 8

# 普通网络（10-50 Mbps）
./target/release/depmap-downloader download --workers 4  # 默认值

# 慢速网络（<10 Mbps）
./target/release/depmap-downloader download --workers 2
```

### 2. 磁盘空间管理

大文件下载前检查磁盘空间：

```bash
# 查看可用磁盘空间
df -h

# 仅下载必要文件
./target/release/depmap-downloader download --dataset CRISPR
```

### 3. 网络稳定性

在不稳定的网络环境中：

```bash
# 使用跳过已存在文件功能
./target/release/depmap-downloader download --skip-existing

# 启用校验和验证
./target/release/depmap-downloader download --verify-checksum

# 降低并发数以减少网络压力
./target/release/depmap-downloader download --workers 1
```

## 常见问题解决

### 数据库错误

```bash
# 数据库锁定或损坏
./target/release/depmap-downloader clear --all

# 重新初始化
rm depmap_cache.db
./target/release/depmap-downloader update
```

### 网络连接问题

```bash
# 检查网络连接
curl -I https://depmap.org/portal/api

# 使用代理（如果需要）
export https_proxy=http://proxy.company.com:8080
./target/release/depmap-downloader update
```

### 权限问题

```bash
# 检查目录权限
ls -la downloads/

# 修复权限
chmod 755 downloads/
```

## 数据使用建议

### 1. 数据选择

根据研究需求选择合适的数据：

- **基础研究**: CRISPR + RNAi + Expression
- **药物研究**: Drug screen + GDSC 数据
- **机制研究**: Mutations + CN + Protein Expression
- **综合分析**: Metadata + 所有数据类型

### 2. 存储管理

```bash
# 按数据类型组织存储
mkdir -p depmap_data/{crispr,expression,mutations}
./target/release/depmap-downloader download --output depmap_data/crispr --data-type CRISPR
```

### 3. 版本控制

```bash
# 记录下载的版本信息
./target/release/depmap-downloader list > downloads_log.txt
./target/release/depmap-downloader stats >> downloads_log.txt
```

这个使用指南涵盖了 DepMap Downloader Rust 版本的所有主要功能和使用场景，帮助用户快速上手并高效使用工具进行癌症依赖性数据的研究和分析。
