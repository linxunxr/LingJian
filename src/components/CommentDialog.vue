<script setup lang="ts">
import { ref } from 'vue'

import AppDialog from './AppDialog.vue'
import { useIssues } from '@/composables/useIssues'

const props = defineProps<{
  issueNumber: number
}>()

const emit = defineEmits<{
  close: []
}>()

const { actOnIssue } = useIssues()
const text = ref('')

async function submit() {
  if (!text.value.trim()) return
  const ok = await actOnIssue(props.issueNumber, 'comment', { body: text.value.trim() })
  if (ok) emit('close')
}
</script>

<template>
  <AppDialog :title="`添加评论 · #${issueNumber}`" @close="emit('close')">
    <textarea
      v-model="text"
      class="dialog-textarea"
      placeholder="输入评论内容（支持 Markdown）..."
      rows="4"
      autofocus
    />
    <template #footer>
      <button class="ghost-btn" @click="emit('close')">取消</button>
      <button class="primary-btn" :disabled="!text.trim()" @click="submit">提交</button>
    </template>
  </AppDialog>
</template>

<style scoped>
.dialog-textarea {
  width: 100%;
  padding: 0.625rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.8125rem;
  font-family: var(--font-mono);
  resize: vertical;
  min-height: 80px;
}

.dialog-textarea:focus {
  outline: none;
  border-color: var(--color-primary);
}
</style>
