use serde::{Deserialize, Serialize};
use tauri::{async_runtime::spawn_blocking, State};

use crate::models::report::Report;
use crate::services::{cache::Cache, downloader};

/// 可选传入的 Issue 元信息（增强项 A：丰富本地 Report 字段）。
///
/// 由前端从 `fetch_issue_info` 的返回值透传过来；纯 reportId 输入时为 None。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueMeta {
    pub issue_number: Option<i32>,
    pub issue_title: Option<String>,
    pub app_version: Option<String>,
    pub platform: Option<String>,
    pub realm: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub report_id: String,
    pub log_count: usize,
    pub file_size: u64,
}

/// 下载日志包：解压 → 缓存入库 → 返回摘要
///
/// `report_id` 由前端从 Issue 解析得到或用户直接输入。
/// `issue_meta` 为可选的 Issue 元信息，存在则一并落库（首页列表更友好）。
#[tauri::command]
pub async fn download_log(
    report_id: String,
    scf_url: String,
    api_key: String,
    #[allow(unused_variables)] issue_meta: Option<IssueMeta>,
    state: State<'_, crate::AppState>,
) -> Result<DownloadResult, String> {
    if scf_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("未配置 SCF 下载端点，请先到设置页填写".to_string());
    }

    let cache_dir = state.cache_dir.clone();
    let http = state.client.clone();

    // 下载 + 解压（IO 密集）
    let (entries, file_size) = downloader::download(&scf_url, &report_id, &api_key, &http, &cache_dir)
        .await?;
    let log_count = entries.len();

    // 构造 Report 并落库（DB 调用走 spawn_blocking）
    let now = chrono::Utc::now().to_rfc3339();
    let report = Report {
        report_id: report_id.clone(),
        issue_number: issue_meta.as_ref().and_then(|m| m.issue_number),
        issue_title: issue_meta.as_ref().and_then(|m| m.issue_title.clone()),
        app_version: issue_meta.as_ref().and_then(|m| m.app_version.clone()),
        platform: issue_meta.as_ref().and_then(|m| m.platform.clone()),
        realm: issue_meta.as_ref().and_then(|m| m.realm.clone()),
        play_time: None,
        user_description: None,
        report_time: now.clone(),
        log_count,
        downloaded_at: now,
    };

    let cache: std::sync::Arc<Cache> = state.cache.clone();
    spawn_blocking(move || cache.save_report(&report, &entries))
        .await
        .map_err(|e| format!("缓存任务失败: {e}"))??;

    Ok(DownloadResult {
        report_id,
        log_count,
        file_size,
    })
}
