use serde::{Deserialize, Serialize};

/// 一次日志上报记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// 上报唯一标识（UUID）
    pub report_id: String,
    /// 关联的 GitHub Issue 编号
    pub issue_number: Option<i32>,
    /// 关联的 GitHub Issue 标题
    pub issue_title: Option<String>,
    /// 游戏版本
    pub app_version: Option<String>,
    /// 平台
    pub platform: Option<String>,
    /// 当前境界
    pub realm: Option<String>,
    /// 游玩时长（秒）
    pub play_time: Option<u64>,
    /// 用户问题描述
    pub user_description: Option<String>,
    /// 上报时间（ISO 8601）
    pub report_time: String,
    /// 日志条目数
    pub log_count: usize,
    /// 灵鉴下载该日志的时间（ISO 8601）
    pub downloaded_at: String,
}
