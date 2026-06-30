mod commands;
mod models;
mod services;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use services::cache::Cache;
use services::paths;

use commands::{analyze, download, export_, issue, reports, secret, settings, storage};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // 系统默认目录（C 盘），仅用于存放 data_dir.txt 标记文件
            let fallback_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取应用数据目录");
            std::fs::create_dir_all(&fallback_dir).expect("无法创建应用目录");

            // 解析生效数据目录：优先 exe 同级 data/，无写权限则降级到 fallback
            let data_dir = paths::resolve_data_dir(&fallback_dir);
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
                data_dir,
                fallback_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            issue::parse_issue_url,
            issue::fetch_issue_info,
            issue::list_issues,
            issue::is_report_id_input,
            download::download_log,
            analyze::analyze_log,
            reports::list_recent_reports,
            export_::export_report,
            secret::set_secret,
            secret::get_secret,
            secret::delete_secret,
            settings::test_scf_endpoint,
            storage::get_storage_info,
            storage::change_data_dir,
            storage::get_cache_size,
            storage::clear_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
