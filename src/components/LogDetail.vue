<script setup lang="ts">
import type { LogEntry } from '@/types'
import { formatTime, levelClass } from '@/utils/format'

/** 未选中日志时的概览信息（由分析页传入，替代单行空态占位） */
defineProps<{
  entry: LogEntry | null
  summary?: {
    shown: number
    total: number
    first: string
    last: string
  } | null
}>()

function formatData(data: unknown): string {
  if (data === null || data === undefined) return ''
  if (typeof data === 'object') return JSON.stringify(data, null, 2)
  return String(data)
}
</script>

<template>
  <div class="log-detail">
    <div v-if="!entry" class="empty">
      <template v-if="summary">
        <p class="empty-title">未选中日志 · 概览</p>
        <div class="summary">
          <div class="summary-row">
            <span class="label">当前匹配</span>
            <span class="value">{{ summary.shown }} 条</span>
          </div>
          <div class="summary-row">
            <span class="label">日志总量</span>
            <span class="value">{{ summary.total }} 条</span>
          </div>
          <div class="summary-row">
            <span class="label">首条时间</span>
            <span class="value">{{ formatTime(summary.first) }}</span>
          </div>
          <div class="summary-row">
            <span class="label">末条时间</span>
            <span class="value">{{ formatTime(summary.last) }}</span>
          </div>
        </div>
        <p class="empty-hint">点击左侧任意一行查看完整详情</p>
      </template>
      <template v-else>点击左侧日志查看详情</template>
    </div>
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
  display: flex;
  flex-direction: column;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.875rem 1rem;
  font-size: 0.8125rem;
}

.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--color-text-muted);
  padding: 1.5rem 0;
}

.empty-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.875rem;
}

.summary {
  width: 100%;
  max-width: 320px;
}

.summary-row {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.375rem 0;
  border-bottom: 1px solid var(--color-border);
}

.summary-row:last-child {
  border-bottom: none;
}

.empty-hint {
  margin-top: 1rem;
  font-size: 0.75rem;
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
.level-fatal {
  background-color: var(--color-danger);
  color: #fff;
  outline: 1px solid var(--color-danger);
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
