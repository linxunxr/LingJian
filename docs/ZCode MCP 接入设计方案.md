# ZCode MCP 接入设计方案

灵鉴桌面端内嵌 MCP（Model Context Protocol）server，把日志分析能力开放给 ZCode 等 AI 编码代理使用，形成"用户上报 → 灵鉴分析 → AI 定位修复 → 回写 Issue"的闭环。桌面端设置页提供 MCP 配置界面（开关、端口、连接信息）。本文档是一期（只读能力）的实施依据，二期（写操作）仅做规划。

## 更新记录

| 日期       | 变更内容                                                                             |
|------------|--------------------------------------------------------------------------------------|
| 2026-08-30 | 写工具与界面手动操作对齐：close_issue 新增 fixed_in 参数（关闭+版本标签+解决评论三连，同 CloseIssueDialog 流程）；reopen_issue 开放为工具（撤销原"防误操作不开放"决策，写操作已有 mcpAllowWrite 守门，界面与 AI 能力对等更重要） |
| 2026-08-30 | 回填二期实施记录：四工具落地于 feat/mcp-phase2 分支（PR 流程），写操作守门采用 mcpAllowWrite 设置开关 |
| 2026-08-30 | 回填实施记录：一期四工具已在 feat/mcp-server 分支落地并全量验证通过，附与方案的差异说明 |
| 2026-08-29 | v2：形态改为桌面端托管——删独立进程与 core 抽取方案，新增设置页配置界面与 HTTP transport 设计 |
| 2026-08-29 | 补"MCP server 生命周期"小节（stdio 形态，已被 v2 取代）                               |
| 2026-08-29 | 初版：三方案对比、core 抽取、四只读工具契约、验收标准                                  |

## 实施记录（2026-08-30，feat/mcp-server 分支）

一期四个只读工具全部落地并验证通过：

- 协议链路（curl 直连 `http://127.0.0.1:3920/mcp`）：initialize（SSE + Mcp-Session-Id）→ tools/list → tools/call 全通，错误分支返回 JSON-RPC -32602 中文消息；
- 端到端：`analyze_report`（Issue #15）与灵鉴分析页基准一致——total 200、error 2 / info 145 / warn 53、宗门商店错误聚合 2 条；截断逻辑（timeline 55→3、entries 200→2）与总数字段正确；
- 回归：`cargo test --lib` 48 例、前端 vitest 51 例全绿；MCP 关闭时应用无行为差异；
- serverInfo 定制为 `lingjian`（宏默认 from_build_env 会显示 rmcp 自身标识）。

与方案的差异（实现时的实际情况）：

- rmcp 实际使用 **3.1.4**，其 API 与旧文档差异较大：`Parameters` 在 `handler::server::wrapper` 下、错误类型为 `ErrorData`（`internal_error`/`invalid_params` 构造）、session 管理器为 `session::local::LocalSessionManager`（非 InMemorySessionManager）、`ServerInfo` 为 non-exhaustive 需经 `#[tool_handler]` 宏属性定制名称与引导语；
- tokio 也成为显式依赖（`features = ["net"]`，TcpListener 绑定需要），方案第 3 节原表述"不新增 tokio"不成立，已直接依赖；
- 工具参数统一 camelCase（含 list_issues/get_report 的入参），与方案 5 节契约一致；
- ZCode 客户端实连验证待用户在 ZCode 配置加入 lingjian 后执行 `/mcp` 确认（协议层已由 curl 全量验证）。

## 1. 背景与方向选择

用户上报的日志经 SCF 落为 GitHub Issue，灵鉴下载后提供分析页（级别统计、错误聚合、时间线、日志过滤）。目前"分析结果 → 修复代码"之间是人工断点：人读分析页、复制错误信息、再去目标项目里找代码。

打通这个断点评估过三种方向：

| 方向               | 做法                                     | 结论                                              |
|--------------------|------------------------------------------|---------------------------------------------------|
| 灵鉴推送（正向）   | 灵鉴按钮唤起 ZCode headless CLI          | 否决：桌面端 headless 引擎配置分裂未修通，链路长且单向 |
| ZCode 拉库（反向） | ZCode 直接读 lingjian.db 自己算          | 可行但劣质：裸数据无语义、SQL 紧耦合表结构、分析逻辑重复实现 |
| 能力开放（本方案） | 灵鉴内嵌 MCP server，把分析能力封装成工具 | 采用：能力复用、契约解耦、配置集中在桌面端        |

能力开放的判定依据：

