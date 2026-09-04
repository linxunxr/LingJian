use flate2::read::GzDecoder;
use regex::Regex;
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use crate::models::log_entry::{LogEntry, LogLevel};
use crate::services::github::{IssueActionResponse, IssueInfo, IssueList};

/// 将 SCF 响应反序列化为 T，失败时把响应原文带进错误信息，便于诊断。
///
/// 不用 `resp.json()` 的原因：它内部消费了 body，解码失败时拿不到原文；
/// 改为先 `bytes()` 取完整响应体再 `serde_json::from_slice`，这样错误信息里
/// 能直接展示 SCF 实际返回了什么（HTML 错误页 / 降级对象 / 缺字段等）。
async fn decode_scf_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, String> {
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取 SCF 响应失败: {e}"))?;
    serde_json::from_slice::<T>(&bytes).map_err(|e| {
        // 响应原文做有损转码并截断，确保错误信息可读且不超长
        let preview = String::from_utf8_lossy(&bytes);
        let snippet = if preview.len() > 500 {
            // 安全截断到 UTF-8 字符边界，避免 panic
            let mut end = 500;
            while end < preview.len() && !preview.is_char_boundary(end) {
                end += 1;
            }
            format!("{}...(共{}字节)", &preview[..end], bytes.len())
        } else {
            preview.into_owned()
        };
        format!("解析 SCF 响应失败: {e}（原始响应: {snippet}）")
    })
}

/// SCF 下载端点返回的 gzip 包解压后的 JSON 结构
///
/// 兼容两种顶层格式：
///   - 包裹对象 `{ "exportedAt": "...", "logs": [...] }`（推荐）
///   - 裸数组 `[...]`（部分上游直接 stringify 数组，无包裹）
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LogPayload {
    Wrapped {
        #[serde(default)]
        #[allow(dead_code)]
        exported_at: Option<String>,
        logs: Vec<RawLog>,
    },
    Bare(Vec<RawLog>),
}

impl LogPayload {
    fn into_logs(self) -> Vec<RawLog> {
        match self {
            LogPayload::Wrapped { logs, .. } => logs,
            LogPayload::Bare(v) => v,
        }
    }
}

/// 原始日志字段，兼容上游可能存在的字段名差异
///
/// tag 解析优先级：`tag` > `category` > `module` > "未知"
/// （不同上游分别用 tag / category / module 表示模块/功能标签）
#[derive(Debug, Deserialize)]
struct RawLog {
    timestamp: String,
    level: String,
    #[serde(default)]
    tag: String,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
    /// 兼容上游可能用 module 而非 tag
    #[serde(default)]
    module: Option<String>,
    /// 兼容上游可能用 category 而非 tag
    #[serde(default)]
    category: Option<String>,
}

impl RawLog {
    fn into_entry(self) -> Result<LogEntry, String> {
        // 未知级别降级为 INFO（兼容上游新增 TRACE 等），
        // 避免单条异常级别导致整份日志解析失败
        let level = LogLevel::parse(&self.level).unwrap_or(LogLevel::Info);
        // tag 优先级：tag > category > module > "未知"
        let tag = if !self.tag.is_empty() {
            self.tag
        } else if let Some(c) = self.category {
            c
        } else {
            self.module.unwrap_or_else(|| "未知".to_string())
        };
        Ok(LogEntry {
            timestamp: self.timestamp,
            level,
            tag,
            message: self.message,
            data: self.data,
        })
    }
}

/// 通过 SCF `/issue/:number` 端点解析 Issue，拿到 reportId 及元信息。
///
/// SCF 服务端用自身 GITHUB_TOKEN 调 GitHub API，提取 Issue body 中的
/// REPORT_ID 注释及环境信息，避免客户端直连 GitHub、无需用户配置 Token。
///
/// - `scf_url`：SCF 函数 URL 根地址
/// - `number`：Issue 编号
/// - `api_key`：与下载端点同一把 X-API-Key
pub async fn resolve_issue(
    scf_url: &str,
    number: u32,
    api_key: &str,
    http: &reqwest::Client,
) -> Result<IssueInfo, String> {
    let url = format!(
        "{}/issue/{}",
        scf_url.trim_end_matches('/'),
        number
    );
    let resp = http
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("连接 SCF 失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        // 尝试从响应体取 error 字段做更友好的提示
        let text = resp.text().await.unwrap_or_default();
        let detail = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(text);
        return Err(match status.as_u16() {
            401 => format!("SCF 鉴权失败（API Key 无效）: {detail}"),
            404 => format!("Issue 不存在或未包含上报编号: {detail}"),
            502 => format!("SCF 上游（GitHub）故障: {detail}"),
            other => format!("SCF 返回 {other}: {detail}"),
        });
    }

    decode_scf_response::<IssueInfo>(resp).await
}

