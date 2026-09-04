<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

import LogFilter from '@/components/LogFilter.vue'
import LogTable from '@/components/LogTable.vue'
import LogDetail from '@/components/LogDetail.vue'
import Timeline from '@/components/Timeline.vue'
import ErrorAggregates from '@/components/ErrorAggregates.vue'
import CommentDialog from '@/components/CommentDialog.vue'
import LabelDialog from '@/components/LabelDialog.vue'
import ReportMetaCard from '@/components/ReportMetaCard.vue'
import { useAnalysis } from '@/composables/useAnalysis'
import { useIssues } from '@/composables/useIssues'
import { settings } from '@/composables/useSettings'
import { exportReport, type ExportFormat } from '@/composables/useExport'
import { formatBytes } from '@/utils/format'
import type { AnalysisResult, IssueInfo, LogEntry, Report, ReportMeta } from '@/types'

const route = useRoute()
const router = useRouter()
const { state, filter, refreshAnalysis, resetAnalysis } = useAnalysis()
const { state: issuesState, actOnIssue, clearActionError } = useIssues()

const selected = ref<LogEntry | null>(null)
const standaloneResult = ref<AnalysisResult | null>(null)
const standaloneReport = ref<Report | null>(null)
const loadingStandalone = ref(false)
const standaloneError = ref<string | null>(null)
const exportBusy = ref<ExportFormat | null>(null)
const exportMessage = ref<string | null>(null)

// ---- Issue 操作（从首页最近分析进入时，issue 号由路由 query 带入） ----

/** 当前 report 对应的 issue 号（无则不可操作） */
const issueNumber = computed(() => {
  const q = route.query.issue
  const n = q ? Number(q) : NaN
  return Number.isFinite(n) && n > 0 ? n : null
})

/** 当前 issue 的真实信息（进入详情页时调 fetch_issue_info 拿到，含 state/labels） */
const issueInfo = ref<IssueInfo | null>(null)

/** 当前 issue 状态：优先用 fetch_issue_info 返回的真实值，未知时默认 open */
const currentIssueState = computed(() => issueInfo.value?.state || 'open')

/** 当前 issue 标签：fetch_issue_info 返回的真实值 */
const currentLabels = computed<string[]>(() => issueInfo.value?.labels ?? [])

const openMenu = ref(false)

/** 评论/标签对话框（内容与提交逻辑在对话框组件内） */
const commentOpen = ref(false)
const labelOpen = ref(false)

/** 加载 issue 真实信息（state/labels），用于操作按钮文案与标签回填 */
async function loadIssueInfo(number: number) {
  // 配置不完整时静默跳过（与 loadIssues 一致的降级策略）
  if (!settings.scfUrl.trim() || !settings.apiKey.trim()) return
  try {
    issueInfo.value = await invoke<IssueInfo>('fetch_issue_info', {
      number,
      scfUrl: settings.scfUrl,
      apiKey: settings.apiKey,
    })
  } catch (e) {
    // 拉取失败不阻断分析（仅影响操作按钮文案，回退为默认 open）
    console.warn('[loadIssueInfo] 失败:', e)
    issueInfo.value = null
  }
}

/** 关闭/重开 */
async function onToggleState(number: number) {
  openMenu.value = false
  const action = currentIssueState.value === 'closed' ? 'reopen' : 'close'
  const ok = await actOnIssue(number, action)
  // 乐观更新本地 issueInfo.state（actOnIssue 也已更新首页列表项）
  if (ok && issueInfo.value) {
    issueInfo.value.state = action === 'close' ? 'closed' : 'open'
  }
}

function openComment() {
  openMenu.value = false
  commentOpen.value = true
}

function openLabels() {
  openMenu.value = false
  labelOpen.value = true
}

/** 标签保存成功后乐观更新本地 issueInfo.labels */
function onLabelsSaved(labels: string[]) {
  if (issueInfo.value) issueInfo.value.labels = labels
}

/** 点页面其他位置关闭菜单 */
function onDocClick(e: MouseEvent) {
  if (!openMenu.value) return
  const target = e.target as HTMLElement
  if (!target.closest('.issue-actions')) openMenu.value = false
}

/** 当前 reportId（来自分析流程或单独加载） */
const currentReportId = computed(() => state.reportId ?? (route.query.id as string | undefined) ?? null)

/** 实际展示的分析结果（来自 useAnalysis 流程 或 单独加载的 report） */
const result = computed(() => state.result ?? standaloneResult.value)