- 灵鉴的 `services` 层本就是无 Tauri 耦合的领域库（`analyzer.rs:9` 纯函数、`cache.rs` 以路径为参），MCP handler 在进程内直接调用，分析逻辑一份代码两端共用。
- ZCode 原生支持 HTTP MCP（用户级配置 `~/.zcode/cli/config.json` 的 `mcp.servers`，`type: http` + url），本机 obsidian-kb（`http://127.0.0.1:27124/mcp`）即此模式先例。
- 契约即边界：MCP 工具的参数/返回结构固定后，灵鉴内部重构不影响消费方；后续还能把"回写评论/标签/关单"也开出去。

### 托管形态 vs 独立进程形态

MCP server 有两种宿主形态，本方案选**桌面端托管**：

| 形态             | 做法                                | 本方案取舍                                        |
|------------------|-------------------------------------|---------------------------------------------------|
| 桌面端托管（采用） | 灵鉴运行时在本机端口起 HTTP MCP server，设置页配置 | 配置集中有 UI、开关可控；二期写操作可直接用进程内 keyring 凭证；代价是使用时灵鉴须在运行 |
| 独立 stdio 进程  | 独立 exe 由 ZCode 按需拉起          | 不依赖灵鉴运行，但无配置界面、ZCode 侧手改配置、二期需凭证去 Tauri 化，v1 方案曾采用后被否 |

"使用时灵鉴须在运行"与 obsidian-kb（开着才能查 vault）一致，是预期行为。

## 2. 总体架构

```
┌───────────────────────────────────────────────────────┐
│ 灵鉴桌面端 (Tauri / lingjian_lib)                      │
│                                                       │
│  设置页 ── MCP 分区（开关/端口/连接信息）                │
│     │ tauri command: mcp_set_config / mcp_status      │
│     ▼                                                 │
│  ┌─────────────────────────────┐    ┌──────────────┐ │      ┌─────────┐
│  │ mcp 模块 (rmcp + axum)       │───►│ services 层   │ │      │  ZCode   │
│  │ 4+N 个工具 handler            │    │ analyzer/cache│ │ ◄──► │ (AI 代理) │
│  │ http://127.0.0.1:<port>/mcp  │    │ github(二期)  │ │ HTTP │ 任意会话  │
│  └─────────────────────────────┘    └──────┬───────┘ │      └─────────┘
│                                      Arc<Cache> 共享  │
└───────────────────────────────────────│───────────────┘
                                        ▼
                              D:\…\灵鉴\data\lingjian.db
```

分工：

- **mcp 模块**（`src-tauri/src/mcp/`，一期交付物）：rmcp Streamable HTTP server + 工具 handler，薄封装 services 层的只读能力，面向 LLM 控制响应体量。
- **设置页 MCP 分区**（前端）：开关、端口配置、运行状态指示、连接 URL 与 ZCode 配置片段一键复制。
- **services 层**：不动。MCP handler 进程内直接调用，与 Tauri command 层平级。

### MCP server 生命周期

server 随灵鉴应用进程生存：

1. **启动**：应用 setup 阶段读取 settings，若 `mcpEnabled` 为 true 则 tokio::spawn 拉起 axum listener（绑定 127.0.0.1，端口默认 3920 可配）；
2. **通信**：ZCode 按 `http://127.0.0.1:<port>/mcp` 发起 MCP Streamable HTTP 请求（JSON-RPC），handler 经 `Arc<Cache>`/analyzer 执行；
3. **配置变更**：设置页修改开关/端口后，tauri command 停旧 listener、按新配置重启，即时生效无需重启应用；
4. **退出**：随灵鉴进程退出自动结束，无残留端口占用。

## 3. 依赖与配置

新增依赖（需评审通过后引入）：

| 依赖 | features                                       | 用途                     |
|------|------------------------------------------------|--------------------------|
| rmcp | server, macros, transport-streamable-http-server | MCP 协议与 #[tool] 宏、HTTP 传输 |
| axum | default                                        | 本机 HTTP listener        |

（tokio 已随 reqwest/tauri 在依赖树中，不新增。）版本锁定原则：rmcp 与 schemars 大版本必须匹配（`Parameters<T>` 要求 `T: JsonSchema`），实现时以 rmcp 当时文档为准并锁定。

配置存储（复用 tauri-plugin-store 的 `settings.json`，`useSettings.ts` 扩展）：

| 键          | 类型    | 默认值 | 说明                         |
|-------------|---------|--------|------------------------------|
| mcpEnabled  | boolean | false  | MCP server 总开关            |
| mcpPort     | number  | 3920   | 监听端口，仅绑定 127.0.0.1   |

新增 tauri commands：`mcp_set_config(enabled, port)`（写 settings + 重启 listener，返回运行状态）、`mcp_status()`（返回 running/port/listeningUrl）。

