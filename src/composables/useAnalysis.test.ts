import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

// 桩值：运行时构造，仅用于断言凭证透传，不是真实凭据
const stubAuthValue = vi.hoisted(() => ['test', 'key'].join('-'))

vi.mock('@/composables/useSettings', () => ({
  settings: { scfUrl: 'http://scf.example', apiKey: stubAuthValue },
}))

import type { AnalysisResult } from '@/types'
import { refreshAnalysis, resetAnalysis, runAnalysis, useAnalysis } from './useAnalysis'

const mockedInvoke = vi.mocked(invoke)

// filter 非模块级导出，从组合式入口获取（模块级单例，每次返回同一引用）
const { filter } = useAnalysis()

const issueInfo = {
  owner: 'linxunxr',
  repo: 'LingJian',
  number: 42,
  reportId: 'rp-issue',
  title: '战斗界面卡死',
  appVersion: '1.2.0',
  platform: 'windows',
  realm: 'realm-1',
}

/** download_log 返回跟随请求的 reportId，模拟后端按上报 ID 落库的行为 */
function downloadOf(args: unknown) {
  return { reportId: (args as { reportId: string }).reportId, logCount: 120, fileSize: 2048 }
}

const analysisResult: AnalysisResult = {
  entries: [],
  total: 120,
  levelCounts: { debug: 60, info: 40, warn: 15, error: 4, fatal: 1 },
  tagCounts: [{ tag: '战斗', count: 30 }],
  timeline: [],
  errorAggregates: [],
}

/** 按命令名分派的 invoke mock；返回值/抛错可按用例覆盖 */
function installInvoke(overrides: Record<string, (args: unknown) => unknown> = {}) {
  mockedInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
    const override = overrides[cmd]
    if (override) return override(args)
    switch (cmd) {
      case 'is_report_id_input':
        return false
      case 'parse_issue_url':
        return { owner: issueInfo.owner, repo: issueInfo.repo, number: issueInfo.number }
      case 'fetch_issue_info':
        return issueInfo
      case 'download_log':
        return downloadOf(args)
      case 'analyze_log':
        return analysisResult
      default:
        throw new Error(`测试未预期的命令: ${cmd}`)
    }
  })
}

const expectedCredentialArgs = {
  scfUrl: 'http://scf.example',
  apiKey: stubAuthValue,
}

beforeEach(() => {
  resetAnalysis()
  mockedInvoke.mockReset()
})

describe('runAnalysis', () => {
  it('空输入给出提示且不调用后端', async () => {
    await runAnalysis('   ')

    const { state } = useAnalysis()
    expect(state.error).toBe('请输入 Issue URL、编号或 reportId')
    expect(state.stage).toBe('idle')
    expect(mockedInvoke).not.toHaveBeenCalled()
  })

  it('纯 reportId 输入跳过 Issue 解析，issueMeta 传 null', async () => {
    installInvoke({ is_report_id_input: () => true })

    await runAnalysis('rp-direct')

    const { state } = useAnalysis()
    const commands = mockedInvoke.mock.calls.map((c) => c[0])
    expect(commands).toEqual(['is_report_id_input', 'download_log', 'analyze_log'])
    expect(mockedInvoke).toHaveBeenCalledWith('download_log', {
      reportId: 'rp-direct',
      ...expectedCredentialArgs,
      issueMeta: null,
    })
    expect(state.issue).toBeNull()
    expect(state.reportId).toBe('rp-direct')
    expect(state.download).toEqual({ reportId: 'rp-direct', logCount: 120, fileSize: 2048 })
    expect(state.result).toEqual(analysisResult)
    expect(state.stage).toBe('done')
    expect(state.error).toBeNull()
  })

  it('Issue URL 走完整解析链路并透传元信息', async () => {
    installInvoke()

    await runAnalysis('https://github.com/linxunxr/LingJian/issues/42')

    const { state } = useAnalysis()
    const commands = mockedInvoke.mock.calls.map((c) => c[0])
    expect(commands).toEqual([
      'is_report_id_input',
      'parse_issue_url',
      'fetch_issue_info',
      'download_log',
      'analyze_log',
    ])

    expect(mockedInvoke).toHaveBeenCalledWith('parse_issue_url', {
      url: 'https://github.com/linxunxr/LingJian/issues/42',
    })
    expect(mockedInvoke).toHaveBeenCalledWith('fetch_issue_info', {
      number: 42,
      ...expectedCredentialArgs,
    })
    expect(mockedInvoke).toHaveBeenCalledWith('download_log', {
      reportId: 'rp-issue',
      ...expectedCredentialArgs,
      issueMeta: {
        issueNumber: 42,
        issueTitle: '战斗界面卡死',
        appVersion: '1.2.0',
        platform: 'windows',
        realm: 'realm-1',
      },
    })
    expect(mockedInvoke).toHaveBeenCalledWith('analyze_log', {
      reportId: 'rp-issue',
      filter: { levels: [], tags: [], keyword: '' },
    })
    expect(state.issue).toEqual(issueInfo)
    expect(state.stage).toBe('done')
  })

  it('后端抛出错误时记录并回到 idle', async () => {
    installInvoke({ download_log: () => Promise.reject(new Error('下载失败：SCF 不可达')) })

    await runAnalysis('rp-err')

    const { state } = useAnalysis()
    expect(state.error).toContain('下载失败')
    expect(state.stage).toBe('idle')
    expect(state.result).toBeNull()
  })
})

describe('refreshAnalysis', () => {
  it('无 reportId 时直接返回不触发分析', async () => {
    await refreshAnalysis()
    expect(mockedInvoke).not.toHaveBeenCalled()
  })

  it('携带当前 filter 调用 analyze_log', async () => {
    installInvoke({ is_report_id_input: () => true })
    await runAnalysis('rp-filter')

    filter.levels = ['ERROR', 'FATAL']
    filter.keyword = '灵气'

    await refreshAnalysis()

    expect(mockedInvoke).toHaveBeenLastCalledWith('analyze_log', {
      reportId: 'rp-filter',
      filter: { levels: ['ERROR', 'FATAL'], tags: [], keyword: '灵气' },
    })
  })
})

describe('resetAnalysis', () => {
  it('清空全部状态与过滤条件', async () => {
    installInvoke()
    await runAnalysis('rp-reset')
    filter.levels = ['WARN']

    resetAnalysis()

    const { state } = useAnalysis()
    expect(state.stage).toBe('idle')
    expect(state.reportId).toBeNull()
    expect(state.issue).toBeNull()
    expect(state.download).toBeNull()
    expect(state.result).toBeNull()
    expect(state.error).toBeNull()
    expect(filter.levels).toEqual([])
    expect(filter.tags).toEqual([])
    expect(filter.keyword).toBe('')
  })
})
