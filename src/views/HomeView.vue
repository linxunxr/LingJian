<script setup lang="ts">
defineOptions({ name: 'HomeView' })
import { onMounted, onActivated, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { open } from '@tauri-apps/plugin-dialog'

import IssueInput from '@/components/IssueInput.vue'
import IssueList from '@/components/IssueList.vue'
import { useAnalysis } from '@/composables/useAnalysis'
import { useSettings, isSettingsComplete } from '@/composables/useSettings'
import { useIssues } from '@/composables/useIssues'
import { formatTime } from '@/utils/format'
import type { Report } from '@/types'

const router = useRouter()
const { runAnalysis, importLocalFile, state } = useAnalysis()
const { loadSettings } = useSettings()
const { loadIssues } = useIssues()

const recentReports = ref<Report[]>([])
// 应用版本号（来自 Tauri 打包元数据，单一数据源，避免与配置文件版本号不一致）
const appVersion = ref('')
// 响应式跟随全局 settings：保存配置后无需重载页面即可刷新横幅状态
const settingsReady = computed(() => isSettingsComplete())
// 拖拽悬停高亮
const dragOver = ref(false)

const stageText: Record<string, string> = {
  parsing: '正在解析 Issue...',
  downloading: '正在下载日志...',
  importing: '正在导入日志...',
  analyzing: '正在分析日志...',
}

async function loadRecent() {
  try {
    const result = await invoke<Report[]>('list_recent_reports', { limit: 10 })
    recentReports.value = Array.isArray(result) ? result : []
  } catch (e) {
    console.error('[loadRecent] 调用失败:', e)
    recentReports.value = []
  }
}

/** 分析入口：手动输入或从问题列表点选，统一走 runAnalysis */
async function onSubmit(input: string) {
  await runAnalysis(input)
  if (state.reportId) {
    router.push({ name: 'analyze', query: { id: state.reportId } })
  }
}

/** 本地导入入口：文件选择对话框，支持多选（逐个导入，跳到最后一个分析） */
async function onPickFile() {
  if (state.stage !== 'idle' && state.stage !== 'done') return
  const files = await open({
    title: '导入本地日志文件',
    multiple: true,
    filters: [
      {
        name: '日志文件',
        extensions: ['log', 'txt', 'json', 'zip', 'gz'],
      },
    ],
  })
  if (!files) return
  const paths = Array.isArray(files) ? files : [files]
  for (const p of paths) {
    await importLocalFile(p)
    if (state.error) return // 单个失败即停，展示错误
  }
  if (state.reportId) {
    router.push({ name: 'analyze', query: { id: state.reportId } })
  }
}

/** 全窗口拖拽导入（dataTransfer 取本地文件路径） */
async function onDrop(e: DragEvent) {
  dragOver.value = false
  if (state.stage !== 'idle' && state.stage !== 'done') return
  const files = Array.from(e.dataTransfer?.files ?? [])
  if (files.length === 0) return
  for (const f of files) {
    // Tauri 2 的 File 对象带 path 属性（非标准但可用）；无路径时跳过并提示
    const p = (f as File & { path?: string }).path
    if (!p) {
      continue
    }
    await importLocalFile(p)
    if (state.error) return
  }
  if (state.reportId) {
    router.push({ name: 'analyze', query: { id: state.reportId } })
  }
}

// keep-alive 首次挂载标记：onMounted 和 onActivated 都会在首次触发，
// 用此标记跳过首次 onActivated 的重复加载
let firstMount = true

onMounted(async () => {
  await loadSettings()
  // 读取应用版本号（getVersion 取 tauri.conf.json 的 version）
  appVersion.value = await getVersion()
  // settingsReady 已是 computed，loadSettings 更新全局 settings 后自动重算
  // 配置完整时自动拉取问题列表；不完整时 loadIssues 内部会静默跳过
  await Promise.all([loadRecent(), loadIssues()])
  firstMount = false
})

// 从分析页等返回时（keep-alive 恢复），仅刷新"最近分析"列表
// （可能刚完成新分析，需要更新列表），问题列表不重拉（已有数据）
onActivated(() => {
  if (firstMount) return // 首次由 onMounted 处理
  loadRecent()
})
</script>

<template>
  <div
    class="home"
    :class="{ 'home--drag': dragOver }"
    @dragover.prevent="dragOver = true"
    @dragleave.prevent="dragOver = false"
    @drop.prevent="onDrop"
  >
    <section class="hero">
      <h2 class="hero-title">灵鉴 <span class="version">v{{ appVersion }}</span></h2>
      <p class="hero-desc">Path of Idle Immortals 日志分析工具 · 支持导入鸿蒙应用本地日志</p>
    </section>

    <section class="search">
      <IssueInput :loading="state.stage !== 'idle' && state.stage !== 'done'" @submit="onSubmit" />
      <div class="import-row">
        <button class="import-btn" :disabled="state.stage !== 'idle' && state.stage !== 'done'" @click="onPickFile">
          📂 导入本地日志
        </button>
        <span class="import-hint">支持 huanyinfeng/logger 落盘日志（.log/.txt）、zlib/gzip 压缩包，也可直接拖入窗口</span>
      </div>
      <p v-if="state.error" class="error-msg">{{ state.error }}</p>
      <p v-else-if="state.stage !== 'idle' && state.stage !== 'done'" class="stage-msg">
        {{ stageText[state.stage] }}
      </p>
      <p v-if="!settingsReady" class="warn-msg">
        ⚠ 检测到配置不完整，请先到
        <RouterLink :to="{ name: 'settings' }">设置页</RouterLink>
        填写 SCF 端点配置（URL + API Key）；本地导入日志无需配置
      </p>
    </section>

    <section class="remote-issues">
      <IssueList @select="onSubmit" />
    </section>

    <section class="recent">
      <h3 class="section-title">最近分析</h3>
      <div v-if="recentReports.length === 0" class="empty">暂无分析记录</div>
      <ul v-else class="report-list">
        <li
          v-for="report in recentReports"
          :key="report.reportId"
          class="report-item"
          @click="router.push({ name: 'analyze', query: { id: report.reportId, issue: report.issueNumber } })"
        >
          <span class="report-issue">
            {{ report.issueNumber ? `#${report.issueNumber}` : (report.appName ? '本地' : '—') }}
          </span>
          <span class="report-title">{{ report.issueTitle ?? (report.reportId ? report.reportId.slice(0, 8) : '—') }}</span>
          <span class="report-count">{{ report.logCount }} 条</span>
          <span class="report-time">{{ formatTime(report.downloadedAt) }}</span>
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.home {
  width: 100%;
  max-width: 960px;
  margin: 0 auto;
  flex-shrink: 0;
}

.hero {
  text-align: center;
  padding: 2.5rem 0 2rem;
}

.hero-title {
  font-size: 1.75rem;
  font-weight: 700;
  color: var(--color-text-bright);
}

.version {
  font-size: 0.8125rem;
  font-weight: 400;
  color: var(--color-text-muted);
  vertical-align: super;
}

.hero-desc {
  margin-top: 0.5rem;
  color: var(--color-text-muted);
  font-size: 0.875rem;
}

.search {
  margin-bottom: 2.5rem;
}

.import-row {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  margin-top: 0.625rem;
}

.import-btn {
  padding: 0.375rem 0.875rem;
  background-color: var(--color-surface);
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-text);
  font-size: 0.8125rem;
  transition: all var(--transition-fast);
}

