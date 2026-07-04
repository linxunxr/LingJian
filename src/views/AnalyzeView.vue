<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

import LogFilter from '@/components/LogFilter.vue'
import LogTable from '@/components/LogTable.vue'
import LogDetail from '@/components/LogDetail.vue'
import Timeline from '@/components/Timeline.vue'
import ErrorAggregates from '@/components/ErrorAggregates.vue'
import { useAnalysis } from '@/composables/useAnalysis'
import { useIssues } from '@/composables/useIssues'
import { exportReport, type ExportFormat } from '@/composables/useExport'
import { formatBytes } from '@/utils/format'
import type { AnalysisResult, LogEntry, Report } from '@/types'

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
/** 预设标签（与 IssueList 保持一致） */
const PRESET_LABELS = ['已修复', '无法复现', '高优先级', '待验证'] as const

/** 当前 report 对应的 issue 号（无则不可操作） */
const issueNumber = computed(() => {
  const q = route.query.issue
  const n = q ? Number(q) : NaN
  return Number.isFinite(n) && n > 0 ? n : null
})

/** 本地维护的 issue 状态：首次操作前为 null（未知），操作后按返回值更新 */
const currentIssueState = ref<string | null>(null)
const openMenu = ref(false)

/** 评论对话框 */
const commentTarget = ref(false)
const commentText = ref('')

/** 标签对话框 */
const labelTarget = ref(false)
const labelDraft = ref<string[]>([])

/** 关闭/重开 */
async function onToggleState(number: number, issueState: string | null) {
  openMenu.value = false
  const action = issueState === 'closed' ? 'reopen' : 'close'
  const ok = await actOnIssue(number, action)
  // 操作成功后按 action 反推状态（close → closed；reopen → open），
  // 不依赖后端返回 state（部分实现可能不回传）
  if (ok) currentIssueState.value = action === 'close' ? 'closed' : 'open'
}

function openComment() {
  openMenu.value = false
  commentTarget.value = true
  commentText.value = ''
}

async function submitComment() {
  if (!issueNumber.value || !commentText.value.trim()) return
  const ok = await actOnIssue(issueNumber.value, 'comment', { body: commentText.value.trim() })
  if (ok) {
    commentTarget.value = false
    commentText.value = ''
  }
}

function openLabels() {
  openMenu.value = false
  labelTarget.value = true
  labelDraft.value = []
}

function togglePresetLabel(label: string) {
  const idx = labelDraft.value.indexOf(label)
  if (idx >= 0) labelDraft.value.splice(idx, 1)
  else labelDraft.value.push(label)
}

