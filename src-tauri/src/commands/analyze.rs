use tauri::{async_runtime::spawn_blocking, State};

use crate::models::analyze::{AnalysisResult, LogFilter};
use crate::services::analyzer::analyze;
use crate::services::cache::Cache;

/// 读取已缓存的日志并执行分析
#[tauri::command]
pub async fn analyze_log(
    report_id: String,
    filter: LogFilter,
    state: State<'_, crate::AppState>,
) -> Result<AnalysisResult, String> {
    let cache: std::sync::Arc<Cache> = state.cache.clone();
    let report_id_for_task = report_id.clone();
    let entries = spawn_blocking(move || cache.get_entries(&report_id_for_task))
        .await
        .map_err(|e| format!("读取任务失败: {e}"))??;

    if entries.is_empty() {
        return Err(format!("未找到上报 {report_id} 的日志，请先下载"));
    }

    // 分析是 CPU 密集，同样放到阻塞线程
    let filter_clone = filter.clone();
    let result = spawn_blocking(move || analyze(&entries, &filter_clone))
        .await
        .map_err(|e| format!("分析任务失败: {e}"))?;

    Ok(result)
}
