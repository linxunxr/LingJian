import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
// useSettings 依赖 Tauri plugin-store，mock 掉只保留 settings 对象本体
vi.mock('./useSettings', async importOriginal => {
  const mod = await importOriginal<typeof import('./useSettings')>()
  return { ...mod, settings: { scfUrl: 'http://scf.test', apiKey: 'key' } }
})

import { actOnIssue, loadIssues, refreshCounts, resetIssuesState, switchState, useIssues } from './useIssues'
import type { IssueList, IssueListItem } from '@/types'

const mockedInvoke = vi.mocked(invoke)

function issue(partial: Partial<IssueListItem>): IssueListItem {
  return {
    number: 1,
    reportId: 'rp',
    title: '问题',
    state: 'open',
    issueUrl: '',
    createdAt: '2026-08-01T00:00:00Z',
    owner: 'o',
    repo: 'r',
    ...partial,
  }
}

/** 组装一页 IssueList 响应 */
function page(issues: IssueListItem[], extra?: Partial<IssueList>): IssueList {
  return { issues, page: 1, hasMore: false, ...extra }
}

beforeEach(() => {
  mockedInvoke.mockReset()
  // 模块级单例状态跨用例残留，重置避免相互污染（readonly 包装外直接清）
  resetIssuesState()
})

describe('useIssues 缓存分支', () => {
  it('默认缓存优先：透传 refresh=false 并记录 fromCache/cachedAt', async () => {
    mockedInvoke.mockResolvedValue(
      page([issue({ number: 44 })], { fromCache: true, cachedAt: '2026-09-06T03:00:00Z' }),
    )

    await loadIssues()

    expect(mockedInvoke).toHaveBeenCalledWith(
      'list_issues', expect.objectContaining({ refresh: false, page: 1 }),
    )
    const { state } = useIssues()
    expect(state.fromCache).toBe(true)
    expect(state.cachedAt).toBe('2026-09-06T03:00:00Z')
    expect(state.issues.map(i => i.number)).toEqual([44])
  })

  it('refresh=true 强刷时把 refresh 标志传给命令层', async () => {
    mockedInvoke.mockResolvedValue(page([issue({ number: 45 })], { fromCache: false }))

    await loadIssues({ refresh: true })

    expect(mockedInvoke).toHaveBeenCalledWith(
      'list_issues', expect.objectContaining({ refresh: true, page: 1 }),
    )
    const { state } = useIssues()
    expect(state.fromCache).toBe(false)
    expect(state.cachedAt).toBeNull()
  })

  it('切 tab 强制回源（refresh=true）并重置到第 1 页', async () => {
    mockedInvoke.mockResolvedValue(page([]))

    await switchState('all')

    expect(mockedInvoke).toHaveBeenCalledWith(
      'list_issues', expect.objectContaining({ refresh: true, state: 'all', page: 1 }),
    )
  })
})

describe('useIssues 计数徽标', () => {
  it('loadIssues 后并行拉取各状态计数', async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'issue_counts') return { open: 3, closed: 5, all: 8 }
      return page([issue({ number: 44 })])
    })

    await loadIssues()
    // refreshCounts 是 fire-and-forget，等一个微任务周期
    await Promise.resolve()

    expect(mockedInvoke).toHaveBeenCalledWith('issue_counts')
    expect(useIssues().state.counts).toEqual({ open: 3, closed: 5, all: 8 })
  })

  it('计数拉取失败静默：不影响列表加载，counts 保持 null', async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'issue_counts') throw new Error('db busy')
      return page([issue({ number: 44 })])
    })

    await loadIssues()
    // fire-and-forget 的 refreshCounts 走微任务链，等几个周期确保 catch 已执行
    await new Promise(r => setTimeout(r, 20))

    const { state } = useIssues()
    expect(state.error).toBeNull()
    expect(state.counts).toBeNull()
  })

  it('actOnIssue 关闭/重开成功后刷新计数', async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'issue_counts') return { open: 2, closed: 6, all: 8 }
      if (cmd === 'act_on_issue') return { ok: true, state: 'closed', labels: [] }
      return page([issue({ number: 44 })])
    })
    // 预置一条 open 列表项供乐观更新
    await loadIssues()

    const ok = await actOnIssue(44, 'close')
    await vi.waitFor(() => {
      expect(useIssues().state.counts).toEqual({ open: 2, closed: 6, all: 8 })
    })
    expect(ok).toBe(true)
  })

  it('refreshCounts 可单独调用并写入 counts', async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'issue_counts') return { open: 1, closed: 2, all: 3 }
      return page([])
    })

    await refreshCounts()

    expect(mockedInvoke).toHaveBeenCalledWith('issue_counts')
    expect(useIssues().state.counts).toEqual({ open: 1, closed: 2, all: 3 })
  })
})
