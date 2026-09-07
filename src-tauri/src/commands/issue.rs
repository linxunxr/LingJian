use serde::Serialize;
use tauri::State;

use crate::services::{cache::Cache, downloader, github};
use std::sync::Arc;

/// URL/编号解析结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub owner: String,
    pub repo: String,
    pub number: u32,
}

/// 解析用户输入为 Issue 定位信息（纯逻辑，不调网络）
#[tauri::command]
pub fn parse_issue_url(url: String) -> Result<ParseResult, String> {
    let parsed = github::parse_issue_input(&url)?;
    Ok(ParseResult {
        owner: parsed.owner,
        repo: parsed.repo,
        number: parsed.number,
    })
}

/// 通过 SCF 端点获取 Issue 信息（服务端代理解析 reportId，无需 GitHub Token）
#[tauri::command]
pub async fn fetch_issue_info(
    number: u32,
    scf_url: String,
    api_key: String,
    http: State<'_, crate::AppState>,
) -> Result<crate::services::github::IssueInfo, String> {
    if scf_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("未配置 SCF 端点，请先到设置页填写".to_string());
    }
    downloader::resolve_issue(&scf_url, number, &api_key, &http.client).await
}

/// 单次回源最多同步的远端页数（与前端排行榜 MAX_PAGES 一致，30 条/页 × 50 页 = 1500 条），
/// 防御 SCF hasMore 异常导致的死循环
const MAX_SYNC_PAGES: u32 = 50;
/// 每页条数与 SCF 端点保持一致（github.js PER_PAGE=30）
const PER_PAGE: u32 = 30;

/// 同步模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncMode {
    /// 增量：以缓存内最新 created_at 为水位线，翻页到页尾时间越过水位线即停，
    /// 稳态下只拉 1 页。已有 Issue 的 state/labels 等变更由本页 upsert 覆盖。
    Incremental,
    /// 全量：拉到远端最后一页，并清理缓存中远端已删除的 Issue。
    Full,
}

impl SyncMode {
    fn parse(s: Option<&str>) -> Self {
        match s {
            Some("full") => SyncMode::Full,
            _ => SyncMode::Incremental,
        }
    }
}

/// 拉取一页远端列表，对 SCF 冷启动超时（腾讯云 3 秒限制，闲置后实例回收，
/// 首次请求必然超时并触发预热，重试即成功）做一次自动重试
async fn fetch_page_with_warmup(
    scf_url: &str,
    state: &str,
    page: u32,
    api_key: &str,
    http: &reqwest::Client,
) -> Result<github::IssueList, String> {
    match downloader::list_issues(scf_url, state, page, api_key, http).await {
        Ok(list) => Ok(list),
        Err(first) => {
            // 等待 SCF 预热完成再重试一次；再失败则返回原始错误
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            downloader::list_issues(scf_url, state, page, api_key, http)
                .await
                .map_err(|_| first)
        }
    }
}

/// 增量同步缓存的公开入口（MCP issue_stats 等跨模块调用；
/// commands 内部直接用 [`sync_issues_to_cache`]）
pub async fn sync_cache_incremental(
    scf_url: &str,
    api_key: &str,
    http: &reqwest::Client,
    cache: &Arc<Cache>,
) -> Result<(), String> {
    sync_issues_to_cache(scf_url, api_key, http, cache, SyncMode::Incremental).await
}

