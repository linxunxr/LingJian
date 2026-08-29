//! 进程内 MCP server（Streamable HTTP）。
//!
//! 生命周期随灵鉴应用：setup 时按 settings 拉起，配置变更时重启，应用退出自动结束。
//! 仅绑定 127.0.0.1，不对外网暴露。

pub mod dto;
pub mod handler;

use std::sync::{Mutex, OnceLock};

use tauri::Manager;

use handler::LingjianServer;

/// 默认监听端口
pub const DEFAULT_PORT: u16 = 3920;

/// MCP server 运行状态（供 mcp_status / 设置页展示）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    /// 配置的总开关（settings.json 的 mcpEnabled）
    pub enabled: bool,
    /// server 是否实际在监听
    pub running: bool,
    /// 配置端口（未启用时也返回，供设置页回填）
    pub port: u16,
    /// 监听 URL（未运行时为 None）
    pub listening_url: Option<String>,
}

struct Runtime {
    task: Option<tauri::async_runtime::JoinHandle<()>>,
    port: Option<u16>,
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

fn runtime() -> &'static Mutex<Runtime> {
    RUNTIME.get_or_init(|| {
        Mutex::new(Runtime {
            task: None,
            port: None,
        })
    })
}

/// 读取 MCP 配置（settings.json 的 mcpEnabled / mcpPort，缺省 false / 3920）
fn read_mcp_settings(app: &tauri::AppHandle) -> (bool, u16) {
    use tauri_plugin_store::StoreExt;
    let Ok(store) = app.store("settings.json") else {
        return (false, DEFAULT_PORT);
    };
    let enabled = store
        .get("mcpEnabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let port = store
        .get("mcpPort")
        .and_then(|v| v.as_u64())
        .map(|v| v as u16)
        .unwrap_or(DEFAULT_PORT);
    (enabled, port)
}

/// 停止当前 server（若有）
pub fn stop() {
    if let Some(task) = runtime().lock().ok().and_then(|mut r| r.task.take()) {
        task.abort();
    }
    if let Ok(mut r) = runtime().lock() {
        r.port = None;
    }
}

/// 拉起 server：绑定 127.0.0.1:port，绑定失败（端口被占等）同步返回错误。
fn start(app: &tauri::AppHandle, port: u16) -> Result<(), String> {
    let cache = app
        .state::<crate::AppState>()
        .cache
        .clone();

    // 在当前上下文完成 bind，端口冲突能立即报给调用方；
    // 随后把 listener 交给后台任务 serve
    let listener = tauri::async_runtime::block_on(async {
        tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|e| format!("MCP 端口 {port} 监听失败：{e}"))
    })?;

    let service =
        rmcp::transport::streamable_http_server::tower::StreamableHttpService::new(
            move || Ok(LingjianServer::new(cache.clone())),
            std::sync::Arc::new(
                rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
            ),
            rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig::default(),
        );
    let router = axum::Router::new().route_service("/mcp", service);

    let task = tauri::async_runtime::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            log::error!("[mcp] server 异常退出: {e}");
        }
    });

    if let Ok(mut r) = runtime().lock() {
        r.task = Some(task);
        r.port = Some(port);
    }
    log::info!("[mcp] listening on http://127.0.0.1:{port}/mcp");
    Ok(())
}

/// 按 settings 应用配置（停旧 → 按新配置启动）。setup 与 mcp_set_config 共用入口。
pub fn apply_config(app: &tauri::AppHandle) -> Result<McpStatus, String> {
    stop();
    let (enabled, port) = read_mcp_settings(app);
    if enabled {
        start(app, port)?;
    }
    Ok(status(app))
}

/// 当前运行状态快照（enabled/port 取自 settings，running 取自运行时）
pub fn status(app: &tauri::AppHandle) -> McpStatus {
    let (enabled, port) = read_mcp_settings(app);
    let running_port = runtime()
        .lock()
        .ok()
        .and_then(|r| r.port.filter(|_| r.task.is_some()));
    McpStatus {
        enabled,
        running: running_port.is_some(),
        port,
        listening_url: running_port.map(|p| format!("http://127.0.0.1:{p}/mcp")),
    }
}