/** 上报上下文卡片：优先 Issue 信息（完整流程或详情页拉取），本地导入回退落库 Report */
const reportMeta = computed<ReportMeta | null>(() => {
  const info = state.issue ?? issueInfo.value
  if (info) {
    return {
      title: info.title || null,
      userDescription: info.userDescription ?? null,
      appName: null,
      appVersion: info.appVersion ?? null,
      platform: info.platform ?? null,
      realm: info.realm ?? null,
      playTime: info.playTime != null ? Number(info.playTime) : null,
      reportTime: null,
      // state.issue 经 readonly() 代理，数组需拷贝脱离只读类型
      screenshotKeys: info.screenshotKeys ? [...info.screenshotKeys] : null,
    }
  }
  const r = standaloneReport.value
  if (!r) return null
  return {
    title: r.issueTitle ?? null,
    userDescription: r.userDescription ?? null,
    appName: r.appName ?? null,
    appVersion: r.appVersion ?? null,
    platform: r.platform ?? null,
    realm: r.realm ?? null,
    playTime: r.playTime ?? null,
    reportTime: r.reportTime,
    screenshotKeys: r.screenshotKeys ?? null,
  }
})

/** 反馈截图（data URL 列表，按 screenshotKeys 顺序） */
const screenshotUrls = ref<string[]>([])

/** 截图拉取序号：连续切换 report 时丢弃旧异步循环的过期结果 */
let screenshotSeq = 0

/** 按需拉取反馈截图：本地缓存命中不发网络请求，单张失败跳过该张 */
async function loadScreenshots(keys: string[]) {
  const seq = ++screenshotSeq
  screenshotUrls.value = []
  if (!keys.length) return
  // 配置不完整时静默跳过（与 loadIssueInfo 一致的降级策略）
  if (!settings.scfUrl.trim() || !settings.apiKey.trim()) return

  const results: string[] = []
  for (const key of keys) {
    if (seq !== screenshotSeq) return
    try {
      const shot = await invoke<{ key: string; dataUrl: string; cached: boolean }>(
        'fetch_screenshot',
        { key, scfUrl: settings.scfUrl, apiKey: settings.apiKey },
      )
      results.push(shot.dataUrl)
    } catch (e) {
      // 单张失败不阻断其余（常见于 SCF 端点未更新到截图版本）
      console.warn('[loadScreenshots] 拉取失败:', key, e)
    }
  }
  if (seq === screenshotSeq) screenshotUrls.value = results
}

watch(
  () => reportMeta.value?.screenshotKeys,
  (keys) => {
    if (keys && keys.length) loadScreenshots(keys)
    else screenshotUrls.value = []
  },
)

/** 详情面板空态概览（当前匹配条数 / 总量 / 首末时间） */
const entrySummary = computed(() => {
  const r = result.value
  if (!r || r.entries.length === 0) return null
  return {
    shown: r.entries.length,
    total: r.total,
    first: r.entries[0].timestamp,
    last: r.entries[r.entries.length - 1].timestamp,
  }
})

/** 监听 filter 变化重新分析（防抖：500ms） */
let debounceTimer: ReturnType<typeof setTimeout> | null = null
watch(
  () => [filter.levels.slice(), filter.tags.slice(), filter.keyword] as const,
  () => {
    if (!state.reportId) return
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => refreshAnalysis(), 500)
  },
  { deep: true },
)

/** 同页换 id 时重新加载（如启用 keep-alive 缓存或直接改 URL 场景） */
watch(
  () => route.query.id,
  (id) => {
    if (typeof id === 'string' && id) {
      loadReport(id)
    }
    // 重置 Issue 操作的本地状态（避免残留上一份报告的菜单/对话框/状态）
    openMenu.value = false
    commentOpen.value = false
    labelOpen.value = false
    issueInfo.value = null
  },
)

/** issue 号变化时重新拉取 issue 真实信息 */
watch(issueNumber, (n) => {
  if (n) loadIssueInfo(n)
  else issueInfo.value = null
})

/** 单独加载某 report 的分析（从首页最近列表点进来），同时取落库元信息供上报卡片展示 */
async function loadReport(reportId: string) {
  loadingStandalone.value = true
  standaloneError.value = null
  try {
    const [analysis, report] = await Promise.all([
      invoke<AnalysisResult>('analyze_log', { reportId, filter }),
      invoke<Report | null>('get_report', { reportId }),
    ])
    standaloneResult.value = analysis
    standaloneReport.value = report
  } catch (e) {
    standaloneError.value = typeof e === 'string' ? e : String(e)
  } finally {
    loadingStandalone.value = false
  }
}

