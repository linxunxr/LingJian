<script setup lang="ts">
import type { TimelinePoint } from '@/types'
import { formatTime, levelClass } from '@/utils/format'

defineProps<{
  points: readonly TimelinePoint[]
}>()
</script>

<template>
  <div class="timeline">
    <div v-if="points.length === 0" class="empty">暂无 WARN/ERROR 日志</div>
    <ul v-else class="timeline-list">
      <li
        v-for="(point, idx) in points"
        :key="idx"
        :class="['timeline-item', levelClass(point.level)]"
      >
        <span class="dot" />
        <span class="time">{{ formatTime(point.timestamp) }}</span>
        <span class="level">{{ point.level }}</span>
        <span class="msg">{{ point.message }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.timeline {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.75rem 1rem;
  max-height: 200px;
  overflow-y: auto;
}

.empty {
  text-align: center;
  color: var(--color-text-muted);
  font-size: 0.8125rem;
  padding: 1rem 0;
}

.timeline-list {
  list-style: none;
}

.timeline-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0;
  font-size: 0.8125rem;
  border-bottom: 1px solid var(--color-border);
}

.timeline-item:last-child {
  border-bottom: none;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.timeline-item.level-warn .dot {
  background-color: var(--color-warning);
}
.timeline-item.level-error .dot {
  background-color: var(--color-danger);
}

.time {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--color-text-muted);
  flex-shrink: 0;
}

.level {
  font-family: var(--font-mono);
  font-size: 0.7rem;
  font-weight: 600;
  flex-shrink: 0;
  width: 48px;
}

.timeline-item.level-warn .level {
  color: var(--color-warning);
}
.timeline-item.level-error .level {
  color: var(--color-danger);
}

.msg {
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
