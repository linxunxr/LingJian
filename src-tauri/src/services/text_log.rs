use regex::Regex;

use crate::models::log_entry::{LogEntry, LogLevel};

/// 鸿蒙 `@huanyinfeng/logger` 库（DefaultFormatter）落盘日志解析器
///
/// 该库产出的日志为纯文本行（UTF-8、LF 行尾、无 BOM）：
///
/// ```text
/// 2026-08-09 10:30:20.364 [I/Player] 断点续播: 128s / 240s
/// 2026-08-09 10:30:20.400 [E/MainAbility] init 失败
/// MyException: 初始化错误
///   at com.foo.Main.onCreate(Main.cj:10)
/// ```
///
/// 规则要点（与库源码 formatter.cj / file_appender.cj 对齐）：
/// - 首行格式：`yyyy-MM-dd HH:mm:ss.SSS [级别字母/Tag] 消息`
/// - 级别字母仅 D/I/W/E/F（`?` 为兜底，理论不可达，按 INFO 处理）
/// - tag 不做转义，可含空格/中文/`[`（正则按“最后一个 `] ` 定界”贪婪匹配）
/// - 带 throwable 的记录为多行：异常首行无缩进，栈帧行为两空格 + `at`；
///   解析上统一按“非时间戳开头的行归属上一条记录”处理
/// - message 可含换行（同样按续行规则并入）
///
/// 时间戳为设备本地时间且不含时区，解析时仅把日期与时间之间的
/// 空格替换为 `T`（`2026-08-09T10:30:20.364`），保持字符串可排序、
/// 前端 `new Date()` 可解析（按本地时区解释，与设备语义一致）。

/// 首行正则：tag 段按第一个 `]` 截断（`[^]]*`），
/// 保证消息中含 `[`/`]`/`] ` 时不被吞入 tag；tag 本身含 `]` 属病态输入，
/// 该行会降级为上一条的续行（有损但可接受）。
fn line_regex() -> Regex {
    Regex::new(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}) \[([DIWEF?])/([^]]*)\] (.*)$")
        .expect("内置正则必然合法")
}

/// 级别字母映射
fn letter_to_level(letter: &str) -> LogLevel {
    match letter {
        "D" => LogLevel::Debug,
        "I" => LogLevel::Info,
        "W" => LogLevel::Warn,
        "E" => LogLevel::Error,
        "F" => LogLevel::Fatal,
        _ => LogLevel::Info, // '?' 兜底
    }
}

/// 解析 huanyinfeng/logger 文本日志为日志条目
///
/// 首条记录之前的杂散行（如文件头残留）会被丢弃；
/// 若整份文本没有任何匹配行，返回 Err（供上层做格式探测降级）。
pub fn parse_hlog_text(text: &str) -> Result<Vec<LogEntry>, String> {
    let re = line_regex();
    let mut entries: Vec<LogEntry> = Vec::new();

    for line in text.lines() {
        if let Some(c) = re.captures(line) {
            // 日期与时间之间的空格替换为 T，形成可排序的 ISO 风格本地时间
            let ts = c[1].replacen(' ', "T", 1);
            let tag = c[3].trim();
            entries.push(LogEntry {
                timestamp: ts,
                level: letter_to_level(&c[2]),
                tag: if tag.is_empty() {
                    "未知".to_string()
                } else {
                    tag.to_string()
                },
                message: c[4].to_string(),
                data: None,
            });
        } else if let Some(last) = entries.last_mut() {
            // 续行：异常首行 / 栈帧 / 多行 message，并入上一条
            last.message.push('\n');
            last.message.push_str(line);
        }
    }

    if entries.is_empty() {
        return Err("未找到任何符合 huanyinfeng/logger 格式的日志行".to_string());
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_lines() {
        let text = "2026-08-09 10:30:20.364 [I/Player] 断点续播: 128s / 240s\n\
                    2026-08-09 10:30:21.001 [W/Pan123] 123 连通测试失败: timeout\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].timestamp, "2026-08-09T10:30:20.364");
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[0].tag, "Player");
        assert_eq!(entries[0].message, "断点续播: 128s / 240s");
        assert_eq!(entries[1].level, LogLevel::Warn);
        assert_eq!(entries[1].tag, "Pan123");
    }

    #[test]
    fn parse_all_level_letters() {
        let text = "2026-08-09 10:30:20.001 [D/A] d\n\
                    2026-08-09 10:30:20.002 [I/A] i\n\
                    2026-08-09 10:30:20.003 [W/A] w\n\
                    2026-08-09 10:30:20.004 [E/A] e\n\
                    2026-08-09 10:30:20.005 [F/A] f\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].level, LogLevel::Debug);
        assert_eq!(entries[1].level, LogLevel::Info);
        assert_eq!(entries[2].level, LogLevel::Warn);
        assert_eq!(entries[3].level, LogLevel::Error);
        assert_eq!(entries[4].level, LogLevel::Fatal);
    }

    #[test]
    fn parse_multiline_exception() {
        // 库源码注释中的官方示例：异常首行无缩进 + 栈帧行两空格 at 前缀
        //（用 \n 拼接而非 \ 续行，保留栈帧的行首两空格）
        let text = concat!(
            "2026-08-09 10:30:20.400 [E/MainAbility] init 失败\n",
            "MyException: 初始化错误\n",
            "  at com.foo.Main.onCreate(Main.cj:10)\n",
            "2026-08-09 10:30:21.000 [I/Player] 恢复播放\n",
        );
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].message,
            "init 失败\nMyException: 初始化错误\n  at com.foo.Main.onCreate(Main.cj:10)"
        );
        assert_eq!(entries[1].message, "恢复播放");
    }

    #[test]
    fn parse_tag_with_spaces_and_chinese() {
        let text = "2026-08-09 10:30:20.364 [I/桌面 歌词] 探测完成\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries[0].tag, "桌面 歌词");
    }

    #[test]
    fn parse_empty_tag_fallback() {
        let text = "2026-08-09 10:30:20.364 [I/] 无标签消息\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries[0].tag, "未知");
    }

    #[test]
    fn leading_garbage_lines_dropped() {
        let text = "some header junk\nanother junk line\n\
                    2026-08-09 10:30:20.364 [I/A] first real\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "first real");
    }

    #[test]
    fn crlf_line_endings_tolerated() {
        // 跨平台传输可能引入 CRLF；str::lines 会剥掉 \r
        let text = "2026-08-09 10:30:20.364 [I/A] hello\r\n2026-08-09 10:30:20.365 [W/A] world\r\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "hello");
    }

    #[test]
    fn unrecognized_text_is_error() {
        assert!(parse_hlog_text("hello world\nfoo bar\n").is_err());
        assert!(parse_hlog_text("").is_err());
    }

    #[test]
    fn timestamp_like_line_without_bracket_not_record() {
        // 有时间戳但缺 [X/Tag] 结构的行按续行处理，不误判为首行
        let text = "2026-08-09 10:30:20.364 [I/A] msg\n\
                    2026-08-09 10:30:20.365 continuation without bracket\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.contains("continuation without bracket"));
    }

    #[test]
    fn message_with_brackets_not_swallowed_into_tag() {
        // 消息中含 "[W] " 时，tag 仍在第一个 ] 截断，消息完整保留
        let text = "2026-08-09 10:30:20.400 [E/Main] init [W] failed: exit(1)\n";
        let entries = parse_hlog_text(text).unwrap();
        assert_eq!(entries[0].tag, "Main");
        assert_eq!(entries[0].message, "init [W] failed: exit(1)");
    }
}
