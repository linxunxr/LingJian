#!/usr/bin/env node
/**
 * 从 CHANGELOG.md 提取指定版本的更新日志，写入 latest.json 的 notes 字段。
 *
 * tauri-action 生成的 latest.json 默认不带更新日志，本脚本补齐该字段，
 * 使 Tauri updater 能将 notes 映射为 update.body，供客户端 UI 展示。
 *
 * 用法: node scripts/inject-changelog.mjs <path/to/latest.json> <version>
 *
 * 参数：
 *   <path/to/latest.json>  tauri-action 生成、已重写 URL 的 manifest 文件
 *   <version>              版本号，应与 tag 一致，如 v0.2.0
 *
 * CHANGELOG.md 约定：
 *   每个版本以 `## v{版本号} - {日期}` 起头，正文用 `### 新增 / ### 修复 / ### 优化`
 *   等分类，列表项以 `- ` 开头。本脚本提取标题行之后、下一个 `## v` 或文件末尾
 *   之间的内容。
 *
 * 缺失处理：找不到对应版本段落时，打印警告（不中断流程），notes 写入简短默认值。
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { exit } from 'node:process'

const __dirname = dirname(fileURLToPath(import.meta.url))
const CHANGELOG_PATH = resolve(__dirname, '..', 'CHANGELOG.md')

const manifestPath = process.argv[2]
const version = process.argv[3]

if (!manifestPath || !version) {
  console.error('用法: node scripts/inject-changelog.mjs <path/to/latest.json> <version>')
  exit(1)
}

// 归一化版本号：确保带 v 前缀，便于与 CHANGELOG 标题匹配
const normalizedVersion = version.startsWith('v') ? version : `v${version}`

/**
 * 从 CHANGELOG.md 提取指定版本段落正文。
 * 按 `^## ` 标题行将文件分段，找到目标版本标题后，取其到下一个 `## ` 标题
 * （或文件末尾）之间的内容。
 *
 * 不用单条正则前瞻（`(?=^##|$)` 在 multiline 下 `$` 会命中行尾导致截断），
 * 改用按行扫描，逻辑直观且无边界陷阱。
 */
function extractChangelogSection(changelog, ver) {
  // 转义版本号中的正则特殊字符（如点），用于标题匹配
  const escaped = ver.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const titlePattern = new RegExp(`^##\\s+${escaped}\\b`)

  const lines = changelog.split(/\r?\n/)
  let inSection = false
  const bodyLines = []

  for (const line of lines) {
    if (/^##\s/.test(line)) {
      if (inSection) {
        // 已在目标段内，遇到下一个 `## ` 标题即结束
        break
      }
      // 命中目标版本标题，进入该段（标题行本身不纳入正文）
      if (titlePattern.test(line)) inSection = true
      continue
    }
    if (inSection) bodyLines.push(line)
  }

  if (!inSection) return null
  // 去除首尾空白行
  return bodyLines.join('\n').trim()
}

// 读取 CHANGELOG
let changelog
try {
  changelog = readFileSync(CHANGELOG_PATH, 'utf-8')
} catch (e) {
  console.warn(`⚠ 无法读取 CHANGELOG.md（${CHANGELOG_PATH}）：${e.message}`)
  console.warn('  notes 将使用默认值。')
}

let notes
if (changelog) {
  const section = extractChangelogSection(changelog, normalizedVersion)
  if (section) {
    notes = section
    const lineCount = section.split('\n').length
    console.log(`✓ 已从 CHANGELOG.md 提取 ${normalizedVersion} 的更新日志（${lineCount} 行）`)
  } else {
    console.warn(`⚠ CHANGELOG.md 中未找到 ${normalizedVersion} 的段落`)
    console.warn('  notes 将使用默认占位值，请补充 CHANGELOG.md。')
    notes = `${normalizedVersion} 更新内容（详见 CHANGELOG.md）`
  }
} else {
  notes = `${normalizedVersion} 更新内容（详见 CHANGELOG.md）`
}

// 读取 manifest
const raw = readFileSync(manifestPath, 'utf-8')
const manifest = JSON.parse(raw)

// 写入 notes
manifest.notes = notes

// 补 pub_date（若缺失），用当前时间 ISO 8601
// COS 同步发生在构建之后，时间足够准确
if (!manifest.pub_date) {
  manifest.pub_date = new Date().toISOString()
  console.log('✓ 补充缺失的 pub_date')
}

writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n', 'utf-8')
console.log(`\n✓ 已注入更新日志到 ${manifestPath}`)