function goHome() {
  resetAnalysis()
  router.push({ name: 'home' })
}

async function onExport(format: ExportFormat) {
  if (!currentReportId.value) return
  exportBusy.value = format
  exportMessage.value = null
  try {
    const result = await exportReport(currentReportId.value, format)
    if (result) {
      exportMessage.value = `已导出到 ${result.path}（${formatBytes(result.bytes)}）`
    }
  } catch (e) {
    exportMessage.value = typeof e === 'string' ? e : String(e)
  } finally {
    exportBusy.value = null
    setTimeout(() => (exportMessage.value = null), 3000)
  }
}

onMounted(() => {
  const id = route.query.id as string | undefined
  if (id) {
    loadReport(id)
  }
  if (issueNumber.value) {
    loadIssueInfo(issueNumber.value)
  }
  document.addEventListener('click', onDocClick)
})

onUnmounted(() => {
  document.removeEventListener('click', onDocClick)
})
</script>

<template>
  <div class="analyze">
    <header class="analyze-header">
      <button class="back-btn" @click="goHome">← 返回</button>
      <h2 class="analyze-title">
        {{ state.issue ? `Issue #${state.issue.number}` : '日志分析' }}
      </h2>
      <span v-if="state.download" class="meta">
        {{ state.download.logCount }} 条 · {{ formatBytes(state.download.fileSize) }}
      </span>
      <div class="issue-actions" v-if="issueNumber">
        <span v-if="issueInfo" :class="['issue-badge', currentIssueState]">
          {{ currentIssueState === 'closed' ? '已关闭' : '未处理' }}
        </span>
        <button
          class="action-btn"
          :disabled="issuesState.actingNumber === issueNumber"
          :title="issuesState.actingNumber === issueNumber ? '处理中...' : 'Issue 操作'"
          @click.stop="openMenu = !openMenu"
        >
          {{ issuesState.actingNumber === issueNumber ? '⋯' : '⋮' }} Issue
        </button>
        <div v-if="openMenu" class="action-menu" @click.stop>
          <button class="menu-item" @click="onToggleState(issueNumber)">
            {{ currentIssueState === 'closed' ? '↻ 重新打开' : '✓ 关闭 Issue' }}
          </button>
          <button class="menu-item" @click="openComment">💬 添加评论</button>
          <button class="menu-item" @click="openLabels">🏷 管理标签</button>
        </div>
      </div>

      <div class="export-actions" v-if="currentReportId">
        <button class="export-btn" :disabled="!!exportBusy" @click="onExport('markdown')">
          {{ exportBusy === 'markdown' ? '...' : 'MD' }}
        </button>
        <button class="export-btn" :disabled="!!exportBusy" @click="onExport('json')">
          {{ exportBusy === 'json' ? '...' : 'JSON' }}
        </button>
        <button class="export-btn" :disabled="!!exportBusy" @click="onExport('csv')">
          {{ exportBusy === 'csv' ? '...' : 'CSV' }}
        </button>
      </div>
    </header>
    <p v-if="exportMessage" class="export-msg">{{ exportMessage }}</p>

    <div v-if="issuesState.actionError" class="action-error">
      {{ issuesState.actionError }}
      <button class="dismiss" @click="clearActionError">×</button>
    </div>

    <p v-if="standaloneError" class="error-msg">{{ standaloneError }}</p>
    <p v-else-if="state.error" class="error-msg">{{ state.error }}</p>

    <!-- 上报上下文：用户反馈 + 截图 + 环境信息（Issue 流程或本地导入的落库记录） -->
    <ReportMetaCard :meta="reportMeta" :screenshots="screenshotUrls" />

    <div v-if="loadingStandalone" class="loading">加载中...</div>

    <template v-else-if="result">
      <LogFilter
        v-model="filter"
        :tag-counts="result.tagCounts"
        :total="result.total"
      />

      <section class="stats">
        <span class="stat debug">DEBUG {{ result.levelCounts.debug }}</span>
        <span class="stat info">INFO {{ result.levelCounts.info }}</span>
        <span class="stat warn">WARN {{ result.levelCounts.warn }}</span>
        <span class="stat error">ERROR {{ result.levelCounts.error }}</span>
        <span v-if="result.levelCounts.fatal > 0" class="stat error">
          FATAL {{ result.levelCounts.fatal }}
        </span>
      </section>

      <Timeline :points="result.timeline" />

      <ErrorAggregates :aggregates="result.errorAggregates" />

      <div class="log-area">
        <div class="log-area__table">
          <LogTable v-model:selected="selected" :entries="result.entries" />
        </div>
        <div class="log-area__detail">
          <LogDetail :entry="selected" :summary="entrySummary" />
        </div>
      </div>
    </template>

    <div v-else-if="!state.error && !standaloneError" class="placeholder">
      请先在首页输入 Issue 进行分析
    </div>

    <!-- 评论对话框 -->
    <CommentDialog
      v-if="commentOpen && issueNumber"
      :issue-number="issueNumber"
      @close="commentOpen = false"
    />

    <!-- 标签对话框 -->
    <LabelDialog
      v-if="labelOpen && issueNumber"
      :issue-number="issueNumber"
      :labels="currentLabels"
      @close="labelOpen = false"
      @saved="onLabelsSaved"
    />
  </div>
