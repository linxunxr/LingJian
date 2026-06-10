use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub issue_url: Option<String>,
    pub downloaded_at: Option<String>,
    pub log_count: usize,
}
