<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

const message = ref('')
const loading = ref(false)

async function testConnection() {
  loading.value = true
  try {
    message.value = await invoke('greet', { name: 'LingJian' })
  } catch (e) {
    message.value = `Error: ${e}`
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="home">
    <section class="hero">
      <h2 class="hero-title">灵鉴 <span class="version">v0.1.0</span></h2>
      <p class="hero-desc">Path of Idle Immortals 日志分析工具</p>
    </section>

    <section class="actions">
      <button class="btn btn-primary" @click="testConnection" :disabled="loading">
        {{ loading ? '连接中...' : '测试后端连接' }}
      </button>
      <p v-if="message" class="response">{{ message }}</p>
    </section>
  </div>
</template>

<style scoped>
.home {
  max-width: 640px;
  margin: 0 auto;
}

.hero {
  text-align: center;
  padding: 3rem 0 2rem;
}

.hero-title {
  font-size: 2rem;
  font-weight: 700;
  color: var(--color-text-bright);
}

.version {
  font-size: 0.875rem;
  font-weight: 400;
  color: var(--color-text-muted);
  vertical-align: super;
}

.hero-desc {
  margin-top: 0.5rem;
  color: var(--color-text-muted);
  font-size: 0.9rem;
}

.actions {
  text-align: center;
  margin-top: 2rem;
}

.btn {
  padding: 0.5rem 1.25rem;
  border: none;
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background-color: var(--color-primary);
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--color-primary-hover);
}

.response {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-family: var(--font-mono);
  font-size: 0.85rem;
  color: var(--color-success);
}
</style>