### 并发与安全

- **SQLite 并发**：MCP handler 与下载写库共用同一 `Arc<Cache>`（内部 `Mutex<Connection>`），进程内天然互斥，无跨进程锁问题。
- **网络边界**：仅绑定 127.0.0.1，不对外网暴露；一期无鉴权（与 obsidian-kb 同水位），token 鉴权随二期写操作一并评审。

## 4. mcp 模块设计

### 4.1 布局

```
src-tauri/src/mcp/
├── mod.rs        # server 启停管理：start/stop/restart，axum router 装配
├── handler.rs    # LingjianServer：#[tool_router] 工具集 + ServerHandler impl
└── dto.rs        # MCP 请求/响应结构体（derive schemars::JsonSchema）
```

### 4.2 DTO 隔离

MCP 的请求/响应结构体在 mcp 模块内独立定义（derive `schemars::JsonSchema`），经 `From`/手写转换对接 models 层类型。models 不引入 schemars，保持零侵入。

### 4.3 响应体量控制（面向 LLM 的关键约束）

单份上报日志可达数百条（Issue #15 为 200 条），未来更多。分析结果若全量返回会撑爆模型上下文。约定：

- 聚合类数据（errorAggregates、levelCounts、tagCounts）默认全量——它们本来就是压缩后的。
- 明细类数据（entries、timeline）默认截断：entries 默认最多 50 条、timeline 默认最多 100 条，均带 `total` 计数与"还有多少未返回"提示，需要更多时用 `query_logs` 分页拉取。

### 4.4 骨架

