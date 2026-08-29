mod commands;
mod mcp;
mod models;
mod services;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use services::cache::Cache;
use services::paths;

use commands::{analyze, download, export_, import, issue, reports, secret, settings, storage};

/// 全局共享的应用状态
pub struct AppState {
    /// 复用的 HTTP 客户端（连接池）
    pub client: reqwest::Client,
    /// SQLite 缓存
    pub cache: Arc<Cache>,
    /// 日志 .gz 缓存目录
    pub cache_dir: PathBuf,
    /// 当前生效的数据目录（db + cache 所在）
    pub data_dir: PathBuf,
    /// 系统默认目录（标记文件存放处，用于目录切换）
    pub fallback_dir: PathBuf,
}

/// 装配全局状态；任一步失败返回带路径上下文的错误信息
fn init_state(app: &tauri::App) -> Result<AppState, String> {
    // 系统默认目录（C 盘），仅用于存放 data_dir.txt 标记文件
    let fallback_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录：{e}"))?;
    std::fs::create_dir_all(&fallback_dir)
        .map_err(|e| format!("无法创建应用目录（{}）：{e}", fallback_dir.display()))?;

    // 解析生效数据目录：优先 exe 同级 data/，无写权限则降级到 fallback
    let data_dir = paths::resolve_data_dir(&fallback_dir);
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("无法创建数据目录（{}）：{e}", data_dir.display()))?;

    let cache_dir = data_dir.join("cache");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("无法创建缓存目录（{}）：{e}", cache_dir.display()))?;

    let db_path = data_dir.join("lingjian.db");
    let cache = Cache::open(&db_path)
        .map_err(|e| format!("无法打开数据库（{}）：{e}", db_path.display()))?;

    let client = reqwest::Client::builder()
        .user_agent(format!("LingJian/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端：{e}"))?;

    Ok(AppState {
        client,
        cache: Arc::new(cache),
        cache_dir,
        data_dir,
        fallback_dir,
    })
}

/// 启动致命错误：弹错误提示框，用户关闭后退出应用。
/// 不能用 expect/panic——GUI 进程 panic 只会白屏，用户看不到原因；
/// 弹窗任务要等事件循环启动后才显示，因此 setup 仍返回 Ok 让主循环跑起来。
fn fatal(app: &tauri::AppHandle, msg: String) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
    let handle = app.clone();
    app.dialog()
        .message(msg)
        .title("灵鉴启动失败")
        .kind(MessageDialogKind::Error)
        .show(move |_| handle.exit(1));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            match init_state(app) {
                Ok(state) => {
                    app.manage(state);
                    // MCP server 启动失败不阻断应用，仅记录（设置页可查看状态重试）
                    if let Err(e) = mcp::apply_config(app.handle()) {
                        log::warn!("[mcp] 启动失败: {e}");
                    }
                }
                Err(msg) => fatal(app.handle(), msg),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            issue::parse_issue_url,
            issue::fetch_issue_info,
            issue::list_issues,
            issue::act_on_issue,
            issue::is_report_id_input,
            download::download_log,
            import::import_log_file,
            analyze::analyze_log,
            reports::list_recent_reports,
            reports::get_report,
            export_::export_report,
            secret::set_secret,
            secret::get_secret,
            secret::delete_secret,
            settings::test_scf_endpoint,
            commands::mcp::mcp_set_config,
            commands::mcp::mcp_status,
            storage::get_storage_info,
            storage::change_data_dir,
            storage::get_cache_size,
            storage::clear_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
