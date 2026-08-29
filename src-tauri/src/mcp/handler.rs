//! MCP server 的工具集实现：薄封装 services 层，面向 LLM 控制响应体量。

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ErrorData, Json, ServerHandler};

use crate::models::analyze::{AnalysisResult, LogFilter};
use crate::models::log_entry::{LogEntry, LogLevel};
use crate::models::report::Report;
use crate::services::cache::Cache;

use super::dto::{
    AnalysisResultDto, AnalyzeReportParams, ErrorAggregateDto, GetReportParams, IssueBriefDto,
    IssueListResult, LevelCountsDto, ListIssuesParams, LogEntryDto, LogFilterDto, QueryLogsParams,
    QueryLogsResult, TagCountDto, TimelinePointDto,
};

/// 灵鉴 MCP server。每个 HTTP 会话一个实例，共享应用级 SQLite 缓存。
pub struct LingjianServer {
    cache: Arc<Cache>,
}

impl LingjianServer {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache }
    }

    /// 在异步上下文里执行同步 SQLite 查询
    async fn query_reports(&self, limit: usize) -> Result<Vec<Report>, ErrorData> {
        let cache = self.cache.clone();
        tauri::async_runtime::spawn_blocking(move || cache.list_recent_reports(limit))
            .await
            .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
            .map_err(|e| ErrorData::internal_error(format!("查询上报失败: {e}"), None))
    }

    async fn query_report(&self, report_id: String) -> Result<Option<Report>, ErrorData> {
        let cache = self.cache.clone();
        tauri::async_runtime::spawn_blocking(move || cache.get_report(&report_id))
            .await
            .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
            .map_err(|e| ErrorData::internal_error(format!("查询单条上报失败: {e}"), None))
    }

    async fn query_entries(&self, report_id: String) -> Result<Vec<LogEntry>, ErrorData> {
        let cache = self.cache.clone();
        tauri::async_runtime::spawn_blocking(move || cache.get_entries(&report_id))
            .await
            .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
            .map_err(|e| ErrorData::internal_error(format!("查询日志失败: {e}"), None))
    }
}

/// DTO 过滤条件转领域 LogFilter，非法级别名报 invalid_params
fn to_log_filter(f: &LogFilterDto) -> Result<LogFilter, ErrorData> {
    let mut levels = Vec::with_capacity(f.levels.len());
    for lv in &f.levels {
        let parsed = LogLevel::parse(lv).ok_or_else(|| {
            ErrorData::invalid_params(format!("未知日志级别 \"{lv}\"（可选 DEBUG/INFO/WARN/ERROR/FATAL）"), None)
        })?;
        levels.push(parsed);
    }
    Ok(LogFilter {
        levels,
        tags: f.tags.clone(),
        keyword: f.keyword.clone(),
    })
}

/// AnalysisResult → DTO，按上限截断明细与时间线（聚合类全量保留）
fn to_analysis_dto(r: AnalysisResult, entry_limit: usize, timeline_limit: usize) -> AnalysisResultDto {
    let timeline_total = r.timeline.len();
    let entry_total = r.entries.len();
    AnalysisResultDto {
        total: r.total,
        level_counts: LevelCountsDto {
            debug: r.level_counts.debug,
            info: r.level_counts.info,
            warn: r.level_counts.warn,
            error: r.level_counts.error,
            fatal: r.level_counts.fatal,
        },
        tag_counts: r
            .tag_counts
            .into_iter()
            .map(|t| TagCountDto { tag: t.tag, count: t.count })
            .collect(),
        error_aggregates: r
            .error_aggregates
            .into_iter()
            .map(|a| ErrorAggregateDto {
                message: a.message,
                count: a.count,
                first_seen: a.first_seen,
                last_seen: a.last_seen,
            })
            .collect(),
        timeline: r
            .timeline
            .into_iter()
            .take(timeline_limit)
            .map(|p| TimelinePointDto {
                timestamp: p.timestamp,
                level: p.level.as_str().to_string(),
                message: p.message,
            })
            .collect(),
        timeline_total,
        entries: r.entries.iter().take(entry_limit).map(LogEntryDto::from).collect(),
        entry_total,
    }
}

