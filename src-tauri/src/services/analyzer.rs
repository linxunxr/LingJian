use std::collections::HashMap;

use crate::models::analyze::{
    AnalysisResult, ErrorAggregate, LevelCounts, LogFilter, TagCount, TimelinePoint,
};
use crate::models::log_entry::{LogEntry, LogLevel};

/// 对日志数组执行过滤 + 统计分析
pub fn analyze(entries: &[LogEntry], filter: &LogFilter) -> AnalysisResult {
    // 全量统计：级别分布 + tag 分布
    let mut level_counts = LevelCounts::default();
    let mut tag_map: HashMap<String, usize> = HashMap::new();

    for e in entries {
        match e.level {
            LogLevel::Debug => level_counts.debug += 1,
            LogLevel::Info => level_counts.info += 1,
            LogLevel::Warn => level_counts.warn += 1,
            LogLevel::Error => level_counts.error += 1,
        }
        *tag_map.entry(e.tag.clone()).or_insert(0) += 1;
    }

    let mut tag_counts: Vec<TagCount> = tag_map
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    // 按计数降序，计数相同按 tag 字典序（稳定展示）
    tag_counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.tag.cmp(&b.tag)));

    // 过滤
    let filtered: Vec<LogEntry> = entries
        .iter()
        .filter(|e| filter.matches(&e.level, &e.tag, &e.message, &e.data))
        .cloned()
        .collect();

    // 时间线：过滤后的 WARN/ERROR
    let timeline: Vec<TimelinePoint> = filtered
        .iter()
        .filter(|e| matches!(e.level, LogLevel::Warn | LogLevel::Error))
        .map(|e| TimelinePoint {
            timestamp: e.timestamp.clone(),
            level: e.level,
            message: e.message.clone(),
        })
        .collect();

    // 错误聚合：过滤后的 ERROR 按 message 去重计数
    let error_aggregates = aggregate_errors(
        filtered.iter().filter(|e| matches!(e.level, LogLevel::Error)),
    );

    AnalysisResult {
        total: entries.len(),
        level_counts,
        tag_counts,
        timeline,
        error_aggregates,
        entries: filtered,
    }
}

/// 将 ERROR 日志按 message 聚合，输出 first/last 出现时间
fn aggregate_errors<'a, I>(errors: I) -> Vec<ErrorAggregate>
where
    I: Iterator<Item = &'a LogEntry>,
{
    let mut map: HashMap<String, ErrorAggregate> = HashMap::new();
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
            .or_insert(ErrorAggregate {
                message: e.message.clone(),
                count: 1,
                first_seen: e.timestamp.clone(),
                last_seen: e.timestamp.clone(),
            });
    }

    let mut result: Vec<ErrorAggregate> = map.into_values().collect();
    // 按出现次数降序，高频错误排前
    result.sort_by(|a, b| b.count.cmp(&a.count).then(a.message.cmp(&b.message)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(level: LogLevel, tag: &str, msg: &str, ts: &str) -> LogEntry {
        LogEntry {
            timestamp: ts.to_string(),
            level,
            tag: tag.to_string(),
            message: msg.to_string(),
            data: None,
        }
    }

    #[test]
    fn level_and_tag_counts() {
        let entries = vec![
            entry(LogLevel::Error, "战斗", "溢出", "t1"),
            entry(LogLevel::Error, "战斗", "溢出", "t2"),
            entry(LogLevel::Info, "修炼", "入定", "t3"),
        ];
        let r = analyze(&entries, &LogFilter::default());
        assert_eq!(r.total, 3);
        assert_eq!(r.level_counts.error, 2);
        assert_eq!(r.level_counts.info, 1);
        assert_eq!(r.tag_counts[0].tag, "战斗"); // count 2 排前
        assert_eq!(r.tag_counts[0].count, 2);
    }

    #[test]
    fn filter_by_level() {
        let entries = vec![
            entry(LogLevel::Debug, "a", "d", "t1"),
            entry(LogLevel::Error, "a", "e", "t2"),
        ];
        let filter = LogFilter {
            levels: vec![LogLevel::Error],
            ..Default::default()
        };
        let r = analyze(&entries, &filter);
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].message, "e");
    }

    #[test]
    fn error_aggregation_dedup() {
        let entries = vec![
            entry(LogLevel::Error, "a", "溢出", "t3"),
            entry(LogLevel::Error, "a", "溢出", "t1"),
            entry(LogLevel::Error, "a", "溢出", "t2"),
            entry(LogLevel::Error, "a", "其他", "t4"),
        ];
        let r = analyze(&entries, &LogFilter::default());
        assert_eq!(r.error_aggregates.len(), 2);
        let agg = r.error_aggregates.iter().find(|a| a.message == "溢出").unwrap();
        assert_eq!(agg.count, 3);
        assert_eq!(agg.first_seen, "t1");
        assert_eq!(agg.last_seen, "t3");
    }

    #[test]
    fn keyword_in_message() {
        let entries = vec![
            entry(LogLevel::Info, "a", "灵气消耗", "t1"),
            entry(LogLevel::Info, "a", "进入战斗", "t2"),
        ];
        let filter = LogFilter {
            keyword: "灵气".to_string(),
            ..Default::default()
        };
        let r = analyze(&entries, &filter);
        assert_eq!(r.entries.len(), 1);
    }
}
