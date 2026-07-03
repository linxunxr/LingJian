#!/usr/bin/env node
/**
 * 扫描已下载的 Release 产物，拼装 latest.json（Tauri updater 清单）。
 *
 * 背景：tauri-action 在多平台并行构建时，latest.json 会被各平台互相覆盖
 * 或干脆不上传，导致后续同步步骤找不到文件。本脚本完全自给自足——
 * 直接从 dist/ 中的产物（安装包 + 同名 .sig 签名）拼装清单，不再依赖
 * tauri-action 生成的 latest.json。
 *
 * 策略：扫描 dist/*.sig，每个 .sig 对应的安装包即为更新器产物
 *       （createUpdaterArtifacts 控制只有更新产物才生成 .sig），
 *       按文件名特征映射到 Tauri 平台 key，URL 直接拼成 COS 地址。
 *       同时从 CHANGELOG.md 提取对应版本日志写入 notes。
 *
 * 用法: node scripts/generate-latest-json.mjs <path/to/dist-dir> <version>
 *
 * 参数：
 *   <path/to/dist-dir>  gh release download 的产物目录（含安装包与 .sig）
 *   <version>           版本号，与 tag 一致，如 v0.1.0
 *
 * 产物：在 dist-dir 下生成 latest.json
 */
import { readFileSync, writeFileSync, readdirSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve, join, basename } from 'node:path'
import { exit } from 'node:process'

const __dirname = dirname(fileURLToPath(import.meta.url))
const CHANGELOG_PATH = resolve(__dirname, '..', 'CHANGELOG.md')
// 使用全球加速域名（CI 跨洲上传 + 用户下载均走加速接入点）
const COS_BASE = 'https://lingjian-releases-1433733625.cos.accelerate.myqcloud.com'

const distDir = process.argv[2]
const version = process.argv[3]

if (!distDir || !version) {
  console.error('用法: node scripts/generate-latest-json.mjs <path/to/dist-dir> <version>')
  exit(1)
}

// 归一化版本号：latest.json 的 version 字段不带 v 前缀（如 "0.1.0"）
const normalizedVersion = version.startsWith('v') ? version : `v${version}`
const bareVersion = normalizedVersion.replace(/^v/, '')

/**
 * 从文件名检测 Tauri 平台 key。
 * Tauri updater 的 platforms key 标准格式：<os>-<arch>
 *   windows-x86_64 / darwin-aarch64 / darwin-x86_64 / linux-x86_64
 */
function detectPlatform(filename) {
  const lower = filename.toLowerCase()
  let os, arch

  if (lower.endsWith('.exe') || lower.includes('setup.exe')) os = 'windows'
  else if (lower.endsWith('.app.tar.gz') || lower.endsWith('.dmg')) os = 'darwin'
  else if (lower.endsWith('.appimage')) os = 'linux'
  else return null

  if (lower.includes('aarch64') || lower.includes('arm64')) arch = 'aarch64'
  else if (lower.includes('x86_64') || lower.includes('x64')) arch = 'x86_64'
  else arch = 'x86_64' // 默认 x86_64

  // Windows 当前只构建 x86_64；Linux 同理
  if (os === 'windows') return 'windows-x86_64'
  if (os === 'linux') return 'linux-x86_64'
  return `${os}-${arch}`
}

/**
 * 从 CHANGELOG.md 提取指定版本段落正文（标题行之后到下一个 ## 或文件末尾）。
 */
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

  if (!inSection) return null
  return bodyLines.join('\n').trim()
}

// ---- 主流程 ----

// 1. 收集所有 .sig 文件，建立 platform → {signature, url} 映射
if (!existsSync(distDir)) {
  console.error(`✗ 产物目录不存在：${distDir}`)
  console.error('  请确认 gh release download 已执行并下载产物到该目录。')
  exit(1)
}
const files = readdirSync(distDir)
const sigFiles = files.filter((f) => f.endsWith('.sig'))

if (sigFiles.length === 0) {
  console.error(`✗ 在 ${distDir} 中未找到任何 .sig 签名文件`)
  console.error('  这通常意味着构建产物未正确生成或未上传到 Release。')
  exit(1)
}

const platforms = {}
let used = 0
for (const sigFile of sigFiles) {
  // 主文件 = 去掉 .sig 后缀
  const assetName = sigFile.replace(/\.sig$/, '')
  const platform = detectPlatform(assetName)
  if (!platform) {
    console.warn(`⚠ 跳过无法识别平台的产物：${assetName}`)
    continue
  }
  // 校验主文件确实存在
  const assetPath = join(distDir, assetName)
  if (!existsSync(assetPath)) {
    console.warn(`⚠ 找到 ${sigFile} 但对应产物 ${assetName} 不存在，跳过`)
    continue
  }
  if (platforms[platform]) {
    console.warn(`⚠ 平台 ${platform} 已存在（${platforms[platform].url}），覆盖为 ${assetName}`)
  }
  const signature = readFileSync(join(distDir, sigFile), 'utf-8').trim()
  platforms[platform] = {
    signature,
    // 版本分目录：产物按版本归档到 /v0.1.x/ 下，避免所有版本混在根目录
    url: `${COS_BASE}/${normalizedVersion}/${assetName}`,
  }
  console.log(`✓ ${platform} ← ${assetName}`)
  used++
}

if (used === 0) {
  console.error('✗ 未能识别出任何有效平台产物')
  exit(1)
}

// 2. 从 CHANGELOG 提取更新日志
let notes = `${normalizedVersion} 更新内容`
if (existsSync(CHANGELOG_PATH)) {
  const changelog = readFileSync(CHANGELOG_PATH, 'utf-8')
  const section = extractChangelogSection(changelog, normalizedVersion)
  if (section) {
    notes = section
    console.log(`✓ 已从 CHANGELOG.md 提取 ${normalizedVersion} 的更新日志`)
  } else {
    console.warn(`⚠ CHANGELOG.md 中未找到 ${normalizedVersion} 的段落，使用默认 notes`)
  }
} else {
  console.warn('⚠ 未找到 CHANGELOG.md，使用默认 notes')
}

// 3. 拼装并写入 latest.json
const manifest = {
  version: bareVersion,
  notes,
  pub_date: new Date().toISOString(),
  platforms,
}

const outPath = join(distDir, 'latest.json')
writeFileSync(outPath, JSON.stringify(manifest, null, 2) + '\n', 'utf-8')

console.log(`\n✓ 已生成 ${outPath}`)
console.log(`  版本: ${bareVersion} | 平台数: ${used}`)
console.log(`  platforms: ${Object.keys(platforms).join(', ')}`)
