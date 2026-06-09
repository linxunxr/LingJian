# 灵鉴项目 Tauri 2 深入分析

> 版本：v1.0 | 日期：2026-06-09

基于灵鉴日志分析工具的具体需求，对 Tauri 2 进行深入技术分析，覆盖项目骨架、Commands、插件、权限、构建分发等实际开发中的关键点。

---

## 1. 项目初始化

### 1.1 创建项目

```bash
npm create tauri-app@latest lingjian -- --template vue-ts --manager npm
```

### 1.2 生成的结构

```
lingjian/
├── src/                    ← Vue 3 前端
│   ├── App.vue
│   ├── main.ts
│   └── style.css
├── src-tauri/              ← Rust 后端
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json    ← Tauri 2 权限配置（新增）
│   ├── icons/
│   └── src/
│       ├── main.rs         ← 仅做启动入口
│       └── lib.rs          ← 逻辑和命令注册（Tauri 2 新增）
├── index.html
├── package.json
└── vite.config.ts
```

**Tauri 2 关键变化**：拆分为 `main.rs`（启动）+ `lib.rs`（逻辑），便于测试和复用。

### 1.3 入口文件

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    lingjian_lib::run()
}

// src-tauri/src/lib.rs
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 2. Tauri Commands 详解

### 2.1 定义与注册

Commands 是前后端通信的核心机制。Rust 端用 `#[tauri::command]` 标注函数，前端用 `invoke()` 调用。

```rust
// src-tauri/src/commands/issue.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedIssue {
    pub number: i32,
    pub title: String,
    pub report_id: Option<String>,
}

#[tauri::command]
pub async fn parse_issue(
    url: String,
    github_token: String,
) -> Result<ParsedIssue, String> {
    let re = regex::Regex::new(r"github\.com/([^/]+)/([^/]+)/issues/(\d+)")
        .map_err(|e| e.to_string())?;
    let caps = re.captures(&url).ok_or("无法解析 Issue URL")?;
    // ...调用 GitHub API，提取 reportId
    Ok(ParsedIssue { number, title, report_id })
}
```

注册到 Builder：

```rust
// src-tauri/src/lib.rs
mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::issue::parse_issue,
            commands::download::download_logs,
            commands::analyze::analyze_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2.2 前端调用

```typescript
import { invoke } from '@tauri-apps/api/core'  // Tauri 2 路径（非 tauri）

const parsed = await invoke<ParsedIssue>('parse_issue', {
    url: 'https://github.com/...',
    githubToken: 'ghp_...',
})
```

**命名转换规则**：Rust `snake_case` 参数自动映射为前端 `camelCase`。例如 `report_id` → `reportId`，`scf_url` → `scfUrl`。如需保留 snake_case，加 `#[tauri::command(rename_all = "snake_case")]`。

### 2.3 使用 Tauri State 共享资源

多个 Command 共享资源（如 SQLite 连接、HTTP Client）时使用 `tauri::State`：

```rust
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub http_client: reqwest::Client,
}

#[tauri::command]
pub async fn download_logs(
    report_id: String,
    state: State<'_, AppState>,   // 自动注入
) -> Result<DownloadResult, String> {
    let client = &state.http_client;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // ...
}
```

注册 State：

```rust
pub fn run() {
    let db = rusqlite::Connection::open(get_db_path())
        .expect("无法打开数据库");
    let http_client = reqwest::Client::new();

    tauri::Builder::default()
        .manage(AppState {
            db: Mutex::new(db),
            http_client,
        })
        .invoke_handler(tauri::generate_handler![...])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**最佳实践**：将 `reqwest::Client` 和 `rusqlite::Connection` 都放入 State 管理，避免每次 Command 重复创建。`reqwest::Client` 内部维护连接池，应全局复用。

---

## 3. Rust 依赖与使用

### 3.1 Cargo.toml

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-store = "2"
tauri-plugin-updater = "2"
tauri-plugin-dialog = "2"       # 文件选择对话框（导出用）
tauri-plugin-shell = "2"        # 打开外部链接
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "gzip"] }
flate2 = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
regex = "1"
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 3.2 关键 crate 使用要点

**reqwest**：
- 使用 `features = ["json", "gzip"]` 支持 JSON 响应自动解析和 gzip 传输
- `Client::new()` 内部维护连接池，必须全局复用，不要每次请求创建
- async 运行时由 Tauri 的 tokio 提供，无需自行配置

**flate2**：
- `GzDecoder::new(&bytes[..])` 解压 gzip
- 仅需 `read_to_string()` 即可，无需额外 feature
- 500 条日志 gzip 后约 50-200KB，解压瞬间完成

**rusqlite**：
- `features = ["bundled"]` 自动编译 SQLite，无需系统安装
- **注意**：首次 `cargo build` 编译 SQLite 源码需 2-5 分钟，后续增量编译很快
- `Connection` 不是 `Send`，需用 `Mutex` 包裹放入 State
- SQL 参数用 `params![]` 宏，比字符串拼接安全

**regex**：
- `Regex::new()` 编译正则，应缓存复用（用 `lazy_static!` 或 `OnceLock`）
- Tauri Command 中每次调用都编译正则有性能开销

---

## 4. 插件系统

### 4.1 Store 插件：凭证持久化

**安装**：

```bash
npm install @tauri-apps/plugin-store
```

**Rust 端注册**：

```rust
.plugin(tauri_plugin_store::Builder::default().build())
```

**前端使用**：

```typescript
import { Store } from '@tauri-apps/plugin-store'