/// 拉取远端列表并落库。
///
/// state 固定拉 "all"：缓存是全量镜像，open/closed 筛选在查询侧做——
/// 若按请求的 state 分家同步（首页 open 同步、排行页 all 同步），
/// closed 的 Issue 可能从未落库，排行页聚合时"已解决反馈消失"。
///
/// 增量模式以缓存内最新 created_at 为水位线提前停页（稳态 1 页）；
/// 全量模式拉到最后一页并清理远端已删除的条目（上限 MAX_SYNC_PAGES 页）。
async fn sync_issues_to_cache(
    scf_url: &str,
    api_key: &str,
    http: &reqwest::Client,
    cache: &Arc<Cache>,
    mode: SyncMode,
) -> Result<(), String> {
    // 水位线读取走阻塞线程，避免卡住异步运行时
    let watermark = match mode {
        SyncMode::Incremental => {
            let c = cache.clone();
            tauri::async_runtime::spawn_blocking(move || c.latest_issue_created_at())
                .await
                .map_err(|e| format!("查询任务失败: {e}"))??
        }
        SyncMode::Full => None,
    };

    let mut page = 1u32;
    let mut remote_numbers: Vec<u32> = Vec::new();
    loop {
        let list = fetch_page_with_warmup(scf_url, "all", page, api_key, http).await?;
        let empty = list.issues.is_empty();
        let has_more = list.has_more;
        let items: Vec<github::IssueListItem> = list.issues;
        // 页尾（时间最早的一条）越过水位线即已同步过，后续页更旧，直接停
        if let (Some(mark), Some(oldest)) = (
            watermark.as_deref(),
            items.iter().map(|i| i.created_at.as_str()).min(),
        ) {
            if oldest <= mark {
                // 本页可能新旧混杂（新增数 < 一页），仍整页 upsert 后停页
                upsert_page(cache, &items).await?;
                remote_numbers.extend(items.iter().map(|i| i.number));
                return Ok(());
            }
        }
        remote_numbers.extend(items.iter().map(|i| i.number));
        upsert_page(cache, &items).await?;
        if !has_more || empty || page >= MAX_SYNC_PAGES {
            // 全量拉完后清掉本地库中远端已删除的 Issue（增量模式不清：没拉全，
            // 无法区分"远端已删"和"本次没翻到"）
            if mode == SyncMode::Full && !empty {
                let c = cache.clone();
                let nums = remote_numbers.clone();
                let (removed, _total) = tauri::async_runtime::spawn_blocking(move || {
                    c.purge_issues_not_in(&nums)
                })
                .await
                .map_err(|e| format!("清理任务失败: {e}"))??;
                if removed > 0 {
                    log::info!("全量同步清理了 {removed} 条远端已删除的 Issue 缓存");
                }
            }
            return Ok(());
        }
        page += 1;
    }
}

/// 一页远端数据落库（DB 调用走阻塞线程，避免卡住异步运行时）
async fn upsert_page(cache: &Arc<Cache>, items: &[github::IssueListItem]) -> Result<(), String> {
    let c2 = cache.clone();
    let items = items.to_vec();
    tauri::async_runtime::spawn_blocking(move || c2.upsert_issues(&items))
        .await
        .map_err(|e| format!("落库任务失败: {e}"))?
}

/// 通过 SCF 端点拉取上报问题列表（首页问题列表用）
///
/// 缓存优先：refresh=false 时先读本地 issue_list 缓存并立即返回，
/// 同时后台静默增量回源 SCF 刷新缓存（下次打开即最新）；refresh=true 同步回源，
/// 失败时降级返回缓存。翻页（page>1）直接读本地库，不打网络。
///
/// - `state`：状态筛选，可选 "open" / "closed" / "all"，默认 "open"
/// - `page`：页码，默认 1
/// - `refresh`：true 强制回源（手动刷新按钮/切 tab/排行榜）
/// - `sync_mode`：回源模式，"incremental"（默认，拉到增量水位线即停）/
///   "full"（拉到最后一页并清理远端已删除的条目，排行榜「全量同步」按钮用）
#[tauri::command]
pub async fn list_issues(
    state: Option<String>,
    page: Option<u32>,
    refresh: Option<bool>,
    sync_mode: Option<String>,
    scf_url: String,
    api_key: String,
    http: State<'_, crate::AppState>,
) -> Result<crate::services::github::IssueList, String> {
    if scf_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("未配置 SCF 端点，请先到设置页填写".to_string());
    }
    // 规范化参数，防止传入异常值
    let st = match state.as_deref().unwrap_or("open") {
        "all" => "all",
        "closed" => "closed",
        _ => "open",
    };
    let pg = page.unwrap_or(1).max(1);
    let force_refresh = refresh.unwrap_or(false);
    let mode = SyncMode::parse(sync_mode.as_deref());

    let cache: Arc<Cache> = http.cache.clone();
    // 读本地缓存分页（spawn_blocking 包裹 DB 调用）
    let read_cache = {
        let st = st.to_string();
        let cache = cache.clone();
        move |page: u32| {
            let cache = cache.clone();
            let st = st.clone();
            async move {
                tauri::async_runtime::spawn_blocking(move || {
                    let (items, has_more) = cache.list_issues(&st, page, PER_PAGE)?;
                    let cached_at = cache.issues_cached_at()?;
                    Ok::<_, String>((items, has_more, cached_at))
                })
                .await
                .map_err(|e| format!("查询任务失败: {e}"))?
            }
        }
    };

    // 翻页：只读本地库（后台回源已把各页拉全），不打网络
    if pg > 1 && !force_refresh {
        let (items, has_more, cached_at) = read_cache(pg).await?;
        return Ok(crate::services::github::IssueList {
            issues: items,
            page: pg,
            has_more,
            from_cache: Some(true),
            degraded_error: None,
            cached_at,
        });
    }

    // 强刷：同步回源 + 落库；失败降级返回缓存并携带错误说明。
    // 回源成功后直接回读本地库第 1 页——数据与远端一致（含全量清理效果），
    // 也省掉旧实现里对远端第 1 页的一次重复请求
    if force_refresh {
        if let Err(e) = sync_issues_to_cache(&scf_url, &api_key, &http.client, &cache, mode).await {
            // 回源失败降级读缓存；缓存也为空时才把错误抛给前端
            let (items, has_more, cached_at) = read_cache(1).await?;
            if items.is_empty() {
                return Err(e);
            }
            return Ok(crate::services::github::IssueList {
                issues: items,
                page: 1,
                has_more,
                // 强刷失败对调用方是事实错误（排行榜会拿残缺数据聚合），
                // 不能伪装成正常缓存命中；错误文本随响应带回由前端提示
                from_cache: Some(true),
                degraded_error: Some(e),
                cached_at,
            });
        }
        let (items, has_more, cached_at) = read_cache(1).await?;
        return Ok(crate::services::github::IssueList {
            issues: items,
            page: 1,
            has_more,
            from_cache: Some(false),
            degraded_error: None,
            cached_at,
        });
    }

    // 默认：读缓存立即返回；缓存为空（首次使用）则同步回源一次再读，
    // 否则排行榜这类全量消费者会聚合出空结果
    let (items, has_more, cached_at) = read_cache(1).await?;
    if items.is_empty() {
        sync_issues_to_cache(&scf_url, &api_key, &http.client, &cache, SyncMode::Incremental)
            .await?;
        let (items, has_more, cached_at) = read_cache(pg).await?;
        return Ok(crate::services::github::IssueList {
            issues: items,
            page: pg,
            has_more,
            from_cache: Some(true),
            degraded_error: None,
            cached_at,
        });
    }

    // 缓存已有：后台静默增量刷新（fire-and-forget，失败仅记日志）
    let bg_sc = scf_url.clone();
    let bg_key = api_key.clone();
    let bg_cache = cache.clone();
    let bg_http = http.client.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) =
            sync_issues_to_cache(&bg_sc, &bg_key, &bg_http, &bg_cache, SyncMode::Incremental).await
        {
            log::debug!("后台刷新问题列表失败（缓存保持不变）: {e}");
        }
    });

    Ok(crate::services::github::IssueList {
        issues: items,
        page: 1,
        has_more,
        from_cache: Some(true),
        degraded_error: None,
        cached_at,
    })
}

