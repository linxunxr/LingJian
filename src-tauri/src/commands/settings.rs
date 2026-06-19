use tauri::State;

use crate::services::downloader;
use crate::services::github::GitHubClient;

/// 验证 GitHub Token：调用 /user 端点，成功返回登录名
#[tauri::command]
pub async fn verify_github_token(
    github_token: String,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    if github_token.trim().is_empty() {
        return Err("请先填写 GitHub Token".to_string());
    }
    let client = GitHubClient::new(state.client.clone(), github_token);
    client.verify_token().await
}

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
