<script setup lang="ts">
import type { LogEntry } from '@/types'
import { formatTime, levelClass } from '@/utils/format'

defineProps<{
  entries: readonly LogEntry[]
}>()

const selected = defineModel<LogEntry | null>('selected', { default: null })
</script>

<template>
  <div class="log-table">
    <div class="log-table__header">
      <span class="col-level">级别</span>
      <span class="col-time">时间</span>
      <span class="col-tag">模块</span>
      <span class="col-msg">消息</span>
    </div>
    <div class="log-table__body">
      <div
        v-for="(entry, idx) in entries"
        :key="idx"
        :class="['log-row', levelClass(entry.level), { selected: selected?.timestamp === entry.timestamp && selected?.message === entry.message }]"
        @click="selected = entry"
      >
        <span class="col-level">{{ entry.level }}</span>
        <span class="col-time">{{ formatTime(entry.timestamp) }}</span>
        <span class="col-tag">{{ entry.tag }}</span>
        <span class="col-msg">{{ entry.message }}</span>
      </div>
      <div v-if="entries.length === 0" class="empty">无匹配日志</div>
    </div>
  </div>
</template>

<style scoped>
.log-table {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.log-table__header,
.log-row {
  display: grid;
  grid-template-columns: 70px 160px 90px 1fr;
  gap: 0.75rem;
  padding: 0.5rem 0.875rem;
  font-size: 0.8125rem;
}

.log-table__header {
  background-color: var(--color-surface-alt);
  color: var(--color-text-muted);
  font-weight: 600;
  font-size: 0.75rem;
  border-bottom: 1px solid var(--color-border);
}

.log-table__body {
  max-height: 420px;
  overflow-y: auto;
}

.log-row {
  cursor: pointer;
  border-bottom: 1px solid var(--color-border);
  transition: background-color var(--transition-fast);
}

.log-row:hover {
  background-color: var(--color-surface-alt);
}

.log-row.selected {
  background-color: rgba(59, 130, 246, 0.15);
}

.col-level {
  font-family: var(--font-mono);
  font-weight: 600;
}

.col-time {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.col-tag {
  color: var(--color-text-muted);
  font-size: 0.75rem;
}

.col-msg {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.level-debug .col-level {
  color: var(--color-text-muted);
}
.level-info .col-level {
  color: var(--color-primary);
}
.level-warn .col-level {
  color: var(--color-warning);
}
.level-error .col-level {
  color: var(--color-danger);
}

.empty {
  padding: 2rem 0;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 0.85rem;
}
</style>
