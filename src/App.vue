<script setup lang="ts">
import { RouterView, RouterLink, useRoute } from 'vue-router'

const route = useRoute()
</script>

<template>
  <div class="app">
    <header class="app-header">
      <h1 class="app-title">灵鉴</h1>
      <span class="app-subtitle">日志分析工具</span>
      <nav class="app-nav">
        <RouterLink to="/" :class="{ active: route.name === 'home' }">首页</RouterLink>
        <RouterLink to="/analyze" :class="{ active: route.name === 'analyze' }">分析</RouterLink>
        <RouterLink to="/settings" :class="{ active: route.name === 'settings' }">设置</RouterLink>
      </nav>
    </header>
    <main class="app-main">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background-color: var(--color-bg);
  color: var(--color-text);
}

.app-header {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  padding: 0.75rem 1.5rem;
  background-color: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  -webkit-user-select: none;
  user-select: none;
}

.app-title {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--color-gold);
  margin: 0;
  letter-spacing: 0.1em;
}

.app-subtitle {
  font-size: 0.8rem;
  color: var(--color-text-muted);
}

.app-main {
  flex: 1;
  overflow: auto;
  padding: 1.5rem;
}
</style>