/// 通过 SCF `/issues` 端点拉取上报问题列表。
///
/// 服务端调 GitHub API 列出带 has-logs 标签的 Issue，已过滤 PR 与无 reportId 的项。
///
/// - `scf_url`：SCF 函数 URL 根地址
/// - `state`：状态筛选，"open" 或 "all"
/// - `page`：页码，从 1 开始
/// - `api_key`：与下载端点同一把 X-API-Key
pub async fn list_issues(
    scf_url: &str,
    state: &str,
    page: u32,
    api_key: &str,
    http: &reqwest::Client,
) -> Result<IssueList, String> {
    let url = format!(
        "{}/issues?state={}&page={}",
        scf_url.trim_end_matches('/'),
        state,
        page
    );
    let resp = http
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("连接 SCF 失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let detail = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(text);
        return Err(match status.as_u16() {
            401 => format!("SCF 鉴权失败（API Key 无效）: {detail}"),
            502 => format!("SCF 上游（GitHub）故障: {detail}"),
            other => format!("SCF 返回 {other}: {detail}"),
        });
    }

    decode_scf_response::<IssueList>(resp).await
}

/// 通过 SCF `/issue/:number/action` 端点操作 Issue。
///
/// 统一入口，由 `action` 区分：close / reopen / comment / setLabels。
///
/// - `scf_url`：SCF 函数 URL 根地址
/// - `number`：Issue 编号
/// - `action`：操作类型
/// - `body`：评论内容（action=comment 时）
/// - `labels`：标签数组（action=setLabels 时，整体替换）
/// - `api_key`：与下载端点同一把 X-API-Key
pub async fn act_on_issue(
    scf_url: &str,
    number: u32,
    action: &str,
    body: Option<&str>,
    labels: Option<&[String]>,
    api_key: &str,
    http: &reqwest::Client,
) -> Result<IssueActionResponse, String> {
    let url = format!(
        "{}/issue/{}/action",
        scf_url.trim_end_matches('/'),
        number
    );

    // 构造请求体：只包含 action + 对应字段（避免传 null）
    let mut payload = serde_json::json!({ "action": action });
    if let Some(b) = body {
        payload["body"] = serde_json::Value::String(b.to_string());
    }
    if let Some(labels) = labels {
        payload["labels"] = serde_json::Value::Array(
            labels.iter().map(|l| serde_json::Value::String(l.clone())).collect(),
        );
    }

    let resp = http
        .post(&url)
        .header("X-API-Key", api_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("连接 SCF 失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let detail = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(text);
        return Err(match status.as_u16() {
            400 => format!("参数无效: {detail}"),
            401 => format!("SCF 鉴权失败（API Key 无效）: {detail}"),
            404 => format!("Issue 不存在: {detail}"),
            502 => format!("SCF 上游（GitHub）故障: {detail}"),
            other => format!("SCF 返回 {other}: {detail}"),
        });
    }

    decode_scf_response::<IssueActionResponse>(resp).await
}

/// 下载 gzip 日志包并解压为日志条目。
///
/// - `scf_url`：SCF 函数 URL 根地址
/// - `report_id`：上报编号
/// - `api_key`：下载端点鉴权密钥
/// - `http`：复用的 reqwest 客户端
/// - `cache_dir`：缓存目录，命中则跳过下载（离线优先）
///
/// 返回 `(日志条目, gzip 文件字节数)`
pub async fn download(
    scf_url: &str,
    report_id: &str,
    api_key: &str,
    http: &reqwest::Client,
    cache_dir: &Path,
) -> Result<(Vec<LogEntry>, u64), String> {
    // 1. 缓存命中优先（离线分析）
    let gz_path = cache_dir.join(format!("{report_id}.gz"));
    if gz_path.exists() {
        let bytes = std::fs::read(&gz_path)
            .map_err(|e| format!("读取缓存失败: {e}"))?;
        let entries = decode_gzip(&bytes)?;
        return Ok((entries, bytes.len() as u64));
    }

    // 2. 下载 .gz
    let url = format!("{}/logs/{}", scf_url.trim_end_matches('/'), report_id);
    let resp = http
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("下载失败 {status}: {text}"));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取响应体失败: {e}"))?;
    let file_size = bytes.len() as u64;

    // 3. 落盘缓存（失败不阻断本次分析）
    if let Err(e) = std::fs::create_dir_all(cache_dir)
        .and_then(|_| std::fs::write(&gz_path, &bytes))
    {
        log::warn!("缓存写入失败（不影响本次分析）: {e}");
    }

    // 4. 解压解析
    let entries = decode_gzip(&bytes)?;
    Ok((entries, file_size))
}

