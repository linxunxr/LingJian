use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};

use crate::models::log_entry::{LogEntry, LogLevel};
use crate::models::report::Report;

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

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 写入一条上报记录及其日志（已存在则覆盖）
    pub fn save_report(&self, report: &Report, entries: &[LogEntry]) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| format!("数据库锁失败: {e}"))?;

        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;

        // upsert report
        tx.execute(
            "INSERT OR REPLACE INTO reports
                (report_id, issue_number, issue_title, app_version, platform, realm,
                 play_time, user_description, report_time, log_count, downloaded_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            params![
                report.report_id,
                report.issue_number,
                report.issue_title,
                report.app_version,
                report.platform,
                report.realm,
                report.play_time.map(|v| v as i64),
                report.user_description,
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
                "SELECT report_id, issue_number, issue_title, app_version, platform, realm,
                        play_time, user_description, report_time, log_count, downloaded_at
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
                "SELECT report_id, issue_number, issue_title, app_version, platform, realm,
                        play_time, user_description, report_time, log_count, downloaded_at
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
}

/// rusqlite 行映射到 Report
fn row_to_report(row: &rusqlite::Row) -> rusqlite::Result<Report> {
    let play_time_i: Option<i64> = row.get(6)?;
    let log_count_i: i64 = row.get(9)?;
    Ok(Report {
        report_id: row.get(0)?,
        issue_number: row.get(1)?,
        issue_title: row.get(2)?,
        app_version: row.get(3)?,
        platform: row.get(4)?,
        realm: row.get(5)?,
        play_time: play_time_i.map(|v| v as u64),
        user_description: row.get(7)?,
        report_time: row.get(8)?,
        log_count: log_count_i as usize,
        downloaded_at: row.get(10)?,
    })
}
