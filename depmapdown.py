import requests
import pandas as pd
import os
import time
from tqdm import tqdm
import urllib.parse
from concurrent.futures import ThreadPoolExecutor, as_completed
import logging
import re
from datetime import datetime
import json

# 设置日志
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class DepMapReleaseManager:
    def __init__(self):
        """
        初始化DepMap版本管理器
        """
        self.base_url = "https://depmap.org/portal/api"
        self.figshare_search_url = "https://api.figshare.com/v2/articles/search"
        self.headers = {
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
        }
    
    def get_figshare_releases(self):
        """
        从Figshare获取DepMap的发布版本列表
        
        Returns:
            list: 发布版本信息列表
        """
        try:
            logger.info("正在从Figshare获取DepMap发布版本...")
            
            # 搜索DepMap数据集
            params = {
                'search_for': 'DepMap',
                'item_type': 2,  # 2 表示数据集
                'order': 'published_date',
                'order_direction': 'desc'
            }
            
            response = requests.get(self.figshare_search_url, params=params, headers=self.headers)
            response.raise_for_status()
            
            articles = response.json()
            
            # 筛选DepMap发布版本
            releases = []
            for article in articles:
                title = article.get('title', '')
                if 'DepMap' in title and ('Public' in title or re.search(r'\d{2}Q\d', title)):
                    release_info = {
                        'title': title,
                        'id': article.get('id'),
                        'url': article.get('url'),
                        'published_date': article.get('published_date'),
                        'version': self._extract_version(title)
                    }
                    releases.append(release_info)
            
            # 按版本号排序
            releases.sort(key=lambda x: x['version'], reverse=True)
            
            logger.info(f"从Figshare找到 {len(releases)} 个DepMap发布版本")
            return releases
            
        except requests.exceptions.RequestException as e:
            logger.error(f"从Figshare获取发布版本失败: {e}")
            return []
        except Exception as e:
            logger.error(f"解析Figshare数据失败: {e}")
            return []
    
    def _extract_version(self, title):
        """
        从标题中提取版本号
        
        Args:
            title (str): 数据集标题
            
        Returns:
            tuple: (年份, 季度) 用于排序
        """
        # 匹配版本号格式如 24Q4, 23Q1 等
        match = re.search(r'(\d{2})Q(\d)', title)
        if match:
            year = int(match.group(1))
            quarter = int(match.group(2))
            return (year + 2000, quarter)
        return (0, 0)
    
    def get_current_release_files(self):
        """
        获取当前最新版本的文件列表
        
        Returns:
            pandas.DataFrame: 文件列表
        """
        try:
            logger.info("正在获取当前发布版本的文件列表...")
            files_url = f"{self.base_url}/download/files"
            response = requests.get(files_url, headers=self.headers)
            response.raise_for_status()
            
            files_df = pd.read_csv(pd.compat.StringIO(response.text))
            logger.info(f"当前版本包含 {len(files_df)} 个文件")
            
            return files_df
            
        except requests.exceptions.RequestException as e:
            logger.error(f"获取当前版本文件列表失败: {e}")
            return None
        except Exception as e:
            logger.error(f"解析文件列表失败: {e}")
            return None
    
    def get_figshare_files(self, article_id):
        """
        获取Figshare上特定版本的文件列表
        
        Args:
            article_id (int): Figshare文章ID
            
        Returns:
            list: 文件信息列表
        """
        try:
            logger.info(f"正在获取Figshare文章 {article_id} 的文件列表...")
            
            # 获取文章详情
            article_url = f"https://api.figshare.com/v2/articles/{article_id}"
            response = requests.get(article_url, headers=self.headers)
            response.raise_for_status()
            
            article_data = response.json()
            files = article_data.get('files', [])
            
            logger.info(f"Figshare版本包含 {len(files)} 个文件")
            return files
            
        except requests.exceptions.RequestException as e:
            logger.error(f"获取Figshare文件列表失败: {e}")
            return []
        except Exception as e:
            logger.error(f"解析Figshare文件数据失败: {e}")
            return []

