# 灵鉴 (LingJian) - Claude Code 工作规范

## 项目简介
Path of Idle Immortals 日志分析工具 — 独立桌面应用（Tauri 2 + Vue 3 + Rust）。

## 沟通语言
请始终使用简体中文与我对话。

---

## 📝 Git 提交规范

### 小步提交原则
- **每次变更都必须使用 git 提交**
- **提交粒度**：一个语义一个提交
- **提交信息**：使用中文描述，简洁清晰

### 提交信息格式
```
<类型>: <描述>
```

**类型说明**：
- `feat:` 新功能、新内容
- `fix:` 修复 bug
- `refactor:` 代码重构（无功能变更）
- `style:` 代码格式、样式调整
- `docs:` 文档更新
- `perf:` 性能优化
- `chore:` 构建工具、依赖更新等杂项

---

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri 2 |
| 前端 | Vue 3 + TypeScript + Vite |
| 后端 | Rust (reqwest, flate2, serde, rusqlite, regex) |
| 图表 | Chart.js |
| 主题 | 深色专业风格（Slate Dark + Blue） |

## 开发命令

```bash
npm run tauri dev    # 开发模式
npm run tauri build  # 构建发布
```

---

*最后更新：2026-06-09*
