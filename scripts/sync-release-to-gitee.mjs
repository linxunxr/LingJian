#!/usr/bin/env node
/**
 * 同步 GitHub Release 到 Gitee 发行版（国内镜像，全自动）。
 *
 * 做三件事：
 *   1. 在 Gitee 创建发行版（tag / 标题 / changelog 全量带上）
 *   2. 把安装包产物逐个上传为发行版附件（attach_files 接口）
 *   3. 幂等：已存在的同名附件跳过，重复执行不报错
 *
 * 鉴权：Gitee 私人令牌（Authorization: token <GITEE_TOKEN>），
 *       需勾选 projects 权限。接口参照 Yeelight/china-mirror-sync 生产实践。
 *
 * 已知限制（Gitee 平台约束）：
 *   - 每个发行版最多 20 个附件，超出部分跳过并告警（本项目 15 个在限内）
 *   - 偶发文件被 Gitee 内容扫描拒收，单文件失败不阻断整体（告警继续）
 *
 * 定位：GitHub 是权威源，Gitee 只是镜像——本脚本任何失败都只告警退出码 0
 * 之外的场景（脚本自身 bug），网络/平台错误均不阻断发布流水线。
 *
 * 用法: node scripts/sync-release-to-gitee.mjs <dist-dir> <version>
 *
 * 环境变量：
 *   GITEE_TOKEN   必填，私人令牌
 *   GITEE_OWNER   仓库归属，默认 mwcxlinxun
 *   GITEE_REPO    仓库名，默认 ling-jian
 */
import { readFileSync, readdirSync, existsSync, statSync } from 'node:fs'
import { join, basename } from 'node:path'
import { exit } from 'node:process'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'

const execFileAsync = promisify(execFile)

const GITEE_API = 'https://gitee.com/api/v5'
const OWNER = process.env.GITEE_OWNER || 'mwcxlinxun'
const REPO = process.env.GITEE_REPO || 'ling-jian'
// Gitee 发行版附件上限（平台硬限制，超出会被拒绝）
const MAX_ATTACHMENTS = 20
// 上传失败单文件重试次数（跨洲链路抖动兜底）
const UPLOAD_RETRY = 3

const token = process.env.GITEE_TOKEN
const distDir = process.argv[2]
const version = process.argv[3]?.replace(/^v/, '')

if (!distDir || !version) {
  console.error('用法: node scripts/sync-release-to-gitee.mjs <dist-dir> <version>')
  exit(1)
}
if (!token) {
  // 未配置令牌：跳过而非失败（发布流水线不依赖 Gitee，令牌后补即生效）
  console.warn('⚠ 未设置 GITEE_TOKEN，跳过 Gitee 发行版同步')
  exit(0)
}

const headers = { authorization: `token ${token}` }

// Gitee API 超时：v0.4.2 发布实测 CI runner → Gitee 链路异常时 fetch 无超时会
// 无限挂起（卡 40+ 分钟直到手动取消，job 级 6 小时超时才兜底）。所有 API 调用
// 统一走本封装，超时即抛错交给上层重试/跳过，不再依赖 job 超时。
const API_TIMEOUT_MS = 30_000

async function gitee(path, init = {}) {
  const resp = await fetch(`${GITEE_API}${path}`, {
    ...init,
    headers: { ...headers, ...(init.headers || {}) },
    signal: AbortSignal.timeout(API_TIMEOUT_MS),
  })
  const text = await resp.text()
  let body
  try {
    body = text ? JSON.parse(text) : null
  } catch {
    body = text
  }
  if (!resp.ok) {
    const msg = typeof body === 'object' ? JSON.stringify(body) : String(body).slice(0, 200)
    throw new Error(`Gitee API ${resp.status} ${path}: ${msg}`)
  }
  return body
}

// ---- 第 1 步：创建（或复用）发行版 ----

// 从 CHANGELOG.md 提取版本段落作发行说明（与 generate-latest-json 同一套逻辑）
function extractChangelogSection(changelog, ver) {
  const escaped = ver.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const titlePattern = new RegExp(`^##\\s+${escaped}\\b`)
  const lines = changelog.split(/\r?\n/)
  let inSection = false
  const bodyLines = []
  for (const line of lines) {
    if (/^##\s/.test(line)) {
      if (inSection) break
      if (titlePattern.test(line)) inSection = true
      continue
    }
    if (inSection) bodyLines.push(line)
  }
  return inSection ? bodyLines.join('\n').trim() : null
}

let body = `v${version} 更新内容`
const localChangelog = join(import.meta.dirname, '..', 'CHANGELOG.md')
if (existsSync(localChangelog)) {
  const section = extractChangelogSection(readFileSync(localChangelog, 'utf-8'), `v${version}`)
  if (section) body = section
}

