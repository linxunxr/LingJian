use serde::Serialize;
use tauri::State;

use crate::services::{downloader, github};

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

/// 通过 SCF 端点获取 Issue 信息（服务端代理解析 reportId，无需 GitHub Token）
#[tauri::command]
pub async fn fetch_issue_info(
    number: u32,
    scf_url: String,
    api_key: String,
    http: State<'_, crate::AppState>,
) -> Result<crate::services::github::IssueInfo, String> {
    if scf_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("未配置 SCF 端点，请先到设置页填写".to_string());
    }
    downloader::resolve_issue(&scf_url, number, &api_key, &http.client).await
}

/// 通过 SCF 端点拉取上报问题列表（首页问题列表用）
///
/// - `state`：状态筛选，可选 "open" / "all"，默认 "open"
/// - `page`：页码，默认 1
#[tauri::command]
pub async fn list_issues(
    state: Option<String>,
    page: Option<u32>,
    scf_url: String,
    api_key: String,
    http: State<'_, crate::AppState>,
) -> Result<crate::services::github::IssueList, String> {
    if scf_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("未配置 SCF 端点，请先到设置页填写".to_string());
    }
    // 规范化参数，防止传入异常值
    let st = match state.as_deref().unwrap_or("open") {
        "all" => "all",
        _ => "open",
    };
    let pg = page.unwrap_or(1).max(1);
    downloader::list_issues(&scf_url, st, pg, &api_key, &http.client).await
}

/// 判断输入是否为纯 reportId（供前端决定是否跳过 Issue 解析）
#[tauri::command]
pub fn is_report_id_input(input: String) -> bool {
    github::is_report_id(&input)
}
