-- 灵鉴 SQLite 初始化
CREATE TABLE IF NOT EXISTS reports (
    id TEXT PRIMARY KEY,
    issue_url TEXT,
    downloaded_at TEXT,
    log_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS log_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,
    module TEXT,
    message TEXT,
    FOREIGN KEY (report_id) REFERENCES reports(id)
);

CREATE INDEX IF NOT EXISTS idx_log_entries_report ON log_entries(report_id);
CREATE INDEX IF NOT EXISTS idx_log_entries_level ON log_entries(level);
