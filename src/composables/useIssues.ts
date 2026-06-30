import { reactive, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import type { IssueList, IssueListItem } from '@/types'
import { settings } from './useSettings'

type IssueState = 'open' | 'all'

interface IssuesState {
  /** 当前列表（跨页累积） */
  issues: IssueListItem[]
  loading: boolean
  loadingMore: boolean
  error: string | null
  /** 当前状态筛选 tab */
  state: IssueState
  /** 当前已加载页码 */
  page: number
  /** 是否还有更多 */
  hasMore: boolean
  /** 是否已初始化（避免重复首拉） */
  loaded: boolean
}

const state = reactive<IssuesState>({
  issues: [],
  loading: false,
  loadingMore: false,
  error: null,
  state: 'open',
  page: 0,
  hasMore: false,
  loaded: false,
})

/** 重置列表到初始状态（切换 tab / 手动刷新时用） */
function resetList() {
  state.issues = []
  state.page = 0
  state.hasMore = false
  state.loaded = false
}

/**
 * 拉取第一页（或重新拉取）。
 * 配置不完整时静默跳过，不报错（首页会显示配置提示）。
 */
export async function loadIssues(): Promise<void> {
  // 配置不完整：静默跳过（首页已有配置提示横幅）
  if (!settings.scfUrl.trim() || !settings.apiKey.trim()) {
    return
  }

  resetList()
  state.loading = true
  state.error = null

  try {
    const result = await invoke<IssueList>('list_issues', {
      state: state.state,
      page: 1,
      scfUrl: settings.scfUrl,
      apiKey: settings.apiKey,
    })
    state.issues = result.issues
    state.page = result.page
    state.hasMore = result.hasMore
    state.loaded = true
  } catch (e) {
    state.error = typeof e === 'string' ? e : String(e)
  } finally {
    state.loading = false
  }
}

/** 切换状态 tab（未处理 / 全部），切换后重新从第 1 页拉 */
export async function switchState(s: IssueState): Promise<void> {
  if (s === state.state) return
  state.state = s
  await loadIssues()
}

/** 加载下一页（追加到列表末尾） */
export async function loadMore(): Promise<void> {
  if (!state.hasMore || state.loadingMore) return

  state.loadingMore = true
  state.error = null

  try {
    const nextPage = state.page + 1
    const result = await invoke<IssueList>('list_issues', {
      state: state.state,
      page: nextPage,
      scfUrl: settings.scfUrl,
      apiKey: settings.apiKey,
    })
    state.issues.push(...result.issues)
    state.page = result.page
    state.hasMore = result.hasMore
  } catch (e) {
    state.error = typeof e === 'string' ? e : String(e)
  } finally {
    state.loadingMore = false
  }
}

export function useIssues() {
  return {
    state: readonly(state),
    loadIssues,
    switchState,
    loadMore,
  }
}
