use flate2::read::DeflateDecoder;
use flate2::read::GzDecoder;
use serde::Serialize;
use std::io::Read;
use std::path::Path;
use tauri::{async_runtime::spawn_blocking, State};

use crate::models::log_entry::LogEntry;
use crate::models::report::Report;
use crate::services::cache::Cache;
use crate::services::downloader;
use crate::services::text_log;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub report_id: String,
    pub log_count: usize,
    /// 原始文件字节数
    pub file_size: u64,
    /// 识别出的日志格式（hlog-text / json）
    pub format: String,
    /// 从文件名推断的应用名（baseName，如 "starlight"）
    pub app_name: Option<String>,
}

/// 导入本地日志文件：读取 → 压缩探测解压 → 格式探测解析 → 入库
///
/// 支持三种输入：
///   - huanyinfeng/logger 落盘/导出的纯文本（`.log` / `.txt`）
///   - 该库 compressTo 导出的 zlib 单流（文件名常为 `.zip` 但并非 zip 归档）
///   - gzip 压缩的 JSON 日志（与 SCF 下载包同构）
///
/// 未识别的压缩格式（真 zip 等）返回错误并提示。
#[tauri::command]
pub async fn import_log_file(
    path: String,
    state: State<'_, crate::AppState>,
) -> Result<ImportResult, String> {
    let file_path = Path::new(&path);
    let meta = std::fs::metadata(file_path)
        .map_err(|e| format!("无法读取文件: {e}"))?;
    if meta.is_dir() {
        return Err("路径是目录，请选择日志文件（可多选后逐个导入）".to_string());
    }
    let file_size = meta.len();

    // 读取 + 解压 + 解析（IO/CPU 密集，走阻塞线程池）
    let p = file_path.to_path_buf();
    let (entries, format) = spawn_blocking(move || -> Result<(Vec<LogEntry>, String), String> {
        let bytes = std::fs::read(&p).map_err(|e| format!("读取文件失败: {e}"))?;
        let text = decode_to_text(&bytes)?;
        parse_detected(&text)
    })
    .await
    .map_err(|e| format!("导入任务失败: {e}"))??;
    let log_count = entries.len();

    let app_name = infer_app_name(file_path);
    let now = chrono::Utc::now().to_rfc3339();
    let report_id = format!("local-{}", uuid::Uuid::new_v4());
    let report = Report {
        report_id: report_id.clone(),
        issue_number: None,
        issue_title: file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned()),
        app_name: app_name.clone(),
        app_version: None,
        platform: Some("HarmonyOS".to_string()),
        realm: None,
        play_time: None,
        user_description: None,
        screenshot_keys: None,
        report_time: now.clone(),
        log_count,
        downloaded_at: now,
    };

    let cache: std::sync::Arc<Cache> = state.cache.clone();
    spawn_blocking(move || cache.save_report(&report, &entries))
        .await
        .map_err(|e| format!("入库任务失败: {e}"))??;

    Ok(ImportResult {
        report_id,
        log_count,
        file_size,
        format,
        app_name,
    })
}

/// 按魔数探测压缩格式并解压为文本；非压缩则按 UTF-8 有损转字符串
///
/// - gzip：RFC 1952，魔数 `1f 8b`
/// - zlib：RFC 1950（huanyinfeng/logger compressTo 产物），首字节 CMF=0x78
///   （ deflate 方法位恒为 8；窗口档位 07..0f，即首字节 0x38..0xff 中
///   低 4 位为 8 的组合，为避免误判仅认常见档位 0x08/0x18/0x28/../0x78/0x98）
fn decode_to_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut out = String::new();
        GzDecoder::new(bytes)
            .read_to_string(&mut out)
            .map_err(|e| format!("gzip 解压失败: {e}"))?;
        return Ok(out);
    }
    // zlib CMF/FLG：((CMF << 8) + FLG) % 31 == 0 且 (CMF & 0x0f) == 8
    if bytes.len() >= 2 {
        let cmf = bytes[0];
        let flg = bytes[1];
        if cmf & 0x0f == 8 && ((cmf as u16) << 8 | flg as u16) % 31 == 0 {
            let mut out = String::new();
            DeflateDecoder::new(&bytes[2..])
                .read_to_string(&mut out)
                .map_err(|_| {
                    "zlib 解压失败：文件可能不是 zlib 单流（真 zip 归档请先解压出其中的日志文件再导入）"
                        .to_string()
                })?;
            return Ok(out);
        }
    }
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// 格式探测：先按 huanyinfeng/logger 文本行解析，失败再尝试 JSON
fn parse_detected(text: &str) -> Result<(Vec<LogEntry>, String), String> {
    if let Ok(entries) = text_log::parse_hlog_text(text) {
        return Ok((entries, "hlog-text".to_string()));
    }
    if let Ok(entries) = downloader::parse_json_logs(text) {
        if !entries.is_empty() {
            return Ok((entries, "json".to_string()));
        }
    }
    Err("未识别的日志格式：既不是 huanyinfeng/logger 文本行，也不是 JSON 日志".to_string())
}

