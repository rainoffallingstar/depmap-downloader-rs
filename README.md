# DepMap 数据下载器

一个用于下载 DepMap (Cancer Dependency Map) 数据的 Python 工具，支持从官方 API 和 Figshare 获取癌症依赖性数据集。

## 项目概述

DepMap (癌症依赖性图谱) 是由博德研究所主导的科学研究项目，旨在系统性地识别癌细胞赖以生存的基因及分子通路。本工具提供了程序化访问 DepMap 数据的便捷方式，支持：

- 下载最新版本的 DepMap 数据
- 获取历史版本数据
- 多线程并发下载
- 进度显示和断点续传
- 自定义筛选和批量处理

## 功能特性

- 🔍 **版本管理**: 自动获取 DepMap 发布版本列表
- ⬇️ **多源下载**: 支持官方 API 和 Figshare 数据源
- 🚀 **高性能**: 多线程并发下载，支持大文件处理
- 📊 **进度可视**: 实时显示下载进度
- 🔄 **断点续传**: 支持下载失败后重试
- 🛡️ **错误处理**: 完善的异常处理和日志记录

## 环境要求

- Python 3.8+
- 网络连接

## 安装和配置

### 方法一：使用 uv (推荐)

```bash
# 安装 uv 包管理工具
curl -LsSf https://astral.sh/uv/install.sh | sh

# 克隆项目
git clone <repository-url>
cd depmapdown

# 初始化项目并安装依赖
uv init
uv add requests pandas tqdm

# 运行程序
uv run python depmapdown.py
```

### 方法二：使用传统 pip

```bash
# 克隆项目
git clone <repository-url>
cd depmapdown

# 创建虚拟环境
python -m venv venv
source venv/bin/activate  # Linux/Mac
# 或 venv\Scripts\activate  # Windows

# 安装依赖
pip install requests pandas tqdm

# 运行程序
python depmapdown.py
```

## 使用方法

### 交互式使用

直接运行主程序：

```bash
python depmapdown.py
```

程序会显示选项菜单：

```
=== DepMap 数据下载器 ===
正在获取可用的DepMap发布版本...

请选择下载选项:
1. 下载当前最新版本
2. 从Figshare下载特定历史版本
3. 查看所有可用版本详情

请输入选择 (1-3):
```

### 选项说明

1. **下载当前最新版本**: 下载 DepMap 官方发布的最新数据集
2. **下载历史版本**: 从 Figshare 选择并下载特定历史版本
3. **查看版本详情**: 浏览所有可用版本的详细信息

### 编程方式使用

```python
from depmapdown import DepMapReleaseManager, DepMapDataDownloader

# 初始化版本管理器
release_manager = DepMapReleaseManager()

# 获取发布版本列表
releases = release_manager.get_figshare_releases()
print(f"找到 {len(releases)} 个发布版本")

# 初始化下载器
downloader = DepMapDataDownloader(
    download_dir="my_depmap_data",  # 自定义下载目录
    max_workers=8  # 设置并发下载数
)

# 下载当前最新版本
downloader.download_current_release(max_files=10)

# 下载特定版本
if releases:
    selected_release = releases[0]  # 选择第一个版本
    downloader.download_figshare_release(selected_release)
```

### 高级用法

#### 自定义筛选

```python
# 获取当前版本文件列表
files_df = release_manager.get_current_release_files()

# 筛选特定类型的文件
filtered_files = files_df[files_df['name'].str.contains('CRISPR')]
```

#### 单文件下载

```python
file_info = {
    'name': 'example_file.csv',
    'url': 'https://example.com/file.csv'
}

download_dir = downloader.create_download_dir("custom")
success, error = downloader.download_file(file_info, download_dir)
```

## 项目结构

```
depmapdown/
├── depmapdown.py          # 主程序文件
├── depmap-api-deepresearch.md  # DepMap API 详细分析文档
├── pyproject.toml         # 项目配置文件
├── uv.lock               # 依赖锁定文件
├── README.md             # 本文档
└── .venv/                # 虚拟环境目录
```

## 核心类说明

### DepMapReleaseManager

负责管理 DepMap 版本信息：

- `get_figshare_releases()`: 获取 Figshare 发布版本列表
- `get_current_release_files()`: 获取当前版本文件列表
- `get_figshare_files(article_id)`: 获取特定 Figshare 版本的文件列表
- `_extract_version(title)`: 从标题提取版本号

### DepMapDataDownloader

负责数据下载：

- `download_current_release(max_files=None)`: 下载当前版本
- `download_figshare_release(release_info, max_files=None)`: 下载 Figshare 版本
- `download_file(file_info, download_dir, retry_count=3)`: 下载单个文件
- `create_download_dir(release_name)`: 创建下载目录

## 数据源说明

### 官方 API

- **基础 URL**: `https://depmap.org/portal/api`
- **文件列表**: `/download/files`
- **自定义下载**: `/download/custom`

### Figshare

- **搜索 API**: `https://api.figshare.com/v2/articles/search`
- **数据集**: 搜索关键词 "DepMap"

## 输出格式

下载的数据以原始格式保存：

- **CSV 文件**: 表格数据，可使用 pandas 等工具处理
- **其他格式**: 根据 DepMap 原始格式保存

## 错误处理

程序包含完善的错误处理机制：

- **网络错误**: 自动重试，支持指数退避
- **文件错误**: 检查文件完整性，支持断点续传
- **权限错误**: 提示用户检查目录权限
- **内存错误**: 流式下载，支持大文件处理

## 日志记录

程序使用 Python logging 模块记录运行信息：

```
2024-12-19 10:00:00 - INFO - 正在从Figshare获取DepMap发布版本...
2024-12-19 10:00:05 - INFO - 从Figshare找到 15 个DepMap发布版本
2024-12-19 10:00:10 - INFO - 正在下载: example_file.csv (尝试 1/3)
```

## 性能优化建议

1. **并发设置**: 根据网络带宽调整 `max_workers` 参数
2. **磁盘空间**: 确保有足够的磁盘空间存储下载文件
3. **网络稳定**: 在稳定网络环境下运行以获得最佳性能
4. **筛选下载**: 使用 `max_files` 参数限制下载数量进行测试

## 注意事项

- 仅用于研究目的，不得用于商业用途
- 使用时请引用 DepMap 相关文献
- 部分数据文件较大，请确保有足够存储空间
- 网络不稳定时可能需要多次重试

## 故障排除

### 常见问题

1. **依赖安装失败**
   ```bash
   # 尝试更新包管理工具
   pip install --upgrade pip
   uv self update
   ```

2. **网络连接问题**
   ```bash
   # 检查网络连接
   curl -I https://depmap.org/portal/api
   ```

3. **权限问题**
   ```bash
   # 检查目录权限
   ls -la /path/to/download/directory
   ```

4. **内存不足**
   - 减少 `max_workers` 参数
   - 使用 `max_files` 限制下载数量

### 调试模式

启用详细日志：

```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

## 相关资源

- [DepMap 官方网站](https://depmap.org)
- [DepMap API 文档](https://depmap.org/portal/api)
- [DepMap 数据页面](https://depmap.org/portal/data_page)
- [Figshare API 文档](https://docs.figshare.com)

## 许可证

本项目仅用于研究目的。DepMap 数据的使用请遵循官方条款。

## 贡献

欢迎提交 Issue 和 Pull Request 来改进这个工具。

---

**注意**: 本工具基于 DepMap 实验性 API 开发，API 可能会有变更，请关注官方更新。
