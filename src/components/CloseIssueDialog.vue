<script setup lang="ts">
import { ref } from 'vue'

import AppDialog from './AppDialog.vue'
import { useIssues } from '@/composables/useIssues'

const props = defineProps<{
  issueNumber: number
  labels: readonly string[]
}>()

const emit = defineEmits<{
  close: []
}>()

const { actOnIssue } = useIssues()
const version = ref('')

/** 确认关闭：串行 close → setLabels → comment */
async function confirmClose() {
  if (!version.value.trim()) return
  const tagLabel = `v${version.value.trim().replace(/^v/, '')}`

  // 1) 关闭 Issue —— 失败则保留弹窗让用户重试
  const ok = await actOnIssue(props.issueNumber, 'close')
  if (!ok) return

  // 2) 追加版本标签 + 3) 解决评论 —— 后续步骤失败不阻断关弹窗
  //    （Issue 已关闭是主目标，标签/评论失败仅作次要，错误会进 actionError 横幅）
  try {
    const base = Array.isArray(props.labels) ? props.labels : []
    const newLabels = [...new Set([...base, tagLabel])]
    await actOnIssue(props.issueNumber, 'setLabels', { labels: newLabels })
    await actOnIssue(props.issueNumber, 'comment', { body: `已在挂机仙途 ${tagLabel} 中标记为已处理` })
  } finally {
    // 无论后续步骤成败，Issue 已关闭，弹窗必须关闭
    emit('close')
  }
}
</script>

<template>
  <AppDialog :title="`关闭 Issue · #${issueNumber}`" @close="emit('close')">
    <p class="close-hint">
      输入解决的挂机仙途版本号（如 0.9.19），关闭后将自动添加版本标签和评论。
    </p>
    <div class="close-input-row">
      <span class="version-prefix">v</span>
      <input
        v-model="version"
        type="text"
        class="version-input"
        placeholder="0.9.19"
        autofocus
        @keyup.enter="confirmClose"
      />
    </div>
    <template #footer>
      <button class="ghost-btn" @click="emit('close')">取消</button>
      <button class="primary-btn" :disabled="!version.trim()" @click="confirmClose">
        确认关闭
      </button>
    </template>
  </AppDialog>
</template>

<style scoped>
.close-hint {
  font-size: 0.8125rem;
  color: var(--color-text-muted);
  margin-bottom: 0.875rem;
  line-height: 1.5;
}

.close-input-row {
  display: flex;
  align-items: center;
  gap: 0;
}

.version-prefix {
  padding: 0.5rem 0.625rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-right: none;
  border-radius: var(--radius-sm) 0 0 var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.875rem;
  font-family: var(--font-mono);
}

.version-input {
  flex: 1;
  padding: 0.5rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  color: var(--color-text);
  font-size: 0.875rem;
  font-family: var(--font-mono);
}

.version-input:focus {
  outline: none;
  border-color: var(--color-primary);
}
</style>
