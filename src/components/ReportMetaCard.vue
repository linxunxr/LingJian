<script setup lang="ts">
import { computed, ref } from 'vue'

import type { ReportMeta } from '@/types'
import { formatDuration, formatTime } from '@/utils/format'

const props = defineProps<{
  meta: ReportMeta | null
  /** 反馈截图（data URL 列表，由分析页按 screenshotKeys 拉取后传入；空数组不渲染） */
  screenshots?: string[]
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

/** 点击放大查看的截图（null 关闭） */
const lightboxSrc = ref<string | null>(null)
</script>

<template>
  <section v-if="meta" class="report-meta">
    <div class="header">
      <span class="label">上报信息</span>
      <span v-if="meta.title" class="title">{{ meta.title }}</span>
    </div>
    <p v-if="meta.userDescription" class="description">{{ meta.userDescription }}</p>
    <div v-if="screenshots && screenshots.length" class="screenshots">
      <button
        v-for="(src, i) in screenshots"
        :key="i"
        class="shot"
        type="button"
        :title="`点击放大查看截图 ${i + 1}`"
        @click="lightboxSrc = src"
      >
        <img :src="src" :alt="`反馈截图 ${i + 1}`" loading="lazy" />
      </button>
    </div>
    <div v-if="envItems.length" class="env">
      <span v-for="item in envItems" :key="item.label" class="env-item">
        <span class="env-label">{{ item.label }}</span> {{ item.value }}
      </span>
    </div>

    <!-- 点击放大：Teleport 到 body，避开分析页的 flex 布局约束 -->
    <Teleport to="body">
      <div v-if="lightboxSrc" class="lightbox" @click="lightboxSrc = null">
        <img :src="lightboxSrc" class="lightbox-img" alt="反馈截图（放大）" @click.stop />
        <button class="lightbox-close" type="button" @click="lightboxSrc = null">×</button>
      </div>
    </Teleport>
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

/* 反馈截图缩略图行：等高缩略、点击放大 */
.screenshots {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.625rem;
}

.shot {
  padding: 0;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  overflow: hidden;
  cursor: zoom-in;
  transition: border-color var(--transition-fast);
}

.shot:hover {
  border-color: var(--color-primary);
}

.shot img {
  display: block;
  height: 88px;
  max-width: 220px;
  object-fit: cover;
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

/* 放大查看遮罩（fixed + Teleport，覆盖全屏） */
.lightbox {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(2, 6, 23, 0.88);
  cursor: zoom-out;
}

.lightbox-img {
  max-width: 92vw;
  max-height: 90vh;
  border-radius: var(--radius-md);
  cursor: default;
}

.lightbox-close {
  position: absolute;
  top: 0.75rem;
  right: 1rem;
  background: transparent;
  border: none;
  color: #e2e8f0;
  font-size: 1.75rem;
  line-height: 1;
  cursor: pointer;
}

.lightbox-close:hover {
  color: #fff;
}
</style>
