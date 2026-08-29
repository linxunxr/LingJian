#!/usr/bin/env node
/**
 * 把 Gitee 版更新清单提交到 Gitee 仓库根目录的 latest.json。
 *
 * 为什么走仓库文件而非 Release 附件：Gitee 没有 GitHub 式
 * releases/latest/download/{file} 固定地址，updater 端点需要一个不随版本
 * 变化的 URL。Gitee 仓库 raw 地址（/raw/master/latest.json）永久稳定，
 * 302 到带短时签名的 CDN 后仍匿名可读，updater 跟随重定向即可。
 * 清单内的安装包 url 指向 Gitee Release 的 tag 固定直链（tag 由本次发版
 * 决定，写在清单里没有问题）。
 *
 * 幂等：文件已存在时走 PUT 更新（需带上一版 sha），重复执行结果一致。
 * 失败不阻断发布（GitHub/COS 才是权威源，Gitee 只是镜像）。
 *
 * 用法: node scripts/sync-latest-to-gitee.mjs <latest.gitee.json 路径>
 *
 * 环境变量：
 *   GITEE_TOKEN   必填，私人令牌（projects 权限）
 *   GITEE_OWNER   仓库归属，默认 mwcxlinxun
 *   GITEE_REPO    仓库名，默认 ling-jian
 *   GITEE_BRANCH  目标分支，默认 master
 */
import { readFileSync, existsSync } from 'node:fs'
import { exit } from 'node:process'

const GITEE_API = 'https://gitee.com/api/v5'
const OWNER = process.env.GITEE_OWNER || 'mwcxlinxun'
const REPO = process.env.GITEE_REPO || 'ling-jian'
const BRANCH = process.env.GITEE_BRANCH || 'master'
const FILE_PATH = 'latest.json'

const token = process.env.GITEE_TOKEN
const manifestPath = process.argv[2]

if (!manifestPath || !existsSync(manifestPath)) {
  console.error(`用法: node scripts/sync-latest-to-gitee.mjs <latest.gitee.json 路径>（文件不存在: ${manifestPath}）`)
  exit(1)
}
if (!token) {
  console.warn('⚠ 未设置 GITEE_TOKEN，跳过 Gitee 更新清单提交')
  exit(0)
}

async function gitee(path, init = {}) {
  const resp = await fetch(`${GITEE_API}${path}`, {
    ...init,
    headers: { authorization: `token ${token}`, ...(init.headers || {}) },
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

const content = readFileSync(manifestPath, 'utf-8')

// 幂等：先查现有文件拿 sha（存在则 PUT 更新，不存在则 POST 新建）
let sha = null
try {
  const existing = await gitee(`/repos/${OWNER}/${REPO}/contents/${FILE_PATH}?ref=${BRANCH}`)
  sha = existing?.sha || null
  console.log(`↻ Gitee 仓库已有 ${FILE_PATH}（sha=${String(sha).slice(0, 8)}...），将更新`)
} catch (e) {
  if (!e.message.includes('404')) throw e
  console.log(`+ Gitee 仓库尚无 ${FILE_PATH}，将新建`)
}

const payload = {
  access_token: token,
  content: Buffer.from(content, 'utf-8').toString('base64'),
  message: `chore(release): 更新 latest.json（自动更新清单，CI 发布时自动提交）`,
  branch: BRANCH,
  ...(sha ? { sha } : {}),
}

await gitee(`/repos/${OWNER}/${REPO}/contents/${FILE_PATH}`, {
  method: sha ? 'PUT' : 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify(payload),
})

console.log(`✓ 已提交 ${FILE_PATH} 到 Gitee ${OWNER}/${REPO}@${BRANCH}`)
console.log(`  updater 端点: https://gitee.com/${OWNER}/${REPO}/raw/${BRANCH}/${FILE_PATH}`)
