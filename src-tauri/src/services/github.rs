use regex::Regex;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 默认仓库（仅给纯编号输入兜底用）
const DEFAULT_OWNER: &str = "linxunxr";
const DEFAULT_REPO: &str = "PathofIdleImmortals-bugs";

/// 从 Issue URL/编号解析出的定位信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedIssue {
    pub owner: String,
    pub repo: String,
    pub number: u32,
}

/// Issue 完整信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueInfo {
    pub owner: String,
    pub repo: String,
    pub number: u32,
    /// 从 Issue body 解析出的上报编号
    pub report_id: String,
    pub title: String,
}

/// REPORT_ID 提取正则：匹配 Issue body 中注入的 HTML 注释
/// 形如 `<!-- REPORT_ID: 550e8400-e29b-41d4-a716-446655440000 -->`
fn report_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"<!--\s*REPORT_ID:\s*([0-9a-fA-F-]{36})\s*-->").unwrap()
    })
}

/// Issue URL 正则：https://github.com/{owner}/{repo}/issues/{number}
fn issue_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"github\.com/([^/]+)/([^/]+)/issues/(\d+)").unwrap()
    })
}

/// UUID 正则（用于识别纯 reportId 输入）
fn uuid_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap()
    })
}

/// 解析用户输入（URL / `#42` / `42`）为 Issue 定位信息。
///
/// 注意：纯 reportId 输入不在此处理（由上层直接走下载流程）。
pub fn parse_issue_input(input: &str) -> Result<ParsedIssue, String> {
    let trimmed = input.trim();

    // 1. 完整 URL
    if let Some(caps) = issue_url_re().captures(trimmed) {
        return Ok(ParsedIssue {
            owner: caps[1].to_string(),
            repo: caps[2].to_string(),
            number: caps[3].parse().map_err(|_| "Issue 编号格式错误".to_string())?,
        });
    }

    // 2. #42 或 42
    let num_str = trimmed.trim_start_matches('#');
    if let Ok(num) = num_str.parse::<u32>() {
        if !trimmed.is_empty() {
            return Ok(ParsedIssue {
                owner: DEFAULT_OWNER.to_string(),
                repo: DEFAULT_REPO.to_string(),
                number: num,
            });
        }
    }

    Err(format!("无法解析 Issue 输入: {trimmed}"))
}

/// 判断输入是否为纯 reportId（UUID 形式）
pub fn is_report_id(input: &str) -> bool {
    uuid_re().is_match(input.trim())
}

/// 从 Issue body 中提取 reportId
pub fn extract_report_id(body: &str) -> Option<String> {
    report_id_re()
        .captures(body)
        .map(|c| c[1].to_lowercase())
}

/// GitHub API 返回的 Issue JSON 子集
#[derive(Debug, Deserialize)]
struct GithubIssue {
    title: String,
    body: Option<String>,
}

/// GitHub API 交互客户端
pub struct GitHubClient {
    client: reqwest::Client,
    token: String,
}

impl GitHubClient {
    pub fn new(client: reqwest::Client, token: String) -> Self {
        Self { client, token }
    }

    /// 获取 Issue 并解析出 reportId
    pub async fn fetch_issue(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
    ) -> Result<IssueInfo, String> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{number}");

        let resp = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(USER_AGENT, "LingJian/0.1")
            .send()
            .await
            .map_err(|e| format!("GitHub 请求失败: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("GitHub 返回 {status}: {text}"));
        }

        let issue: GithubIssue = resp
            .json()
            .await
            .map_err(|e| format!("解析 GitHub 响应失败: {e}"))?;

        let body = issue.body.unwrap_or_default();
        let report_id = extract_report_id(&body)
            .ok_or_else(|| "Issue body 中未找到 REPORT_ID".to_string())?;

        Ok(IssueInfo {
            owner: owner.to_string(),
            repo: repo.to_string(),
            number,
            report_id,
            title: issue.title,
        })
    }

    /// 验证 Token 有效性：调用 /user 端点，成功返回用户登录名
    pub async fn verify_token(&self) -> Result<String, String> {
        let resp = self
            .client
            .get("https://api.github.com/user")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(USER_AGENT, "LingJian/0.1")
            .send()
            .await
            .map_err(|e| format!("GitHub 请求失败: {e}"))?;

        let status = resp.status();
        if status.as_u16() == 401 {
            return Err("Token 无效或已过期".to_string());
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("GitHub 返回 {status}: {text}"));
        }

        #[derive(serde::Deserialize)]
        struct User {
            login: String,
        }
        let user: User = resp
            .json()
            .await
            .map_err(|e| format!("解析用户信息失败: {e}"))?;
        Ok(user.login)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_url() {
        let r = parse_issue_input(
            "https://github.com/linxunxr/PathofIdleImmortals-bugs/issues/42",
        )
        .unwrap();
        assert_eq!(r.owner, "linxunxr");
        assert_eq!(r.repo, "PathofIdleImmortals-bugs");
        assert_eq!(r.number, 42);
    }

    #[test]
    fn parse_hash_number() {
        let r = parse_issue_input("#41").unwrap();
        assert_eq!(r.number, 41);
        assert_eq!(r.owner, "linxunxr");
    }

    #[test]
    fn parse_plain_number() {
        let r = parse_issue_input("39").unwrap();
        assert_eq!(r.number, 39);
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_issue_input("not an issue").is_err());
    }

    #[test]
    fn extract_id_from_body() {
        let body = "## 反馈\n<!-- REPORT_ID: 550E8400-e29b-41d4-a716-446655440000 -->\n正文";
        assert_eq!(
            extract_report_id(body),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn extract_id_missing() {
        assert!(extract_report_id("没有注释的正文").is_none());
    }

    #[test]
    fn detect_report_id() {
        assert!(is_report_id("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_report_id("42"));
        assert!(!is_report_id("https://github.com/x/y/issues/1"));
    }
}
