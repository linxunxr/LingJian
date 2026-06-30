use tauri::State;

use crate::services::downloader;

/// 测试 SCF 下载端点连通性（用探测 id 验证端点可达 + 鉴权配置）
#[tauri::command]
pub async fn test_scf_endpoint(
    scf_url: String,
    api_key: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    if scf_url.trim().is_empty() {
        return Err("请先填写 SCF URL".to_string());
    }
    downloader::test_endpoint(&scf_url, &api_key, &state.client).await
}
