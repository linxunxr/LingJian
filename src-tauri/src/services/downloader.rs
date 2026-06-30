use flate2::read::GzDecoder;
use serde::Deserialize;
use std::io::Read;
use std::path::Path;

use crate::models::log_entry::{LogEntry, LogLevel};
use crate::services::github::{IssueInfo, IssueList};

/// SCF 下载端点返回的 gzip 包解压后的 JSON 结构
#[derive(Debug, Deserialize)]
struct LogPayload {
    #[serde(default)]
    #[allow(dead_code)]
    exported_at: Option<String>,
    logs: Vec<RawLog>,
}

/// 原始日志字段，兼容上游可能存在的字段名差异（level/tag/data 等）
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
}

impl RawLog {
    fn into_entry(self) -> Result<LogEntry, String> {
        let level = LogLevel::parse(&self.level)
            .ok_or_else(|| format!("未知日志级别: {}", self.level))?;
        // tag 优先，缺失时回退到 module
        let tag = if !self.tag.is_empty() {
            self.tag
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

    resp.json::<IssueInfo>()
        .await
        .map_err(|e| format!("解析 SCF 响应失败: {e}"))
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

    resp.json::<IssueList>()
        .await
        .map_err(|e| format!("解析 SCF 响应失败: {e}"))
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
fn decode_gzip(bytes: &[u8]) -> Result<Vec<LogEntry>, String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut json_str = String::new();
    decoder
        .read_to_string(&mut json_str)
        .map_err(|e| format!("gzip 解压失败: {e}"))?;

    let payload: LogPayload =
        serde_json::from_str(&json_str).map_err(|e| format!("JSON 解析失败: {e}"))?;

    payload
        .logs
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
    fn decode_invalid_level() {
        let json = r#"{"logs":[{"timestamp":"t","level":"FATAL","message":"x"}]}"#;
        let gz = gzip_json(json);
        assert!(decode_gzip(&gz).is_err());
    }
}
