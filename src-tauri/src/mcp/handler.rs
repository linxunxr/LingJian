//! MCP server 的工具集实现：薄封装 services 层，面向 LLM 控制响应体量。
//!
//! 只读工具（list_issues 等）始终可用；sync_latest 仅写本地库；
//! 评论/标签/关闭属外部写操作，需设置页开启「允许写操作」后方可调用。

use std::sync::Arc;

use base64::Engine as _;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ErrorData, Json, ServerHandler};
use tauri::Manager;

use crate::models::analyze::{AnalysisResult, LogFilter};
use crate::models::log_entry::{LogEntry, LogLevel};
use crate::models::report::Report;
use crate::services::cache::Cache;
use crate::services::downloader;

use super::dto::{
    AddCommentParams, AnalysisResultDto, AnalyzeReportParams, CloseIssueParams, ErrorAggregateDto,
    GetReportParams, GetScreenshotsParams, IssueActionResultDto, IssueBriefDto, IssueListResult,
    IssueStatsDto, LevelCountsDto, ListIssuesParams, ListRemoteIssuesParams, LogEntryDto,
    LogFilterDto, QueryLogsParams, QueryLogsResult, RemoteIssueDto, RemoteIssueEntryDto,
    RemoteIssueListResult, ReopenIssueParams, SyncLatestParams, SyncResultDto, TagCountDto,
    TimelinePointDto, UpdateLabelsParams,
};

/// 灵鉴 MCP server。每个 HTTP 会话一个实例，经 AppHandle 共享应用状态。
pub struct LingjianServer {
    app: tauri::AppHandle,
}

