<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

const message = ref('')
const loading = ref(false)

async function testGreet() {
  loading.value = true
  try {
    message.value = await invoke('greet', { name: '修仙者' })
  } catch (e) {
    message.value = `错误: ${e}`
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
      <button class="btn btn-gold" @click="testGreet" :disabled="loading">
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
  color: var(--color-gold);
  letter-spacing: 0.15em;
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
  padding: 0.6rem 1.5rem;
  border: none;
  border-radius: var(--radius-md);
  font-size: 0.9rem;
  cursor: pointer;
  transition: all 0.2s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-gold {
  background: linear-gradient(135deg, var(--color-gold-dim), var(--color-gold));
  color: #0a0e1a;
  font-weight: 600;
}

.btn-gold:hover:not(:disabled) {
  background: linear-gradient(135deg, var(--color-gold), var(--color-gold-bright));
  box-shadow: 0 0 16px rgba(212, 168, 67, 0.3);
}

.response {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-family: var(--font-mono);
  font-size: 0.85rem;
  color: var(--color-success);
}
</style>
