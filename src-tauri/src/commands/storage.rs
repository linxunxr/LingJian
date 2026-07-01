use serde::Serialize;
use tauri::State;

use crate::services::paths;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    /// 当前生效的数据目录绝对路径
    pub data_dir: String,
    /// 数据目录总大小（字节）
    pub size: u64,
    /// 是否为跟随安装位置（exe 同级 data/）
    pub portable: bool,
}

/// 获取当前数据目录信息
#[tauri::command]
pub fn get_storage_info(state: State<'_, crate::AppState>) -> StorageInfo {
    let data_dir = &state.data_dir;
    let size = paths::dir_size(data_dir);
    let portable = is_portable(data_dir, &state.fallback_dir);
    StorageInfo {
        data_dir: data_dir.to_string_lossy().to_string(),
        size,
        portable,
    }
}

/// 切换数据目录：迁移现有数据到新目录，提示用户重启生效。
///
/// 注意：迁移后数据库连接仍指向旧库文件，需重启应用以重新打开新位置的库。
#[tauri::command]
pub fn change_data_dir(new_dir: String, state: State<'_, crate::AppState>) -> Result<String, String> {
    let new_path = std::path::PathBuf::from(&new_dir);
    let result = paths::change_data_dir(&state.fallback_dir, &new_path)?;
    Ok(result.to_string_lossy().to_string())
}

/// 计算当前数据目录大小（字节）
#[tauri::command]
pub fn get_cache_size(state: State<'_, crate::AppState>) -> u64 {
    paths::dir_size(&state.cache_dir)
}

/// 清理 gzip 缓存（保留数据库）
#[tauri::command]
pub fn clear_cache(state: State<'_, crate::AppState>) -> Result<(), String> {
    paths::clear_cache(&state.data_dir)
}

/// 判断数据目录是否为便携模式（exe 同级而非系统默认目录）
fn is_portable(data_dir: &std::path::Path, fallback_dir: &std::path::Path) -> bool {
    data_dir != fallback_dir
}
