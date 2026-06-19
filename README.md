# 灵鉴 LingJian

> Path of Idle Immortals 日志分析工具

灵鉴是仙侠游戏《Path of Idle Immortals》日志上报系统的消费端工具。开发团队收到 GitHub Issue 后，通过灵鉴一键下载并分析用户上报的 gzip 压缩日志，快速定位 bug 根因。

## 核心工作流

```
输入 Issue URL/编号/reportId
        ↓
解析 GitHub Issue → 提取 reportId（Issue body 中的 HTML 注释）
        ↓
调用 SCF 下载端点 → 获取 .gz 压缩包
        ↓
gunzip 解压 → JSON 解析 → SQLite 缓存入库
        ↓
级别过滤 / 关键词搜索 / 时间线分析 / 错误聚合
        ↓
表格浏览 / 详情查看 / 导出报告（MD/JSON/CSV）
```

## 技术栈

| 层 | 技术 | 说明 |
|----|------|------|
| 桌面框架 | [Tauri 2](https://tauri.app/) | 轻量原生应用，Rust 后端 + WebView 前端 |
| 前端 | Vue 3 + TypeScript + Vite 6 | Composition API，模块级单例状态管理 |
| 后端 | Rust | reqwest / flate2 / serde / rusqlite / regex / keyring |
| 存储 | SQLite (rusqlite) | 本地缓存日志与上报记录 |
| 凭证 | 系统 Keyring | Windows DPAPI / macOS Keychain / Linux Secret Service |
| 图表 | Chart.js | 散点图时间线可视化 |

## 环境要求

### 通用依赖

- **[Node.js](https://nodejs.org/)** ≥ 18（建议 LTS 版本）
- **[Rust](https://www.rust-lang.org/)** ≥ 1.70（含 `cargo`）
- **[Git](https://git-scm.com/)**

### 平台特定

<TABLE>

| 平台 | 额外依赖 |
|------|----------|
| **Windows** | [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) 或完整版 VS（含桌面 C++ 工作负载）；WebView2（Win10/11 通常已预装） |
| **macOS** | Xcode Command Line Tools（`xcode-select --install`） |
| **Linux** | 见 [Tauri 先决条件](https://v2.tauri.app/start/prerequisites/)，需 `webkit2gtk`、`libssl`、`libayatana-appindicator` 等 |

</TABLE>

> 💡 Windows 下构建需要 MSVC 工具链（`rustup default stable-x86_64-pc-windows-msvc`）。

## 快速开始

### 1. 克隆仓库

```bash
git clone https://github.com/linxunxr/LingJian.git
cd LingJian
```

### 2. 安装前端依赖

```bash
npm install
```

### 3. 开发模式运行

```bash
npm run tauri dev
```

该命令会同时启动：
- Vite 前端开发服务器（`http://localhost:1420`，热更新）
- Rust 后端编译与 Tauri 应用窗口

首次运行会自动弹出**首次启动引导**，按提示配置 GitHub Token 和 SCF 下载端点（每步可即时验证连接）。

### 4. 构建生产版本

```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`：

| 平台 | 产物格式 |
|------|----------|
| Windows | `.msi` 安装包、`.exe`（NSIS） |
| macOS | `.dmg`、`.app` |
| Linux | `.deb`、`.rpm`、`.AppImage` |

> ⚠️ Linux/macOS 构建需在对应平台进行（Tauri 不支持交叉编译打包）。

## 常用命令

| 命令 | 说明 |
|------|------|
| `npm run dev` | 仅启动前端 Vite 开发服务器（浏览器预览，无 Tauri 后端） |
| `npm run build` | 前端类型检查（`vue-tsc --noEmit`）+ 生产构建到 `dist/` |
| `npm run tauri dev` | 启动完整桌面应用（开发模式） |
| `npm run tauri build` | 构建可分发的安装包 |
| `npm run tauri build --debug` | 构建带调试符号的版本 |
| `cargo test --lib` | 运行 Rust 后端单元测试（`src-tauri/` 目录下） |
| `cargo build` | 仅编译 Rust 后端（`src-tauri/` 目录下） |

## 项目结构

```
LingJian/
├── src/                         # Vue 前端
│   ├── App.vue                  # 应用骨架（导航 + 全局快捷键 + 引导）
│   ├── router/                  # 路由配置（首页/分析/设置）
│   ├── views/                   # 页面视图
│   │   ├── HomeView.vue         # 首页（Issue 输入 + 最近分析）
│   │   ├── AnalyzeView.vue      # 分析页（过滤/时间线/表格/详情/导出）
│   │   └── SettingsView.vue     # 设置页（凭证配置 + 连接验证）
│   ├── components/              # 可复用组件
│   │   ├── IssueInput.vue       # Issue 输入框
│   │   ├── LogFilter.vue        # 日志过滤（级别/tag/关键词）
│   │   ├── LogTable.vue         # 虚拟滚动日志表格
│   │   ├── LogDetail.vue        # 单条日志详情 + JSON 数据
│   │   ├── Timeline.vue         # Chart.js 散点图时间线
│   │   ├── ErrorAggregates.vue  # 错误聚合面板
│   │   ├── Onboarding.vue       # 首次启动引导
│   │   ├── UpdateCard.vue       # 应用更新卡片
│   │   └── DataCard.vue         # 本地数据管理（目录/大小/迁移）
│   ├── composables/             # 组合式状态管理
│   │   ├── useSettings.ts       # 凭证读写（keyring + store）
│   │   ├── useAnalysis.ts       # 分析流程状态机
│   │   ├── useExport.ts         # 导出流程封装
│   │   └── useUpdater.ts        # 自动更新状态机
│   ├── types/                   # TypeScript 类型定义
│   ├── utils/                   # 工具函数（格式化等）
│   └── styles/                  # 全局样式与主题
│
├── src-tauri/                   # Rust 后端 + Tauri 配置
│   ├── src/
│   │   ├── lib.rs               # 应用入口（插件注册 + 状态装配）
│   │   ├── main.rs              # 二进制入口
│   │   ├── commands/            # Tauri 命令（前端可调用）
│   │   │   ├── issue.rs         # Issue 解析 + GitHub API
│   │   │   ├── download.rs      # 日志下载
│   │   │   ├── analyze.rs       # 日志分析
│   │   │   ├── reports.rs       # 最近上报列表
│   │   │   ├── export_.rs       # 导出（MD/JSON/CSV）
│   │   │   ├── secret.rs        # 凭证钥匙串读写
│   │   │   ├── settings.rs      # 连接验证
│   │   │   └── storage.rs       # 数据目录管理
│   │   ├── services/            # 业务逻辑层
│   │   │   ├── github.rs        # GitHub API 客户端
│   │   │   ├── downloader.rs    # 下载 + gzip 解压
│   │   │   ├── cache.rs         # SQLite 缓存
│   │   │   ├── analyzer.rs      # 统计分析引擎
│   │   │   ├── exporter.rs      # 报告生成器
│   │   │   ├── secret.rs        # keyring 封装
│   │   │   └── paths.rs         # 数据目录解析（便携/降级/迁移）
│   │   └── models/              # 数据模型
│   ├── migrations/              # SQLite 迁移脚本
│   ├── capabilities/            # Tauri 权限配置
│   ├── icons/                   # 应用图标
│   └── tauri.conf.json          # Tauri 构建配置
│
├── .github/
│   └── workflows/
│       └── release.yml          # GitHub Actions 发布工作流（多平台构建 + 签名）
│
└── docs/                        # 设计文档
    ├── 灵鉴日志分析工具设计方案.md
    └── 日志上报系统设计方案.md
```

## 配置说明

### 首次使用

启动应用后会自动弹出引导对话框，依次配置：

1. **GitHub Token**（Fine-grained，仅需目标仓库的 Issues 读取权限）
2. **SCF 下载端点** URL 与 API Key
3. 完成保存

> 也可跳过引导，后续在「设置」页随时配置。

### 凭证存储

| 凭证 | 存储位置 | 说明 |
|------|----------|------|
| GitHub Token | 系统 Keyring | 加密存储，Windows DPAPI / macOS Keychain / Linux Secret Service |
| SCF API Key | 系统 Keyring | 加密存储 |
| SCF URL | 本地配置文件 | 明文（`tauri-plugin-store`，非敏感信息） |

### 数据位置

灵鉴采用**便携优先**策略，避免占用 C 盘：

1. **便携模式（默认）**：数据存放在安装目录下的 `data/` 子目录（如 `D:\LingJian\data\`），跟随安装位置，卸载即清除
2. **系统模式（降级）**：当安装目录无写权限（如装在 `Program Files`）时，自动降级到系统默认目录
3. **自定义目录**：可在「设置 → 本地数据」中手动指定任意磁盘位置，数据自动迁移

数据目录内容：
- `lingjian.db` — SQLite 数据库（上报记录 + 日志条目）
- `cache/` — gzip 压缩包缓存（离线复用）

> 💡 **建议**：安装时选择非系统盘目录（如 D 盘），数据自动便携存储，无需额外配置。
> Windows 安装包为 NSIS exe，安装向导支持自定义安装路径。

降级时的系统默认目录：

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\com.lingjian.app\` |
| macOS | `~/Library/Application Support/com.lingjian.app/` |
| Linux | `~/.local/share/com.lingjian.app/` |

> 无论数据目录在哪，系统目录下都会保留一个 `data_dir.txt` 标记文件（记录当前数据目录路径），用于跨重启/升级保持一致。

## 使用指南

### 基本分析流程

1. **首页**输入 GitHub Issue URL（如 `https://github.com/owner/repo/issues/42`）、编号（`#42`）或直接输入 reportId
2. 点击「分析」，灵鉴会自动完成：解析 Issue → 下载日志 → 缓存 → 分析
3. 跳转**分析页**查看结果

### 分析页功能

| 功能 | 操作 |
|------|------|
| **级别过滤** | 点击 DEBUG / INFO / WARN / ERROR 标签多选 |
| **模块筛选** | tag 下拉框多选 |
| **关键词搜索** | 输入框（命中 message 或 data 字段） |
| **时间线** | 散点图展示 WARN/ERROR 分布，hover 看详情 |
| **错误聚合** | 相同错误消息去重计数 + 首末次时间 |
| **日志浏览** | 虚拟滚动表格（支持 500+ 条流畅渲染），点击行查看详情 |
| **导出** | 导航栏 MD / JSON / CSV 按钮，选择保存路径 |

### 自动更新

灵鉴内置自动更新机制，基于 Tauri Updater + GitHub Releases：

- **启动时**自动静默检查更新，若有新版本，导航栏右侧出现绿色「新版本」闪烁提示
- 点击提示或进入「设置」页底部「应用更新」卡片，可查看新版本号、发布日期和更新说明
- 点击「下载并安装」自动下载签名验证后的安装包，完成后自动重启

更新流程包含 Ed25519 签名验证，确保安装包未被篡改。

### 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl` + `F`（macOS `⌘` + `F`） | 聚焦当前页搜索框 |
| `Esc` | 分析页返回首页 |

### 最近分析

首页「最近分析」列表展示已下载的上报记录，点击可直接重新打开分析结果（从本地 SQLite 缓存读取，无需重新下载）。

## 开发说明

### 单元测试

Rust 后端内置单元测试，覆盖核心业务逻辑：

```bash
cd src-tauri
cargo test --lib
```

测试覆盖：Issue URL 解析、reportId 提取、gzip 解压、日志字段兼容、级别统计、错误聚合、CSV 转义、导出格式等。

### 日志格式约定

灵鉴解析的日志 JSON 结构：

```json
{
  "exportedAt": "2026-06-08T14:35:22Z",
  "logs": [
    {
      "timestamp": "2026-06-08T14:35:22Z",
      "level": "ERROR",
      "tag": "战斗",
      "message": "灵气溢出",
      "data": { "value": -120 }
    }
  ]
}
```

字段说明：
- `timestamp`：ISO 8601 时间戳
- `level`：`DEBUG` / `INFO` / `WARN` / `ERROR`（大小写不敏感）
- `tag`：模块/功能标签（兼容上游可能使用的 `module` 字段名）
- `message`：日志消息
- `data`：可选的结构化附加数据

### 命名约定

- Rust 遵循 snake_case，通过 `#[serde(rename_all = "camelCase")]` 与前端 camelCase 对齐
- 前端类型定义集中在 `src/types/index.ts`，与 Rust models 保持字段一致

## 发布流程

应用通过 GitHub Actions 自动构建发布，触发条件为推送 `v*` 格式的 tag。

### 前置配置（仅首次）

在 GitHub 仓库 Settings → Secrets and variables → Actions 中添加两个 Secret（值见 `.env.example` 说明）：

| Secret 名称 | 说明 |
|-------------|------|
| `TAURI_SIGNING_PRIVATE_KEY` | Ed25519 私钥（base64 长字符串）|
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码 |

> ⚠️ 私钥由 `npx tauri signer generate` 生成，**切勿提交到仓库或泄露**。丢失私钥将无法发布可更新的版本。

### 发布新版本

```bash
# 1. 更新版本号（package.json 与 src-tauri/tauri.conf.json 的 version 字段需一致）
# 2. 提交变更
git add -A
git commit -m "chore: bump version to v0.2.0"

# 3. 打 tag 并推送
git tag v0.2.0
git push origin v0.2.0
```

推送 tag 后，GitHub Actions 会自动：
1. 在 Windows / macOS (x86_64 + aarch64) / Linux 四个平台并行构建
2. 用私钥对安装包签名
3. 创建 GitHub Release，附带各平台安装包 + `latest.json`（更新清单）
4. 已安装旧版本的应用下次启动时自动检测到更新

构建产物：

| 平台 | 安装包 |
|------|--------|
| Windows | `.msi` / `.exe` (NSIS) |
| macOS | `.dmg` (Intel + Apple Silicon) |
| Linux | `.deb` / `.rpm` / `.AppImage` |

> 💡 本地构建签名版本（测试用）：设置环境变量 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 后执行 `npm run tauri build`。

## 相关文档

- [灵鉴日志分析工具设计方案](docs/灵鉴日志分析工具设计方案.md) — 工具整体设计与架构
- [日志上报系统设计方案](docs/日志上报系统设计方案.md) — 上报系统（游戏客户端 + SCF + 存储）完整方案

## 许可证

私有项目，未公开发布。