impl LingjianServer {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }

    fn cache(&self) -> Arc<Cache> {
        self.app.state::<crate::AppState>().cache.clone()
    }

    fn scf_config(&self) -> Result<(String, String), ErrorData> {
        let (url, key) = super::read_scf_settings(&self.app);
        if url.trim().is_empty() || key.trim().is_empty() {
            return Err(ErrorData::internal_error(
                "未配置 SCF 端点，请先在灵鉴设置页填写 URL 与 API Key".to_string(),
                None,
            ));
        }
        Ok((url, key))
    }

    fn write_allowed(&self) -> Result<(), ErrorData> {
        if !super::read_allow_write(&self.app) {
            return Err(ErrorData::internal_error(
                "写操作未开放：请到灵鉴设置页开启「允许写操作」后重试".to_string(),
                None,
            ));
        }
        Ok(())
    }

    /// 在异步上下文里执行同步 SQLite 查询
    async fn query_reports(&self, limit: usize) -> Result<Vec<Report>, ErrorData> {
        let cache = self.cache();
        tauri::async_runtime::spawn_blocking(move || cache.list_recent_reports(limit))
            .await
            .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
            .map_err(|e| ErrorData::internal_error(format!("查询上报失败: {e}"), None))
    }

    async fn query_report(&self, report_id: String) -> Result<Option<Report>, ErrorData> {
        let cache = self.cache();
        tauri::async_runtime::spawn_blocking(move || cache.get_report(&report_id))
            .await
            .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
            .map_err(|e| ErrorData::internal_error(format!("查询单条上报失败: {e}"), None))
    }

    async fn query_entries(&self, report_id: String) -> Result<Vec<LogEntry>, ErrorData> {
        let cache = self.cache();
        tauri::async_runtime::spawn_blocking(move || cache.get_entries(&report_id))
            .await
            .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
            .map_err(|e| ErrorData::internal_error(format!("查询日志失败: {e}"), None))
    }

    /// 统一的 Issue 操作入口（评论/标签/关闭/重开），SCF 代理转发 GitHub
    async fn act_on_issue(
        &self,
        number: u32,
        action: &str,
        body: Option<&str>,
        labels: Option<&[String]>,
    ) -> Result<IssueActionResultDto, ErrorData> {
        self.write_allowed()?;
        let (scf_url, api_key) = self.scf_config()?;
        let client = self.app.state::<crate::AppState>().client.clone();
        let r = downloader::act_on_issue(&scf_url, number, action, body, labels, &api_key, &client)
            .await
            .map_err(|e| ErrorData::internal_error(format!("操作 Issue #{number} 失败: {e}"), None))?;
        Ok(IssueActionResultDto {
            ok: r.ok,
            state: r.state,
            labels: r.labels,
            followup_notes: Vec::new(),
        })
    }

    /// 与灵鉴界面「关闭 Issue」一致的完整流程：关闭 → 追加 v<版本号> 标签 → 解决评论。
    ///
    /// 与 CloseIssueDialog 的取舍相同：关闭是主目标必须成功；标签/评论是次要
    /// 步骤，失败不回滚已关闭状态，进 followup_notes 让调用方知晓后可自行补做。
    async fn close_issue_full(
        &self,
        number: u32,
        fixed_in: &str,
    ) -> Result<IssueActionResultDto, ErrorData> {
        self.write_allowed()?;
        let (scf_url, api_key) = self.scf_config()?;
        let client = self.app.state::<crate::AppState>().client.clone();

        // 先取当前标签（追加需基于现状，setLabels 是整体替换）
        let current = downloader::resolve_issue(&scf_url, number, &api_key, &client)
            .await
            .map_err(|e| ErrorData::internal_error(format!("获取 Issue #{number} 现状失败: {e}"), None))?;
        let tag_label = format!("v{}", fixed_in.trim().trim_start_matches('v'));

        // 1) 关闭——主目标，失败直接报错
        let mut result = self
            .act_on_issue(number, "close", None, None)
            .await?;

        // 2) 追加版本标签 + 3) 解决评论——次要步骤，失败记录不阻断
        let mut notes = Vec::new();
        let new_labels: Vec<String> = {
            let mut l = current.labels.clone();
            if !l.contains(&tag_label) {
                l.push(tag_label.clone());
            }
            l
        };
        if let Err(e) = downloader::act_on_issue(
            &scf_url,
            number,
            "setLabels",
            None,
            Some(&new_labels),
            &api_key,
            &client,
        )
        .await
        {
            notes.push(format!("追加版本标签 {tag_label} 失败: {e}"));
        } else {
            result.labels = Some(new_labels);
        }
        if let Err(e) = downloader::act_on_issue(
            &scf_url,
            number,
            "comment",
            Some(&format!("已在挂机仙途 v{} 中标记为已处理", tag_label.trim_start_matches('v'))),
            None,
            &api_key,
            &client,
        )
        .await
        {
            notes.push(format!("发表解决评论失败: {e}"));
        }

        result.followup_notes = notes;
        Ok(result)
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

    /// 读取上报反馈的截图，以 MCP ImageContent（base64 PNG）返回。
    ///
    /// 返回 CallToolResult 而非 Json：内容是「说明文本 + 图片块」混合列表，
    /// 仅多模态模型客户端能查看图片本体，纯文本模型只能读到说明文字。
    #[tool(
        name = "get_report_screenshots",
        description = "读取用户反馈附带的上报截图（PNG 图片内容，需多模态模型查看）。本地缓存优先，缺失时经 SCF 下载；截图总数与 key 见 get_report 的 screenshotKeys",
        annotations(read_only_hint = true)
    )]
    async fn get_report_screenshots(
        &self,
        Parameters(GetScreenshotsParams {
            report_id,
            offset,
            limit,
        }): Parameters<GetScreenshotsParams>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        let report = self
            .query_report(report_id)
            .await?
            .ok_or_else(|| {
                ErrorData::invalid_params("未找到该 reportId 的上报记录".to_string(), None)
            })?;
        let keys = report.screenshot_keys.clone().unwrap_or_default();
        if keys.is_empty() {
            return Ok(rmcp::model::CallToolResult::success(vec![
                rmcp::model::ContentBlock::text(format!(
                    "上报 {} 没有截图附件（老上报或用户未附带）",
                    report.report_id
                )),
            ]));
        }

        let offset = offset.unwrap_or(0).min(keys.len());
        let limit = limit.unwrap_or(2).clamp(1, 4);
        let selected: Vec<&String> = keys.iter().skip(offset).take(limit).collect();
        let mut contents = vec![rmcp::model::ContentBlock::text(format!(
            "上报 {} 共 {} 张截图，以下为第 {}–{} 张（offset={}，后续可用 offset 翻页）：",
            report.report_id,
            keys.len(),
            offset + 1,
            offset + selected.len(),
            offset,
        ))];

        let (scf_url, api_key) = self.scf_config()?;
        let app_state = self.app.state::<crate::AppState>();
        let client = app_state.client.clone();
        let cache_dir = app_state.cache_dir.clone();

        for key in selected {
            match downloader::fetch_screenshot_bytes(&scf_url, key, &api_key, &client, &cache_dir)
                .await
            {
                Ok((bytes, _cached)) => {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    contents.push(rmcp::model::ContentBlock::Image(
                        rmcp::model::ImageContent::new(b64, "image/png"),
                    ));
                }
                // 单张失败降级为文本说明，不影响其余截图
                Err(e) => contents.push(rmcp::model::ContentBlock::text(format!(
                    "截图 {key} 拉取失败: {e}"
                ))),
            }
        }
        Ok(rmcp::model::CallToolResult::success(contents))
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

    #[tool(
        name = "sync_latest",
        description = "从 SCF 同步最新数据到本地（与灵鉴界面「⇩ 日志」按钮同一入口）：先增量刷新 Issue 列表缓存镜像，再下载本地缺失的日志（download=false 跳过下载）。默认只下载 open（未处理），需要补全量传 all。同步后 list_remote_issues/issue_stats/list_issues 即为最新。仅写本地库，不改动 GitHub Issue",
        annotations(read_only_hint = true)
    )]
    async fn sync_latest(
        &self,
        Parameters(SyncLatestParams { state, download }): Parameters<SyncLatestParams>,
    ) -> Result<Json<SyncResultDto>, ErrorData> {
        let (scf_url, api_key) = self.scf_config()?;
        let cache = self.cache();
        let app_state = self.app.state::<crate::AppState>();
        let client = app_state.client.clone();
        let cache_dir = app_state.cache_dir.clone();

        let st = match state.as_deref().unwrap_or("open") {
            "all" => "all",
            "closed" => "closed",
            _ => "open",
        };

        let result = if download.unwrap_or(true) {
            crate::commands::issue::sync_and_download(st, &scf_url, &api_key, &client, &cache, &cache_dir)
                .await
                .map_err(|e| ErrorData::internal_error(format!("同步失败: {e}"), None))?
        } else {
            crate::commands::issue::sync_cache_incremental(&scf_url, &api_key, &client, &cache)
                .await
                .map_err(|e| ErrorData::internal_error(format!("同步缓存失败: {e}"), None))?;
            crate::commands::issue::DownloadMissingResult {
                downloaded: 0,
                skipped: 0,
                failed: Vec::new(),
            }
        };

        // 返回同步后的最新一页（与界面同源），供 AI 直接看当前反馈
        let (items, _has_more) = {
            let c = cache.clone();
            tauri::async_runtime::spawn_blocking(move || c.list_issues("all", 1, 30))
                .await
                .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
                .map_err(|e| ErrorData::internal_error(format!("读取 Issue 缓存失败: {e}"), None))?
        };

        Ok(Json(SyncResultDto {
            issues: items
                .into_iter()
                .map(|i| RemoteIssueDto {
                    number: i.number,
                    report_id: i.report_id,
                    title: i.title,
                    state: i.state,
                    issue_url: i.issue_url,
                    created_at: i.created_at,
                })
                .collect(),
            has_more: false,
            downloaded: result.downloaded,
            skipped: result.skipped,
            failed: result.failed,
        }))
    }

    #[tool(
        name = "issue_stats",
        description = "查询各状态的用户反馈 Issue 数量（未处理 open / 已处理 closed / 全部 all，来自本地缓存镜像，与灵鉴界面问题列表同源，不实时回源）。数据不是最新的话先调 sync_latest 同步",
        annotations(read_only_hint = true)
    )]
    async fn issue_stats(&self) -> Result<Json<IssueStatsDto>, ErrorData> {
        let _ = self.scf_config()?;
        let cache = self.cache();

        let counts = {
            let c = cache.clone();
            tauri::async_runtime::spawn_blocking(move || c.count_issues_by_state())
                .await
                .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
                .map_err(|e| ErrorData::internal_error(format!("查询 Issue 计数失败: {e}"), None))?
        };
        let cached_at = {
            let c = cache.clone();
            tauri::async_runtime::spawn_blocking(move || c.issues_cached_at())
                .await
                .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
                .map_err(|e| ErrorData::internal_error(format!("查询缓存时间失败: {e}"), None))?
        };

        Ok(Json(IssueStatsDto {
            open: counts.open,
            closed: counts.closed,
            all: counts.all,
            cached_at,
        }))
    }

    /// 每页条数与界面问题列表/SCF 端点一致（github.js PER_PAGE=30）
    const REMOTE_PER_PAGE: u32 = 30;

    #[tool(
        name = "list_remote_issues",
        description = "列出远端反馈 Issue（未处理/已处理/全部，来自本地缓存镜像，与灵鉴界面问题列表同源，不实时回源）。条目带 downloaded 标记——日志已下载的可直接 analyze_report；未下载的调 sync_latest 同步（会下载缺失日志）。要的是已分析过的本地记录用 list_issues",
        annotations(read_only_hint = true)
    )]
    async fn list_remote_issues(
        &self,
        Parameters(ListRemoteIssuesParams { state, page }): Parameters<ListRemoteIssuesParams>,
    ) -> Result<Json<RemoteIssueListResult>, ErrorData> {
        let _ = self.scf_config()?;
        let cache = self.cache();

        let st = match state.as_deref().unwrap_or("open") {
            "all" => "all",
            "closed" => "closed",
            _ => "open",
        };
        let pg = page.unwrap_or(1).max(1);

        let (items, has_more) = {
            let c = cache.clone();
            let s = st.to_string();
                tauri::async_runtime::spawn_blocking(move || c.list_issues(&s, pg, Self::REMOTE_PER_PAGE))
                .await
                .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
                .map_err(|e| ErrorData::internal_error(format!("读取 Issue 缓存失败: {e}"), None))?
        };
        let cached_at = {
            let c = cache.clone();
            tauri::async_runtime::spawn_blocking(move || c.issues_cached_at())
                .await
                .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
                .map_err(|e| ErrorData::internal_error(format!("查询缓存时间失败: {e}"), None))?
        };
        // 批量标记日志下载状态（已下载的才能 analyze_report）
        let downloaded = {
            let c = cache.clone();
            let rids: Vec<String> = items.iter().map(|i| i.report_id.clone()).collect();
            tauri::async_runtime::spawn_blocking(move || c.reports_exist(&rids))
                .await
                .map_err(|e| ErrorData::internal_error(format!("查询任务失败: {e}"), None))?
                .map_err(|e| ErrorData::internal_error(format!("查询下载状态失败: {e}"), None))?
        };

        let issues = items
            .into_iter()
            .zip(downloaded)
            .map(|(i, dl)| RemoteIssueEntryDto {
                number: i.number,
                report_id: i.report_id,
                title: i.title,
                state: i.state,
                issue_url: i.issue_url,
                created_at: i.created_at,
                labels: i.labels,
                app_version: i.app_version,
                platform: i.platform,
                realm: i.realm,
                player_id: i.player_id,
                player_name: i.player_name,
                downloaded: dl,
            })
            .collect();

        Ok(Json(RemoteIssueListResult {
            issues,
            page: pg,
            has_more,
            cached_at,
        }))
    }

    #[tool(
        name = "add_comment",
        description = "在 GitHub Issue 上发表评论（经灵鉴本机端 → SCF 服务端代理转发，需灵鉴设置页开启「允许写操作」）"
    )]
    async fn add_comment(
        &self,
        Parameters(AddCommentParams { issue_number, body }): Parameters<AddCommentParams>,
    ) -> Result<Json<IssueActionResultDto>, ErrorData> {
        self.act_on_issue(issue_number, "comment", Some(&body), None)
            .await
            .map(Json)
    }

    #[tool(
        name = "update_labels",
        description = "整体替换 GitHub Issue 的标签集合（经 SCF 代理转发，需灵鉴设置页开启「允许写操作」）。注意是替换而非追加，调用前先用 list_issues 确认现有标签"
    )]
    async fn update_labels(
        &self,
        Parameters(UpdateLabelsParams { issue_number, labels }): Parameters<UpdateLabelsParams>,
    ) -> Result<Json<IssueActionResultDto>, ErrorData> {
        self.act_on_issue(issue_number, "setLabels", None, Some(&labels))
            .await
            .map(Json)
    }

    #[tool(
        name = "close_issue",
        description = "关闭 GitHub Issue（经 SCF 代理转发，需灵鉴设置页开启「允许写操作」）。传 fixed_in 版本号时执行与灵鉴界面一致的完整流程：关闭 + 追加 v<版本号> 标签 + 发表解决评论；不传则仅关闭。AI 定位修复后建议带上修复版本号"
    )]
    async fn close_issue(
        &self,
        Parameters(CloseIssueParams { issue_number, fixed_in }): Parameters<CloseIssueParams>,
    ) -> Result<Json<IssueActionResultDto>, ErrorData> {
        match fixed_in.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(v) => self.close_issue_full(issue_number, v).await.map(Json),
            None => self.act_on_issue(issue_number, "close", None, None).await.map(Json),
        }
    }

    #[tool(
        name = "reopen_issue",
        description = "重新打开已关闭的 GitHub Issue（经 SCF 代理转发，需灵鉴设置页开启「允许写操作」），与灵鉴界面的「重新打开」一致"
    )]
    async fn reopen_issue(
        &self,
        Parameters(ReopenIssueParams { issue_number }): Parameters<ReopenIssueParams>,
    ) -> Result<Json<IssueActionResultDto>, ErrorData> {
        self.act_on_issue(issue_number, "reopen", None, None)
            .await
            .map(Json)
    }
}

// get_info 由 #[tool_handler] 生成，名称/引导语在此定制（version 宏属性不支持表达式，用默认值）
#[tool_handler(
    name = "lingjian",
    instructions = "灵鉴（LingJian）日志分析工具。先用 issue_stats 看处理进度、list_remote_issues 看远端反馈（数据来自本地缓存），需要最新数据或日志未下载时调 sync_latest 同步（列表 + 缺失日志）；analyze_report 获取错误聚合与统计，query_logs 分页查看原始日志，list_issues/get_report 查本地已有上报；分析定位后回写处理结果：close_issue 传 fixed_in 版本号即走完整关单流程（关闭+版本标签+解决评论，与灵鉴界面一致），另有 add_comment / update_labels / reopen_issue（写操作需灵鉴设置页开启「允许写操作」）。"
)]
impl ServerHandler for LingjianServer {}