class DepMapDataDownloader:
    def __init__(self, download_dir="depmap_data", max_workers=4):
        """
        初始化DepMap数据下载器
        
        Args:
            download_dir (str): 下载目录
            max_workers (int): 最大并发下载数
        """
        self.download_dir = download_dir
        self.max_workers = max_workers
        self.base_url = "https://depmap.org/portal/api"
        self.release_manager = DepMapReleaseManager()
        
        # 设置请求头
        self.headers = {
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
        }
    
    def create_download_dir(self, release_name):
        """
        为特定版本创建下载目录
        
        Args:
            release_name (str): 版本名称
            
        Returns:
            str: 下载目录路径
        """
        # 清理版本名称，用作目录名
        clean_name = re.sub(r'[^\w\-_\.]', '_', release_name)
        download_dir = os.path.join(self.download_dir, clean_name)
        os.makedirs(download_dir, exist_ok=True)
        return download_dir
    
    def download_file(self, file_info, download_dir, retry_count=3):
        """
        下载单个文件
        
        Args:
            file_info (dict): 文件信息
            download_dir (str): 下载目录
            retry_count (int): 重试次数
            
        Returns:
            tuple: (文件名, 是否成功, 错误信息)
        """
        file_name = file_info.get('name', file_info.get('title', 'unknown'))
        file_url = file_info.get('url', file_info.get('download_url', ''))
        
        if not file_url:
            return file_name, False, "URL为空"
        
        file_path = os.path.join(download_dir, file_name)
        
        # 如果文件已存在且大小大于0，则跳过
        if os.path.exists(file_path) and os.path.getsize(file_path) > 0:
            logger.info(f"文件已存在，跳过: {file_name}")
            return file_name, True, "文件已存在"
        
        for attempt in range(retry_count):
            try:
                logger.info(f"正在下载: {file_name} (尝试 {attempt + 1}/{retry_count})")
                
                # 流式下载大文件
                with requests.get(file_url, headers=self.headers, stream=True) as r:
                    r.raise_for_status()
                    
                    # 获取文件大小
                    total_size = int(r.headers.get('content-length', 0))
                    
                    # 创建进度条
                    progress_bar = tqdm(
                        total=total_size,
                        unit='B',
                        unit_scale=True,
                        desc=file_name,
                        ncols=100
                    )
                    
                    # 下载文件
                    with open(file_path, 'wb') as f:
                        for chunk in r.iter_content(chunk_size=8192):
                            if chunk:  # 过滤掉保持连接的新块
                                f.write(chunk)
                                progress_bar.update(len(chunk))
                    
                    progress_bar.close()
                    
                    logger.info(f"下载完成: {file_name}")
                    return file_name, True, None
                    
            except requests.exceptions.RequestException as e:
                logger.warning(f"下载失败 (尝试 {attempt + 1}/{retry_count}): {file_name} - {e}")
                if attempt < retry_count - 1:
                    time.sleep(2 ** attempt)  # 指数退避
                else:
                    return file_name, False, str(e)
            
            except Exception as e:
                logger.error(f"下载出错: {file_name} - {e}")
                return file_name, False, str(e)
    
    def download_current_release(self, max_files=None):
        """
        下载当前最新版本
        
        Args:
            max_files (int): 最大下载数量（用于测试）
        """
        files_df = self.release_manager.get_current_release_files()
        if files_df is None:
            logger.error("无法获取当前版本文件列表")
            return
        
        # 创建下载目录
        download_dir = self.create_download_dir("current_release")
        
        # 准备文件信息列表
        files_to_download = []
        for _, row in files_df.iterrows():
            file_info = {
                'name': row.get('name', ''),
                'url': row.get('url', ''),
                'size': row.get('size', 0)
            }
            files_to_download.append(file_info)
        
        # 限制下载数量
        if max_files:
            files_to_download = files_to_download[:max_files]
            logger.info(f"限制下载数量为: {max_files}")
        
        logger.info(f"准备下载 {len(files_to_download)} 个文件到: {download_dir}")
        self._download_files(files_to_download, download_dir)
    
    def download_figshare_release(self, release_info, max_files=None):
        """
        下载Figshare上的特定版本
        
        Args:
            release_info (dict): 版本信息
            max_files (int): 最大下载数量（用于测试）
        """
        article_id = release_info['id']
        title = release_info['title']
        
        files = self.release_manager.get_figshare_files(article_id)
        if not files:
            logger.error(f"无法获取版本 {title} 的文件列表")
            return
        
        # 创建下载目录
        download_dir = self.create_download_dir(title)
        
        # 准备文件信息列表
        files_to_download = []
        for file in files:
            file_info = {
                'name': file.get('name', ''),
                'url': file.get('download_url', ''),
                'size': file.get('size', 0)
            }
            files_to_download.append(file_info)
        
        # 限制下载数量
        if max_files:
            files_to_download = files_to_download[:max_files]
            logger.info(f"限制下载数量为: {max_files}")
        
        logger.info(f"准备下载 {len(files_to_download)} 个文件到: {download_dir}")
        self._download_files(files_to_download, download_dir)
    
    def _download_files(self, files_to_download, download_dir):
        """
        下载文件列表的通用方法
        
        Args:
            files_to_download (list): 文件信息列表
            download_dir (str): 下载目录
        """
        if not files_to_download:
            logger.warning("没有文件需要下载")
            return
        
        # 统计信息
        success_count = 0
        failed_count = 0
        skipped_count = 0
        
        # 使用多线程下载
        with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
            # 提交所有下载任务
            future_to_file = {
                executor.submit(self.download_file, file_info, download_dir): file_info['name'] 
                for file_info in files_to_download
            }
            
            # 处理完成的任务
            for future in as_completed(future_to_file):
                file_name = future_to_file[future]
                try:
                    file_name, success, error = future.result()
                    if success:
                        if error == "文件已存在":
                            skipped_count += 1
                        else:
                            success_count += 1
                    else:
                        failed_count += 1
                        logger.error(f"下载失败: {file_name} - {error}")
                except Exception as e:
                    failed_count += 1
                    logger.error(f"下载异常: {file_name} - {e}")
        
        # 输出统计信息
        total_files = len(files_to_download)
        logger.info(f"\n下载完成统计:")
        logger.info(f"总文件数: {total_files}")
        logger.info(f"成功下载: {success_count}")
        logger.info(f"下载失败: {failed_count}")
        logger.info(f"跳过已存在: {skipped_count}")
        logger.info(f"下载目录: {os.path.abspath(download_dir)}")

