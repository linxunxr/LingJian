use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::services::downloader;
use crate::services::secret::{self, Secret};

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

/// 迁移结果（前端据此提示；ApiKeySource 供 UI 说明当前存储位置）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateResult {
    /// 钥匙串里是否已有 Key（迁移成功或本就存在）
    pub keyring_ready: bool,
    /// 本次是否发生了实际迁移（settings.json 里有明文并已擦除）
    pub migrated: bool,
}

/// 把 settings.json 里的明文 apiKey 迁入系统钥匙串并擦除明文。
///
/// 背景：apiKey 曾因旧版 keyring 在部分 Windows 环境写入静默失败而
/// 降级明文存储（411d123）；现 keyring 3.x 已验证可靠，回迁系统
/// 凭据管理器（Windows 凭据管理器 / macOS 钥匙串 / Linux Secret
/// Service），settings.json 只留非敏感配置。
///
/// 幂等：钥匙串已有 Key 时跳过写入只擦明文；明文不存在时为空跑。
/// 钥匙串写入失败时不擦明文并报错——保底配置可用，下次启动重试。
#[tauri::command]
pub fn migrate_api_key(app: AppHandle) -> Result<MigrateResult, String> {
    let store = app
        .store("settings.json")
        .map_err(|e| format!("打开设置失败: {e}"))?;
    let plaintext = store
        .get("apiKey")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    // 已在钥匙串（此前迁移过或本就存在）：只负责清掉残留明文
    let existing = secret::get(Secret::ScfApiKey)?;
    if existing.is_empty() && !plaintext.is_empty() {
        secret::set(Secret::ScfApiKey, &plaintext)?;
    }

    let migrated = !plaintext.is_empty();
    if migrated {
        store.delete("apiKey");
        store.save().map_err(|e| format!("保存设置失败: {e}"))?;
    }
    Ok(MigrateResult {
        keyring_ready: !existing.is_empty() || !plaintext.is_empty(),
        migrated,
    })
}