// 幂等：先查同名 tag 的发行版是否已存在（重跑流水线场景）。
// 仅 404 视为"不存在"；其他错误（如令牌无效 401）原样抛出，避免误判。
let release
let existing = null
try {
  existing = await gitee(`/repos/${OWNER}/${REPO}/releases/tags/v${version}`)
} catch (e) {
  if (!e.message.startsWith('Gitee API 404')) throw e
}
if (existing) {
  console.log(`↻ 发行版 v${version} 已存在（id=${existing.id}），复用并补传附件`)
  release = existing
} else {
  release = await gitee(`/repos/${OWNER}/${REPO}/releases`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      tag_name: `v${version}`,
      name: `灵鉴 v${version}`,
      body,
      target_commitish: `v${version}`,
      prerelease: false,
    }),
  })
  console.log(`✓ 已创建 Gitee 发行版 v${version}（id=${release.id}）`)
}

// ---- 第 2 步：上传附件 ----

if (!existsSync(distDir)) {
  console.error(`✗ 产物目录不存在: ${distDir}`)
  exit(1)
}

// 已有附件（幂等跳过）：Gitee 列表接口返回 attach_files 字段
const existingAssets = new Set(
  (release.attach_files || release.assets || []).map((a) => a.name),
)

// 收集产物：安装包 + .sig + latest.json，跳过 GitHub 专用清单（latest.github.json）
const files = readdirSync(distDir)
  .filter((f) => {
    if (!/\.(exe|dmg|appimage|deb|rpm|AppImage|tar\.gz|json|sig)$/.test(f)) return false
    if (f === 'latest.github.json') return false
    const path = join(distDir, f)
    return existsSync(path) && statSync(path).isFile()
  })
  .sort()

if (files.length > MAX_ATTACHMENTS) {
  console.warn(`⚠ 产物 ${files.length} 个超出 Gitee 附件上限 ${MAX_ATTACHMENTS}，仅上传前 ${MAX_ATTACHMENTS} 个`)
}

let ok = 0
let skipped = 0
let failed = 0
let uploaded = 0
for (const file of files) {
  if (uploaded >= MAX_ATTACHMENTS) {
    console.warn(`⚠ 已达附件上限，跳过 ${file}`)
    skipped++
    continue
  }
  if (existingAssets.has(file)) {
    console.log(`↻ ${file} 已存在，跳过`)
    skipped++
    uploaded++
    continue
  }

  const filePath = join(distDir, file)
  const sizeMB = statSync(filePath).size / 1024 / 1024
  let done = false
  for (let attempt = 1; attempt <= UPLOAD_RETRY && !done; attempt++) {
    try {
      // 大文件走 curl 子进程：CI runner 的 Node undici 对大 multipart body
      // 存在断连问题（v0.2.3 实测 7MB+ 全部 fetch failed、KB 级小文件正常），
      // curl -F 无此问题且对代理/重试更健壮；小文件仍走 fetch 保持零依赖快路径
      if (statSync(filePath).size > 1024 * 1024) {
        const { stdout } = await execFileAsync('curl', [
          '-sS', '--fail-with-body', '--max-time', '600',
          '-X', 'POST',
          '-H', `Authorization: token ${token}`,
          '-F', `file=@${filePath}`,
          `${GITEE_API}/repos/${OWNER}/${REPO}/releases/${release.id}/attach_files`,
        ])
        if (!stdout.includes('"id"') && stdout.trim()) {
          // Gitee 成功返回 JSON 含 id 字段；其余内容打出来辅助排查
          console.warn(`  ? ${file} 响应异常: ${stdout.slice(0, 120)}`)
        }
      } else {
        const form = new FormData()
        form.append('file', new Blob([readFileSync(filePath)]), basename(file))
        await gitee(`/repos/${OWNER}/${REPO}/releases/${release.id}/attach_files`, {
          method: 'POST',
          body: form,
        })
      }
      console.log(`✓ ${file}（${sizeMB.toFixed(1)}MB）`)
      done = true
      ok++
      uploaded++
    } catch (e) {
      const msg = e.stderr?.toString().trim() || e.message
      if (attempt < UPLOAD_RETRY) {
        console.warn(`  ✗ ${file} 第 ${attempt} 次失败: ${msg}，重试...`)
        await new Promise((r) => setTimeout(r, attempt * 3000))
      } else {
        console.warn(`  ✗ ${file} 上传失败（跳过，不阻断）: ${msg}`)
        failed++
      }
    }
  }
}

console.log(`\nGitee 同步完成: 成功 ${ok} | 跳过 ${skipped} | 失败 ${failed}`)
if (failed > 0) {
  console.warn('⚠ 有附件上传失败，可重跑本 job 补传（幂等），或到 Gitee 网页手动补传')
}
