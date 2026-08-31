import { reactive, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'

import type { IssueListItem, IssueList } from '@/types'
import { settings } from './useSettings'

/** 排行榜单行（一名玩家的聚合结果） */
export interface LeaderboardEntry {
  playerId: string
  /** 玩家昵称（取该玩家最近一条携带昵称的上报；device 身份为空串） */
  playerName: string
  count: number
  /** 数组字段声明 readonly：readonly(state) 的 DeepReadonly 才能与本类型互相兼容 */
  issueNumbers: readonly number[]
  issueUrls: readonly string[]
  firstAt: string
  lastAt: string
  /** 参与过的客户端版本（去重，时间正序） */
  versions: readonly string[]
}

/** 聚合期间的内部可变形状（issueNumbers 等需要 push，聚合完成后赋给 entries） */
interface MutableEntry {
  playerId: string
  playerName: string
  count: number
  issueNumbers: number[]
  issueUrls: string[]
  firstAt: string
  lastAt: string
  versions: string[]
}

interface LeaderboardState {
  loading: boolean
  error: string | null
  entries: LeaderboardEntry[]
  /** 拉取到的全部反馈 Issue 数（含未标识） */
  totalIssues: number
  /** 无 playerId 的老 Issue 数（身份字段上线前的数据，无法归属到人） */
  unidentified: number
  /** 数据对应的最晚上报时间（ISO 8601），导出与展示标注用 */
  loadedAt: string | null
}

const state = reactive<LeaderboardState>({
  loading: false,
  error: null,
  entries: [],
  totalIssues: 0,
  unidentified: 0,
  loadedAt: null,
})

/** 拉全量分页上限（30 条/页 × 50 页 = 1500 条），防御 SCF hasMore 异常导致的死循环 */
const MAX_PAGES = 50

/** 循环拉取 state=all 的全部 Issue 列表（排行榜数据源，不依赖日志下载） */
async function fetchAllIssues(): Promise<IssueListItem[]> {
  const all: IssueListItem[] = []
  for (let page = 1; page <= MAX_PAGES; page++) {
    const result = await invoke<IssueList>('list_issues', {
      state: 'all',
      page,
      scfUrl: settings.scfUrl,
      apiKey: settings.apiKey,
    })
    if (Array.isArray(result.issues)) all.push(...result.issues)
    if (!result.hasMore) break
  }
  return all
}

/**
 * 加载并聚合排行榜。
 * 按 playerId 聚合（steam:SteamID64 发奖凭据 / device:UUID 兜底），
 * 无 playerId 的老 Issue 不进排行，单独计入 unidentified。
 */
export async function loadLeaderboard(): Promise<void> {
  if (!settings.scfUrl.trim() || !settings.apiKey.trim()) {
    state.error = '未配置 SCF 端点，请先到设置页填写'
    return
  }

  state.loading = true
  state.error = null

  try {
    const issues = await fetchAllIssues()
    const byPlayer = new Map<string, MutableEntry>()

    // 按创建时间正序聚合，保证 firstAt/versions 顺序稳定
    const sorted = [...issues].sort((a, b) => a.createdAt.localeCompare(b.createdAt))
    for (const it of sorted) {
      if (!it.playerId) continue
      let entry = byPlayer.get(it.playerId)
      if (!entry) {
        entry = {
          playerId: it.playerId,
          playerName: '',
          count: 0,
          issueNumbers: [],
          issueUrls: [],
          firstAt: it.createdAt,
          lastAt: it.createdAt,
          versions: [],
        }
        byPlayer.set(it.playerId, entry)
      }
      entry.count++
      entry.issueNumbers.push(it.number)
      entry.issueUrls.push(it.issueUrl)
      entry.lastAt = it.createdAt
      if (it.playerName) entry.playerName = it.playerName
      if (it.appVersion && !entry.versions.includes(it.appVersion)) entry.versions.push(it.appVersion)
    }

    // 反馈数优先，同数按最近活跃时间
    state.entries = [...byPlayer.values()]
      .sort((a, b) => {
        if (b.count !== a.count) return b.count - a.count
        return b.lastAt.localeCompare(a.lastAt)
      })
      .map(e => ({ ...e }))
    state.totalIssues = issues.length
    state.unidentified = issues.filter(i => !i.playerId).length
    state.loadedAt = issues.length > 0
      ? issues.reduce((max, i) => (i.createdAt > max ? i.createdAt : max), issues[0].createdAt)
      : null
  } catch (e) {
    state.error = typeof e === 'string' ? e : String(e)
  } finally {
    state.loading = false
  }
}

/** CSV 单元格转义：含逗号/引号/换行的值加引号包裹，内部引号翻倍 */
function csvCell(value: string): string {
  if (/[",\n]/.test(value)) return `"${value.replace(/"/g, '""')}"`
  return value
}

/** 生成排行榜 CSV 文本（带 BOM，Excel 打开中文不乱码） */
export function buildLeaderboardCsv(entries: readonly LeaderboardEntry[], unidentified: number): string {
  const lines = ['排名,玩家ID,昵称,反馈数,首次上报,最近上报,版本,Issue编号']
  entries.forEach((e, i) => {
    lines.push([
      String(i + 1),
      csvCell(e.playerId),
      csvCell(e.playerName),
      String(e.count),
      csvCell(e.firstAt),
      csvCell(e.lastAt),
      csvCell(e.versions.join(' / ')),
      csvCell(e.issueNumbers.map(n => `#${n}`).join(' ')),
    ].join(','))
  })
  lines.push(`,未标识（老数据）,,${unidentified},,,,`)
  return '\uFEFF' + lines.join('\r\n') + '\r\n'
}

/** 弹出保存对话框并导出排行榜 CSV，返回导出路径（用户取消返回 null） */
export async function exportLeaderboardCsv(
  entries: readonly LeaderboardEntry[],
  unidentified: number,
): Promise<string | null> {
  const filePath = await save({
    title: '导出反馈排行榜',
    defaultPath: '反馈排行榜.csv',
    filters: [{ name: 'CSV', extensions: ['csv'] }],
  })
  if (!filePath) return null

  const result = await invoke<{ path: string; bytes: number }>('save_text_file', {
    path: filePath,
    content: buildLeaderboardCsv(entries, unidentified),
  })
  return result.path
}

export function useLeaderboard() {
  return {
    state: readonly(state),
    loadLeaderboard,
    exportLeaderboardCsv,
  }
}
