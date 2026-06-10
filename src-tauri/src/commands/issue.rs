use serde::Serialize;

#[derive(Serialize)]
pub struct IssueInfo {
    pub owner: String,
    pub repo: String,
    pub number: u32,
    pub report_id: String,
    pub title: String,
}

#[tauri::command]
pub async fn parse_issue_url(url: String) -> Result<IssueInfo, String> {
    // TODO: 实现 Issue URL 解析
    Err("尚未实现".into())
}

#[tauri::command]
pub async fn fetch_issue_info(owner: String, repo: String, number: u32) -> Result<IssueInfo, String> {
    // TODO: 实现通过 GitHub API 获取 Issue 信息并解析 reportId
    Err("尚未实现".into())
}
