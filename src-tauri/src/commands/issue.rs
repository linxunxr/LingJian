use serde::Serialize;
use tauri::State;

use crate::services::github::{self, GitHubClient};

/// URL/编号解析结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub owner: String,
    pub repo: String,
    pub number: u32,
}

/// 解析用户输入为 Issue 定位信息（纯逻辑，不调网络）
#[tauri::command]
pub fn parse_issue_url(url: String) -> Result<ParseResult, String> {
    let parsed = github::parse_issue_input(&url)?;
    Ok(ParseResult {
        owner: parsed.owner,
        repo: parsed.repo,
        number: parsed.number,
    })
}

/// 通过 GitHub API 获取 Issue 信息并解析 reportId
#[tauri::command]
pub async fn fetch_issue_info(
    owner: String,
    repo: String,
    number: u32,
    github_token: String,
    http: State<'_, crate::AppState>,
) -> Result<crate::services::github::IssueInfo, String> {
    if github_token.trim().is_empty() {
        return Err("未配置 GitHub Token，请先到设置页填写".to_string());
    }
    let client = GitHubClient::new(http.client.clone(), github_token);
    client.fetch_issue(&owner, &repo, number).await
}

/// 判断输入是否为纯 reportId（供前端决定是否跳过 Issue 解析）
#[tauri::command]
pub fn is_report_id_input(input: String) -> bool {
    github::is_report_id(&input)
}
