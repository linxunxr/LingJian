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
    /// 反馈截图的 COS key 列表（无截图为 None；图像本体需经灵鉴界面查看）
    pub screenshot_keys: Option<Vec<String>>,
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
            screenshot_keys: r.screenshot_keys,
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

/// get_report_screenshots 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetScreenshotsParams {
    /// 上报编号（reportId）
    pub report_id: String,
    /// 起始序号（0 基，默认 0；配合 limit 查看后续截图）
    #[serde(default)]
    pub offset: Option<usize>,
    /// 最多返回几张（1-4，默认 2；图片 token 开销大，逐批查看更省）
    #[serde(default)]
    pub limit: Option<usize>,
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

/// filter 字段反序列化：兼容对象与 JSON 字符串两种形式。
/// 部分客户端（如 ZCode）会把嵌套 object 参数整体序列化为字符串传入，
/// 严格 struct 反序列化会直接拒绝；字符串在这里多 parse 一次，
/// null 与空白串视为缺省，字符串内容非法 JSON 时仍报错。
mod filter_de {
    use std::fmt;

    use serde::de::{self, MapAccess, Visitor};
    use serde::{Deserialize, Deserializer};

    use super::LogFilterDto;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<LogFilterDto, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FilterVisitor;

        impl<'de> Visitor<'de> for FilterVisitor {
            type Value = LogFilterDto;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("过滤条件对象或其 JSON 字符串形式")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<LogFilterDto, E> {
                if v.trim().is_empty() {
                    return Ok(LogFilterDto::default());
                }
                serde_json::from_str(v).map_err(de::Error::custom)
            }

            fn visit_unit<E: de::Error>(self) -> Result<LogFilterDto, E> {
                Ok(LogFilterDto::default())
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<LogFilterDto, A::Error> {
                LogFilterDto::deserialize(de::value::MapAccessDeserializer::new(map))
            }
        }

        deserializer.deserialize_any(FilterVisitor)
    }
}

/// analyze_report 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeReportParams {
    pub report_id: String,
    /// 过滤条件，默认无过滤；兼容对象或 JSON 字符串传参
    #[serde(default, deserialize_with = "filter_de::deserialize")]
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
    #[serde(default, deserialize_with = "filter_de::deserialize")]
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

// ===== 二期：同步与写操作（需设置页开启"允许写操作"，sync_latest 除外） =====

/// sync_latest 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncLatestParams {
    /// Issue 状态筛选：open / closed / all，默认 open
    #[serde(default)]
    pub state: Option<String>,
    /// 页码，默认 1
    #[serde(default)]
    pub page: Option<u32>,
    /// 是否下载缺失的日志到本地（默认 true；false 时仅返回远端列表）
    #[serde(default)]
    pub download: Option<bool>,
}

/// SCF 侧 Issue 列表条目
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoteIssueDto {
    pub number: u32,
    pub report_id: String,
    pub title: String,
    pub state: String,
    pub issue_url: String,
    pub created_at: String,
}

/// sync_latest 返回
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncResultDto {
    /// 远端 Issue 列表
    pub issues: Vec<RemoteIssueDto>,
    pub has_more: bool,
    /// 本次新下载落库的条数
    pub downloaded: usize,
    /// 本地已有跳过的条数
    pub skipped: usize,
    /// 下载失败的 issue 编号与原因
    pub failed: Vec<String>,
}

/// add_comment 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddCommentParams {
    pub issue_number: u32,
    /// 评论内容（Markdown）
    pub body: String,
}

/// update_labels 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLabelsParams {
    pub issue_number: u32,
    /// 目标标签集合（整体替换，不是追加）
    pub labels: Vec<String>,
}

/// close_issue 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloseIssueParams {
    pub issue_number: u32,
    /// 解决该问题的游戏版本号（如 "0.9.19"）。提供时执行与灵鉴界面「关闭 Issue」
    /// 相同的完整流程：关闭 → 追加 v<版本号> 标签 → 发表解决评论；缺省则仅关闭
    #[serde(default)]
    pub fixed_in: Option<String>,
}

/// reopen_issue 入参
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReopenIssueParams {
    pub issue_number: u32,
}

/// Issue 操作（评论/标签/关闭/重开）的返回
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueActionResultDto {
    pub ok: bool,
    /// 操作后的 Issue 状态（close 后为 "closed"，reopen 后为 "open"）
    pub state: Option<String>,
    /// 操作后的标签集合（setLabels/close 后返回）
    pub labels: Option<Vec<String>>,
    /// 附带步骤（版本标签 / 解决评论）的执行结果，全成功为空
    pub followup_notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // 模拟 rmcp Parameters 的反序列化路径：arguments JSON 值 → params 结构体
    #[test]
    fn filter_accepts_object_form() {
        let p: QueryLogsParams = serde_json::from_value(serde_json::json!({
            "reportId": "r1",
            "filter": {"levels": ["WARN"], "keyword": "abc"}
        }))
        .unwrap();
        assert_eq!(p.filter.levels, vec!["WARN".to_string()]);
        assert_eq!(p.filter.keyword, "abc");
    }

    #[test]
    fn filter_accepts_json_string_form() {
        let p: QueryLogsParams = serde_json::from_value(serde_json::json!({
            "reportId": "r1",
            "filter": "{\"levels\":[\"WARN\"],\"tags\":[\"ShopLoader\"]}"
        }))
        .unwrap();
        assert_eq!(p.filter.levels, vec!["WARN".to_string()]);
        assert_eq!(p.filter.tags, vec!["ShopLoader".to_string()]);
    }

    #[test]
    fn filter_missing_blank_and_null_fall_back_to_default() {
        let p: QueryLogsParams =
            serde_json::from_value(serde_json::json!({"reportId": "r1"})).unwrap();
        assert!(p.filter.levels.is_empty());

        let p: QueryLogsParams = serde_json::from_value(serde_json::json!({
            "reportId": "r1", "filter": ""
        }))
        .unwrap();
        assert!(p.filter.tags.is_empty());

        let p: QueryLogsParams = serde_json::from_value(serde_json::json!({
            "reportId": "r1", "filter": null
        }))
        .unwrap();
        assert!(p.filter.keyword.is_empty());
    }

    #[test]
    fn filter_invalid_json_string_still_errors() {
        let r = serde_json::from_value::<QueryLogsParams>(serde_json::json!({
            "reportId": "r1",
            "filter": "{not json"
        }));
        assert!(r.is_err());
    }

    #[test]
    fn analyze_params_accepts_string_filter_too() {
        let p: AnalyzeReportParams = serde_json::from_value(serde_json::json!({
            "reportId": "r1",
            "filter": "{\"levels\":[\"ERROR\"]}"
        }))
        .unwrap();
        assert_eq!(p.filter.levels, vec!["ERROR".to_string()]);
    }
}
