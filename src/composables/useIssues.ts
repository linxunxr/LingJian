import { reactive, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import type { IssueActionResponse, IssueList, IssueListItem } from '@/types'
import { settings } from './useSettings'

type IssueState = 'open' | 'closed' | 'all'

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
  /** 正在操作的 Issue 编号（防重复点击，禁用对应行按钮） */
  actingNumber: number | null
  /** 操作错误（独立于列表加载错误，便于菜单内提示） */
  actionError: string | null
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
  actingNumber: null,
  actionError: null,
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
    // 防御：SCF 返回异常结构时兜底为空数组，避免 v-for 遍历 undefined 崩溃
    state.issues = Array.isArray(result.issues) ? result.issues : []
    state.page = result.page ?? 1
    state.hasMore = !!result.hasMore
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
    if (Array.isArray(result.issues)) {
      state.issues.push(...result.issues)
    }
    state.page = result.page ?? nextPage
    state.hasMore = !!result.hasMore
  } catch (e) {
    state.error = typeof e === 'string' ? e : String(e)
  } finally {
    state.loadingMore = false
  }
}

/**
 * 对指定 Issue 执行操作（关闭/重开/评论/标签）。
 *
 * 成功后**乐观更新**本地列表对应项的 state/labels，不重拉整个列表。
 * 模块内直接改 state.issues（对外 readonly 不影响，因为 readonly 只约束外部引用）。
 *
 * @param number Issue 编号
 * @param action close / reopen / comment / setLabels
 * @param opts.body 评论内容（action=comment 时）
 * @param opts.labels 标签数组（action=setLabels 时，整体替换）
 */
export async function actOnIssue(
  number: number,
  action: 'close' | 'reopen' | 'comment' | 'setLabels',
  opts?: { body?: string; labels?: string[] },
): Promise<boolean> {
  state.actionError = null
  state.actingNumber = number

  try {
    const result = await invoke<IssueActionResponse>('act_on_issue', {
      number,
      action,
      body: opts?.body,
      labels: opts?.labels,
      scfUrl: settings.scfUrl,
      apiKey: settings.apiKey,
    })

    // 乐观更新本地列表项
    const item = state.issues.find(i => i.number === number)
    if (item) {
      if (typeof result.state === 'string') item.state = result.state
      if (Array.isArray(result.labels)) item.labels = result.labels
    }
    return true
  } catch (e) {
    state.actionError = typeof e === 'string' ? e : String(e)
    return false
  } finally {
    state.actingNumber = null
  }
}

/** 清除操作错误提示 */
export function clearActionError() {
  state.actionError = null
}

export function useIssues() {
  return {
    state: readonly(state),
    loadIssues,
    switchState,
    loadMore,
    actOnIssue,
    clearActionError,
  }
}
