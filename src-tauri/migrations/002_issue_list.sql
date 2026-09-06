-- Issue 列表缓存：远端 Issue 元数据镜像（与 reports 解耦，未下载日志的 Issue 也在列表内）
CREATE TABLE IF NOT EXISTS issue_list (
    issue_number INTEGER PRIMARY KEY,
    report_id TEXT NOT NULL,
    issue_title TEXT,
    state TEXT NOT NULL DEFAULT 'open',
    labels TEXT,
    issue_url TEXT,
    created_at TEXT NOT NULL,
    app_version TEXT,
    platform TEXT,
    realm TEXT,
    player_id TEXT,
    player_name TEXT,
    fetched_at TEXT NOT NULL
);

-- 列表按创建时间倒序展示
CREATE INDEX IF NOT EXISTS idx_issue_list_created_at
    ON issue_list(state, created_at DESC);