.import-btn:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.import-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.import-hint {
  color: var(--color-text-muted);
  font-size: 0.75rem;
}

/* 拖拽悬停时整页高亮提示可放置 */
.home--drag {
  outline: 2px dashed var(--color-primary);
  outline-offset: -8px;
  border-radius: var(--radius-md);
}

.remote-issues {
  margin-bottom: 2.5rem;
}

.error-msg {
  margin-top: 0.75rem;
  padding: 0.625rem 0.875rem;
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  font-size: 0.8125rem;
}

.stage-msg {
  margin-top: 0.75rem;
  color: var(--color-text-muted);
  font-size: 0.8125rem;
}

.warn-msg {
  margin-top: 0.75rem;
  padding: 0.625rem 0.875rem;
  background-color: rgba(245, 158, 11, 0.1);
  border: 1px solid var(--color-warning);
  border-radius: var(--radius-md);
  color: var(--color-warning);
  font-size: 0.8125rem;
}

.section-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.75rem;
}

.empty {
  padding: 2rem 0;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 0.85rem;
}

.report-list {
  list-style: none;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.report-item {
  display: grid;
  grid-template-columns: 60px 1fr 80px 160px;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  font-size: 0.8125rem;
  cursor: pointer;
  transition: background-color var(--transition-fast);
  border-bottom: 1px solid var(--color-border);
}

.report-item:last-child {
  border-bottom: none;
}

.report-item:hover {
  background-color: var(--color-surface-alt);
}

.report-issue {
  color: var(--color-primary);
  font-weight: 600;
  font-family: var(--font-mono);
}

.report-title {
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.report-count {
  color: var(--color-text-muted);
  text-align: right;
}

.report-time {
  color: var(--color-text-muted);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  text-align: right;
}
</style>
