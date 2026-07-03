#!/usr/bin/env python3
"""
上传 Release 产物到腾讯云 COS（国内加速分发）。

使用官方 cos-python-sdk-v5，针对跨洲链路（GitHub Actions 美国节点 → 腾讯云广州区）
网络波动场景做多层加固：

  ① SDK 内置分块级重试：upload_file 自动分块，单块失败 SDK 内部重试
  ② 外层文件级重试：整文件失败后重试，带指数退避避开网络抖动窗口
  ③ 碎片清理：失败后 abort COS 上残留的 multipart upload（避免占存储 + 计费）
  ④ 进度可见：progress_callback 输出每个文件的上传百分比到 CI 日志

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
from qcloud_cos.cos_exception import CosClientError, CosServiceError

# ---- 上传策略参数（针对跨洲不稳定链路调优）----
# 实测跨洲链路（GitHub Actions 美国节点 → 腾讯云广州）有效速率约 30~50 KB/s，
# 因此用「大分块 + 低并发 + 长 timeout」策略：
#   - 分块加大：减少分块数与协调往返，单块一次性吃满带宽
#   - 并发降低：避免 5 个连接互相抢占本就有限的跨洲带宽
#   - timeout 拉长：给单块足够时间写完（详见 build_client 注释）
PART_SIZE = 8          # 分块大小 MB（5→8：减少分块数；7~8MB 文件可单块完成）
MAX_THREAD = 2         # 并发上传线程数（5→2：跨洲带宽小，降并发避免互相拖慢）
MAX_RETRY = 6          # 单文件最大重试次数（带指数退避）
BASE_INTERVAL = 5      # 首次重试间隔（秒），后续指数退避：5, 10, 20, 40, 80
SMALL_FILE_THRESHOLD = 1024 * 1024  # ≤1MB 用简单上传（put_object），无需分块

# 普通域名连续失败 N 次后，自动切换到全球加速域名（cos.accelerate.myqcloud.com）。
# 全球加速走腾讯内部优化路由，对跨洲上传（美国→广州）非常有效，但会产生少量加速费用，
# 因此只在确认是持续性慢链路（而非瞬时抖动）后才启用：
#   - 实测瞬时抖动（如 dmg 案例）在第 2 次普通续传就能成功 → 不会误触发加速
#   - 持续性慢链路（如 AppImage 连续失败）能及时换路，避免后续无效重试
ACCELERATE_THRESHOLD = 3

logging.basicConfig(level=logging.INFO, format='%(asctime)s %(levelname)s %(message)s')
logger = logging.getLogger('upload-to-cos')

# 压制 cos-python-sdk-v5 的内部噪音：
#   SDK 在 send_request 重试失败时会 logger.exception() 打印整段 retry_time 列表
#   （每次 4 条 + 完整 traceback，共 40+ 行），单文件多次重试就把 CI 日志刷爆。
#   提到 WARNING 后只保留它的警告/错误摘要，我们自己的进度日志（INFO）不受影响。
logging.getLogger('qcloud_cos').setLevel(logging.WARNING)


def build_client():
    """构造 COS 客户端，超时参数针对跨洲链路放宽。"""
    secret_id = os.environ.get('COS_SECRET_ID')
    secret_key = os.environ.get('COS_SECRET_KEY')
    bucket = os.environ.get('COS_BUCKET')
    region = os.environ.get('COS_REGION', 'ap-guangzhou')

    if not all([secret_id, secret_key, bucket]):
        logger.error('缺少环境变量：COS_SECRET_ID / COS_SECRET_KEY / COS_BUCKET')
        sys.exit(1)

    # ⚠️ Timeout 必须够大，且不能像之前那样设 60s：
    #   cos-python-sdk-v5 的 send_request 逻辑是「一旦用户设置了 _timeout，
    #   就用它覆盖所有请求的默认 timeout」——包括原本默认 1200s 的
    #   complete-multipart，以及 upload_part 的数据 PUT。
    #   跨洲慢链路（~30-50KB/s）写完一个 8MB 分块需 3~4 分钟，
    #   Timeout=60 会让单块 PUT 在写完前就被强制中断 → write operation timed out。
    #   600s 对 part 写入够用，对秒级的 complete/put_object 也无副作用。
    #
    # 普通地域域名（全球加速已关闭），跨洲上传靠分块 + 重试兜底
    config = CosConfig(
        Region=region,
        SecretId=secret_id,
        SecretKey=secret_key,
        Scheme='https',
        Timeout=600,
    )
    return CosS3Client(config), bucket


def build_accelerated_client(secret_id, secret_key):
    """
    构造走全球加速域名的客户端。

    普通域名连续失败 ACCELERATE_THRESHOLD 次后调用，切换到腾讯内部优化路由
    （cos.accelerate.myqcloud.com），对跨洲链路（美国→广州）尤其有效。

    ⚠️ 前置条件：需先在腾讯云 COS 控制台为存储桶开启「全球加速」
       （存储桶 → 域名与传输管理 → 全球加速域名 → 开启，约 15 分钟生效）。
       若未开启，加速请求会返回错误，此时不影响主流程——异常会被 upload_one
       的重试逻辑捕获，最终回退到普通域名或判定失败。

    SDK 的 format_endpoint 逻辑：传 Endpoint 时直接使用，不再用 Region 拼域名，
    因此这里不传 Region，仅通过 Endpoint 指定加速域名。最终请求 URL 形如：
      https://<bucket-appid>.cos.accelerate.myqcloud.com/<key>
    """
    config = CosConfig(
        Appid=None,
        SecretId=secret_id,
        SecretKey=secret_key,
        Scheme='https',
        Timeout=600,
        Endpoint='cos.accelerate.myqcloud.com',
    )
    return CosS3Client(config)


def abort_residual_multipart(client, bucket, cos_key):
    """
    清理 COS 上某 Key 的残留分块上传。

    upload_file 失败后，COS 端可能残留 initialized multipart upload（碎片），
    占用存储并产生费用。重试前先 abort 掉所有进行中的 upload。
    """
    try:
        resp = client.list_multipart_uploads(Bucket=bucket, Prefix=cos_key)
        uploads = resp.get('Upload', [])
        for u in uploads:
            uid = u.get('UploadId')
            ukey = u.get('Key')
            if uid and ukey == cos_key.lstrip('/'):
                client.abort_multipart_upload(Bucket=bucket, Key=ukey, UploadId=uid)
                logger.info(f'  ↻ 清理残留分块上传 UploadId={uid[:12]}...')
    except Exception as e:
        # 清理失败不影响主流程（最多留点碎片，COS 生命周期规则可兜底）
        logger.debug(f'  清理碎片跳过: {e}')


def make_progress_callback(filename, total_bytes_holder):
    """构造进度回调，输出百分比到 CI 日志（每 20% 打一行，避免刷屏）。"""
    state = {'last_pct': -1}

    def cb(consumed, total):
        total_bytes_holder['size'] = total
        if total <= 0:
            return
        pct = int(100 * consumed / total)
        # 每 20% 打印一次（0/20/40/60/80/100）
        bucket = (pct // 20) * 20
        if bucket > state['last_pct']:
            state['last_pct'] = bucket
            logger.info(f'    {filename} 上传进度: {pct}%')

    return cb


def upload_small_file(client, bucket, local_path, cos_key):
    """小文件（≤1MB）用简单上传，一次 HTTP 完成。"""
    filename = os.path.basename(local_path)
    client.put_object(
        Bucket=bucket,
        Key=cos_key,
        Body=open(local_path, 'rb').read(),
        EnableMD5=True,
    )
    logger.info(f'  ✓ {filename} 上传成功（简单上传）')


def upload_large_file(client, bucket, local_path, cos_key):
    """
    大文件用分块上传（SDK upload_file），带进度回调。

    SDK 内部自动：分块 → 逐块上传 → 失败块重试 → 合并。
    外层调用方负责失败后的整体重试与碎片清理。
    """
    filename = os.path.basename(local_path)
    size_mb = os.path.getsize(local_path) / (1024 * 1024)
    total_holder = {'size': 0}
    progress_cb = make_progress_callback(filename, total_holder)

    client.upload_file(
        Bucket=bucket,
        Key=cos_key,
        LocalFilePath=local_path,
        PartSize=PART_SIZE,
        MAXThread=MAX_THREAD,
        EnableMD5=True,
        progress_callback=progress_cb,
    )
    logger.info(f'  ✓ {filename} 上传成功（{size_mb:.1f}MB，分块 {PART_SIZE}MB）')


def upload_one(client, bucket, local_path, cos_key, file_idx=None, accelerated_client=None):
    """
    上传单个文件，带指数退避重试 + 碎片清理 + 自动切换加速域名。

    重试策略：
    - 小文件：put_object 失败直接重试（无碎片问题）
    - 大文件：upload_file 失败 → abort 残留碎片 → 退避等待 → 重试
    - 间隔指数退避：5s, 10s, 20s, 40s, 80s（避开网络抖动窗口）
    - 普通域名连续失败 ACCELERATE_THRESHOLD 次后，改用全球加速域名客户端重试

    :param file_idx: (序号, 总数) 元组，用于日志显示 [i/N] 文件进度
    :param accelerated_client: 全球加速域名客户端（None 表示不启用加速降级）
    """
    size = os.path.getsize(local_path)
    filename = os.path.basename(local_path)
    is_small = size <= SMALL_FILE_THRESHOLD
    last_err = None
    seq = f'[{file_idx[0]}/{file_idx[1]}] ' if file_idx else ''

    for attempt in range(1, MAX_RETRY + 1):
        # 第 1~ACCELERATE_THRESHOLD 次走普通域名；之后改走全球加速域名。
        # 切换的那一刻打一条醒目提示，方便 CI 日志追溯加速触发情况。
        use_accel = accelerated_client is not None and attempt > ACCELERATE_THRESHOLD
        active_client = accelerated_client if use_accel else client
        if use_accel and attempt == ACCELERATE_THRESHOLD + 1:
            logger.warning(f'  ⚡ 普通域名连续失败 {ACCELERATE_THRESHOLD} 次，切换全球加速域名重试')

        try:
            # 拼装日志前缀：[i/N] 文件序号 + [重试 N/M]（首次不标）+ [加速]（仅加速时）
            prefix = seq
            if attempt > 1:
                prefix += f'[重试 {attempt}/{MAX_RETRY}] '
            if use_accel:
                prefix += '[加速] '
            logger.info(f'{prefix}{filename} → {cos_key}')
            if is_small:
                upload_small_file(active_client, bucket, local_path, cos_key)
            else:
                upload_large_file(active_client, bucket, local_path, cos_key)
            return True

        except (CosClientError, CosServiceError) as e:
            last_err = e
            # 大文件失败后清理 COS 残留碎片（小文件无此问题）
            if not is_small:
                abort_residual_multipart(active_client, bucket, cos_key)

            if attempt < MAX_RETRY:
                interval = BASE_INTERVAL * (2 ** (attempt - 1))
                logger.warning(f'  ✗ 第 {attempt} 次失败: {e}')
                logger.info(f'  ⏳ {interval}s 后重试（指数退避）...')
                time.sleep(interval)
            else:
                logger.error(f'  ✗ {filename} 重试 {MAX_RETRY} 次后仍失败: {e}')

        except Exception as e:
            last_err = e
            if attempt < MAX_RETRY:
                interval = BASE_INTERVAL * (2 ** (attempt - 1))
                logger.warning(f'  ✗ 第 {attempt} 次异常: {e}，{interval}s 后重试...')
                time.sleep(interval)

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

    # 预构造全球加速域名客户端（仅普通域名连续失败时启用，故惰性使用）
    secret_id = os.environ.get('COS_SECRET_ID')
    secret_key = os.environ.get('COS_SECRET_KEY')
    accelerated_client = build_accelerated_client(secret_id, secret_key)

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
    total = len(files)
    for idx, (local_path, cos_key) in enumerate(files, start=1):
        ok = upload_one(
            client, bucket, local_path, cos_key,
            file_idx=(idx, total),
            accelerated_client=accelerated_client,
        )
        if not ok:
            failed.append(os.path.basename(local_path))

    if failed:
        logger.error(f'✗ 以下文件上传失败: {", ".join(failed)}')
        sys.exit(1)

    logger.info(f'✓ COS 同步完成（{len(files)} 个文件）')


if __name__ == '__main__':
    main()
