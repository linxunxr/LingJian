#!/usr/bin/env node
/**
 * 校验 Gitee 发行版附件是否已补齐（发布链路「降级/升级」清单的开关）。
 *
 * 背景（v0.5.1 发布事故）：sync-to-gitee 传附件挂死/部分失败时，已提交的
 * Gitee-URL 清单指向不存在的附件，updater 下载 404 直接失败（tauri-plugin-updater
 * 的多端点回退只发生在拉清单阶段，下载失败不回退）。因此清单提交策略改为
 * 「先降级后升级」：先提交 GitHub-URL 清单保底，本脚本确认附件齐全后才允许
 * 升级为 Gitee-URL 清单。
 *
 * 预期附件集合与 sync-release-to-gitee.mjs 的收集逻辑保持一致（同一正则、
 * 同一排除项），避免两处口径漂移。
 *
 * 用法: node scripts/verify-gitee-assets.mjs <version> <dist-dir>
 *   附件齐全 exit 0；有缺失/发行版不存在 exit 1（打印缺失清单）。
 *   API 查询失败 exit 2（区别于"确认缺附件"的失败）。
 *
 * 环境变量：
 *   GITEE_TOKEN   可选，带 token 查询避开匿名限流
 *   GITEE_OWNER   仓库归属，默认 mwcxlinxun
 *   GITEE_REPO    仓库名，默认 ling-jian
 */
import { readdirSync, existsSync, statSync } from 'node:fs'
import { join } from 'node:path'

const GITEE_API = 'https://gitee.com/api/v5'
const OWNER = process.env.GITEE_OWNER || 'mwcxlinxun'
const REPO = process.env.GITEE_REPO || 'ling-jian'

const token = process.env.GITEE_TOKEN
const version = process.argv[2]?.replace(/^v/, '')
const distDir = process.argv[3]

async function main() {
  if (!version || !distDir || !existsSync(distDir)) {
    console.error('用法: node scripts/verify-gitee-assets.mjs <version> <dist-dir>')
    return 2
  }

  // 与 sync-release-to-gitee.mjs 相同的产物收集口径（含 latest.json，排除 latest.github.json）
  const expected = readdirSync(distDir)
    .filter((f) => {
      if (!/\.(exe|dmg|appimage|deb|rpm|AppImage|tar\.gz|json|sig)$/.test(f)) return false
      if (f === 'latest.github.json') return false
      return statSync(join(distDir, f)).isFile()
    })
    .sort()

  if (expected.length === 0) {
    console.error(`✗ ${distDir} 中未收集到任何预期产物，无法校验`)
    return 2
  }

  const resp = await fetch(`${GITEE_API}/repos/${OWNER}/${REPO}/releases/tags/v${version}`, {
    headers: token ? { authorization: `token ${token}` } : {},
    signal: AbortSignal.timeout(30_000),
  })
  if (resp.status === 404) {
    console.error(`✗ Gitee 发行版 v${version} 不存在（附件 0/${expected.length}）`)
    return 1
  }
  if (!resp.ok) {
    console.error(`✗ Gitee API ${resp.status}，无法校验附件`)
    return 2
  }
  const release = await resp.json()
  // Gitee 自动为发行版附带源码包（vX.Y.Z.zip / .tar.gz），不在预期集合内，天然被差集排除
  const existing = new Set((release.attach_files || release.assets || []).map((a) => a.name))
  const missing = expected.filter((f) => !existing.has(f))

  if (missing.length === 0) {
    console.log(`✓ Gitee 发行版 v${version} 附件齐全（${expected.length}/${expected.length}），可升级清单为 Gitee 源`)
    return 0
  }
  console.error(`⚠ 附件未齐（${expected.length - missing.length}/${expected.length}），缺失 ${missing.length} 个：`)
  for (const f of missing) console.error(`  - ${f}`)
  return 1
}

// 设 exitCode 而非 process.exit：Windows Node 24 上 exit() 会与活跃的
// fetch keep-alive 连接冲突触发 libuv 断言（exit=127 淹没真实退出码）
main()
  .then((code) => {
    process.exitCode = code
  })
  .catch((e) => {
    console.error(`✗ 校验异常: ${e.message}`)
    process.exitCode = 2
  })
