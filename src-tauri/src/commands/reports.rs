use tauri::{async_runtime::spawn_blocking, State};

use crate::models::report::Report;
use crate::services::cache::Cache;

/// 列出最近下载的上报记录（首页"最近分析"用）
#[tauri::command]
pub async fn list_recent_reports(
    limit: Option<usize>,
    state: State<'_, crate::AppState>,
) -> Result<Vec<Report>, String> {
    let cache: std::sync::Arc<Cache> = state.cache.clone();
    let limit = limit.unwrap_or(20);
    spawn_blocking(move || cache.list_recent_reports(limit))
        .await
        .map_err(|e| format!("查询任务失败: {e}"))?
}
