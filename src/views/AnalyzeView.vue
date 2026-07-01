<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

import LogFilter from '@/components/LogFilter.vue'
import LogTable from '@/components/LogTable.vue'
import LogDetail from '@/components/LogDetail.vue'
import Timeline from '@/components/Timeline.vue'
import ErrorAggregates from '@/components/ErrorAggregates.vue'
import { useAnalysis } from '@/composables/useAnalysis'
import { exportReport, type ExportFormat } from '@/composables/useExport'
import { formatBytes } from '@/utils/format'
import type { AnalysisResult, LogEntry, Report } from '@/types'

const route = useRoute()
const router = useRouter()
const { state, filter, refreshAnalysis, resetAnalysis } = useAnalysis()

const selected = ref<LogEntry | null>(null)
const standaloneResult = ref<AnalysisResult | null>(null)
const standaloneReport = ref<Report | null>(null)
const loadingStandalone = ref(false)
const standaloneError = ref<string | null>(null)
const exportBusy = ref<ExportFormat | null>(null)
const exportMessage = ref<string | null>(null)

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
</style>
