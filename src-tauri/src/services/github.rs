use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 默认仓库（仅给纯编号输入兜底用，仅用于解析 Issue 编号）
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
///
/// 注意：owner/repo/title 由 SCF `/issue/:number` 端点返回；
/// `report_id` 是后续下载日志的关键字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueInfo {
    pub owner: String,
    pub repo: String,
    pub number: u32,
    /// 从 Issue body 解析出的上报编号
    pub report_id: String,
    pub title: String,
    /// 上报环境信息（增强项，由 SCF 从 Issue body 环境表格提取，可选）
    #[serde(default)]
    pub app_version: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub realm: Option<String>,
}

/// Issue 列表项（SCF `/issues` 端点返回）
///
/// 与 `IssueInfo` 的区别：列表项额外携带 `state` / `issue_url` / `created_at`，
/// 用于首页列表展示状态与时间。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueListItem {
    pub number: u32,
    pub report_id: String,
    pub title: String,
    /// Issue 状态：open / closed
    pub state: String,
    pub issue_url: String,
    /// 创建时间（ISO 8601）
    pub created_at: String,
    pub owner: String,
    pub repo: String,
    #[serde(default)]
    pub app_version: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub realm: Option<String>,
    /// 当前标签（操作后更新）
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

/// SCF `/issues` 端点的完整响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueList {
    pub issues: Vec<IssueListItem>,
    pub page: u32,
    pub has_more: bool,
}

/// SCF `/issue/:number/action` 端点的响应
///
/// close/reopen 后含 state + labels；comment 后只有 ok；setLabels 后含 labels。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueActionResponse {
    pub ok: bool,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
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
    fn detect_report_id() {
        assert!(is_report_id("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_report_id("42"));
        assert!(!is_report_id("https://github.com/x/y/issues/1"));
    }
}