let store: Store | null = null

async function getStore(): Promise<Store> {
    if (!store) {
        store = await Store.load('settings.json')
    }
    return store
}

export async function saveGitHubToken(token: string) {
    const s = await getStore()
    await s.set('githubToken', token)
}

export async function getGitHubToken(): Promise<string | undefined> {
    const s = await getStore()
    return await s.get<string>('githubToken')
}
```

**加密方案**：

Store 插件本身以 JSON 明文存储。对于灵鉴的敏感凭证（GitHub Token、SCF API Key），推荐两种方案：

| 方案 | 实现 | 安全性 | 复杂度 |
|------|------|--------|--------|
| A: Rust 端 AES-256 加密后存 Store | Rust 端加解密，前端只拿明文 | 高 | 中 |
| B: 系统 Keyring | Rust `keyring` crate，Windows 用 DPAPI | 最高 | 低 |

**推荐方案 B**：Rust crate `keyring` 直接调用系统密钥链，零配置加密。灵鉴只需存储 2 个密钥，Keyring 完全够用。

```rust
// Rust 端使用 keyring
use keyring::Entry;

fn save_credential(service: &str, key: &str, value: &str) -> Result<(), keyring::Error> {
    let entry = Entry::new(service, key)?;
    entry.set_password(value)
}

fn get_credential(service: &str, key: &str) -> Result<String, keyring::Error> {
    let entry = Entry::new(service, key)?;
    entry.get_password()
}
```

### 4.2 Updater 插件：自动更新

**安装**：

```bash
npm install @tauri-apps/plugin-updater @tauri-apps/plugin-process
```

**生成签名密钥对**：

```bash
npm run tauri signer generate -w ~/.tauri/lingjian.key
```

生成私钥（CI 签名用）和公钥（写入配置）。

**tauri.conf.json 配置**：

```json
{
    "plugins": {
        "updater": {
            "endpoints": [
                "https://releases.lingjian.dev/{{target}}/{{arch}}/{{current_version}}"
            ],
            "pubkey": "公钥内容"
        }
    }
}
```

**前端检查更新**：

```typescript
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

async function checkForUpdate() {
    const update = await check()
    if (update?.available) {
        await update.downloadAndInstall()
        await relaunch()
    }
}
```

**更新端点返回格式**：

```json
{
    "version": "1.1.0",
    "notes": "修复日志解析问题",
    "pub_date": "2026-06-09T00:00:00Z",
    "platforms": {
        "windows-x86_64": {
            "signature": "CI 生成的签名",
            "url": "https://releases.lingjian.dev/LingJian_1.1.0_x64-setup.exe"
        }
    }
}
```

**建议**：初期可不配置 Updater，手动发布到 GitHub Releases 即可。等功能稳定后再接入自动更新。

### 4.3 Dialog 插件：文件导出对话框

```bash
npm install @tauri-apps/plugin-dialog
```

```typescript
import { save } from '@tauri-apps/plugin-dialog'