def display_releases(releases):
    """
    显示版本列表
    
    Args:
        releases (list): 版本信息列表
    """
    if not releases:
        print("未找到可用的DepMap发布版本")
        return
    
    print("\n=== 可用的DepMap发布版本 ===")
    print("编号 | 版本名称 | 发布日期 | 文件数")
    print("-" * 60)
    
    for i, release in enumerate(releases, 1):
        title = release['title']
        pub_date = release.get('published_date', '未知')
        pub_date = datetime.strptime(pub_date, '%Y-%m-%dT%H:%M:%SZ').strftime('%Y-%m-%d') if pub_date != '未知' else '未知'
        
        # 尝试获取文件数量
        file_count = "未知"
        try:
            downloader = DepMapDataDownloader()
            if 'figshare.com' in release.get('url', ''):
                files = downloader.release_manager.get_figshare_files(release['id'])
                file_count = str(len(files))
        except:
            pass
        
        print(f"{i:2d} | {title:<20} | {pub_date:<10} | {file_count:>6}")

def main():
    """
    主函数
    """
    print("=== DepMap 数据下载器 ===")
    
    # 初始化版本管理器
    release_manager = DepMapReleaseManager()
    
    # 获取版本列表
    print("正在获取可用的DepMap发布版本...")
    releases = release_manager.get_figshare_releases()
    
    if not releases:
        print("无法从Figshare获取版本列表，将只提供当前版本下载选项")
        releases = []
    
    # 显示选项
    print("\n请选择下载选项:")
    print("1. 下载当前最新版本")
    
    if releases:
        print("2. 从Figshare下载特定历史版本")
        print("3. 查看所有可用版本详情")
    
    choice = input("\n请输入选择 (1-3): ").strip()
    
    downloader = DepMapDataDownloader(max_workers=4)
    
    if choice == "1":
        print("\n=== 下载当前最新版本 ===")
        downloader.download_current_release(max_files=5)  # 限制5个文件用于演示
        
    elif choice == "2" and releases:
        display_releases(releases)
        
        try:
            release_choice = int(input("\n请选择要下载的版本编号: "))
            if 1 <= release_choice <= len(releases):
                selected_release = releases[release_choice - 1]
                print(f"\n=== 下载版本: {selected_release['title']} ===")
                downloader.download_figshare_release(selected_release, max_files=5)  # 限制5个文件用于演示
            else:
                print("无效选择")
        except ValueError:
            print("请输入有效的数字")
    
    elif choice == "3" and releases:
        display_releases(releases)
        
        # 显示详细信息
        try:
            detail_choice = int(input("\n输入版本编号查看详细信息 (输入0返回): "))
            if detail_choice == 0:
                return
            elif 1 <= detail_choice <= len(releases):
                selected_release = releases[detail_choice - 1]
                print(f"\n=== 版本详细信息: {selected_release['title']} ===")
                print(f"URL: {selected_release['url']}")
                print(f"发布日期: {selected_release.get('published_date', '未知')}")
                
                # 获取文件列表
                files = downloader.release_manager.get_figshare_files(selected_release['id'])
                if files:
                    print(f"文件数量: {len(files)}")
                    print("\n前10个文件:")
                    for i, file in enumerate(files[:10], 1):
                        name = file.get('name', '未知')
                        size = file.get('size', 0)
                        size_mb = size / (1024 * 1024) if size else 0
                        print(f"{i:2d}. {name:<40} {size_mb:>8.2f} MB")
                    
                    if len(files) > 10:
                        print(f"... 还有 {len(files) - 10} 个文件")
                else:
                    print("无法获取文件列表")
            else:
                print("无效选择")
        except ValueError:
            print("请输入有效的数字")
    
    else:
        print("无效选择")

if __name__ == "__main__":
    main()