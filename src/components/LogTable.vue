<script setup lang="ts">
import { computed, ref } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import type { LogEntry } from '@/types'
import { formatTime, levelClass } from '@/utils/format'

const props = defineProps<{
  entries: readonly LogEntry[]
}>()

const selected = defineModel<LogEntry | null>('selected', { default: null })

const ROW_HEIGHT = 32

const parentRef = ref<HTMLElement | null>(null)

const virtualizer = useVirtualizer(
  computed(() => ({
    count: props.entries.length,
    getScrollElement: () => parentRef.value,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  })),
)

// virtualizer 是 Ref<Virtualizer>，响应式获取当前可见项
const virtualItems = computed(() => virtualizer.value.getVirtualItems())
const totalHeight = computed(() => virtualizer.value.getTotalSize())
</script>

<template>
  <div class="log-table">
    <div class="log-table__header">
      <span class="col-level">级别</span>
      <span class="col-time">时间</span>
      <span class="col-tag">模块</span>
      <span class="col-msg">消息</span>
    </div>
    <div ref="parentRef" class="log-table__body">
      <div
        :style="{
          height: `${totalHeight}px`,
          width: '100%',
          position: 'relative',
        }"
      >
        <div
          v-for="virtualRow in virtualItems"
          :key="virtualRow.index"
          :class="[
            'log-row',
            levelClass(entries[virtualRow.index].level),
            {
              selected:
                selected?.timestamp === entries[virtualRow.index].timestamp &&
                selected?.message === entries[virtualRow.index].message,
            },
          ]"
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: `${ROW_HEIGHT}px`,
            transform: `translateY(${virtualRow.start}px)`,
          }"
          @click="selected = entries[virtualRow.index]"
        >
          <span class="col-level">{{ entries[virtualRow.index].level }}</span>
          <span class="col-time">{{ formatTime(entries[virtualRow.index].timestamp) }}</span>
          <span class="col-tag">{{ entries[virtualRow.index].tag }}</span>
          <span class="col-msg">{{ entries[virtualRow.index].message }}</span>
        </div>
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
  padding: 0 0.875rem;
  align-items: center;
  font-size: 0.8125rem;
}

.log-table__header {
  background-color: var(--color-surface-alt);
  color: var(--color-text-muted);
  font-weight: 600;
  font-size: 0.75rem;
  border-bottom: 1px solid var(--color-border);
  height: 32px;
}

.log-table__body {
  height: 420px;
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
