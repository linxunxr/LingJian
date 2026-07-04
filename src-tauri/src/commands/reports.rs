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
    let db_path = state.data_dir.join("lingjian.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    spawn_blocking(move || cache.list_recent_reports(limit))
        .await
        .map_err(|e| format!("查询任务失败: {e}"))?
        .map(|reports| {
            // 首条记录带数据库路径前缀，便于前端诊断
            if reports.is_empty() {
                log::warn!("[list_recent_reports] 数据库 {} 返回 0 条记录", db_path_str);
            }
            reports
        })
}