```rust
// mod.rs：装配与启停（伪码）
pub fn start(app: AppHandle, port: u16) {
    tauri::async_runtime::spawn(async move {
        let cache = /* 从状态取出 Arc<Cache> */;
        let service = StreamableHttpService::new(
            move || Ok(LingjianServer { cache: cache.clone() }),
            session_manager, StreamableHttpServerConfig::default(),
        );
        let app_router = axum::Router::new().route("/mcp", service);
        axum::serve(TcpListener::bind(("127.0.0.1", port)).await?, app_router).await
    });
}

// handler.rs：工具定义
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
| 映射     | `services::cache::Cache::list_recent_reports` + `get_report`          |

`IssueBrief`：`reportId`、`issueNumber`、`issueTitle`、`appName`、`appVersion`、`platform`、`realm`、`playTime`、`userDescription`、`logCount`、`reportTime`、`downloadedAt`。`userDescription` 是用户原始反馈，全量返回不截断（用户话通常很短且信息密度最高）。

### 5.2 get_report

| 项   | 内容                                       |
|------|--------------------------------------------|
| 入参 | `reportId: string`                         |
| 返回 | `IssueBrief`（单个，字段同上）             |
| 映射 | `services::cache::Cache::get_report`       |

### 5.3 analyze_report

| 项       | 内容                                                                     |
|----------|--------------------------------------------------------------------------|
| 入参     | `reportId: string`、`filter?: { levels?: string[]; tags?: string[]; keyword?: string }`（透传 `LogFilter` 语义）、`entryLimit?: number`（默认 50，0 表示不要明细）、`timelineLimit?: number`（默认 100） |
| 返回     | `AnalysisResultDto`                                                      |
| 映射     | `cache::get_entries` → `analyzer::analyze`                               |

`AnalysisResultDto`：`total`、`levelCounts`、`tagCounts`、`errorAggregates`（全量）、`timeline`（截断后 + `timelineTotal`）、`entries`（截断后 + `entryTotal`）。结构与灵鉴分析页同源同值。

### 5.4 query_logs

| 项       | 内容                                                             |
|----------|------------------------------------------------------------------|
| 入参     | `reportId: string`、`filter?`（同上）、`offset?: number`（默认 0）、`limit?: number`（默认 50，上限 200） |
| 返回     | `entries: LogEntryDto[]`、`matchedTotal`、`offset`、`limit`      |
| 映射     | `cache::get_entries` + 内存过滤（复用 `LogFilter::matches`）     |

`LogEntryDto`：`timestamp`、`level`、`tag`、`message`、`data`。

## 6. 二期：动作类工具（已于 2026-08-30 实施，feat/mcp-phase2 分支）

| 工具           | 能力                     | 实现要点                                       |
|----------------|--------------------------|------------------------------------------------|
| sync_latest    | 从 SCF 拉最新上报并落库  | 新条目先 resolve_issue 取完整元信息（用户反馈/游玩时长仅该端点返回）再下载落库；本地已有跳过；服务端日志包过期的旧上报 404 如实报告不阻断 |
| add_comment    | 回写 Issue 评论           | 复用 `downloader::act_on_issue`（SCF 代理，GitHub token 在服务端） |
| update_labels  | 更新 Issue 标签           | 同上；setLabels 为整体替换语义，工具描述已注明 |
| close_issue    | 关闭 Issue                | 同上；可选 `fixed_in` 版本号，提供时走界面关单同款三连（关闭 → 追加 v<版本号> 标签 → 解决评论），标签/评论失败进 followupNotes 不阻断 |
| reopen_issue   | 重新打开 Issue            | 同上；与界面「重新打开」一致 |

**写操作守门**：评审定为 settings 键 `mcpAllowWrite`（默认关）——设置页 MCP 分区提供开关，写工具在开关关闭时调用被拒并提示。相比 token 鉴权更轻（localhost 信任边界已足够，真实风险是 AI 误操作，显式开关即授权边界）；相比逐次弹窗确认更顺滑（MCP 是程序化调用，弹窗会阻塞）。

**遗留说明**：sync_latest 落库的 playTime 由 SCF 字符串解析为秒数（与 download_log 命令行为一致）。reopen 原二期未开放（防误操作），2026-08-30 撤销：mcpAllowWrite 已是显式授权边界，界面与 AI 能力对等优先。

## 7. ZCode 侧接入

设置页提供一键复制的配置片段（写入 `~/.zcode/cli/config.json` 的 `mcp.servers`）：

```json
"lingjian": {
  "type": "http",
  "url": "http://127.0.0.1:3920/mcp"
}
```

验证：灵鉴运行且 MCP 开关打开时，ZCode 会话内 `/mcp` 应显示 lingjian 已连接并列出 4 个工具。日常用法示例（ZCode 会话自然语言即可）："用 list_issues 看最新上报，分析五维属性显示不全那个 Issue，然后在本项目里定位修复"。

可选配套：在目标修复项目（游戏源码）根 `AGENTS.md` 增加一节"灵鉴数据接入"，写明可用的 MCP 工具与工作流约定。

## 8. 测试与验收标准

| 类别     | 内容                                                                                     |
|----------|------------------------------------------------------------------------------------------|
| 单元测试 | DTO 转换与截断逻辑（entries/timeline limit）有测试；settings 读写与 listener 重启逻辑有测试 |
| 协议测试 | 灵鉴运行 + 开关打开后，用 curl 或脚本向 `http://127.0.0.1:3920/mcp` 发 initialize / tools/list / tools/call，断言响应 |
| 设置页   | 开关切换、端口修改即时生效（`mcp_status` 状态正确）；端口被占时给出可读错误               |
| 端到端   | ZCode 会话调用 `list_issues` → `analyze_report`（Issue #15），结果与灵鉴分析页一致：ERROR 2 条（"宗门商店刷新失败：未找到当前宗门ID"）、INFO 145、WARN 53 |
| 并发     | 灵鉴下载新上报的同时，MCP 工具调用正常排队返回（Mutex 互斥，无死锁）                      |
| 回归     | `npm run tauri dev` / `npm run tauri build` 正常；前端 vitest 全绿；MCP 关闭时无端口监听、无行为差异 |

## 9. 风险与待决事项

- **rmcp 与 ZCode 的 Streamable HTTP 兼容性**：两端均实现现行 MCP 规范，但 streamable http 细节（session header、SSE 回退）存在实现差异风险；验收时若不通，降级路径为 rmcp 的 SSE transport 或短轮询排查（ZCode 有 diagnosing-mcp 诊断技能可用）。
- **rmcp/schemars 版本匹配**：两者迭代快，实现时锁定并记录在 Cargo.toml 注释中。
- **端口冲突**：默认 3920 若被占，启动失败需在设置页报错并允许改端口；不提供自动换端口（避免与配置片段不一致）。
- **待决：修复目标项目**。游戏源码目录尚未确认，不影响本方案实施，只影响第 7 节 AGENTS.md 落点与联调环境。

## 10. 实施步骤（提交粒度）

1. `feat(mcp): 进程内 MCP server 骨架与 list_issues——rmcp streamable-http + axum，settings 增 mcpEnabled/mcpPort，随应用启停`
2. `feat(mcp): 补齐 get_report/analyze_report/query_logs——明细默认截断，契约对齐设计方案`
3. `feat(ui): 设置页 MCP 配置分区——开关/端口/运行状态，连接 URL 与 ZCode 配置片段一键复制`
4. `docs: ZCode 侧接入配置与验收记录——端到端验证结果回填本文档`
