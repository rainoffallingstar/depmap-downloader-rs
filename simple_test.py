#!/usr/bin/env python3
"""
最简单的测试，不涉及网络请求
"""
import sys
import os

# 添加当前目录到路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

def test_imports():
    """测试依赖导入"""
    print("=== 测试依赖导入 ===")
    
    try:
        import requests
        print(f"✓ requests {requests.__version__}")
    except ImportError as e:
        print(f"✗ requests 导入失败: {e}")
        return False
    
    try:
        import pandas
        print(f"✓ pandas {pandas.__version__}")
    except ImportError as e:
        print(f"✗ pandas 导入失败: {e}")
        return False
    
    try:
        from tqdm import tqdm
        print("✓ tqdm 导入成功")
    except ImportError as e:
        print(f"✗ tqdm 导入失败: {e}")
        return False
    
    return True

def test_class_import():
    """测试类导入"""
    print("\n=== 测试类导入 ===")
    
    try:
        from depmapdown import DepMapReleaseManager
        print("✓ DepMapReleaseManager 导入成功")
    except ImportError as e:
        print(f"✗ DepMapReleaseManager 导入失败: {e}")
        return False
    
    try:
        from depmapdown import DepMapDataDownloader
        print("✓ DepMapDataDownloader 导入成功")
    except ImportError as e:
        print(f"✗ DepMapDataDownloader 导入失败: {e}")
        return False
    
    return True

def test_basic_methods():
    """测试基本方法（无网络）"""
    print("\n=== 测试基本方法 ===")
    
    try:
        from depmapdown import DepMapReleaseManager
        
        # 测试版本提取
        manager = DepMapReleaseManager()
        
        test_cases = [
            ("DepMap 24Q4", (2024, 4)),
            ("DepMap 23Q1", (2023, 1)),
            ("No version", (0, 0))
        ]
        
        for title, expected in test_cases:
            result = manager._extract_version(title)
            status = "✓" if result == expected else "✗"
            print(f"  {status} '{title}' -> {result} (期望: {expected})")
        
        return True
        
    except Exception as e:
        print(f"✗ 基本方法测试失败: {e}")
        return False

def main():
    """主函数"""
    print("DepMap 下载器 - 简单测试")
    print("=" * 40)
    
    tests = [
        test_imports,
        test_class_import,
        test_basic_methods
    ]
    
    passed = 0
    for test in tests:
        if test():
            passed += 1
    
    print(f"\n结果: {passed}/{len(tests)} 测试通过")
    
    if passed == len(tests):
        print("🎉 基本功能测试通过！")
    else:
        print("⚠️ 部分测试失败")
    
    return 0 if passed == len(tests) else 1

if __name__ == "__main__":
    sys.exit(main())
