use crate::models::log_entry::{LogEntry, LogLevel};
use crate::models::report::Report;

/// 导出格式
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Markdown,
    Json,
    Csv,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "markdown" | "md" => Ok(ExportFormat::Markdown),
            "json" => Ok(ExportFormat::Json),
            "csv" => Ok(ExportFormat::Csv),
            other => Err(format!("不支持的导出格式: {other}")),
        }
    }

    /// 文件扩展名
    #[allow(dead_code)]
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Markdown => "md",
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
        }
    }
}

/// 生成导出内容字符串
pub fn render(report: &Report, entries: &[LogEntry], format: ExportFormat) -> String {
    match format {
        ExportFormat::Markdown => render_markdown(report, entries),
        ExportFormat::Json => render_json(report, entries),
        ExportFormat::Csv => render_csv(entries),
    }
}

/// 生成 Markdown 报告
fn render_markdown(report: &Report, entries: &[LogEntry]) -> String {
    let mut md = String::new();
    md.push_str("# 灵鉴日志分析报告\n\n");

    // 元信息
    md.push_str("## 上报信息\n\n");
    md.push_str(&format!("| 项目 | 值 |\n|------|-----|\n"));
    md.push_str(&format!("| 上报编号 | `{}` |\n", report.report_id));
    if let Some(n) = report.issue_number {
        md.push_str(&format!("| Issue | #{} |\n", n));
    }
    if let Some(ref t) = report.issue_title {
        md.push_str(&format!("| 标题 | {} |\n", t));
    }
    if let Some(ref v) = report.app_version {
        md.push_str(&format!("| 游戏版本 | {} |\n", v));
    }
    if let Some(ref p) = report.platform {
        md.push_str(&format!("| 平台 | {} |\n", p));
    }
    md.push_str(&format!("| 上报时间 | {} |\n", report.report_time));
    md.push_str(&format!("| 日志条数 | {} |\n", report.log_count));
    md.push_str("\n");

    // 级别统计
    let (debug, info, warn, error) = count_levels(entries);
    md.push_str("## 级别统计\n\n");
    md.push_str(&format!(
        "| DEBUG | INFO | WARN | ERROR |\n|-------|------|------|-------|\n| {} | {} | {} | {} |\n\n",
        debug, info, warn, error
    ));

    // 错误聚合
    let error_entries: Vec<&LogEntry> = entries
        .iter()
        .filter(|e| matches!(e.level, LogLevel::Error))
        .collect();
    if !error_entries.is_empty() {
        md.push_str("## 错误聚合\n\n");
        for agg in aggregate_errors(&error_entries) {
            md.push_str(&format!(
                "- **{}**（{} 次）| 首次 {} | 末次 {}\n",
                agg.message, agg.count, agg.first_seen, agg.last_seen
            ));
        }
        md.push_str("\n");
    }

    // 完整日志
    md.push_str("## 完整日志\n\n");
    md.push_str("| 时间 | 级别 | 模块 | 消息 |\n");
    md.push_str("|------|------|------|------|\n");
    for e in entries {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            e.timestamp, e.level.as_str(), e.tag, e.message
        ));
    }

    md
}

/// 生成 JSON
fn render_json(report: &Report, entries: &[LogEntry]) -> String {
    // 复用 serde 序列化，输出带 report + logs 结构
    #[derive(serde::Serialize)]
    struct ExportPayload<'a> {
        report: &'a Report,
        logs: &'a [LogEntry],
    }
    let payload = ExportPayload { report, logs: entries };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|e| format!("序列化失败: {e}"))
}

/// 生成 CSV
fn render_csv(entries: &[LogEntry]) -> String {
    let mut csv = String::from("timestamp,level,tag,message\n");
    for e in entries {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            csv_escape(&e.timestamp),
            e.level.as_str(),
            csv_escape(&e.tag),
            csv_escape(&e.message)
        ));
    }
    csv
}

/// CSV 字段转义：含逗号/引号/换行时用双引号包裹并转义内部引号
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// 统计各级别数量
fn count_levels(entries: &[LogEntry]) -> (usize, usize, usize, usize) {
    let mut d = 0;
    let mut i = 0;
    let mut w = 0;
    let mut e = 0;
    for entry in entries {
        match entry.level {
            LogLevel::Debug => d += 1,
            LogLevel::Info => i += 1,
            LogLevel::Warn => w += 1,
            LogLevel::Error => e += 1,
        }
    }
    (d, i, w, e)
}

/// 错误聚合结构（导出专用，避免依赖 analyzer）
struct ErrorAgg {
    message: String,
    count: usize,
    first_seen: String,
    last_seen: String,
}

fn aggregate_errors(errors: &[&LogEntry]) -> Vec<ErrorAgg> {
    use std::collections::HashMap;
    let mut map: HashMap<String, ErrorAgg> = HashMap::new();
    for e in errors {
        map.entry(e.message.clone())
            .and_modify(|agg| {
                agg.count += 1;
                if e.timestamp < agg.first_seen {
                    agg.first_seen = e.timestamp.clone();
                }
                if e.timestamp > agg.last_seen {
                    agg.last_seen = e.timestamp.clone();
                }
            })
            .or_insert(ErrorAgg {
                message: e.message.clone(),
                count: 1,
                first_seen: e.timestamp.clone(),
                last_seen: e.timestamp.clone(),
            });
    }
    let mut result: Vec<ErrorAgg> = map.into_values().collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(level: LogLevel, msg: &str) -> LogEntry {
        LogEntry {
            timestamp: "2026-06-08T14:00:00Z".to_string(),
            level,
            tag: "战斗".to_string(),
            message: msg.to_string(),
            data: None,
        }
    }

    fn sample_report() -> Report {
        Report {
            report_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            issue_number: Some(42),
            issue_title: Some("测试".to_string()),
            app_version: Some("1.0".to_string()),
            platform: Some("electron".to_string()),
            realm: None,
            play_time: None,
            user_description: None,
            report_time: "2026-06-08T14:00:00Z".to_string(),
            log_count: 2,
            downloaded_at: "2026-06-08T15:00:00Z".to_string(),
        }
    }

    #[test]
    fn parse_formats() {
        assert!(matches!(ExportFormat::parse("md"), Ok(ExportFormat::Markdown)));
        assert!(matches!(ExportFormat::parse("JSON"), Ok(ExportFormat::Json)));
        assert!(matches!(ExportFormat::parse("csv"), Ok(ExportFormat::Csv)));
        assert!(ExportFormat::parse("xml").is_err());
    }

    #[test]
    fn markdown_includes_aggregates() {
        let entries = vec![
            entry(LogLevel::Error, "溢出"),
            entry(LogLevel::Error, "溢出"),
            entry(LogLevel::Info, "正常"),
        ];
        let md = render(&sample_report(), &entries, ExportFormat::Markdown);
        assert!(md.contains("错误聚合"));
        assert!(md.contains("**溢出**（2 次）"));
        assert!(md.contains("#42"));
    }

    #[test]
    fn csv_escapes_comma() {
        let entries = vec![entry(LogLevel::Error, "值: 1, 2, 3")];
        let csv = render(&sample_report(), &entries, ExportFormat::Csv);
        assert!(csv.contains("\"值: 1, 2, 3\""));
    }

    #[test]
    fn json_is_valid() {
        let entries = vec![entry(LogLevel::Info, "x")];
        let json = render(&sample_report(), &entries, ExportFormat::Json);
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
    }
}
