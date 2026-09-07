use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::models::log_entry::{LogEntry, LogLevel};
use crate::models::report::Report;
use crate::services::github::IssueListItem;

/// issue_list 缓存按 state 的计数（open/closed 实存，all 为两者之和）
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct IssueCounts {
    pub open: u32,
    pub closed: u32,
    pub all: u32,
}

/// SQLite 缓存，持有连接（内部可变，跨线程共享）
pub struct Cache {
    conn: Mutex<Connection>,
}

impl Cache {
    /// 打开/创建数据库并执行初始化迁移
    pub fn open(db_path: &Path) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建数据目录失败: {e}"))?;
        }
        let conn = Connection::open(db_path)
            .map_err(|e| format!("打开数据库失败: {e}"))?;

        let sql = include_str!("../../migrations/001_init.sql");
        conn.execute_batch(sql)
            .map_err(|e| format!("初始化数据库失败: {e}"))?;

        let issue_list_sql = include_str!("../../migrations/002_issue_list.sql");
        conn.execute_batch(issue_list_sql)
            .map_err(|e| format!("初始化 issue_list 失败: {e}"))?;

        // 增量迁移：CREATE TABLE IF NOT EXISTS 对旧库不生效，补列需显式 ALTER。
        // 幂等设计：列已存在时报错可忽略，逐条独立执行互不影响。
        if let Err(e) = conn.execute_batch("ALTER TABLE reports ADD COLUMN app_name TEXT;") {
            log::debug!("跳过 app_name 迁移（列已存在）: {e}");
        }
        if let Err(e) = conn.execute_batch("ALTER TABLE reports ADD COLUMN screenshot_keys TEXT;") {
            log::debug!("跳过 screenshot_keys 迁移（列已存在）: {e}");
        }

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 写入一条上报记录及其日志（已存在则覆盖）
    pub fn save_report(&self, report: &Report, entries: &[LogEntry]) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;

        // 截图 key 列表以 JSON 文本落库（SQLite 无数组类型）
        let screenshot_keys_json = report
            .screenshot_keys
            .as_ref()
            .map(|keys| serde_json::to_string(keys))
            .transpose()
            .map_err(|e| format!("序列化截图 key 失败: {e}"))?;

        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;

        // upsert report
        tx.execute(
            "INSERT OR REPLACE INTO reports
                (report_id, issue_number, issue_title, app_name, app_version, platform, realm,
                 play_time, user_description, screenshot_keys, report_time, log_count, downloaded_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                report.report_id,
                report.issue_number,
                report.issue_title,
                report.app_name,
                report.app_version,
                report.platform,
                report.realm,
                report.play_time.map(|v| v as i64),
                report.user_description,
                screenshot_keys_json,
                report.report_time,
                report.log_count as i64,
                report.downloaded_at,
            ],
        )
        .map_err(|e| format!("写入 report 失败: {e}"))?;

        // 先删旧日志再插入（重复下载场景）
        tx.execute(
            "DELETE FROM log_entries WHERE report_id = ?",
            params![report.report_id],
        )
        .map_err(|e| format!("清理旧日志失败: {e}"))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO log_entries
                        (report_id, seq, timestamp, level, tag, message, data_json)
                     VALUES (?,?,?,?,?,?,?)",
                )
                .map_err(|e| format!("预编译失败: {e}"))?;

            for (seq, e) in entries.iter().enumerate() {
                let data_json = e.data.as_ref().map(|v| v.to_string());
                stmt.execute(params![
                    report.report_id,
                    seq as i64,
                    e.timestamp,
                    e.level.as_str(),
                    e.tag,
                    e.message,
                    data_json,
                ])
                .map_err(|e| format!("写入日志失败: {e}"))?;
            }
        }

        tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
        Ok(())
    }

    /// 读取某次上报的全部日志条目
    pub fn get_entries(&self, report_id: &str) -> Result<Vec<LogEntry>, String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT timestamp, level, tag, message, data_json
                 FROM log_entries
                 WHERE report_id = ?
                 ORDER BY seq ASC",
            )
            .map_err(|e| format!("查询预编译失败: {e}"))?;

        let rows = stmt
            .query_map(params![report_id], |row| {
                let timestamp: String = row.get(0)?;
                let level_str: String = row.get(1)?;
                let tag: String = row.get(2)?;
                let message: String = row.get(3)?;
                let data_json: Option<String> = row.get(4)?;
                let data = match data_json {
                    Some(s) => serde_json::from_str(&s).ok(),
                    None => None,
                };
                Ok((timestamp, level_str, tag, message, data))
            })
            .map_err(|e| format!("查询失败: {e}"))?;

        let mut entries = Vec::new();
        for r in rows {
            let (timestamp, level_str, tag, message, data) =
                r.map_err(|e| format!("读取行失败: {e}"))?;
            let level = LogLevel::parse(&level_str)
                .ok_or_else(|| format!("数据库中存在未知级别: {level_str}"))?;
            entries.push(LogEntry {
                timestamp,
                level,
                tag,
                message,
                data,
            });
        }
        Ok(entries)
    }

    /// 列出最近的若干条上报记录
    pub fn list_recent_reports(&self, limit: usize) -> Result<Vec<Report>, String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT report_id, issue_number, issue_title, app_name, app_version, platform, realm,
                        play_time, user_description, screenshot_keys, report_time, log_count, downloaded_at
                 FROM reports
                 ORDER BY downloaded_at DESC
                 LIMIT ?",
            )
            .map_err(|e| format!("查询预编译失败: {e}"))?;

        let rows = stmt
            .query_map(params![limit as i64], row_to_report)
            .map_err(|e| format!("查询失败: {e}"))?;

        let mut reports = Vec::new();
        for r in rows {
            reports.push(r.map_err(|e| format!("读取行失败: {e}"))?);
        }
        Ok(reports)
    }

    /// 获取单个 report 元信息
    pub fn get_report(&self, report_id: &str) -> Result<Option<Report>, String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT report_id, issue_number, issue_title, app_name, app_version, platform, realm,
                        play_time, user_description, screenshot_keys, report_time, log_count, downloaded_at
                 FROM reports
                 WHERE report_id = ?",
            )
            .map_err(|e| format!("查询预编译失败: {e}"))?;

        let mut rows = stmt
            .query_map(params![report_id], row_to_report)
            .map_err(|e| format!("查询失败: {e}"))?;

        match rows.next() {
            None => Ok(None),
            Some(r) => Ok(Some(r.map_err(|e| format!("读取行失败: {e}"))?)),
        }
    }

    /// 批量 upsert Issue 列表缓存（fetched_at 记为本次写入时刻）
    pub fn upsert_issues(&self, items: &[IssueListItem]) -> Result<(), String> {
        let fetched_at = chrono::Utc::now().to_rfc3339();
        let mut conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO issue_list
                        (issue_number, report_id, issue_title, state, labels, issue_url,
                         created_at, app_version, platform, realm, player_id, player_name, fetched_at)
                     VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
                )
                .map_err(|e| format!("预编译失败: {e}"))?;

            for item in items {
                // 标签列表以 JSON 文本落库（SQLite 无数组类型）
                let labels_json = item
                    .labels
                    .as_ref()
                    .map(|l| serde_json::to_string(l))
                    .transpose()
                    .map_err(|e| format!("序列化标签失败: {e}"))?;
                stmt.execute(params![
                    item.number as i64,
                    item.report_id,
                    item.title,
                    item.state,
                    labels_json,
                    item.issue_url,
                    item.created_at,
                    item.app_version,
                    item.platform,
                    item.realm,
                    item.player_id,
                    item.player_name,
                    fetched_at,
                ])
                .map_err(|e| format!("写入 issue_list 失败: {e}"))?;
            }
        }

        tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
        Ok(())
    }

    /// 按状态筛选分页读取 Issue 列表缓存（created_at 倒序，与远端排序一致）
    ///
    /// 返回 (列表项, 是否还有更多)。state ∈ open / closed / all。
    pub fn list_issues(
        &self,
        state: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<IssueListItem>, bool), String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let page = page.max(1);
        let per_page = per_page.max(1);
        let offset = ((page - 1) * per_page) as i64;

        // 多取一条判断 has_more，避免再发一次 COUNT 查询
        let mut stmt = conn
            .prepare(
                "SELECT issue_number, report_id, issue_title, state, labels, issue_url,
                        created_at, app_version, platform, realm, player_id, player_name
                 FROM issue_list
                 WHERE (?1 = 'all' OR state = ?1)
                 ORDER BY created_at DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| format!("查询预编译失败: {e}"))?;

        let rows = stmt
            .query_map(
                params![state, per_page as i64 + 1, offset],
                row_to_issue_list_item,
            )
            .map_err(|e| format!("查询失败: {e}"))?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r.map_err(|e| format!("读取行失败: {e}"))?);
        }
        let has_more = items.len() as u32 > per_page;
        items.truncate(per_page as usize);
        Ok((items, has_more))
    }

    /// 最近一次列表回源时间（全部 issue 中最大的 fetched_at）
    pub fn issues_cached_at(&self) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let cached_at: Option<String> = conn
            .query_row("SELECT MAX(fetched_at) FROM issue_list", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("查询失败: {e}"))?;
        Ok(cached_at)
    }

    /// 缓存中最新的 Issue 创建时间（增量同步水位线）。
    ///
    /// 远端按 created_at 倒序分页，增量同步翻页时一旦页尾时间早于该值，
    /// 即可判定后续页全是已同步过的旧数据而提前停止。空库返回 None（退化为全量）。
    pub fn latest_issue_created_at(&self) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let latest: Option<String> = conn
            .query_row("SELECT MAX(created_at) FROM issue_list", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("查询失败: {e}"))?;
        Ok(latest)
    }

    /// 各状态的 Issue 缓存计数（问题列表 tab 徽标用）。
    /// open/closed 为实际值，all 为两者之和（库内不存"all"状态）。
    pub fn count_issues_by_state(&self) -> Result<IssueCounts, String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT state, COUNT(*) FROM issue_list GROUP BY state")
            .map_err(|e| format!("查询预编译失败: {e}"))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(|e| format!("查询失败: {e}"))?;

        let mut counts = IssueCounts::default();
        for r in rows {
            let (state, n) = r.map_err(|e| format!("读取行失败: {e}"))?;
            match state.as_str() {
                "open" => counts.open = n as u32,
                "closed" => counts.closed = n as u32,
                _ => {}
            }
        }
        counts.all = counts.open + counts.closed;
        Ok(counts)
    }

    /// 批量判断 report_id 是否已有下载的上报日志（MCP 远端列表的 downloaded 标记用）
    pub fn reports_exist(&self, report_ids: &[String]) -> Result<Vec<bool>, String> {
        let conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT 1 FROM reports WHERE report_id = ? LIMIT 1")
            .map_err(|e| format!("查询预编译失败: {e}"))?;
        report_ids
            .iter()
            .map(|rid| {
                stmt.exists(params![rid])
                    .map_err(|e| format!("查询失败: {e}"))
            })
            .collect()
    }

    /// 全量同步收尾：删除本地库中已不在远端的 Issue（远端被删/误同步的脏数据）。
    ///
    /// 全量模式已拉到远端最后一页，本地多出来的必然是远端已删除的，
    /// 留着会让排行榜聚合出幽灵条目。按 issue_number 主键比对。
    pub fn purge_issues_not_in(
        &self,
        remote_numbers: &[u32],
    ) -> Result<(usize, usize), String> {
        if remote_numbers.is_empty() {
            // 远端为空却清空本地风险太大（可能是拉取异常），宁可保留
            return Ok((0, 0));
        }
        let mut conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;

        let local: Vec<i64> = {
            let mut stmt = tx
                .prepare("SELECT issue_number FROM issue_list")
                .map_err(|e| format!("查询预编译失败: {e}"))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, i64>(0))
                .map_err(|e| format!("查询失败: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("读取行失败: {e}"))?
        };

        let remote: std::collections::HashSet<i64>
            = remote_numbers.iter().map(|&n| n as i64).collect();
        let stale: Vec<i64> = local.into_iter().filter(|n| !remote.contains(n)).collect();

        for number in &stale {
            tx.execute("DELETE FROM issue_list WHERE issue_number = ?", params![number])
                .map_err(|e| format!("删除过期 Issue #{number} 失败: {e}"))?;
        }
        tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;

        Ok((stale.len(), remote_numbers.len()))
    }

    /// 局部刷新某条 Issue 缓存的 state/labels（操作 Issue 后同步，避免缓存读到旧状态）
    pub fn patch_issue_state(
        &self,
        issue_number: u32,
        state: Option<&str>,
        labels: Option<&[String]>,
    ) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;
        if let Some(new_state) = state {
            tx.execute(
                "UPDATE issue_list SET state = ? WHERE issue_number = ?",
                params![new_state, issue_number as i64],
            )
            .map_err(|e| format!("更新 issue 状态失败: {e}"))?;
        }
        if let Some(new_labels) = labels {
            let labels_json = serde_json::to_string(new_labels)
                .map_err(|e| format!("序列化标签失败: {e}"))?;
            tx.execute(
                "UPDATE issue_list SET labels = ? WHERE issue_number = ?",
                params![labels_json, issue_number as i64],
            )
            .map_err(|e| format!("更新 issue 标签失败: {e}"))?;
        }
        tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
        Ok(())
    }
}

