import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))
// useSettings 依赖 Tauri plugin-store，mock 掉只保留 settings 对象本体
vi.mock('./useSettings', async importOriginal => {
  const mod = await importOriginal<typeof import('./useSettings')>()
  return { ...mod, settings: { scfUrl: 'http://scf.test', apiKey: 'key' } }
})

import { buildLeaderboardCsv, loadLeaderboard, useLeaderboard } from './useLeaderboard'
import type { IssueList, IssueListItem } from '@/types'

const mockedInvoke = vi.mocked(invoke)

function issue(partial: Partial<IssueListItem>): IssueListItem {
  return {
    number: 1,
    reportId: 'rp',
    title: '',
    state: 'closed',
    issueUrl: '',
    createdAt: '2026-08-01T00:00:00Z',
    owner: 'o',
    repo: 'r',
    ...partial,
  }
}

/** 配置 list_issues 分页响应序列（每次调用弹出一页） */
function mockPages(pages: IssueListItem[][]) {
  let call = 0
  mockedInvoke.mockImplementation(async () => {
    const issues = pages[Math.min(call, pages.length - 1)]
    call++
    return {
      issues,
      page: call,
      hasMore: call < pages.length,
    } satisfies IssueList
  })
}

beforeEach(() => {
  mockedInvoke.mockReset()
})

describe('loadLeaderboard 聚合', () => {
  it('按 playerId 聚合并按反馈数降序、同数按最近活跃排序', async () => {
    mockPages([
      [
        issue({ number: 10, playerId: 'steam:111', playerName: '甲', createdAt: '2026-08-01T00:00:00Z', appVersion: '0.10.14' }),
        issue({ number: 11, playerId: 'steam:111', playerName: '甲', createdAt: '2026-08-05T00:00:00Z', appVersion: '0.10.14' }),
        issue({ number: 12, playerId: 'steam:222', playerName: '乙', createdAt: '2026-08-03T00:00:00Z' }),
        issue({ number: 13, createdAt: '2026-08-04T00:00:00Z' }), // 无 playerId 的老数据
      ],
    ])

    await loadLeaderboard()
    const { state } = useLeaderboard()

    expect(state.entries).toHaveLength(2)
    expect(state.entries[0]).toMatchObject({
      playerId: 'steam:111',
      playerName: '甲',
      count: 2,
      issueNumbers: [10, 11],
      firstAt: '2026-08-01T00:00:00Z',
      lastAt: '2026-08-05T00:00:00Z',
      versions: ['0.10.14'],
    })
    expect(state.entries[1].playerId).toBe('steam:222')
    expect(state.totalIssues).toBe(4)
    expect(state.unidentified).toBe(1)
  })

  it('跨页拉取：hasMore=true 时继续请求下一页直到取完', async () => {
    mockPages([
      [issue({ number: 1, playerId: 'steam:111' })],
      [issue({ number: 2, playerId: 'steam:111' })],
    ])

    await loadLeaderboard()
    const { state } = useLeaderboard()

    expect(mockedInvoke).toHaveBeenCalledTimes(2)
    expect(state.entries[0].count).toBe(2)
    expect(state.totalIssues).toBe(2)
  })

  it('昵称取该玩家最近一条携带昵称的上报', async () => {
    mockPages([
      [
        issue({ number: 1, playerId: 'steam:111', createdAt: '2026-08-01T00:00:00Z' }),
        issue({ number: 2, playerId: 'steam:111', playerName: '新昵称', createdAt: '2026-08-02T00:00:00Z' }),
      ],
    ])

    await loadLeaderboard()
    expect(useLeaderboard().state.entries[0].playerName).toBe('新昵称')
  })
})

describe('buildLeaderboardCsv', () => {
  it('含表头、BOM、未标识尾行，含逗号的昵称加引号转义', () => {
    const csv = buildLeaderboardCsv(
      [
        {
          playerId: 'steam:111',
          playerName: '甲, 试炼',
          count: 2,
          issueNumbers: [10, 11],
          issueUrls: ['u1', 'u2'],
          firstAt: '2026-08-01T00:00:00Z',
          lastAt: '2026-08-05T00:00:00Z',
          versions: ['0.10.14'],
        },
      ],
      3,
    )

    expect(csv.charCodeAt(0)).toBe(0xfeff)
    const lines = csv.replace(/^\uFEFF/, '').trimEnd().split('\r\n')
    expect(lines[0]).toBe('排名,玩家ID,昵称,反馈数,首次上报,最近上报,版本,Issue编号')
    expect(lines[1]).toBe('1,steam:111,"甲, 试炼",2,2026-08-01T00:00:00Z,2026-08-05T00:00:00Z,0.10.14,#10 #11')
    // 尾行汇总未标识老数据，前四列足以表达意图，避免逐字符对齐空列
    expect(lines[2].startsWith(',未标识（老数据）,,3,')).toBe(true)
  })
})
