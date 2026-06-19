use serde::{Deserialize, Serialize};

use super::log_entry::LogLevel;

/// 日志过滤条件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogFilter {
    /// 选中的级别集合，空表示不过滤级别
    #[serde(default)]
    pub levels: Vec<LogLevel>,
    /// 选中的 tag 集合，空表示不过滤模块
    #[serde(default)]
    pub tags: Vec<String>,
    /// 关键词，命中 message 或 data 即保留
    #[serde(default)]
    pub keyword: String,
}

impl LogFilter {
    pub fn matches(&self, level: &LogLevel, tag: &str, message: &str, data: &Option<serde_json::Value>) -> bool {
        if !self.levels.is_empty() && !self.levels.contains(level) {
            return false;
        }
        if !self.tags.is_empty() && !self.tags.iter().any(|t| t == tag) {
            return false;
        }
        if !self.keyword.is_empty() {
            let kw = self.keyword.to_lowercase();
            if !message.to_lowercase().contains(&kw) {
                // 关键词未命中 message，再看 data 字段
                let hit_data = match data {
                    Some(serde_json::Value::String(s)) => s.to_lowercase().contains(&kw),
                    Some(v) => v.to_string().to_lowercase().contains(&kw),
                    None => false,
                };
                if !hit_data {
                    return false;
                }
            }
        }
        true
    }
}

/// 时间线上的一个点（ERROR/WARN 等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePoint {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

/// 错误聚合（相同 message 去重计数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAggregate {
    pub message: String,
    pub count: usize,
    pub first_seen: String,
    pub last_seen: String,
}

/// 分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    /// 过滤后保留的日志条目
    pub entries: Vec<crate::models::log_entry::LogEntry>,
    /// 全量日志条目数（应用过滤前）
    pub total: usize,
    /// 各级别计数（基于全量日志）
    pub level_counts: LevelCounts,
    /// 所有出现过的 tag 及其计数（基于全量日志）
    pub tag_counts: Vec<TagCount>,
    /// ERROR/WARN 时间线（基于过滤后日志）
    pub timeline: Vec<TimelinePoint>,
    /// 错误聚合（基于过滤后 ERROR 日志）
    pub error_aggregates: Vec<ErrorAggregate>,
}

/// 各级别计数
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelCounts {
    pub debug: usize,
    pub info: usize,
    pub warn: usize,
    pub error: usize,
}

/// tag 计数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}