/// 通过 SCF 端点操作 Issue（关闭/重开/评论/标签）
///
/// - `action`：close / reopen / comment / setLabels
/// - `body`：评论内容（action=comment 时必填）
/// - `labels`：标签数组（action=setLabels 时，整体替换）
#[tauri::command]
pub async fn act_on_issue(
    number: u32,
    action: String,
    body: Option<String>,
    labels: Option<Vec<String>>,
    scf_url: String,
    api_key: String,
    http: State<'_, crate::AppState>,
) -> Result<crate::services::github::IssueActionResponse, String> {
    if scf_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("未配置 SCF 端点，请先到设置页填写".to_string());
    }
    let result = downloader::act_on_issue(
        &scf_url,
        number,
        &action,
        body.as_deref(),
        labels.as_deref(),
        &api_key,
        &http.client,
    )
    .await?;

    // state/labels 有变化时同步刷新本地缓存，避免下次从缓存读到旧状态
    let new_state = result.state.clone();
    let new_labels = result.labels.clone();
    if new_state.is_some() || new_labels.is_some() {
        let cache = http.cache.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(e) = cache.patch_issue_state(
                number,
                new_state.as_deref(),
                new_labels.as_deref(),
            ) {
                log::debug!("同步 Issue #{number} 状态到本地缓存失败: {e}");
            }
        })
        .await
        .map_err(|e| format!("缓存同步任务失败: {e}"))?;
    }

    Ok(result)
}

/// 各状态 Issue 缓存计数（问题列表 tab 徽标；open/closed 实存，all 为两者之和）
#[tauri::command]
pub async fn issue_counts(
    http: State<'_, crate::AppState>,
) -> Result<crate::services::cache::IssueCounts, String> {
    let cache: Arc<Cache> = http.cache.clone();
    tauri::async_runtime::spawn_blocking(move || cache.count_issues_by_state())
        .await
        .map_err(|e| format!("查询任务失败: {e}"))?
}

/// 下载缺失日志的结果（download_missing_reports 返回）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadMissingResult {
    /// 本次新下载落库的条数
    pub downloaded: usize,
    /// 本地已有跳过的条数
    pub skipped: usize,
    /// 下载失败的 issue 编号与原因
    pub failed: Vec<String>,
}