async function submitLabels() {
  if (!issueNumber.value) return
  const ok = await actOnIssue(issueNumber.value, 'setLabels', { labels: labelDraft.value })
  if (ok) {
    labelTarget.value = false
    labelDraft.value = []
  }
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
const report = computed(() => standaloneReport.value ?? state.issue)

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

/** 单独加载某 report 的分析（从首页最近列表点进来） */
async function loadReport(reportId: string) {
  loadingStandalone.value = true
  standaloneError.value = null
  try {
    standaloneResult.value = await invoke<AnalysisResult>('analyze_log', {
      reportId,
      filter,
    })
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
        {{ report ? (state.issue ? `Issue #${state.issue.number}` : '日志分析') : '日志分析' }}
      </h2>
      <span v-if="state.download" class="meta">
        {{ state.download.logCount }} 条 · {{ formatBytes(state.download.fileSize) }}
      </span>
      <div class="issue-actions" v-if="issueNumber">
        <button
          class="action-btn"
          :disabled="issuesState.actingNumber === issueNumber"
          :title="issuesState.actingNumber === issueNumber ? '处理中...' : 'Issue 操作'"
          @click.stop="openMenu = !openMenu"
        >
          {{ issuesState.actingNumber === issueNumber ? '⋯' : '⋮' }} Issue
        </button>
        <div v-if="openMenu" class="action-menu" @click.stop>
          <button class="menu-item" @click="onToggleState(issueNumber, currentIssueState)">
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
      </section>

      <Timeline :points="result.timeline" />

      <ErrorAggregates :aggregates="result.errorAggregates" />

      <div class="log-area">
        <div class="log-area__table">
          <LogTable v-model:selected="selected" :entries="result.entries" />
        </div>
        <div class="log-area__detail">
          <LogDetail :entry="selected" />
        </div>
      </div>
    </template>

    <div v-else-if="!state.error && !standaloneError" class="placeholder">
      请先在首页输入 Issue 进行分析
    </div>

    <!-- 评论对话框 -->
    <div v-if="commentTarget" class="dialog-overlay" @click.self="commentTarget = false">
      <div class="dialog">
        <h4 class="dialog-title">
          添加评论 · #{{ issueNumber }}
        </h4>
        <textarea
          v-model="commentText"
          class="dialog-textarea"
          placeholder="输入评论内容（支持 Markdown）..."
          rows="4"
          autofocus
        />
        <div class="dialog-actions">
          <button class="ghost-btn" @click="commentTarget = false">取消</button>
          <button class="primary-btn" :disabled="!commentText.trim()" @click="submitComment">
            提交
          </button>
        </div>
      </div>
    </div>

    <!-- 标签对话框 -->
    <div v-if="labelTarget" class="dialog-overlay" @click.self="labelTarget = false">
      <div class="dialog">
        <h4 class="dialog-title">管理标签 · #{{ issueNumber }}</h4>
        <div class="label-editor">
          <p class="label-section-title">快速切换</p>
          <div class="preset-labels">
            <label
              v-for="lab in PRESET_LABELS"
              :key="lab"
              :class="['preset-label', { checked: labelDraft.includes(lab) }]"
            >
              <input
                type="checkbox"
                :checked="labelDraft.includes(lab)"
                @change="togglePresetLabel(lab)"
              />
              {{ lab }}
            </label>
          </div>
          <p class="label-section-title">当前全部标签</p>
          <p class="label-current">{{ labelDraft.length ? labelDraft.join('、') : '（无）' }}</p>
          <p class="label-hint">预设标签为切换开关；当前列表为整体替换结果（提交后覆盖远端）</p>
        </div>
        <div class="dialog-actions">
          <button class="ghost-btn" @click="labelTarget = false">取消</button>
          <button class="primary-btn" @click="submitLabels">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.analyze {
  max-width: 1080px;
  margin: 0 auto;
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
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 1rem;
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

/* 对话框 */
.dialog-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.dialog {
  width: 460px;
  max-width: 90vw;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md);
  padding: 1.25rem 1.5rem;
}

.dialog-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.875rem;
}

.dialog-textarea {
  width: 100%;
  padding: 0.625rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.8125rem;
  font-family: var(--font-mono);
  resize: vertical;
  min-height: 80px;
}

.dialog-textarea:focus {
  outline: none;
  border-color: var(--color-primary);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.625rem;
  margin-top: 1rem;
}

.ghost-btn {
  padding: 0.4rem 0.875rem;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.8125rem;
}

.ghost-btn:hover {
  color: var(--color-text);
}

.primary-btn {
  padding: 0.4rem 1rem;
  background-color: var(--color-primary);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 0.8125rem;
  font-weight: 500;
}

.primary-btn:hover:not(:disabled) {
  background-color: var(--color-primary-hover);
}

.primary-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 标签编辑器 */
.label-editor {
  font-size: 0.8125rem;
}

.label-section-title {
  font-size: 0.75rem;
  color: var(--color-text-muted);
  margin: 0.5rem 0 0.375rem;
}

.preset-labels {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.preset-label {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.625rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  color: var(--color-text-muted);
  cursor: pointer;
  user-select: none;
}

.preset-label.checked {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
}

.preset-label input[type='checkbox'] {
  margin: 0;
  accent-color: currentColor;
}

.label-current {
  font-size: 0.8125rem;
  color: var(--color-text);
  margin: 0.25rem 0;
}

.label-hint {
  font-size: 0.7rem;
  color: var(--color-text-muted);
  margin-top: 0.5rem;
  line-height: 1.5;
}
</style>