/// 截图 COS key 文件名白名单：{reportId}{_N}.png（与 SCF 端点约定一致）
///
/// 拼接本地缓存路径前先校验，防 `../` 等构造的 key 越权写盘。
fn screenshot_filename_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}(_\d+)?\.png$",
        )
        .unwrap()
    })
}

/// 从截图 COS key 提取白名单内的文件名（`screenshots/xxx.png` → `xxx.png`）
fn screenshot_filename(key: &str) -> Result<&str, String> {
    let filename = key.rsplit('/').next().unwrap_or("");
    if !screenshot_filename_re().is_match(filename) {
        return Err(format!("非法截图 key: {key}"));
    }
    Ok(filename)
}

/// 拉取单张反馈截图：本地缓存命中直接返回，否则经 SCF `/screenshots/:filename` 下载后落缓存。
///
/// 截图存于 `cache/screenshots/{filename}`（文件名含 reportId，天然按上报隔离），
/// 与 `.gz` 同目录 —— 清缓存 / 迁移数据目录的递归逻辑自动覆盖。
/// 返回 `(PNG 字节, 是否命中本地缓存)`。
pub async fn fetch_screenshot_bytes(
    scf_url: &str,
    key: &str,
    api_key: &str,
    http: &reqwest::Client,
    cache_dir: &Path,
) -> Result<(Vec<u8>, bool), String> {
    let filename = screenshot_filename(key)?;
    let local = cache_dir.join("screenshots").join(filename);
    if local.exists() {
        let bytes = std::fs::read(&local).map_err(|e| format!("读取截图缓存失败: {e}"))?;
        return Ok((bytes, true));
    }

    let url = format!("{}/screenshots/{}", scf_url.trim_end_matches('/'), filename);
    let resp = http
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("截图下载请求失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("截图下载失败 {status}: {text}"));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取截图响应失败: {e}"))?
        .to_vec();

    // 落盘缓存（失败不阻断本次展示）
    if let Some(parent) = local.parent() {
        if let Err(e) =
            std::fs::create_dir_all(parent).and_then(|_| std::fs::write(&local, &bytes))
        {
            log::warn!("截图缓存写入失败（不影响本次展示）: {e}");
        }
    }
    Ok((bytes, false))
}

/// 测试 SCF 下载端点连通性。
///
/// 用一个不存在的 reportId 发请求，预期返回 401（鉴权通过但资源不存在）
/// 或 404（资源不存在），二者都证明端点可达且鉴权配置正确。
/// 返回 401 且是鉴权失败则说明 API Key 错误。
pub async fn test_endpoint(
    scf_url: &str,
    api_key: &str,
    http: &reqwest::Client,
) -> Result<(), String> {
    // 用一个符合 UUID 格式但不存在的 id 测试
    let probe = "00000000-0000-0000-0000-000000000000";
    let url = format!("{}/logs/{}", scf_url.trim_end_matches('/'), probe);

    let resp = http
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("连接失败: {e}"))?;

    let status = resp.status().as_u16();
    match status {
        // 资源不存在 = 端点可达 + 鉴权通过
        404 => Ok(()),
        // 端点可达，但需区分鉴权失败
        401 => Err("API Key 无效".to_string()),
        // 某些实现可能用 403 表示鉴权失败
        403 => Err("API Key 无效或无权限".to_string()),
        // 意外命中真实数据（极低概率）也算通过
        200 => Ok(()),
        other => Err(format!("端点返回异常状态: {other}")),
    }
}

/// 解压 gzip 字节并解析为日志条目
///
/// 宽容兼容多种上游格式：
///   - 顶层：包裹对象 `{logs:[...]}` 或裸数组 `[...]`
///   - 字段：tag / category / module 均可作为模块标签
fn decode_gzip(bytes: &[u8]) -> Result<Vec<LogEntry>, String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut json_str = String::new();
    decoder
        .read_to_string(&mut json_str)
        .map_err(|e| format!("gzip 解压失败: {e}"))?;

    parse_json_logs(&json_str)
}

