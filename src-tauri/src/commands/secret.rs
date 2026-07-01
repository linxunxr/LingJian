use serde::{Deserialize, Serialize};

use crate::services::secret::{self, Secret};

/// 凭证类型：SCF API Key
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretKind {
    ScfApiKey,
}

impl From<SecretKind> for Secret {
    fn from(kind: SecretKind) -> Self {
        match kind {
            SecretKind::ScfApiKey => Secret::ScfApiKey,
        }
    }
}

/// 保存凭证到系统钥匙串
#[tauri::command]
pub fn set_secret(kind: SecretKind, value: String) -> Result<(), String> {
    secret::set(kind.into(), &value)
}

/// 读取凭证（不存在返回空串）
#[tauri::command]
pub fn get_secret(kind: SecretKind) -> Result<String, String> {
    secret::get(kind.into())
}

/// 删除凭证
#[tauri::command]
pub fn delete_secret(kind: SecretKind) -> Result<(), String> {
    secret::delete(kind.into())
}
