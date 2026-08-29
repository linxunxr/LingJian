# ZCode MCP 接入设计方案

灵鉴通过 MCP（Model Context Protocol）把日志分析能力开放给 ZCode 等 AI 编码代理使用，形成"用户上报 → 灵鉴分析 → AI 定位修复 → 回写 Issue"的闭环。本文档是一期（只读能力）的实施依据，二期（写操作）仅做规划。

## 更新记录

| 日期         | 变更内容                                 |
|--------------|------------------------------------------|
| 2026-08-29   | 初版：三方案对比、core 抽取、四只读工具契约、验收标准 |

## 1. 背景与方向选择

用户上报的日志经 SCF 落为 GitHub Issue，灵鉴下载后提供分析页（级别统计、错误聚合、时间线、日志过滤）。目前"分析结果 → 修复代码"之间是人工断点：人读分析页、复制错误信息、再去目标项目里找代码。

打通这个断点评估过三种方向：

| 方向               | 做法                                   | 结论                                        |
|--------------------|----------------------------------------|---------------------------------------------|
| 灵鉴推送（正向）   | 灵鉴按钮唤起 ZCode headless CLI        | 否决：桌面端 headless 引擎配置分裂未修通，链路长且单向 |
| ZCode 拉库（反向） | ZCode 直接读 lingjian.db 自己算        | 可行但劣质：裸数据无语义、SQL 紧耦合表结构、分析逻辑重复实现 |
| 能力开放（本方案） | 灵鉴做 MCP server，把分析能力封装成工具 | 采用：能力复用、契约解耦、可扩展到写操作      |

能力开放的判定依据：

- 灵鉴的 `services` 层本就是无 Tauri 耦合的领域库（`analyzer.rs:9` 纯函数、`cache.rs` 以路径为参、`paths.rs` fallback 参数化），加一层协议适配即可开放，分析逻辑一份代码两端共用。
- ZCode 原生支持 stdio MCP（用户级配置 `~/.zcode/cli/config.json` 的 `mcp.servers`），接入是配置级改动；本机已有 obsidian-kb（HTTP）、godot-mcp-pro（stdio）两个同类先例。
- 契约即边界：灵鉴内部重构不影响消费方；后续还能把"回写评论/标签/关单"也开出去。

## 2. 总体架构

```
┌─────────┐  stdio (JSON-RPC)   ┌──────────────────┐        ┌───────────────────┐
│  ZCode   │ ◄────────────────► │ lingjian-mcp.exe │ ────── │ lingjian-core     │
│ (任意会话) │    按需拉起/退出     │  (rmcp server)   │  复用  │  analyzer/cache/  │
└─────────┘                     └──────────────────┘        │  paths + models   │
                                                           └────────┬──────────┘
┌─────────┐  进程内直接调用                                      │ 只读连接
│ 灵鉴桌面端 │ ◄──────────────────────────────────────────────── ┘          ▼
│ (Tauri)  │                                              D:\…\灵鉴\data\lingjian.db
└─────────┘
```

分工：

- **lingjian-core**（新 crate）：领域逻辑——日志解析、分析聚合、SQLite 读写、数据目录寻址。桌面端与 MCP server 共同依赖，唯一事实来源。
- **lingjian-mcp**（新 crate，一期交付物）：stdio MCP server，薄封装 core 的只读能力，面向 LLM 控制响应体量。
- **灵鉴桌面端**：不变。MCP server 是独立进程，不要求灵鉴在运行。

## 3. Workspace 重构：抽取 lingjian-core

现状 `src-tauri` 是单 crate（lib 名 `lingjian_lib`，crate-type 含 `rlib`）。若让 MCP 直接依赖 `lingjian_lib`，会拖入整个 Tauri 依赖树（tauri-build、wry 等），编译重、产物大。因此先把领域逻辑抽为独立 crate。

目标结构（`src-tauri` 升为 workspace root，root package 即灵鉴本体）：

```
src-tauri/
├── Cargo.toml          # [workspace] members = ["core", "mcp"]；本体 package lingjian
├── core/               # 新：lingjian-core，无 tauri 依赖
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── models/     # log_entry.rs / report.rs / analyze.rs（自 src-tauri/src/models 迁入）
│       ├── analyzer.rs # 自 services/analyzer.rs 迁入
│       ├── cache.rs    # 自 services/cache.rs 迁入（include_str 路径同步调整）
│       ├── paths.rs    # 自 services/paths.rs 迁入
│       └── migrations/ # 001_init.sql 迁入
├── mcp/                # 新：lingjian-mcp 二进制
│   ├── Cargo.toml
│   └── src/main.rs
└── src/                # 灵鉴本体：commands/、剩余 services（github/download/exporter…）
```

迁移约定：