/// 解析 JSON 文本（gzip 解压后或本地 JSON 日志）为日志条目
///
/// 供 SCF 下载流程与本地导入（JSON 格式探测）共用。
pub fn parse_json_logs(text: &str) -> Result<Vec<LogEntry>, String> {
    let payload: LogPayload =
        serde_json::from_str(text).map_err(|e| format!("JSON 解析失败: {e}"))?;

    payload
        .into_logs()
        .into_iter()
        .map(RawLog::into_entry)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn gzip_json(json: &str) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn decode_basic_payload() {
        let json = r#"{"exportedAt":"2026-06-08T10:00:00Z","logs":[
            {"timestamp":"2026-06-08T14:35:22Z","level":"ERROR","tag":"战斗","message":"灵气溢出","data":{"v":-120}},
            {"timestamp":"2026-06-08T14:35:21Z","level":"warn","tag":"战斗","message":"灵气异常"}
        ]}"#;
        let gz = gzip_json(json);
        let entries = decode_gzip(&gz).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, LogLevel::Error);
        assert!(entries[0].data.is_some());
        assert_eq!(entries[1].level, LogLevel::Warn); // 小写级别兼容
    }

    #[test]
    fn decode_module_fallback() {
        // 上游用 module 而非 tag
        let json = r#"{"logs":[
            {"timestamp":"t","level":"INFO","module":"修炼","message":"入定"}
        ]}"#;
        let gz = gzip_json(json);
        let entries = decode_gzip(&gz).unwrap();
        assert_eq!(entries[0].tag, "修炼");
    }

    #[test]
    fn decode_fatal_level() {
        // FATAL 是合法级别（鸿蒙侧 F 级），不再导致解析失败
        let json = r#"{"logs":[
            {"timestamp":"t","level":"FATAL","tag":"崩溃","message":"native crash"}
        ]}"#;
        let gz = gzip_json(json);
        let entries = decode_gzip(&gz).unwrap();
        assert_eq!(entries[0].level, LogLevel::Fatal);
    }

    #[test]
    fn decode_unknown_level_tolerated() {
        // 完全未知的级别降级为 INFO，不阻断整份日志
        let json = r#"{"logs":[
            {"timestamp":"t1","level":"TRACE","tag":"a","message":"x"},
            {"timestamp":"t2","level":"ERROR","tag":"a","message":"y"}
        ]}"#;
        let gz = gzip_json(json);
        let entries = decode_gzip(&gz).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[1].level, LogLevel::Error);
    }

    #[test]
    fn decode_bare_array() {
        // 上游直接 stringify 数组，无 {logs: [...]} 包裹（SCF test-report.js 格式）
        let json = r#"[
            {"timestamp":"t1","level":"info","category":"系统","message":"游戏启动"},
            {"timestamp":"t2","level":"error","category":"网络","message":"请求超时"}
        ]"#;
        let gz = gzip_json(json);
        let entries = decode_gzip(&gz).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tag, "系统"); // category 回退为 tag
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[1].tag, "网络");
        assert_eq!(entries[1].level, LogLevel::Error);
    }

    #[test]
    fn decode_category_fallback() {
        // 包裹格式但用 category 而非 tag
        let json = r#"{"logs":[
            {"timestamp":"t","level":"WARN","category":"资源","message":"加载缓慢"}
        ]}"#;
        let gz = gzip_json(json);
        let entries = decode_gzip(&gz).unwrap();
        assert_eq!(entries[0].tag, "资源");
    }

    #[test]
    fn decode_tag_priority() {
        // tag > category > module：同时存在时 tag 优先
        let json = r#"{"logs":[
            {"timestamp":"t","level":"INFO","tag":"TAG","category":"CAT","module":"MOD","message":"m"}
        ]}"#;
        let gz = gzip_json(json);
        let entries = decode_gzip(&gz).unwrap();
        assert_eq!(entries[0].tag, "TAG");
    }

    #[test]
    fn decode_empty_logs() {
        // 空数组（合法但无日志）
        let gz = gzip_json("[]");
        let entries = decode_gzip(&gz).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn screenshot_filename_accepts_valid_keys() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(
            screenshot_filename(&format!("screenshots/{uuid}.png")).unwrap(),
            format!("{uuid}.png")
        );
        assert_eq!(
            screenshot_filename(&format!("screenshots/{uuid}_2.png")).unwrap(),
            format!("{uuid}_2.png")
        );
    }

    #[test]
    fn screenshot_filename_rejects_traversal() {
        // 路径遍历 / 非法扩展名 / 缺后缀 / 裸文件名均拒绝
        assert!(screenshot_filename("screenshots/../../logs/x.png").is_err());
        assert!(screenshot_filename("screenshots/550e8400-e29b-41d4-a716-446655440000.gz").is_err());
        assert!(screenshot_filename("screenshots/550e8400-e29b-41d4-a716-446655440000").is_err());
        assert!(screenshot_filename("not-a-key").is_err());
        assert!(screenshot_filename("").is_err());
    }
}
