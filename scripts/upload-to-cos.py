#!/usr/bin/env python3
"""
上传 Release 产物到腾讯云 COS（国内加速分发）。

替代 coscmd：使用官方 cos-python-sdk-v5，针对跨洲链路（GitHub Actions 美国节点
→ 腾讯云广州区）做以下加固：
  - 大文件自动分块上传（MB 级阈值）
  - 断点续传（失败块自动重试，不重传已完成块）
  - 单块超时 + 全局重试次数控制
  - 失败文件整体重试（最多 3 次）

用法：
  python scripts/upload-to-cos.py <dist-dir> <version>

参数：
  <dist-dir>  gh release download 的产物目录（含安装包、.sig、latest.json）
  <version>   版本号（如 v0.1.1），安装包上传到 /<version>/ 子目录

环境变量（由 GitHub Secrets 注入）：
  COS_SECRET_ID     腾讯云 SecretId
  COS_SECRET_KEY    腾讯云 SecretKey
  COS_BUCKET        存储桶名（如 lingjian-releases-1433733625）
  COS_REGION        地域（如 ap-guangzhou）

目录结构：
  /latest.json              ← 永远根目录（Tauri updater endpoint 固定）
  /<version>/xxx.exe        ← 安装包按版本归档
  /<version>/xxx.exe.sig
"""
import os
import sys
import time
import logging
from qcloud_cos import CosConfig, CosS3Client

# 上传策略参数（针对跨洲不稳定链路调优）
PART_SIZE = 5 * 1024 * 1024      # 分块大小 5MB（COS 最小分块）
SINGLE_FILE_RETRY = 3            # 单个文件失败后的整体重试次数
RETRY_INTERVAL = 5               # 重试间隔（秒）

# SDK 内部日志（便于排查分块上传失败）
logging.basicConfig(level=logging.INFO, format='%(asctime)s %(levelname)s %(message)s')
logger = logging.getLogger('upload-to-cos')


def build_client():
    """构造 COS 客户端，超时参数针对跨洲链路放宽。"""
    secret_id = os.environ.get('COS_SECRET_ID')
    secret_key = os.environ.get('COS_SECRET_KEY')
    bucket = os.environ.get('COS_BUCKET')
    region = os.environ.get('COS_REGION', 'ap-guangzhou')

    if not all([secret_id, secret_key, bucket]):
        logger.error('缺少环境变量：COS_SECRET_ID / COS_SECRET_KEY / COS_BUCKET')
        sys.exit(1)

    # scheme + 超时：连接 60s（跨洲握手慢），读写 60s
    # Endpoint 指向全球加速域名，CI 跨洲上传走最近接入点
    config = CosConfig(
        Region=region,
        SecretId=secret_id,
        SecretKey=secret_key,
        Scheme='https',
        Timeout=60,
        Endpoint='cos.accelerate.myqcloud.com',
    )
    return CosS3Client(config), bucket


def upload_one(client, bucket, local_path, cos_key):
    """
    上传单个文件，带断点续传 + 重试。

    - 文件 <= 10MB：直接 put_object（简单上传）
    - 文件 > 10MB：分块上传（自动断点续传）
    每次失败后整体重试，最多 SINGLE_FILE_RETRY 次。
    """
    size = os.path.getsize(local_path)
    size_mb = size / (1024 * 1024)
    last_err = None

    for attempt in range(1, SINGLE_FILE_RETRY + 1):
        try:
            logger.info(f'[{attempt}/{SINGLE_FILE_RETRY}] {os.path.basename(local_path)} '
                        f'({size_mb:.1f}MB) → cos://{bucket}/{cos_key}')

            # 大文件走分块上传（CI 环境无持久断点续传上下文，但分块本身
            # 能降低单次请求时长，超时概率远低于整文件上传）
            client.upload_file(
                Bucket=bucket,
                Key=cos_key,
                LocalFilePath=local_path,
                PartSize=PART_SIZE,
                EnableMD5=True,
            )
            logger.info(f'  ✓ 上传成功')
            return True
        except Exception as e:
            last_err = e
            logger.warning(f'  ✗ 第 {attempt} 次失败: {e}')
            if attempt < SINGLE_FILE_RETRY:
                time.sleep(RETRY_INTERVAL)

    logger.error(f'  ✗✗ {os.path.basename(local_path)} 重试 {SINGLE_FILE_RETRY} 次后仍失败')
    return False


def main():
    if len(sys.argv) < 3:
        print(f'用法: {sys.argv[0]} <dist-dir> <version>', file=sys.stderr)
        sys.exit(1)

    dist_dir = sys.argv[1]
    version = sys.argv[2].lstrip('/')
    if not os.path.isdir(dist_dir):
        logger.error(f'目录不存在: {dist_dir}')
        sys.exit(1)
    if not version:
        logger.error('版本号不能为空')
        sys.exit(1)

    client, bucket = build_client()

    # 收集待上传文件，按规则分配 COS 路径：
    #   latest.json → 根目录（Tauri updater endpoint 固定）
    #   其他产物   → /<version>/ 子目录（按版本归档，避免根目录混乱）
    files = []
    for name in sorted(os.listdir(dist_dir)):
        path = os.path.join(dist_dir, name)
        if not os.path.isfile(path) or os.path.getsize(path) == 0:
            continue
        if name == 'latest.json':
            cos_key = '/' + name
        else:
            cos_key = f'/{version}/{name}'
        files.append((path, cos_key))

    if not files:
        logger.error(f'目录中无可上传文件: {dist_dir}')
        sys.exit(1)

    logger.info(f'共 {len(files)} 个文件待上传（版本目录: /{version}/）')
    failed = []
    for local_path, cos_key in files:
        ok = upload_one(client, bucket, local_path, cos_key)
        if not ok:
            failed.append(os.path.basename(local_path))

    if failed:
        logger.error(f'✗ 以下文件上传失败: {", ".join(failed)}')
        sys.exit(1)

    logger.info(f'✓ COS 同步完成（{len(files)} 个文件）')


if __name__ == '__main__':
    main()