#[tool_router]
impl LingjianServer {
    #[tool(
        name = "list_issues",
        description = "列出灵鉴已下载的用户日志上报（按下载时间倒序，含用户原始反馈描述、游戏版本、平台等信息）",
        annotations(read_only_hint = true)
    )]
    async fn list_issues(
        &self,
        Parameters(ListIssuesParams {
            limit,
            issue_number,
        }): Parameters<ListIssuesParams>,
    ) -> Result<Json<IssueListResult>, ErrorData> {
        let limit = limit.unwrap_or(20).clamp(1, 100);
        let mut reports = self.query_reports(limit).await?;
        if let Some(num) = issue_number {
            reports.retain(|r| r.issue_number == Some(num as i32));
        }
        Ok(Json(IssueListResult {
            issues: reports.into_iter().map(IssueBriefDto::from).collect(),
        }))
    }

    #[tool(
        name = "get_report",
        description = "查询单条上报的完整元信息（用户反馈、游戏版本、平台、境界、游玩时长等）",
        annotations(read_only_hint = true)
    )]
    async fn get_report(
        &self,
        Parameters(GetReportParams { report_id }): Parameters<GetReportParams>,
    ) -> Result<Json<IssueBriefDto>, ErrorData> {
        match self.query_report(report_id).await? {
            Some(r) => Ok(Json(IssueBriefDto::from(r))),
            None => Err(ErrorData::invalid_params("未找到该 reportId 的上报记录".to_string(), None)),
        }
    }

    #[tool(
        name = "analyze_report",
        description = "分析一份上报日志：级别统计、tag 分布、错误聚合（全量）+ WARN/ERROR 时间线与明细（默认截断，可用 query_logs 翻页）。结果与灵鉴分析页同源",
        annotations(read_only_hint = true)
    )]
    async fn analyze_report(
        &self,
        Parameters(AnalyzeReportParams {
            report_id,
            filter,
            entry_limit,
            timeline_limit,
        }): Parameters<AnalyzeReportParams>,
    ) -> Result<Json<AnalysisResultDto>, ErrorData> {
        let entry_limit = entry_limit.unwrap_or(50);
        let timeline_limit = timeline_limit.unwrap_or(100);
        let log_filter = to_log_filter(&filter)?;

        let entries = self.query_entries(report_id).await?;
        if entries.is_empty() {
            return Err(ErrorData::invalid_params(
                "未找到该 reportId 的日志（可能未下载过）".to_string(),
                None,
            ));
        }
        let result = crate::services::analyzer::analyze(&entries, &log_filter);
        Ok(Json(to_analysis_dto(result, entry_limit, timeline_limit)))
    }

    #[tool(
        name = "query_logs",
        description = "按级别/tag/关键词过滤上报日志并分页返回原始条目（时间正序）",
        annotations(read_only_hint = true)
    )]
    async fn query_logs(
        &self,
        Parameters(QueryLogsParams {
            report_id,
            filter,
            offset,
            limit,
        }): Parameters<QueryLogsParams>,
    ) -> Result<Json<QueryLogsResult>, ErrorData> {
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(50).clamp(1, 200);
        let log_filter = to_log_filter(&filter)?;

        let entries = self.query_entries(report_id).await?;
        let matched: Vec<&LogEntry> = entries
            .iter()
            .filter(|e| {
                log_filter.matches(&e.level, &e.tag, &e.message, &e.data)
            })
            .collect();
        let matched_total = matched.len();
        Ok(Json(QueryLogsResult {
            entries: matched
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(LogEntryDto::from)
                .collect(),
            matched_total,
            offset,
            limit,
        }))
    }
}

// get_info 由 #[tool_handler] 自动生成（server 名称/版本取自 Cargo.toml）
#[tool_handler]
impl ServerHandler for LingjianServer {}
