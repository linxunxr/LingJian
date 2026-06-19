<script setup lang="ts">
import type { LogLevel, TagCount } from '@/types'

defineProps<{
  tagCounts: readonly TagCount[]
  total: number
}>()

const filter = defineModel<{
  levels: LogLevel[]
  tags: string[]
  keyword: string
}>({ required: true })

const allLevels: LogLevel[] = ['DEBUG', 'INFO', 'WARN', 'ERROR']

function toggleLevel(level: LogLevel) {
  const idx = filter.value.levels.indexOf(level)
  if (idx >= 0) {
    filter.value.levels.splice(idx, 1)
  } else {
    filter.value.levels.push(level)
  }
}

function clearAll() {
  filter.value.levels = []
  filter.value.tags = []
  filter.value.keyword = ''
}
</script>

<template>
  <div class="log-filter">
    <div class="filter-row">
      <div class="level-chips">
        <button
          v-for="level in allLevels"
          :key="level"
          :class="['chip', `level-${level.toLowerCase()}`, { active: filter.levels.includes(level) }]"
          @click="toggleLevel(level)"
        >
          {{ level }}
        </button>
      </div>
      <button class="clear-btn" @click="clearAll">清除</button>
    </div>

    <div class="filter-row">
      <select v-model="filter.tags" multiple class="tag-select" :size="1">
        <option v-for="t in tagCounts" :key="t.tag" :value="t.tag">
          {{ t.tag }} ({{ t.count }})
        </option>
      </select>
      <input
        v-model="filter.keyword"
        class="keyword-input"
        type="text"
        placeholder="搜索关键词..."
      />
      <span class="total">共 {{ total }} 条</span>
    </div>
  </div>
</template>

<style scoped>
.log-filter {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  padding: 0.75rem 1rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.filter-row {
  display: flex;
  align-items: center;
  gap: 0.625rem;
}

.level-chips {
  display: flex;
  gap: 0.375rem;
}

.chip {
  padding: 0.25rem 0.625rem;
  background-color: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.75rem;
  font-weight: 600;
  font-family: var(--font-mono);
  transition: all var(--transition-fast);
}

.chip:hover {
  color: var(--color-text);
}

.chip.active {
  color: #fff;
  border-color: transparent;
}

.chip.level-debug.active {
  background-color: var(--color-text-muted);
}

.chip.level-info.active {
  background-color: var(--color-primary);
}

.chip.level-warn.active {
  background-color: var(--color-warning);
}

.chip.level-error.active {
  background-color: var(--color-danger);
}

.clear-btn {
  margin-left: auto;
  padding: 0.25rem 0.625rem;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.75rem;
}

.clear-btn:hover {
  color: var(--color-text);
}

.tag-select {
  padding: 0.375rem 0.5rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.8125rem;
  min-width: 140px;
}

.keyword-input {
  flex: 1;
  padding: 0.375rem 0.625rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.8125rem;
}

.keyword-input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.total {
  color: var(--color-text-muted);
  font-size: 0.75rem;
  font-family: var(--font-mono);
}
</style>
