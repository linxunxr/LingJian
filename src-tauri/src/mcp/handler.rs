//! MCP server 的工具集实现：薄封装 services 层，面向 LLM 控制响应体量。

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ErrorData, Json, ServerHandler};

use crate::models::report::Report;
use crate::services::cache::Cache;

use super::dto::{IssueBriefDto, IssueListResult, ListIssuesParams};

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
}

// get_info 由 #[tool_handler] 自动生成（server 名称/版本取自 Cargo.toml）
#[tool_handler]
impl ServerHandler for LingjianServer {}
