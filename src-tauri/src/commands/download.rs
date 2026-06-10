use serde::Serialize;

#[derive(Serialize)]
pub struct DownloadResult {
    pub report_id: String,
    pub log_count: usize,
    pub file_size: u64,
}

#[tauri::command]
pub async fn download_log(_report_id: String) -> Result<DownloadResult, String> {
    // TODO: 实现 COS 日志下载 + gzip 解压
    Err("尚未实现".into())
}