/// rusqlite 行映射到 Report
fn row_to_report(row: &rusqlite::Row) -> rusqlite::Result<Report> {
    let play_time_i: Option<i64> = row.get(7)?;
    // JSON 文本 → key 数组；解析失败降级 None（仅展示增强，不让脏数据阻断读取）
    let screenshot_keys = row
        .get::<_, Option<String>>(9)?
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
    let log_count_i: i64 = row.get(11)?;
    Ok(Report {
        report_id: row.get(0)?,
        issue_number: row.get(1)?,
        issue_title: row.get(2)?,
        app_name: row.get(3)?,
        app_version: row.get(4)?,
        platform: row.get(5)?,
        realm: row.get(6)?,
        play_time: play_time_i.map(|v| v as u64),
        user_description: row.get(8)?,
        screenshot_keys,
        report_time: row.get(10)?,
        log_count: log_count_i as usize,
        downloaded_at: row.get(12)?,
    })
}

/// rusqlite 行映射到 IssueListItem（owner/repo 从 issue_url 反推；标签 JSON 解析失败降级 None）
fn row_to_issue_list_item(row: &rusqlite::Row) -> rusqlite::Result<IssueListItem> {
    let labels = row
        .get::<_, Option<String>>(4)?
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
    let issue_url: String = row.get(5)?;
    // owner/repo 列表展示用不到，从 URL 提取，取不到时留空
    // URL 形如 https://github.com/{owner}/{repo}/issues/{n}
    let segments: Vec<&str> = issue_url
        .split("github.com/")
        .nth(1)
        .map_or(Vec::new(), |rest| rest.split('/').collect());
    let owner = segments.first().copied().unwrap_or("").to_string();
    let repo = segments.get(1).copied().unwrap_or("").to_string();
    Ok(IssueListItem {
        number: row.get::<_, i64>(0)? as u32,
        report_id: row.get(1)?,
        title: row.get(2).unwrap_or_default(),
        state: row.get(3)?,
        issue_url,
        created_at: row.get(6)?,
        owner,
        repo,
        app_version: row.get(7)?,
        platform: row.get(8)?,
        realm: row.get(9)?,
        labels,
        player_id: row.get(10)?,
        player_name: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧版库（reports 表无 app_name 列）打开时自动迁移，旧数据可读
    #[test]
    fn open_migrates_legacy_db() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("legacy.db");

        // 构造旧 schema（本次扩展之前）
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch(
                "CREATE TABLE reports (
                    report_id TEXT PRIMARY KEY,
                    issue_number INTEGER,
                    issue_title TEXT,
                    app_version TEXT,
                    platform TEXT,
                    realm TEXT,
                    play_time INTEGER,
                    user_description TEXT,
                    report_time TEXT NOT NULL,
                    log_count INTEGER NOT NULL DEFAULT 0,
                    downloaded_at TEXT NOT NULL
                );
                INSERT INTO reports VALUES ('r1', 42, '旧问题', '1.0', 'electron', '筑基期',
                    NULL, NULL, '2026-06-08T14:00:00Z', 2, '2026-06-08T15:00:00Z');",
            )
            .unwrap();
        }

        // 打开 = 执行迁移
        let cache = Cache::open(&db).unwrap();
        let report = cache.get_report("r1").unwrap().unwrap();
        assert_eq!(report.issue_number, Some(42));
        assert_eq!(report.app_name, None); // 新列默认 NULL
        assert_eq!(report.app_version, Some("1.0".to_string()));
    }

    /// app_name 字段读写往返
    #[test]
    fn save_and_read_app_name() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("new.db");
        let cache = Cache::open(&db).unwrap();

        let report = Report {
            report_id: "local-x".to_string(),
            issue_number: None,
            issue_title: Some("starlight-2026-08-11.log".to_string()),
            app_name: Some("starlight".to_string()),
            app_version: None,
            platform: Some("HarmonyOS".to_string()),
            realm: None,
            play_time: None,
            user_description: None,
            screenshot_keys: None,
            report_time: "2026-08-22T10:00:00Z".to_string(),
            log_count: 1,
            downloaded_at: "2026-08-22T10:00:01Z".to_string(),
        };
        let entry = LogEntry {
            timestamp: "2026-08-11T10:30:20.364".to_string(),
            level: LogLevel::Fatal,
            tag: "Player".to_string(),
            message: "boom".to_string(),
            data: None,
        };
        cache.save_report(&report, &[entry]).unwrap();

        let loaded = cache.get_report("local-x").unwrap().unwrap();
        assert_eq!(loaded.app_name.as_deref(), Some("starlight"));
        let entries = cache.get_entries("local-x").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Fatal); // FATAL 级别往返不丢
    }

    /// screenshot_keys 字段读写往返（JSON 文本落库）
    #[test]
    fn save_and_read_screenshot_keys() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("shots.db");
        let cache = Cache::open(&db).unwrap();

        let report = Report {
            report_id: "shot-x".to_string(),
            issue_number: Some(19),
            issue_title: None,
            app_name: None,
            app_version: Some("0.11.9".to_string()),
            platform: Some("electron".to_string()),
            realm: None,
            play_time: None,
            user_description: Some("战斗界面背景颜色异常".to_string()),
            screenshot_keys: Some(vec![
                "screenshots/550e8400-e29b-41d4-a716-446655440000.png".to_string(),
                "screenshots/550e8400-e29b-41d4-a716-446655440000_1.png".to_string(),
            ]),
            report_time: "2026-09-04T11:59:21Z".to_string(),
            log_count: 1,
            downloaded_at: "2026-09-04T12:00:44Z".to_string(),
        };
        cache.save_report(&report, &[]).unwrap();

        let loaded = cache.get_report("shot-x").unwrap().unwrap();
        assert_eq!(
            loaded.screenshot_keys,
            Some(vec![
                "screenshots/550e8400-e29b-41d4-a716-446655440000.png".to_string(),
                "screenshots/550e8400-e29b-41d4-a716-446655440000_1.png".to_string(),
            ])
        );
    }

    /// 旧版库（reports 表无 screenshot_keys 列）打开时自动迁移，无截图记录读出 None
    #[test]
    fn open_migrates_screenshot_keys_column() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("legacy2.db");

        // 构造上一版 schema（有 app_name、无 screenshot_keys）
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch(
                "CREATE TABLE reports (
                    report_id TEXT PRIMARY KEY,
                    issue_number INTEGER,
                    issue_title TEXT,
                    app_name TEXT,
                    app_version TEXT,
                    platform TEXT,
                    realm TEXT,
                    play_time INTEGER,
                    user_description TEXT,
                    report_time TEXT NOT NULL,
                    log_count INTEGER NOT NULL DEFAULT 0,
                    downloaded_at TEXT NOT NULL
                );
                INSERT INTO reports VALUES ('r2', NULL, NULL, NULL, NULL, NULL, NULL,
                    NULL, '旧反馈', '2026-06-08T14:00:00Z', 1, '2026-06-08T15:00:00Z');",
            )
            .unwrap();
        }

        let cache = Cache::open(&db).unwrap();
        let report = cache.get_report("r2").unwrap().unwrap();
        assert_eq!(report.user_description.as_deref(), Some("旧反馈"));
        assert_eq!(report.screenshot_keys, None); // 新列默认 NULL → None
    }

    /// issue_list upsert + 分页 + state 筛选
    #[test]
    fn issue_list_upsert_and_query() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("issues.db");
        let cache = Cache::open(&db).unwrap();

        // 空库：无缓存
        assert!(cache.issues_cached_at().unwrap().is_none());
        let (items, has_more) = cache.list_issues("open", 1, 30).unwrap();
        assert!(items.is_empty() && !has_more);

        let item = |n: u32, state: &str, created: &str, labels: Option<Vec<String>>| IssueListItem {
            number: n,
            report_id: format!("rid-{n}"),
            title: format!("问题{n}"),
            state: state.to_string(),
            issue_url: format!("https://github.com/linxunxr/PathofIdleImmortals-bugs/issues/{n}"),
            created_at: created.to_string(),
            owner: "linxunxr".to_string(),
            repo: "PathofIdleImmortals-bugs".to_string(),
            app_version: Some("0.12.2".to_string()),
            platform: Some("electron".to_string()),
            realm: Some("凡人期".to_string()),
            labels,
            player_id: None,
            player_name: None,
        };

        // 乱序写入，验证 created_at 倒序读出
        cache
            .upsert_issues(&[
                item(1, "open", "2026-09-01T10:00:00Z", Some(vec!["已修复".into()])),
                item(3, "closed", "2026-09-03T10:00:00Z", None),
                item(2, "open", "2026-09-02T10:00:00Z", None),
            ])
            .unwrap();
        assert!(cache.issues_cached_at().unwrap().is_some());

        let (all, _) = cache.list_issues("all", 1, 30).unwrap();
        let numbers: Vec<u32> = all.iter().map(|i| i.number).collect();
        assert_eq!(numbers, vec![3, 2, 1]);

        // open 筛选
        let (open, _) = cache.list_issues("open", 1, 30).unwrap();
        let numbers: Vec<u32> = open.iter().map(|i| i.number).collect();
        assert_eq!(numbers, vec![2, 1]);
        assert_eq!(open[1].labels.as_deref(), Some(&["已修复".to_string()][..]));
        // owner/repo 从 issue_url 反推
        assert_eq!(open[0].owner, "linxunxr");
        assert_eq!(open[0].repo, "PathofIdleImmortals-bugs");

        // 分页：per_page=2 第一页 2 条 + has_more
        let (page1, has_more) = cache.list_issues("all", 1, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert!(has_more);
        let (page2, has_more) = cache.list_issues("all", 2, 2).unwrap();
        assert_eq!(page2.len(), 1);
        assert!(!has_more);

        // upsert 覆盖：#1 关闭后标签被替换，其余项不变
        cache
            .upsert_issues(&[item(1, "closed", "2026-09-01T10:00:00Z", None)])
            .unwrap();
        let (all, _) = cache.list_issues("all", 1, 30).unwrap();
        assert_eq!(all.len(), 3); // 仍是 3 条，不会重复插入
        let one = all.iter().find(|i| i.number == 1).unwrap();
        assert_eq!(one.state, "closed");
        assert_eq!(one.labels, None);
    }

    /// patch_issue_state 局部刷新 state/labels，不影响其它行
    #[test]
    fn patch_issue_state_updates_row() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("patch.db");
        let cache = Cache::open(&db).unwrap();

        let make = |n: u32| IssueListItem {
            number: n,
            report_id: format!("rid-{n}"),
            title: format!("问题{n}"),
            state: "open".to_string(),
            issue_url: format!("https://github.com/o/r/issues/{n}"),
            created_at: format!("2026-09-0{n}T10:00:00Z"),
            owner: "o".to_string(),
            repo: "r".to_string(),
            app_version: None,
            platform: None,
            realm: None,
            labels: Some(vec!["待验证".into()]),
            player_id: None,
            player_name: None,
        };
        cache.upsert_issues(&[make(1), make(2)]).unwrap();

        // 关闭 #1 并换标签
        cache
            .patch_issue_state(1, Some("closed"), Some(&["高优先级".to_string()]))
            .unwrap();
        let (all, _) = cache.list_issues("all", 1, 30).unwrap();
        let one = all.iter().find(|i| i.number == 1).unwrap();
        let two = all.iter().find(|i| i.number == 2).unwrap();
        assert_eq!(one.state, "closed");
        assert_eq!(one.labels.as_deref(), Some(&["高优先级".to_string()][..]));
        assert_eq!(two.state, "open"); // 其余行不受影响
        assert_eq!(two.labels.as_deref(), Some(&["待验证".to_string()][..]));

        // 只改 state 时 labels 不动
        cache.patch_issue_state(2, Some("closed"), None).unwrap();
        let (all, _) = cache.list_issues("all", 1, 30).unwrap();
        let two = all.iter().find(|i| i.number == 2).unwrap();
        assert_eq!(two.state, "closed");
        assert_eq!(two.labels.as_deref(), Some(&["待验证".to_string()][..]));
    }

    /// 增量水位线：latest_issue_created_at 返回库内最新 created_at，空库为 None
    #[test]
    fn latest_issue_created_at_watermark() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(&dir.path().join("mark.db")).unwrap();

        assert!(cache.latest_issue_created_at().unwrap().is_none());

        let item = |n: u32, created: &str| IssueListItem {
            number: n,
            report_id: format!("rid-{n}"),
            title: format!("问题{n}"),
            state: "open".to_string(),
            issue_url: format!("https://github.com/o/r/issues/{n}"),
            created_at: created.to_string(),
            owner: "o".to_string(),
            repo: "r".to_string(),
            app_version: None,
            platform: None,
            realm: None,
            labels: None,
            player_id: None,
            player_name: None,
        };
        cache
            .upsert_issues(&[
                item(1, "2026-09-01T10:00:00Z"),
                item(2, "2026-09-05T10:00:00Z"),
                item(3, "2026-09-03T10:00:00Z"),
            ])
            .unwrap();
        assert_eq!(
            cache.latest_issue_created_at().unwrap().as_deref(),
            Some("2026-09-05T10:00:00Z")
        );
    }

    /// 各状态计数：open/closed 实存值，all 为两者之和；未知状态忽略
    #[test]
    fn count_issues_by_state_sums() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(&dir.path().join("counts.db")).unwrap();

        let item = |n: u32, state: &str| IssueListItem {
            number: n,
            report_id: format!("rid-{n}"),
            title: format!("问题{n}"),
            state: state.to_string(),
            issue_url: format!("https://github.com/o/r/issues/{n}"),
            created_at: format!("2026-09-0{n}T10:00:00Z"),
            owner: "o".to_string(),
            repo: "r".to_string(),
            app_version: None,
            platform: None,
            realm: None,
            labels: None,
            player_id: None,
            player_name: None,
        };

        // 空库全 0
        assert_eq!(cache.count_issues_by_state().unwrap().all, 0);

        cache
            .upsert_issues(&[
                item(1, "open"),
                item(2, "open"),
                item(3, "closed"),
            ])
            .unwrap();
        let c = cache.count_issues_by_state().unwrap();
        assert_eq!((c.open, c.closed, c.all), (2, 1, 3));

        // upsert 改状态后计数跟随（#1 关闭）
        cache.upsert_issues(&[item(1, "closed")]).unwrap();
        let c = cache.count_issues_by_state().unwrap();
        assert_eq!((c.open, c.closed, c.all), (1, 2, 3));
    }

    /// reports_exist 批量判断下载状态：空列表/混合存在与不存在均正确
    #[test]
    fn reports_exist_checks_each_id() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(&dir.path().join("exists.db")).unwrap();

        let report = Report {
            report_id: "rid-1".to_string(),
            issue_number: None,
            issue_title: None,
            app_name: None,
            app_version: None,
            platform: None,
            realm: None,
            play_time: None,
            user_description: None,
            screenshot_keys: None,
            report_time: "2026-09-07T10:00:00Z".to_string(),
            log_count: 0,
            downloaded_at: "2026-09-07T10:00:01Z".to_string(),
        };
        cache.save_report(&report, &[]).unwrap();

        assert_eq!(cache.reports_exist(&[]).unwrap(), Vec::<bool>::new());
        assert_eq!(
            cache.reports_exist(&["rid-1".into(), "rid-2".into()]).unwrap(),
            vec![true, false]
        );
    }

    /// 全量清理：远端不存在的本地条目被删；远端列表为空时不动本地（防误清）
    #[test]
    fn purge_issues_not_in_removes_stale() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(&dir.path().join("purge.db")).unwrap();

        let item = |n: u32, created: &str| IssueListItem {
            number: n,
            report_id: format!("rid-{n}"),
            title: format!("问题{n}"),
            state: "open".to_string(),
            issue_url: format!("https://github.com/o/r/issues/{n}"),
            created_at: created.to_string(),
            owner: "o".to_string(),
            repo: "r".to_string(),
            app_version: None,
            platform: None,
            realm: None,
            labels: None,
            player_id: None,
            player_name: None,
        };
        cache
            .upsert_issues(&[
                item(1, "2026-09-01T10:00:00Z"),
                item(2, "2026-09-02T10:00:00Z"),
                item(3, "2026-09-03T10:00:00Z"),
            ])
            .unwrap();

        // 远端只剩 #2、#3：#1 被清理
        let (removed, total) = cache.purge_issues_not_in(&[2, 3]).unwrap();
        assert_eq!((removed, total), (1, 2));
        let (all, _) = cache.list_issues("all", 1, 30).unwrap();
        let numbers: Vec<u32> = all.iter().map(|i| i.number).collect();
        assert_eq!(numbers, vec![3, 2]);

        // 远端列表为空：不动本地（拉取异常时宁可保留）
        let (removed, _) = cache.purge_issues_not_in(&[]).unwrap();
        assert_eq!(removed, 0);
        let (all, _) = cache.list_issues("all", 1, 30).unwrap();
        assert_eq!(all.len(), 2);
    }
}
