import { reactive, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import type {
  AnalysisResult,
  DownloadResult,
  IssueInfo,
  LogFilter,
  Report,
} from '@/types'
import { settings } from './useSettings'

type Stage = 'idle' | 'parsing' | 'downloading' | 'analyzing' | 'done'

interface AnalysisState {
  stage: Stage
  /** 当前正在处理的 reportId */
  reportId: string | null
  /** 解析出的 Issue 信息 */
  issue: IssueInfo | null
  /** 下载结果 */
  download: DownloadResult | null
  /** 分析结果 */
  result: AnalysisResult | null
  /** 错误信息 */
  error: string | null
}

const state = reactive<AnalysisState>({
  stage: 'idle',
  reportId: null,
  issue: null,
  download: null,
  result: null,
  error: null,
})

/** 当前过滤条件（供 AnalyzeView 双向绑定） */
const filter = reactive<LogFilter>({
  levels: [],
  tags: [],
  keyword: '',
})

/**
 * 执行完整分析流程：输入 → 解析 Issue → 下载日志 → 分析
 *
 * @param input Issue URL / `#编号` / 编号 / reportId
 */
export async function runAnalysis(input: string): Promise<void> {
  const trimmed = input.trim()
  if (!trimmed) {
    state.error = '请输入 Issue URL、编号或 reportId'
    return
  }

  state.error = null
  state.stage = 'parsing'
  state.issue = null
  state.download = null
  state.result = null

  try {
    // 1. 确定 reportId
    let reportId: string
    let issueInfo: IssueInfo | null = null
    const isUuid = await invoke<boolean>('is_report_id_input', { input: trimmed })

    if (isUuid) {
      // 纯 reportId 输入，跳过 Issue 解析
      reportId = trimmed
    } else {
      const parsed = await invoke<{ owner: string; repo: string; number: number }>(
        'parse_issue_url',
        { url: trimmed },
      )
      const info = await invoke<IssueInfo>('fetch_issue_info', {
        number: parsed.number,
        scfUrl: settings.scfUrl,
        apiKey: settings.apiKey,
      })
      state.issue = info
      issueInfo = info
      reportId = info.reportId
    }
    state.reportId = reportId

    // 2. 下载（透传 Issue 元信息，存在则一并落库）
    state.stage = 'downloading'
    const downloaded = await invoke<DownloadResult>('download_log', {
      reportId,
      scfUrl: settings.scfUrl,
      apiKey: settings.apiKey,
      issueMeta: issueInfo
        ? {
            issueNumber: issueInfo.number,
            issueTitle: issueInfo.title,
            appVersion: issueInfo.appVersion,
            platform: issueInfo.platform,
            realm: issueInfo.realm,
          }
        : null,
    })
    state.download = downloaded

    // 3. 分析
    state.stage = 'analyzing'
    await refreshAnalysis()

    state.stage = 'done'
  } catch (e) {
    state.error = typeof e === 'string' ? e : String(e)
    state.stage = 'idle'
  }
}

/** 用当前 filter 重新分析（过滤变更时调用） */
export async function refreshAnalysis(): Promise<void> {
  if (!state.reportId) return
  state.result = await invoke<AnalysisResult>('analyze_log', {
    reportId: state.reportId,
    filter,
  })
}

/** 重置状态回到初始 */
export function resetAnalysis(): void {
  state.stage = 'idle'
  state.reportId = null
  state.issue = null
  state.download = null
  state.result = null
  state.error = null
  filter.levels = []
  filter.tags = []
  filter.keyword = ''
}

export function useAnalysis() {
  return {
    state: readonly(state),
    filter,
    runAnalysis,
    refreshAnalysis,
    resetAnalysis,
  }
}

export type { Report }