const filePath = await save({
    defaultPath: 'report.md',
    filters: [{ name: 'Markdown', extensions: ['md'] }],
})
```

### 4.4 Shell 插件：打开外部链接

```bash
npm install @tauri-apps/plugin-shell
```

```typescript
import { open } from '@tauri-apps/plugin-shell'
open('https://github.com/linxunxr/PathofIdleImmortals-bugs/issues/42')
```

---

## 5. Tauri 2 权限模型

### 5.1 核心概念

Tauri 2 用 **Capabilities + Permissions** 替代了 Tauri 1 的 `allowlist`。这是最重大的架构变化。

- **Capability**：定义哪些窗口可以使用哪些权限
- **Permission**：细粒度的操作许可，如 `core:window:allow-close`
- **Scope**：限定可访问的路径/URL 范围

### 5.2 灵鉴最小权限配置

灵鉴的 HTTP 请求和文件操作全在 Rust 端完成（reqwest + std::fs），前端只负责 UI 渲染。这意味着 Tauri 权限需求极少：

```json
// src-tauri/capabilities/default.json
{
    "identifier": "default",
    "description": "灵鉴默认权限",
    "windows": ["main"],
    "permissions": [
        "core:default",
        "core:window:default",
        "core:window:allow-close",
        "core:window:allow-set-title",
        "store:default",
        "updater:default",
        "dialog:default",
        "dialog:allow-save",
        "shell:default",
        "shell:allow-open"
    ]
}
```

**关键洞察**：reqwest 和 std::fs 不受 Tauri 权限系统约束，只有通过 Tauri 插件（如 `tauri-plugin-http`、`tauri-plugin-fs`）发起的操作才受权限控制。灵鉴采用"重 Rust 端、轻前端"架构，天然获得最小权限面。

### 5.3 如果前端也需要 HTTP 请求

虽然不推荐（Token 会经过 WebView），但如果需要前端直接请求 GitHub API：

```bash
npm install @tauri-apps/plugin-http
```

```json
{
    "permissions": [
        "http:default",
        "http:allow-fetch",
        "http:allow-fetch-send"
    ]
}
```

---

## 6. 构建与分发

### 6.1 开发命令

```bash
npm run tauri dev      # 启动开发服务器，前端热更新 + Rust 增量编译
npm run tauri build    # 构建发布版
```

### 6.2 构建产物

| 平台 | 格式 | 路径 |
|------|------|------|
| Windows | MSI | `src-tauri/target/release/bundle/msi/LingJian_1.0.0_x64_en-US.msi` |
| Windows | NSIS exe | `src-tauri/target/release/bundle/nsis/LingJian_1.0.0_x64-setup.exe` |
| macOS | DMG | `src-tauri/target/release/bundle/dmg/LingJian_1.0.0_x64.dmg` |
| Linux | deb | `src-tauri/target/release/bundle/deb/lingjian_1.0.0_amd64.deb` |
| Linux | AppImage | `src-tauri/target/release/bundle/appimage/lingjian_1.0.0_amd64.AppImage` |

### 6.3 构建前置条件

| 平台 | 要求 |
|------|------|
| Windows | Visual Studio Build Tools + WebView2（Win10/11 已内置） |
| macOS | Xcode Command Line Tools |
| Linux | `libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libayatana-appindicator3-dev` 等 |

### 6.4 体积估算

| 组件 | 体积 |
|------|------|
| Tauri Runtime | ~3 MB |
| WebView（系统自带） | 0 |
| Rust 编译产物（含 reqwest/flate2/serde/rusqlite） | ~3-4 MB |
| 前端资源（Vue + CSS） | ~500 KB |
| **总计** | **~6-8 MB** |

rusqlite bundled 模式会增加约 1.5 MB（含 SQLite 静态库），reqwest + tls 增加约 1 MB。

### 6.5 tauri.conf.json 关键配置

```json
{
    "productName": "灵鉴",
    "version": "1.0.0",
    "identifier": "com.lingjian.app",
    "build": {
        "frontendDist": "../dist",
        "devUrl": "http://localhost:1420",
        "beforeDevCommand": "npm run dev",
        "beforeBuildCommand": "npm run build"
    },
    "app": {
        "windows": [
            {
                "title": "灵鉴 LingJian",
                "width": 1024,
                "height": 768,
                "resizable": true,
                "center": true
            }
        ],
        "security": {
            "csp": null
        }
    },
    "bundle": {
        "active": true,
        "targets": "all",
        "icon": [
            "icons/32x32.png",
            "icons/128x128.png",
            "icons/128x128@2x.png",
            "icons/icon.icns",
            "icons/icon.ico"
        ]
    }
}
```

---

## 7. Tauri 2 vs Tauri 1 破坏性变更

| 变更 | Tauri 1 | Tauri 2 | 灵鉴影响 |
|------|---------|---------|---------|
| 项目结构 | 全在 main.rs | main.rs + lib.rs | 必须拆分 |
| 前端 invoke 导入 | `@tauri-apps/api/tauri` | `@tauri-apps/api/core` | 导入路径变更 |
| 权限系统 | allowlist（白名单） | Capabilities + Permissions | 全新体系 |
| 插件 | 手动注册 | 统一 `.plugin()` + 独立 npm 包 | Store/Updater 独立安装 |
| HTTP 请求 | allowlist.http | tauri-plugin-http 或 Rust reqwest | 无影响（用 reqwest） |
| 文件系统 | allowlist.fs | tauri-plugin-fs + scope | 无影响（用 std::fs） |
| Store 插件 | v1 API | v2 API（`Store.load()` 新签名） | API 变化 |
| Updater | 内置 | 独立插件 v2 | 独立安装 |

**灵鉴无需考虑迁移**，因为是从零创建 Tauri 2 项目，直接使用最新 API。

---

## 8. 开发中的常见问题与解决方案

### 8.1 Rust 编译慢

首次 `cargo build` 编译 rusqlite bundled + reqwest tls 需要 3-8 分钟。解决方案：

- 开发时用 `npm run tauri dev`，Rust 增量编译仅重编变更文件
- 考虑用 `sccache` 缓存编译产物：`cargo install sccache && export RUSTC_WRAPPER=sccache`

### 8.2 Tauri Command 中的 async 运行时

Tauri 2 的 async Command 自动在 tokio 运行时执行，无需手动 `#[tokio::main]`。但注意：

