<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open, confirm } from '@tauri-apps/plugin-dialog'
import { relaunch } from '@tauri-apps/plugin-process'
import { formatBytes } from '@/utils/format'

interface StorageInfo {
  dataDir: string
  size: number
  portable: boolean
}

const info = ref<StorageInfo | null>(null)
const loading = ref(false)
const message = ref<string | null>(null)
const messageType = ref<'info' | 'success' | 'error'>('info')
const changing = ref(false)

async function refresh() {
  loading.value = true
  try {
    info.value = await invoke<StorageInfo>('get_storage_info')
  } catch (e) {
    showMessage(typeof e === 'string' ? e : String(e), 'error')
  } finally {
    loading.value = false
  }
}

async function onChangeDir() {
  const selected = await open({ directory: true, multiple: false, title: '选择数据存储目录' })
  if (!selected || typeof selected !== 'string') return

  const ok = await confirm(
    `将把数据迁移到：\n${selected}\n\n迁移完成后需要重启应用生效，是否继续？`,
    { title: '更改数据目录', kind: 'warning' },
  )
  if (!ok) return

  changing.value = true
  try {
    await invoke('change_data_dir', { newDir: selected })
    showMessage('数据迁移成功，应用即将重启...', 'success')
    // 等待用户看到提示后重启
    setTimeout(async () => {
      await relaunch()
    }, 1500)
  } catch (e) {
    showMessage(typeof e === 'string' ? e : String(e), 'error')
  } finally {
    changing.value = false
  }
}

async function onClearCache() {
  const ok = await confirm('将清除所有已下载的 gzip 日志缓存，数据库中的分析记录保留。是否继续？', {
    title: '清理缓存',
    kind: 'warning',
  })
  if (!ok) return
  try {
    await invoke('clear_cache')
    await refresh()
    showMessage('缓存已清理', 'success')
  } catch (e) {
    showMessage(typeof e === 'string' ? e : String(e), 'error')
  }
}

function showMessage(text: string, type: 'info' | 'success' | 'error') {
  message.value = text
  messageType.value = type
  if (type !== 'error') {
    setTimeout(() => (message.value = null), 3000)
  }
}

onMounted(refresh)
</script>

<template>
  <section class="card data-card">
    <h3 class="card-title">本地数据</h3>

    <div v-if="info" class="data-info">
      <div class="info-row">
        <span class="info-label">存储位置</span>
        <span class="info-value" :title="info.dataDir">{{ info.dataDir }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">已用空间</span>
        <span class="info-value">{{ formatBytes(info.size) }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">存储模式</span>
        <span :class="['mode-badge', info.portable ? 'portable' : 'system']">
          {{ info.portable ? '便携（跟随安装位置）' : '系统默认' }}
        </span>
      </div>
    </div>

    <p v-if="loading" class="loading-text">加载中...</p>

    <div class="actions">
      <button class="action-btn" :disabled="changing" @click="onChangeDir">
        {{ changing ? '迁移中...' : '更改目录' }}
      </button>
      <button class="action-btn danger" @click="onClearCache">清理缓存</button>
    </div>

    <p v-if="message" :class="['message', messageType]">{{ message }}</p>

    <p class="hint">
      默认数据存放在安装目录下（便携模式）；装在 Program Files 时自动降级到系统目录。
      可在此手动指定其他磁盘位置，数据将自动迁移。
    </p>
  </section>
</template>

<style scoped>
.data-card {
  margin-bottom: 1rem;
}

.card-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.875rem;
}

.data-info {
  margin-bottom: 1rem;
}

.info-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.375rem 0;
}

.info-label {
  flex-shrink: 0;
  width: 70px;
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.info-value {
  font-size: 0.8125rem;
  color: var(--color-text);
  font-family: var(--font-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mode-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.7rem;
  font-weight: 600;
}

.mode-badge.portable {
  background-color: rgba(34, 197, 94, 0.15);
  color: var(--color-success);
}

.mode-badge.system {
  background-color: var(--color-surface-alt);
  color: var(--color-text-muted);
}

.loading-text {
  color: var(--color-text-muted);
  font-size: 0.75rem;
}

.actions {
  display: flex;
  gap: 0.5rem;
}

.action-btn {
  padding: 0.375rem 0.875rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.75rem;
  font-weight: 500;
}

.action-btn:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.action-btn.danger:hover:not(:disabled) {
  border-color: var(--color-danger);
  color: var(--color-danger);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.message {
  margin-top: 0.625rem;
  font-size: 0.75rem;
}

.message.success {
  color: var(--color-success);
}

.message.error {
  color: var(--color-danger);
}

.message.info {
  color: var(--color-text-muted);
}

.hint {
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-border);
  font-size: 0.7rem;
  color: var(--color-text-muted);
  line-height: 1.6;
}
</style>