</template>

<style scoped>
.analyze {
  width: 100%;
  max-width: 1400px;
  margin: 0 auto;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.analyze-header {
  display: flex;
  align-items: center;
  gap: 0.875rem;
}

.back-btn {
  padding: 0.375rem 0.75rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.8125rem;
}

.back-btn:hover {
  color: var(--color-text);
}

.analyze-title {
  flex: 1;
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text-bright);
}

.meta {
  color: var(--color-text-muted);
  font-size: 0.75rem;
  font-family: var(--font-mono);
}

.export-actions {
  display: flex;
  gap: 0.375rem;
}

.export-btn {
  padding: 0.25rem 0.625rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.7rem;
  font-weight: 600;
  font-family: var(--font-mono);
  transition: all var(--transition-fast);
}

.export-btn:hover:not(:disabled) {
  color: var(--color-text);
  border-color: var(--color-primary);
}

.export-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.export-msg {
  padding: 0.5rem 0.75rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.75rem;
}

.stats {
  display: flex;
  gap: 0.5rem;
}

.stat {
  padding: 0.25rem 0.75rem;
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  font-family: var(--font-mono);
  font-weight: 600;
  border: 1px solid var(--color-border);
}

.stat.debug {
  color: var(--color-text-muted);
}
.stat.info {
  color: var(--color-primary);
}
.stat.warn {
  color: var(--color-warning);
}
.stat.error {
  color: var(--color-danger);
}

.log-area {
  flex: 1;
  min-height: 260px;
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 1rem;
}

/* 日志表格与详情随剩余空间拉伸（LogTable 侧由内容撑开，详情侧铺满） */
.log-area__table {
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.log-area__table > :deep(.log-table) {
  flex: 1;
  min-height: 0;
}
.log-area__detail {
  display: flex;
  flex-direction: column;
}

.log-area__detail > :deep(.log-detail) {
  flex: 1;
}

.error-msg {
  padding: 0.625rem 0.875rem;
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  font-size: 0.8125rem;
}

.loading,
.placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  color: var(--color-text-muted);
  padding: 3rem 0;
  font-size: 0.875rem;
}

/* Issue 操作区 */
.issue-actions {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

/* 状态徽章：进入详情页拉到真实状态后显示 */
.issue-badge {
  font-size: 0.7rem;
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  border: 1px solid var(--color-border);
}

.issue-badge.open {
  color: var(--color-warning);
  border-color: var(--color-warning);
}

.issue-badge.closed {
  color: var(--color-success);
  border-color: var(--color-success);
}

.action-btn {
  padding: 0.25rem 0.625rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.7rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.action-btn:hover:not(:disabled) {
  color: var(--color-text);
  border-color: var(--color-primary);
}

.action-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.action-menu {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 0.25rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  z-index: 50;
  min-width: 140px;
  padding: 0.25rem 0;
}

.menu-item {
  display: block;
  width: 100%;
  padding: 0.5rem 0.875rem;
  background: transparent;
  border: none;
  text-align: left;
  color: var(--color-text);
  font-size: 0.8125rem;
  cursor: pointer;
  transition: background-color var(--transition-fast);
}

.menu-item:hover {
  background-color: var(--color-surface-alt);
}

/* 操作错误提示 */
.action-error {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  font-size: 0.75rem;
}

.dismiss {
  background: none;
  border: none;
  color: var(--color-danger);
  font-size: 1rem;
  cursor: pointer;
  line-height: 1;
}
</style>