- 迁移文件清单：`models/log_entry.rs`、`models/report.rs`、`models/analyze.rs`、`services/analyzer.rs`、`services/cache.rs`、`services/paths.rs`、`migrations/001_init.sql`。`services/github.rs`、`download.rs`、`exporter.rs` 依赖 reqwest/SCF/keyring，留在本体，二期再评估。
- 本体引用改法：`lib.rs` 删掉对应 `pub mod`，改为 `pub use lingjian_core::{models, analyzer, cache, paths};` 式 re-export（具体模块布局实现时定），`commands/` 与前端 Tauri command 层的 `use crate::…` 路径不变或最小改动。
- core 依赖仅：`serde`、`serde_json`、`rusqlite`（bundled）、`chrono`、`log`、`regex`（log_entry 解析若用到）。禁止依赖 tauri。
- 单元测试随文件迁移（analyzer 4 例、paths 8 例等），迁移后 `cargo test --lib` 全绿为准；`src-tauri/Cargo.toml` 的 rusqlite/serde 等版本号同步下沉到 core，本体经 path 依赖传递。

Tauri v2 支持 package 即 workspace root 的布局，`npm run tauri dev/build` 行为不变，需在验收时回归确认。

## 4. lingjian-mcp 设计

### 4.1 依赖

| 依赖    | features                    | 用途                     |
|---------|-----------------------------|--------------------------|
| rmcp    | server, macros, transport-io | MCP 协议与 #[tool] 宏   |
| tokio   | macros, rt-multi-thread     | 异步运行时               |
| schemars| derive                      | 工具参数 JSON Schema     |
| lingjian-core | path 依赖              | 领域逻辑                 |

版本锁定原则：rmcp 与 schemars 大版本必须匹配（rmcp 的 `Parameters<T>` 要求 `T: JsonSchema`），实现时以 rmcp 当时文档标注的兼容版本为准，锁定后不再漂移。

### 4.2 数据目录寻址

优先级：环境变量 `LINGJIAN_DATA_DIR`（显式指定，测试用）→ 复用 `lingjian_core::paths::resolve_data_dir(fallback)`，fallback 取 `%APPDATA%\com.lingjian.app`。与桌面端完全同链（标记文件 `data_dir.txt` 优先），本机实测该链解析到 `D:\200software\219LingJian\灵鉴\data`。

### 4.3 SQLite 并发策略

- MCP 侧用只读连接：`Connection::open_with_flags(path, SQLITE_OPEN_READ_ONLY)` + `busy_timeout(2000)`，杜绝误写。
- 灵鉴现有建库未开 WAL（默认 journal 模式，`cache.rs:16` 无 pragma）。只读 MCP 与桌面端写并发时靠 busy_timeout 扛短事务，一期够用；二期建议桌面端建库时执行 `PRAGMA journal_mode=WAL`（一次设置持久生效，读写不再互斥），随写操作工具一起评审。

### 4.4 DTO 隔离

MCP 的请求/响应结构体在 mcp crate 内独立定义（derive `schemars::JsonSchema`），经 `From`/手写转换对接 core 类型。core 不引入 schemars，保持零侵入。

### 4.5 响应体量控制（面向 LLM 的关键约束）

单份上报日志可达数百条（Issue #15 为 200 条），未来更多。分析结果若全量返回会撑爆模型上下文。约定：

- 聚合类数据（error_aggregates、level_counts、tag_counts）默认全量——它们本来就是压缩后的。
- 明细类数据（entries、timeline）默认截断：entries 默认最多 50 条、timeline 默认最多 100 条，均带 `total` 计数与"还有多少未返回"提示，需要更多时用 `query_logs` 分页拉取。

### 4.6 骨架

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = LingjianServer::new()?;   // 解析数据目录、打开只读连接
    let service = rmcp::serve_server(server, rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[tool_router]
impl LingjianServer {
    #[tool(name = "list_issues",
           description = "列出灵鉴已下载的用户上报（含用户反馈描述）",
           annotations(read_only_hint = true))]
    async fn list_issues(
        &self,
        Parameters(p): Parameters<ListIssuesParams>,
    ) -> Result<Json<IssueListResult>, McpError> { /* … */ }
    // get_report / analyze_report / query_logs 同构
}

