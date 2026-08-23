<script setup lang="ts">
defineProps<{
  /** 对话框标题（如「添加评论 · #42」） */
  title: string
}>()

const emit = defineEmits<{
  close: []
}>()
</script>

<template>
  <div class="dialog-overlay" @click.self="emit('close')">
    <div class="dialog">
      <h4 class="dialog-title">{{ title }}</h4>
      <slot />
      <div v-if="$slots.footer" class="dialog-actions">
        <slot name="footer" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.dialog {
  width: 460px;
  max-width: 90vw;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md);
  padding: 1.25rem 1.5rem;
}

.dialog-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.875rem;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.625rem;
  margin-top: 1rem;
}
</style>
