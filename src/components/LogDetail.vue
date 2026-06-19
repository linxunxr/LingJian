<script setup lang="ts">
import type { LogEntry } from '@/types'
import { formatTime, levelClass } from '@/utils/format'

defineProps<{
  entry: LogEntry | null
}>()

function formatData(data: unknown): string {
  if (data === null || data === undefined) return ''
  if (typeof data === 'object') return JSON.stringify(data, null, 2)
  return String(data)
}
</script>

<template>
  <div class="log-detail">
    <div v-if="!entry" class="empty">点击左侧日志查看详情</div>
    <template v-else>
      <div class="detail-row">
        <span class="label">时间</span>
        <span class="value">{{ formatTime(entry.timestamp) }}</span>
      </div>
      <div class="detail-row">
        <span class="label">级别</span>
        <span :class="['value', 'level-badge', levelClass(entry.level)]">{{ entry.level }}</span>
      </div>
      <div class="detail-row">
        <span class="label">模块</span>
        <span class="value">{{ entry.tag }}</span>
      </div>
      <div class="detail-row">
        <span class="label">消息</span>
        <span class="value">{{ entry.message }}</span>
      </div>
      <div v-if="entry.data !== undefined && entry.data !== null" class="detail-block">
        <span class="label">数据</span>
        <pre class="data-block">{{ formatData(entry.data) }}</pre>
      </div>
    </template>
  </div>
</template>

<style scoped>
.log-detail {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.875rem 1rem;
  font-size: 0.8125rem;
}

.empty {
  text-align: center;
  color: var(--color-text-muted);
  padding: 1.5rem 0;
}

.detail-row {
  display: grid;
  grid-template-columns: 56px 1fr;
  gap: 0.625rem;
  padding: 0.375rem 0;
}

.label {
  color: var(--color-text-muted);
  font-size: 0.75rem;
}

.value {
  color: var(--color-text);
  word-break: break-all;
}

.level-badge {
  display: inline-block;
  width: fit-content;
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: 0.75rem;
}

.level-debug {
  background-color: var(--color-text-muted);
  color: #fff;
}
.level-info {
  background-color: var(--color-primary);
  color: #fff;
}
.level-warn {
  background-color: var(--color-warning);
  color: #fff;
}
.level-error {
  background-color: var(--color-danger);
  color: #fff;
}

.detail-block {
  display: grid;
  grid-template-columns: 56px 1fr;
  gap: 0.625rem;
  padding: 0.375rem 0;
}

.data-block {
  margin: 0;
  padding: 0.625rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--color-text);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