/// 下载缺失日志（软件本体的下载入口，MCP 侧不再承担下载职责）。
///
/// 先增量同步 issue_list 缓存镜像，再逐条下载本地没有日志的 Issue：
/// 先解析完整元信息（用户反馈/游玩时长仅 /issue/:number 端点返回），
/// 失败则降级用列表信息落库。默认只处理 open（未处理）——已处理的
/// 反馈通常无需再分析；需要全量补下载时传 all。
#[tauri::command]
pub async fn download_missing_reports(
    state: Option<String>,
    scf_url: String,
    api_key: String,
    http: State<'_, crate::AppState>,
) -> Result<DownloadMissingResult, String> {
    use crate::models::report::Report;

    if scf_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("未配置 SCF 端点，请先到设置页填写".to_string());
    }
    let st = match state.as_deref().unwrap_or("open") {
        "all" => "all",
        "closed" => "closed",
        _ => "open",
    };
    let cache: Arc<Cache> = http.cache.clone();
    let client = http.client.clone();
    let cache_dir = http.cache_dir.clone();

    // 先把缓存镜像同步到最新（增量，稳态一页）
    sync_issues_to_cache(&scf_url, &api_key, &client, &cache, SyncMode::Incremental).await?;

    // 分页遍历缓存镜像中目标状态的 Issue，下载本地缺失的日志
    let mut downloaded = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<String> = Vec::new();
    let mut page = 1u32;
    loop {
        let listed = {
            let c = cache.clone();
            let s = st.to_string();
            tauri::async_runtime::spawn_blocking(move || c.list_issues(&s, page, PER_PAGE))
                .await
                .map_err(|e| format!("查询任务失败: {e}"))?
        };
        let (items, has_more) = listed?;
        if items.is_empty() {
            break;
        }
        for item in items {
            let rid = item.report_id.clone();
            let exists: bool = {
                let c = cache.clone();
                let exists = tauri::async_runtime::spawn_blocking(move || {
                    c.get_report(&rid).map(|r| r.is_some())
                })
                .await
                .map_err(|e| format!("查询任务失败: {e}"))?;
                exists?
            };
            if exists {
                skipped += 1;
                continue;
            }

            let info = downloader::resolve_issue(&scf_url, item.number, &api_key, &client)
                .await
                .ok();
            let report_id = info
                .as_ref()
                .map(|i| i.report_id.clone())
                .unwrap_or_else(|| item.report_id.clone());

            match downloader::download(&scf_url, &report_id, &api_key, &client, &cache_dir).await {
                Ok((entries, _size)) => {
                    let now = chrono::Utc::now().to_rfc3339();
                    let report = Report {
                        report_id: report_id.clone(),
                        issue_number: Some(item.number as i32),
                        issue_title: Some(item.title.clone()),
                        app_name: None,
                        app_version: info
                            .as_ref()
                            .and_then(|i| i.app_version.clone())
                            .or_else(|| item.app_version.clone()),
                        platform: info
                            .as_ref()
                            .and_then(|i| i.platform.clone())
                            .or_else(|| item.platform.clone()),
                        realm: info
                            .as_ref()
                            .and_then(|i| i.realm.clone())
                            .or_else(|| item.realm.clone()),
                        // SCF 的 playTime 为字符串（如 "1234"），落库解析为秒数
                        play_time: info
                            .as_ref()
                            .and_then(|i| i.play_time.as_deref())
                            .and_then(|s| s.parse::<u64>().ok()),
                        user_description: info.as_ref().and_then(|i| i.user_description.clone()),
                        screenshot_keys: info.as_ref().and_then(|i| i.screenshot_keys.clone()),
                        report_time: now.clone(),
                        log_count: entries.len(),
                        downloaded_at: now,
                    };
                    let c = cache.clone();
                    let saved = tauri::async_runtime::spawn_blocking(move || {
                        c.save_report(&report, &entries)
                    })
                    .await
                    .map_err(|e| format!("落库任务失败: {e}"))?;
                    match saved {
                        Ok(()) => downloaded += 1,
                        Err(e) => failed.push(format!("#{}: 落库失败 {e}", item.number)),
                    }
                }
                Err(e) => failed.push(format!("#{}: 下载失败 {e}", item.number)),
            }
        }
        if !has_more || page >= MAX_SYNC_PAGES {
            break;
        }
        page += 1;
    }

    Ok(DownloadMissingResult {
        downloaded,
        skipped,
        failed,
    })
}

/// 判断输入是否为纯 reportId（供前端决定是否跳过 Issue 解析）
#[tauri::command]
pub fn is_report_id_input(input: String) -> bool {
    github::is_report_id(&input)
}
