<script setup lang="ts">
import { useUpdater } from '@/composables/useUpdater'

const { state, checkForUpdate, downloadAndInstall } = useUpdater()

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
      <div v-if="state.body" class="update-body">
        <pre>{{ state.body }}</pre>
      </div>
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
  padding: 0.625rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  max-height: 160px;
  overflow-y: auto;
}

.update-body pre {
  margin: 0;
  font-family: var(--font-mono);
  font-size: 0.7rem;
  color: var(--color-text);
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.6;
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
