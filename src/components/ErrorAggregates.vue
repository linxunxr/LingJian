<script setup lang="ts">
import type { ErrorAggregate } from '@/types'
import { formatTime } from '@/utils/format'

defineProps<{
  aggregates: readonly ErrorAggregate[]
}>()
</script>

<template>
  <div class="error-aggregates">
    <div class="header">
      <span class="title">错误聚合</span>
      <span v-if="aggregates.length > 0" class="count">{{ aggregates.length }} 类</span>
    </div>
    <div v-if="aggregates.length === 0" class="empty">暂无 ERROR 日志</div>
    <ul v-else class="list">
      <li v-for="(agg, idx) in aggregates" :key="idx" class="item">
        <span class="count-badge">{{ agg.count }}</span>
        <div class="item-body">
          <div class="message">{{ agg.message }}</div>
          <div class="meta">
            首次 {{ formatTime(agg.firstSeen) }} · 末次 {{ formatTime(agg.lastSeen) }}
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.error-aggregates {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.75rem 1rem;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}

.title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--color-text);
}

.count {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.empty {
  text-align: center;
  color: var(--color-text-muted);
  font-size: 0.8125rem;
  padding: 0.75rem 0;
}

.list {
  list-style: none;
  max-height: 160px;
  overflow-y: auto;
}

.item {
  display: flex;
  align-items: flex-start;
  gap: 0.625rem;
  padding: 0.5rem 0;
  border-bottom: 1px solid var(--color-border);
}

.item:last-child {
  border-bottom: none;
}

.count-badge {
  flex-shrink: 0;
  min-width: 24px;
  height: 24px;
  padding: 0 0.375rem;
  background-color: var(--color-danger);
  color: #fff;
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  font-weight: 600;
  font-family: var(--font-mono);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.item-body {
  flex: 1;
  min-width: 0;
}

.message {
  color: var(--color-text);
  font-size: 0.8125rem;
  word-break: break-all;
}

.meta {
  margin-top: 0.125rem;
  color: var(--color-text-muted);
  font-size: 0.7rem;
  font-family: var(--font-mono);
}
</style>
