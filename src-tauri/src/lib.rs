mod commands;
mod models;
mod services;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use services::cache::Cache;

use commands::{analyze, download, issue, reports};

/// 全局共享的应用状态
pub struct AppState {
    /// 复用的 HTTP 客户端（连接池）
    pub client: reqwest::Client,
    /// SQLite 缓存
    pub cache: Arc<Cache>,
    /// 日志 .gz 缓存目录
    pub cache_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 数据目录：<app_data_dir>/cache 存 gzip，<app_data_dir>/lingjian.db 存库
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取应用数据目录");
            std::fs::create_dir_all(&data_dir).expect("无法创建数据目录");

            let cache_dir = data_dir.join("cache");
            std::fs::create_dir_all(&cache_dir).expect("无法创建缓存目录");

            let db_path = data_dir.join("lingjian.db");
            let cache = Arc::new(Cache::open(&db_path).expect("无法打开数据库"));

            let client = reqwest::Client::builder()
                .user_agent("LingJian/0.1")
                .build()
                .expect("无法创建 HTTP 客户端");

            app.manage(AppState {
                client,
                cache,
                cache_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            issue::parse_issue_url,
            issue::fetch_issue_info,
            issue::is_report_id_input,
            download::download_log,
            analyze::analyze_log,
            reports::list_recent_reports,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
