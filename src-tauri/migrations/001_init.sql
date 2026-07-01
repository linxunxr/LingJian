-- 灵鉴 SQLite 初始化

-- 上报记录
CREATE TABLE IF NOT EXISTS reports (
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

-- 日志条目
CREATE TABLE IF NOT EXISTS log_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,
    tag TEXT NOT NULL,
    message TEXT NOT NULL,
    data_json TEXT,
    FOREIGN KEY (report_id) REFERENCES reports(report_id)
);

-- 查询索引：按 report + level / report + tag / report + timestamp
CREATE INDEX IF NOT EXISTS idx_log_entries_report_level
    ON log_entries(report_id, level);
CREATE INDEX IF NOT EXISTS idx_log_entries_report_tag
    ON log_entries(report_id, tag);
CREATE INDEX IF NOT EXISTS idx_log_entries_report_timestamp
    ON log_entries(report_id, timestamp);
