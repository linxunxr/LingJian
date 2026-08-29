//! MCP 工具的请求/响应 DTO。
//!
//! 独立于 models 层定义（models 不引入 schemars），
//! 经 From 转换对接，保持领域类型零侵入。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::models::report::Report;

/// list_issues 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListIssuesParams {
    /// 返回条数上限，默认 20，最大 100
    #[serde(default)]
    pub limit: Option<usize>,
    /// 按 GitHub Issue 编号过滤（在返回窗口内过滤）
    #[serde(default)]
    pub issue_number: Option<i64>,
}

/// 上报记录摘要（用户反馈原文全量返回，通常很短且信息密度最高）
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueBriefDto {
    pub report_id: String,
    pub issue_number: Option<i32>,
    pub issue_title: Option<String>,
    pub app_name: Option<String>,
    pub app_version: Option<String>,
    pub platform: Option<String>,
    pub realm: Option<String>,
    /// 游玩时长（秒）
    pub play_time: Option<u64>,
    /// 用户原始反馈描述
    pub user_description: Option<String>,
    pub log_count: usize,
    pub report_time: String,
    pub downloaded_at: String,
}

impl From<Report> for IssueBriefDto {
    fn from(r: Report) -> Self {
        Self {
            report_id: r.report_id,
            issue_number: r.issue_number,
            issue_title: r.issue_title,
            app_name: r.app_name,
            app_version: r.app_version,
            platform: r.platform,
            realm: r.realm,
            play_time: r.play_time,
            user_description: r.user_description,
            log_count: r.log_count,
            report_time: r.report_time,
            downloaded_at: r.downloaded_at,
        }
    }
}

/// list_issues 返回
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueListResult {
    pub issues: Vec<IssueBriefDto>,
}

/// get_report 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetReportParams {
    pub report_id: String,
}

/// 日志过滤条件（透传 services 层 LogFilter 语义）
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct LogFilterDto {
    /// 级别集合（DEBUG/INFO/WARN/ERROR/FATAL，大小写不敏感），空表示不过滤
    #[serde(default)]
    pub levels: Vec<String>,
    /// tag 集合，空表示不过滤
    #[serde(default)]
    pub tags: Vec<String>,
    /// 关键词，命中 message 或 data 即保留（大小写不敏感）
    #[serde(default)]
    pub keyword: String,
}

/// analyze_report 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeReportParams {
    pub report_id: String,
    /// 过滤条件，默认无过滤
    #[serde(default)]
    pub filter: LogFilterDto,
    /// 明细条目上限，默认 50；0 表示不返回明细（只要统计与聚合）
    #[serde(default)]
    pub entry_limit: Option<usize>,
    /// 时间线点上限，默认 100
    #[serde(default)]
    pub timeline_limit: Option<usize>,
}

/// 单条日志
#[derive(Debug, Serialize, JsonSchema)]
pub struct LogEntryDto {
    pub timestamp: String,
    pub level: String,
    pub tag: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl From<&crate::models::log_entry::LogEntry> for LogEntryDto {
    fn from(e: &crate::models::log_entry::LogEntry) -> Self {
        Self {
            timestamp: e.timestamp.clone(),
            level: e.level.as_str().to_string(),
            tag: e.tag.clone(),
            message: e.message.clone(),
            data: e.data.clone(),
        }
    }
}

/// 错误聚合条目
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorAggregateDto {
    pub message: String,
    pub count: usize,
    pub first_seen: String,
    pub last_seen: String,
}

/// 时间线点
#[derive(Debug, Serialize, JsonSchema)]
pub struct TimelinePointDto {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// analyze_report 返回（聚合全量，明细截断并带总数）
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResultDto {
    /// 过滤前全量条数
    pub total: usize,
    pub level_counts: LevelCountsDto,
    pub tag_counts: Vec<TagCountDto>,
    pub error_aggregates: Vec<ErrorAggregateDto>,
    /// 时间线（截断后）
    pub timeline: Vec<TimelinePointDto>,
    /// 时间线总点数（截断前）
    pub timeline_total: usize,
    /// 过滤后明细（截断后）
    pub entries: Vec<LogEntryDto>,
    /// 过滤后总条数（截断前）
    pub entry_total: usize,
}

/// 各级别计数
#[derive(Debug, Default, Serialize, JsonSchema)]
pub struct LevelCountsDto {
    pub debug: usize,
    pub info: usize,
    pub warn: usize,
    pub error: usize,
    pub fatal: usize,
}

/// tag 计数
#[derive(Debug, Serialize, JsonSchema)]
pub struct TagCountDto {
    pub tag: String,
    pub count: usize,
}

/// query_logs 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueryLogsParams {
    pub report_id: String,
    #[serde(default)]
    pub filter: LogFilterDto,
    /// 分页偏移，默认 0
    #[serde(default)]
    pub offset: Option<usize>,
    /// 页大小，默认 50，上限 200
    #[serde(default)]
    pub limit: Option<usize>,
}

/// query_logs 返回
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueryLogsResult {
    pub entries: Vec<LogEntryDto>,
    /// 过滤后总条数（分页前）
    pub matched_total: usize,
    pub offset: usize,
    pub limit: usize,
}
