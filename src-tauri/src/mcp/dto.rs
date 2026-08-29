//! MCP 工具的请求/响应 DTO。
//!
//! 独立于 models 层定义（models 不引入 schemars），
//! 经 From 转换对接，保持领域类型零侵入。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::models::report::Report;

/// list_issues 入参
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListIssuesParams {
    /// 返回条数上限，默认 20，最大 100
    #[serde(default)]
    pub limit: Option<usize>,
    /// 按 GitHub Issue 编号过滤（在返回窗口内过滤）
    #[serde(default)]
    pub issue_number: Option<i64>,
}

/// 上报记录摘要（用户反馈原文全量返回，通常很短且信息密度最高）
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueBriefDto {
    pub report_id: String,
    pub issue_number: Option<i32>,
    pub issue_title: Option<String>,
    pub app_name: Option<String>,
    pub app_version: Option<String>,
    pub platform: Option<String>,
    pub realm: Option<String>,
    /// 游玩时长（秒）
    pub play_time: Option<u64>,
    /// 用户原始反馈描述
    pub user_description: Option<String>,
    pub log_count: usize,
    pub report_time: String,
    pub downloaded_at: String,
}

impl From<Report> for IssueBriefDto {
    fn from(r: Report) -> Self {
        Self {
            report_id: r.report_id,
            issue_number: r.issue_number,
            issue_title: r.issue_title,
            app_name: r.app_name,
            app_version: r.app_version,
            platform: r.platform,
            realm: r.realm,
            play_time: r.play_time,
            user_description: r.user_description,
            log_count: r.log_count,
            report_time: r.report_time,
            downloaded_at: r.downloaded_at,
        }
    }
}

/// list_issues 返回
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueListResult {
    pub issues: Vec<IssueBriefDto>,
}
