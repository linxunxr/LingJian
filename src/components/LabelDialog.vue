<script setup lang="ts">
import { ref } from 'vue'

import AppDialog from './AppDialog.vue'
import { PRESET_LABELS, useIssues } from '@/composables/useIssues'

const props = defineProps<{
  issueNumber: number
  /** 当前远端标签（打开时回填草稿） */
  labels: readonly string[]
}>()

const emit = defineEmits<{
  close: []
  /** 保存成功，传出整体替换后的标签（父级可乐观更新） */
  saved: [labels: string[]]
}>()

const { actOnIssue } = useIssues()
// 拷贝为可变草稿（预设区只勾选匹配项，提交时整体替换远端）
const draft = ref<string[]>([...props.labels])

function togglePreset(label: string) {
  const idx = draft.value.indexOf(label)
  if (idx >= 0) {
    draft.value.splice(idx, 1)
  } else {
    draft.value.push(label)
  }
}

async function submit() {
  const ok = await actOnIssue(props.issueNumber, 'setLabels', { labels: draft.value })
  if (ok) {
    emit('saved', [...draft.value])
    emit('close')
  }
}
</script>

<template>
  <AppDialog :title="`管理标签 · #${issueNumber}`" @close="emit('close')">
    <div class="label-editor">
      <p class="label-section-title">快速切换</p>
      <div class="preset-labels">
        <label
          v-for="lab in PRESET_LABELS"
          :key="lab"
          :class="['preset-label', { checked: draft.includes(lab) }]"
        >
          <input
            type="checkbox"
            :checked="draft.includes(lab)"
            @change="togglePreset(lab)"
          />
          {{ lab }}
        </label>
      </div>
      <p class="label-section-title">当前全部标签</p>
      <p class="label-current">{{ draft.length ? draft.join('、') : '（无）' }}</p>
      <p class="label-hint">预设标签为切换开关；当前列表为整体替换结果（提交后覆盖远端）</p>
    </div>
    <template #footer>
      <button class="ghost-btn" @click="emit('close')">取消</button>
      <button class="primary-btn" @click="submit">保存</button>
    </template>
  </AppDialog>
</template>

<style scoped>
.label-editor {
  font-size: 0.8125rem;
}

.label-section-title {
  font-size: 0.75rem;
  color: var(--color-text-muted);
  margin: 0.5rem 0 0.375rem;
}

.preset-labels {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.preset-label {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.625rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  color: var(--color-text-muted);
  cursor: pointer;
  user-select: none;
}

.preset-label.checked {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
}

.preset-label input[type='checkbox'] {
  margin: 0;
  accent-color: currentColor;
}

.label-current {
  font-size: 0.8125rem;
  color: var(--color-text);
  margin: 0.25rem 0;
}

.label-hint {
  font-size: 0.7rem;
  color: var(--color-text-muted);
  margin-top: 0.5rem;
  line-height: 1.5;
}
</style>
