<script setup lang="ts">
import { computed } from 'vue'

import type { ReportMeta } from '@/types'
import { formatDuration, formatTime } from '@/utils/format'

const props = defineProps<{
  meta: ReportMeta | null
}>()

/** 环境信息条目（有值才展示，顺序固定） */
const envItems = computed(() => {
  const m = props.meta
  if (!m) return []
  const items: { label: string; value: string }[] = []
  if (m.appName) items.push({ label: '应用', value: m.appName })
  if (m.appVersion) items.push({ label: '版本', value: m.appVersion })
  if (m.platform) items.push({ label: '平台', value: m.platform })
  if (m.realm) items.push({ label: '境界', value: m.realm })
  if (m.playTime != null) items.push({ label: '时长', value: formatDuration(m.playTime) })
  if (m.reportTime) items.push({ label: '上报时间', value: formatTime(m.reportTime) })
  return items
})
</script>

<template>
  <section v-if="meta" class="report-meta">
    <div class="header">
      <span class="label">上报信息</span>
      <span v-if="meta.title" class="title">{{ meta.title }}</span>
    </div>
    <p v-if="meta.userDescription" class="description">{{ meta.userDescription }}</p>
    <div v-if="envItems.length" class="env">
      <span v-for="item in envItems" :key="item.label" class="env-item">
        <span class="env-label">{{ item.label }}</span> {{ item.value }}
      </span>
    </div>
  </section>
</template>

<style scoped>
.report-meta {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.625rem 1rem;
}

.header {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
}

.label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text);
  flex-shrink: 0;
}

.title {
  font-size: 0.8125rem;
  color: var(--color-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.description {
  margin-top: 0.5rem;
  font-size: 0.8125rem;
  line-height: 1.6;
  color: var(--color-text);
  white-space: pre-wrap;
  word-break: break-word;
}

.env {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem 1rem;
  margin-top: 0.5rem;
}

.env-item {
  font-size: 0.75rem;
  color: var(--color-text-muted);
  font-family: var(--font-mono);
}

.env-label {
  color: var(--color-text);
  font-weight: 600;
}
</style>