/// 从文件名推断应用名（baseName）
///
/// 该库落盘命名 `{baseName}-yyyy-MM-dd.log(.N)`、导出 `starlight_log_yyyyMMdd_HHmm.zip`
/// 之类，取第一个 `-`/`_` 前的段；`app.log`（无日期）返回 "app"。
fn infer_app_name(path: &Path) -> Option<String> {
    let name = path.file_stem()?.to_string_lossy().into_owned();
    if name.is_empty() {
        return None;
    }
    let base = name
        .split(['-', '_'])
        .next()
        .unwrap_or(&name)
        .trim()
        .to_string();
    if base.is_empty() {
        None
    } else {
        Some(base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_plain_text_passthrough() {
        let text = "2026-08-09 10:30:20.364 [I/A] hello\n";
        assert_eq!(decode_to_text(text.as_bytes()).unwrap(), text);
    }

    #[test]
    fn decode_zlib_stream() {
        // 模拟 huanyinfeng/logger compressTo 的 zlib 单流（非 zip 容器）
        let raw = b"2026-08-09 10:30:20.364 [I/Player] playing\n2026-08-09 10:30:21.000 [W/Scan] slow\n";
        let mut zlib = vec![0x78, 0x9c]; // CMF=0x78 FLG=0x9c，校验 (0x789c % 31 == 0)
        let mut deflated = Vec::new();
        let mut enc = flate2::write::DeflateEncoder::new(&mut deflated, flate2::Compression::default());
        use std::io::Write;
        enc.write_all(raw).unwrap();
        enc.finish().unwrap();
        zlib.extend_from_slice(&deflated);
        // adler32 校验和（大端）
        let adler = adler32(raw);
        zlib.extend_from_slice(&adler.to_be_bytes());

        let text = decode_to_text(&zlib).unwrap();
        assert!(text.contains("[I/Player] playing"));
        assert!(text.contains("[W/Scan] slow"));
    }

    #[test]
    fn decode_gzip_stream() {
        let raw = b"hello";
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        use std::io::Write;
        enc.write_all(raw).unwrap();
        let gz = enc.finish().unwrap();
        assert_eq!(decode_to_text(&gz).unwrap(), "hello");
    }

    #[test]
    fn decode_invalid_zip_errors_helpfully() {
        // 真 zip 魔数 PK，不匹配 zlib 校验，按文本损转换后在解析阶段报格式错误
        let bytes = vec![0x50u8, 0x4b, 0x03, 0x04, 0x00];
        let text = decode_to_text(&bytes).unwrap(); // 不报错，降级为文本
        assert!(parse_detected(&text).is_err()); // 解析阶段给出格式提示
    }

    #[test]
    fn parse_prefers_hlog_over_json() {
        let text = "2026-08-09 10:30:20.364 [I/A] m\n";
        let (_, fmt) = parse_detected(text).unwrap();
        assert_eq!(fmt, "hlog-text");
    }

    #[test]
    fn parse_falls_back_to_json() {
        let json = r#"{"logs":[{"timestamp":"t","level":"INFO","tag":"a","message":"m"}]}"#;
        let (entries, fmt) = parse_detected(json).unwrap();
        assert_eq!(fmt, "json");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn infer_app_name_from_dated_file() {
        assert_eq!(
            infer_app_name(Path::new("C:/logs/starlight-2026-08-11.log")),
            Some("starlight".to_string())
        );
        assert_eq!(infer_app_name(Path::new("app.log")), Some("app".to_string()));
        assert_eq!(infer_app_name(Path::new("starlight_log_20260811_1010.zip")), Some("starlight".to_string()));
    }

    /// 简易 adler32（与 RFC 1950 一致），仅供测试构造 zlib 流
    fn adler32(data: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in data {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
}
