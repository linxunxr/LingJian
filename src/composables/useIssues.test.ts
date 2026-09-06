import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
// useSettings 依赖 Tauri plugin-store，mock 掉只保留 settings 对象本体
vi.mock('./useSettings', async importOriginal => {
  const mod = await importOriginal<typeof import('./useSettings')>()
  return { ...mod, settings: { scfUrl: 'http://scf.test', apiKey: 'key' } }
})

import { loadIssues, switchState, useIssues } from './useIssues'
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
