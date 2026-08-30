use tauri::AppHandle;

use crate::mcp::{self, McpStatus};

/// 更新 MCP 配置（写 settings 后立即重启 server 生效，无需重启应用）
#[tauri::command]
pub async fn mcp_set_config(
    app: AppHandle,
    enabled: bool,
    port: u16,
    allow_write: bool,
) -> Result<McpStatus, String> {
    // 参数校验放最前：合法端口范围之外直接拒绝，避免写出不可用配置
    if !(1..=65535).contains(&port) {
        return Err(format!("端口 {port} 不合法（1-65535）"));
    }

    {
        use tauri_plugin_store::StoreExt;
        let store = app.store("settings.json").map_err(|e| format!("打开设置失败: {e}"))?;
        store.set("mcpEnabled", serde_json::json!(enabled));
        store.set("mcpPort", serde_json::json!(port));
        store.set("mcpAllowWrite", serde_json::json!(allow_write));
        store.save().map_err(|e| format!("保存设置失败: {e}"))?;
    }

    // 重启 server 涉及网络 bind，放到阻塞线程避免卡 UI
    let r = tauri::async_runtime::spawn_blocking(move || mcp::apply_config(&app))
        .await
        .map_err(|e| format!("配置任务失败: {e}"))??;
    Ok(r)
}

/// 查询 MCP server 运行状态
#[tauri::command]
pub fn mcp_status(app: AppHandle) -> Result<McpStatus, String> {
    Ok(mcp::status(&app))
}