- `Mutex::lock()` 是同步操作，不要在 async 函数中长时间持有锁
- rusqlite 操作是同步阻塞的，如果日志量大（1000+ 条插入），考虑用 `tokio::task::spawn_blocking()` 包装

```rust
#[tauri::command]
pub async fn download_logs(report_id: String, state: State<'_, AppState>) -> Result<DownloadResult, String> {
    // reqwest 异步请求（不阻塞）
    let bytes = state.http_client.get(&url).send().await?.bytes().await?;

    // rusqlite 同步操作用 spawn_blocking 包装
    let db_path = get_db_path();
    let entries = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&db_path)?;
        insert_entries(&conn, &entries)?;
        Ok::<_, String>(entries)
    }).await.map_err(|e| e.to_string())??;

    Ok(DownloadResult { report_id, log_count: entries.len() })
}
```

### 8.3 前端开发中的 WebView 兼容性

Tauri 使用系统 WebView：
- Windows：WebView2（Chromium 内核，Win10/11 已内置）
- macOS：WKWebView（Safari 内核）
- Linux：WebKitGTK

**注意**：macOS 的 WKWebView 对 ES2020+ 支持稍落后于 Chromium，但灵鉴的 Vue 3 + Vite 默认转译到兼容级别，不会有问题。

### 8.4 跨域问题

Tauri 2 前端加载本地文件（`tauri://localhost`），不受浏览器 CORS 限制。但前端通过 `fetch()` 请求外部 API 仍受 CORS 约束。解决方案：

- **推荐**：所有外部请求在 Rust 端通过 reqwest 发起，无 CORS 问题
- **不推荐**：前端 `fetch()` + 配置 CSP，复杂且不安全

灵鉴的 GitHub API 和 SCF 下载都在 Rust 端，天然避免 CORS。

---

## 9. 结论

Tauri 2 完全满足灵鉴日志分析工具的需求：

| 需求 | Tauri 2 方案 | 评价 |
|------|-------------|------|
| 轻量安装包 | 6-8 MB MSI | 远优于 Electron 的 80+ MB |
| gzip 解压 | Rust flate2 | 性能远超 JS 解压 |
| HTTP 请求 | Rust reqwest | 无 CORS、Token 不暴露 |
| 本地缓存 | rusqlite (bundled) | 零依赖，嵌入式 |
| 凭证安全 | keyring crate | 系统 Keyring 加密 |
| 自动更新 | tauri-plugin-updater | 内置签名验证 |
| 仙侠 UI | Vue 3 + 自建组件 | 完全可控 |

**唯一的风险**：Rust 学习曲线。但灵鉴的 Rust 代码主要是"胶水层"（调 API、解压、存库），不涉及复杂并发或 unsafe，上手门槛低。

建议先完成阶段 1（项目骨架 + 窗口跑通），验证 Rust 开发环境后再推进核心功能。

---

*文档由 AI 辅助生成，创建日期：2026-06-09*
