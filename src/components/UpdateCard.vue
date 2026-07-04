<script setup lang="ts">
import { computed } from 'vue'
import { marked } from 'marked'
import { useUpdater } from '@/composables/useUpdater'

// GFM 语法；breaks 让单换行转 <br>，CHANGELOG 换行更自然渲染
marked.setOptions({ gfm: true, breaks: true })

const { state, checkForUpdate, downloadAndInstall } = useUpdater()

// 把更新说明（Markdown 原文，来自 CHANGELOG 经 latest.json notes 字段透传）
// 解析成 HTML 渲染。内容来自项目自有 CHANGELOG（CI 提取、可信来源），无需 XSS 清洗。
const renderedBody = computed(() => {
  if (!state.body) return ''
  return marked.parse(state.body, { async: false }) as string
})

const statusText: Record<string, string> = {
  idle: '',
  checking: '正在检查更新...',
  available: '',
  'not-available': '已是最新版本',
  downloading: `正在下载 ${state.progress}%`,
  installed: '即将重启...',
  error: '',
}

async function onCheck() {
  await checkForUpdate()
}

async function onInstall() {
  await downloadAndInstall()
}
</script>

<template>
  <section class="card update-card">
    <h3 class="card-title">应用更新</h3>

    <div class="update-row">
      <div class="version-info">
        <span class="version-label">当前版本</span>
        <span class="version-value">v{{ state.currentVersion }}</span>
      </div>

      <button class="check-btn" :disabled="state.status === 'checking' || state.status === 'downloading'" @click="onCheck">
        {{ state.status === 'checking' ? '检查中...' : '检查更新' }}
      </button>
    </div>

    <!-- 检查中 / 已是最新 / 出错 -->
    <p v-if="state.status === 'checking' || state.status === 'not-available'" class="status-text">
      {{ statusText[state.status] }}
    </p>

    <!-- 有新版本 -->
    <div v-if="state.status === 'available' && state.version" class="update-available">
      <div class="new-version">
        <span class="new-label">发现新版本</span>
        <span class="new-version-value">v{{ state.version }}</span>
        <span v-if="state.date" class="new-date">{{ state.date.slice(0, 10) }}</span>
      </div>
      <div v-if="state.body" class="update-body markdown-body" v-html="renderedBody" />
      <button class="install-btn" @click="onInstall">下载并安装</button>
    </div>

    <!-- 下载进度 -->
    <div v-if="state.status === 'downloading'" class="downloading">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: `${state.progress}%` }" />
      </div>
      <span class="progress-text">下载中 {{ state.progress }}%</span>
    </div>

    <!-- 已安装 -->
    <p v-if="state.status === 'installed'" class="status-text installed">安装完成，正在重启...</p>

    <!-- 错误 -->
    <p v-if="state.status === 'error'" class="status-text error">{{ state.error }}</p>
  </section>
</template>

<style scoped>
.update-card {
  margin-bottom: 1rem;
}

.card-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.875rem;
}

.update-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.version-info {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
}

.version-label {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.version-value {
  font-size: 0.875rem;
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--color-text);
}

.check-btn {
  padding: 0.375rem 0.875rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.75rem;
  font-weight: 500;
}

.check-btn:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.check-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.status-text {
  margin-top: 0.625rem;
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.status-text.installed {
  color: var(--color-success);
}

.status-text.error {
  color: var(--color-danger);
}

.update-available {
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-border);
}

.new-version {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.new-label {
  font-size: 0.75rem;
  color: var(--color-success);
  font-weight: 600;
}

.new-version-value {
  font-size: 0.9rem;
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--color-text-bright);
}

.new-date {
  font-size: 0.7rem;
  color: var(--color-text-muted);
}

.update-body {
  margin: 0.5rem 0;
  padding: 0.75rem 0.875rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  max-height: 240px;
  overflow-y: auto;
}

/* 更新说明按 Markdown 渲染后的排版（贴合深色主题，全部用项目 CSS 变量）。
   内容来自 CHANGELOG：主要用到 h3（新增/优化/修复）、ul/li、strong、code。 */
.markdown-body {
  font-size: 0.75rem;
  line-height: 1.6;
  color: var(--color-text);
}

.markdown-body > *:first-child {
  margin-top: 0;
}

.markdown-body > *:last-child {
  margin-bottom: 0;
}

.markdown-body h3 {
  margin: 0.625rem 0 0.375rem;
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--color-text-bright);
}

/* 同级第一个 h3 不需要顶起间距（容器已有 padding） */
.markdown-body > h3:first-child {
  margin-top: 0;
}

.markdown-body p {
  margin: 0.375rem 0;
}

.markdown-body ul {
  margin: 0.25rem 0;
  padding-left: 1.25rem;
}

.markdown-body li {
  margin: 0.2rem 0;
}

.markdown-body strong {
  color: var(--color-text-bright);
  font-weight: 600;
}

.markdown-body a {
  color: var(--color-primary);
}

.markdown-body a:hover {
  color: var(--color-primary-hover);
}

.markdown-body code {
  font-family: var(--font-mono);
  font-size: 0.72rem;
  padding: 0.1rem 0.3rem;
  background-color: var(--color-surface-alt);
  border-radius: 3px;
}

.markdown-body hr {
  margin: 0.625rem 0;
  border: none;
  border-top: 1px solid var(--color-border);
}

.install-btn {
  margin-top: 0.5rem;
  padding: 0.5rem 1.25rem;
  background-color: var(--color-primary);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 0.8125rem;
  font-weight: 500;
}

.install-btn:hover {
  background-color: var(--color-primary-hover);
}

.downloading {
  margin-top: 0.75rem;
}

.progress-bar {
  height: 6px;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background-color: var(--color-primary);
  transition: width 0.2s ease;
}

.progress-text {
  display: block;
  margin-top: 0.25rem;
  font-size: 0.7rem;
  color: var(--color-text-muted);
  font-family: var(--font-mono);
}
</style>
