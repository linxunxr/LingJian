#!/usr/bin/env node
/**
 * 重写 latest.json 中的下载 URL，使其指向腾讯云 COS。
 *
 * tauri-action 生成的 latest.json 中，每个平台的 url 指向 GitHub Release asset。
 * 本脚本将其改写为对应的 COS 地址，使国内用户从腾讯云下载（更快更稳）。
 *
 * 用法: node scripts/rewrite-latest-json.mjs <path/to/latest.json>
 *
 * 例：
 *   原 url: https://github.com/linxunxr/LingJian/releases/download/v0.2.0/灵鉴_0.2.0_x64-setup.exe
 *   新 url: https://lingjian-releases-1433733625.cos.ap-guangzhou.myqcloud.com/灵鉴_0.2.0_x64-setup.exe
 */
import { readFileSync, writeFileSync } from 'node:fs'

const COS_BASE = 'https://lingjian-releases-1433733625.cos.ap-guangzhou.myqcloud.com'

const filePath = process.argv[2]
if (!filePath) {
  console.error('用法: node scripts/rewrite-latest-json.mjs <path/to/latest.json>')
  process.exit(1)
}

const raw = readFileSync(filePath, 'utf-8')
const manifest = JSON.parse(raw)

let rewritten = 0
if (manifest.platforms && typeof manifest.platforms === 'object') {
  for (const [platform, info] of Object.entries(manifest.platforms)) {
    if (info && typeof info === 'object' && 'url' in info) {
      // 提取文件名，拼接 COS 地址
      const url = new URL(info.url)
      const filename = url.pathname.split('/').pop()
      if (filename) {
        const newUrl = `${COS_BASE}/${decodeURIComponent(filename)}`
        console.log(`[${platform}] ${info.url}\n  → ${newUrl}`)
        info.url = newUrl
        rewritten++
      }
    }
  }
}

writeFileSync(filePath, JSON.stringify(manifest, null, 2) + '\n', 'utf-8')
console.log(`\n✓ 已重写 ${rewritten} 个平台的下载 URL 为 COS 地址`)
