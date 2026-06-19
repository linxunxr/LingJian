use tauri::{async_runtime::spawn_blocking, State};

use crate::services::cache::Cache;
use crate::services::exporter::{render, ExportFormat};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub bytes: usize,
}

/// 导出指定 report 为文件。
///
/// 前端先用 dialog save 拿到目标路径，再调用此命令写盘。
#[tauri::command]
pub async fn export_report(
    report_id: String,
    format: String,
    path: String,
    state: State<'_, crate::AppState>,
) -> Result<ExportResult, String> {
    let format = ExportFormat::parse(&format)?;

    // 从缓存读取 report 元信息 + 日志
    let cache: std::sync::Arc<Cache> = state.cache.clone();
    let report_id_for_task = report_id.clone();
    let report = spawn_blocking(move || cache.get_report(&report_id_for_task))
        .await
        .map_err(|e| format!("查询任务失败: {e}"))??
        .ok_or_else(|| format!("未找到上报 {report_id}"))?;

    let cache: std::sync::Arc<Cache> = state.cache.clone();
    let report_id_for_entries = report_id.clone();
    let entries = spawn_blocking(move || cache.get_entries(&report_id_for_entries))
        .await
        .map_err(|e| format!("读取任务失败: {e}"))??;

    // 生成内容 + 写盘
    let content = render(&report, &entries, format);
    let bytes = content.len();
    std::fs::write(&path, content).map_err(|e| format!("写入文件失败: {e}"))?;

    Ok(ExportResult { path, bytes })
}