#[tool_handler]
impl ServerHandler for LingjianServer {}
```

## 5. 一期工具契约（4 个只读）

所有工具只读（`read_only_hint = true`）；参数与返回均为 camelCase JSON；错误以 MCP tool error 返回，消息为中文可读文本。

### 5.1 list_issues

| 项       | 内容                                                                 |
|----------|----------------------------------------------------------------------|
| 入参     | `limit?: number`（默认 20，上限 100）、`issueNumber?: number`（按 Issue 过滤） |
| 返回     | `issues: IssueBrief[]`                                               |
| 映射     | `core::cache::Cache::list_recent_reports` + `get_report`             |

`IssueBrief`：`reportId`、`issueNumber`、`issueTitle`、`appName`、`appVersion`、`platform`、`realm`、`playTime`、`userDescription`、`logCount`、`reportTime`、`downloadedAt`。`userDescription` 是用户原始反馈，全量返回不截断（用户话通常很短且信息密度最高）。

### 5.2 get_report

| 项   | 内容                                       |
|------|--------------------------------------------|
| 入参 | `reportId: string`                         |
| 返回 | `IssueBrief`（单个，字段同上）             |
| 映射 | `core::cache::Cache::get_report`           |

### 5.3 analyze_report

| 项       | 内容                                                                     |
|----------|--------------------------------------------------------------------------|
| 入参     | `reportId: string`、`filter?: { levels?: string[]; tags?: string[]; keyword?: string }`（透传 `LogFilter` 语义）、`entryLimit?: number`（默认 50，0 表示不要明细）、`timelineLimit?: number`（默认 100） |
| 返回     | `AnalysisResultDto`                                                      |
| 映射     | `core::cache::get_entries` → `core::analyzer::analyze`                   |

`AnalysisResultDto`：`total`、`levelCounts`、`tagCounts`、`errorAggregates`（全量）、`timeline`（截断后 + `timelineTotal`）、`entries`（截断后 + `entryTotal`）。结构与灵鉴分析页同源同值。

### 5.4 query_logs

| 项       | 内容                                                             |
|----------|------------------------------------------------------------------|
| 入参     | `reportId: string`、`filter?`（同上）、`offset?: number`（默认 0）、`limit?: number`（默认 50，上限 200） |
| 返回     | `entries: LogEntryDto[]`、`matchedTotal`、`offset`、`limit`      |
| 映射     | `core::cache::get_entries` + 内存过滤（复用 `LogFilter::matches`） |

`LogEntryDto`：`timestamp`、`level`、`tag`、`message`、`data`。

## 6. 二期规划（动作类，另行评审后实施）

| 工具           | 能力                     | 依赖与风险                                       |
|----------------|--------------------------|--------------------------------------------------|
| sync_latest    | 从 SCF 拉最新上报并落库  | SCF 契约（Scf 仓库 `API契约.md`）、写入需 WAL    |
| add_comment    | 回写 Issue 评论           | GitHub token（现 keyring/settings 链路需去 Tauri 化） |
| update_labels  | 更新 Issue 标签           | 同上                                             |
| close_issue    | 关闭 Issue                | 同上                                             |

写操作涉及凭证管理与外部副作用，契约单独评审；届时一并处理 github.rs 的 core 化与 WAL 迁移。

## 7. ZCode 侧接入

`~/.zcode/cli/config.json` 的 `mcp.servers` 增加：

```json
"lingjian": {
  "type": "stdio",
  "command": "D:\\100work\\103Tools\\LingJian\\src-tauri\\target\\release\\lingjian-mcp.exe"
}
```

验证：ZCode 会话内 `/mcp` 应显示 lingjian 已连接并列出 4 个工具。日常用法示例（ZCode 会话自然语言即可）："用 list_issues 看最新上报，分析五维属性显示不全那个 Issue，然后在本项目里定位修复"。

可选配套：在目标修复项目（游戏源码）根 `AGENTS.md` 增加一节"灵鉴数据接入"，写明可用的 MCP 工具与工作流约定。发布层面（安装包携带 mcp exe、路径自动化）随二期处理。

## 8. 测试与验收标准

| 类别     | 内容                                                                                     |
|----------|------------------------------------------------------------------------------------------|
| 单元测试 | core 迁移后 `cargo test --lib` 全绿；DTO 转换与截断逻辑（entries/timeline limit）有测试 |
| 协议测试 | 用脚本向 `lingjian-mcp.exe` stdin 发 initialize / tools/list / tools/call JSON-RPC，断言响应 |
| 端到端   | ZCode 会话调用 `list_issues` → `analyze_report`（Issue #15），结果与灵鉴分析页一致：ERROR 2 条（"宗门商店刷新失败：未找到当前宗门ID"）、INFO 145、WARN 53 |
| 并发     | 灵鉴桌面端下载新上报的同时，MCP 工具调用不报 database locked（busy_timeout 生效）        |
| 回归     | `npm run tauri dev` / `npm run tauri build` 在 workspace 布局下正常；前端 vitest 全绿    |

## 9. 风险与待决事项

- **rmcp/schemars 版本匹配**：两者迭代都快，实现时锁定并记录在 core/mcp 的 Cargo.toml 注释中。
- **Tauri workspace 兼容**：root package + members 布局为官方支持姿势，但需按第 8 节回归 dev/build 双命令，防构建配置边缘问题。
- **stdio 中文编码**：Windows 下 JSON-RPC 走 UTF-8，rmcp 自行处理；验收时确认无 GBK 乱码即可。
- **待决：修复目标项目**。游戏源码目录尚未确认，不影响本方案实施，只影响第 7 节 AGENTS.md 落点与联调环境。

## 10. 实施步骤（提交粒度）

1. `refactor: 抽取 lingjian-core crate——models/analyzer/cache/paths 迁入 workspace 子 crate，桌面端改 path 依赖，测试随迁全绿`
2. `feat(mcp): lingjian-mcp 骨架与 list_issues——rmcp stdio server，只读连接与数据目录寻址复用 core`
3. `feat(mcp): 补齐 get_report/analyze_report/query_logs——明细默认截断，契约对齐设计方案`
4. `docs: ZCode 侧接入配置与验收记录——config.json 片段、端到端验证结果回填本文档`
