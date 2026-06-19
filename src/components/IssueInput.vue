<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  loading?: boolean
}>()

const emit = defineEmits<{
  submit: [input: string]
}>()

const input = ref('')

function onSubmit() {
  const value = input.value.trim()
  if (!value || props.loading) return
  emit('submit', value)
}
</script>

<template>
  <div class="issue-input">
    <input
      v-model="input"
      class="issue-input__field"
      type="text"
      placeholder="输入 Issue URL、编号 (#42) 或 reportId..."
      :disabled="loading"
      data-shortcut="search"
      @keydown.enter="onSubmit"
    />
    <button
      class="issue-input__btn"
      :disabled="loading || !input.trim()"
      @click="onSubmit"
    >
      {{ loading ? '分析中...' : '分析' }}
    </button>
  </div>
</template>

<style scoped>
.issue-input {
  display: flex;
  gap: 0.5rem;
}

.issue-input__field {
  flex: 1;
  padding: 0.625rem 0.875rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-text);
  font-size: 0.875rem;
  transition: border-color var(--transition-fast);
}

.issue-input__field:focus {
  outline: none;
  border-color: var(--color-primary);
}

.issue-input__field:disabled {
  opacity: 0.6;
}

.issue-input__btn {
  padding: 0.625rem 1.5rem;
  background-color: var(--color-primary);
  color: #fff;
  border: none;
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  transition: background-color var(--transition-fast);
}

.issue-input__btn:hover:not(:disabled) {
  background-color: var(--color-primary-hover);
}

.issue-input__btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
