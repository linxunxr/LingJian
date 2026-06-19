<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { RouterView, RouterLink, useRoute, useRouter } from 'vue-router'
import { LazyStore } from '@tauri-apps/plugin-store'
import Onboarding from '@/components/Onboarding.vue'
import { loadSettings, isSettingsComplete } from '@/composables/useSettings'

const route = useRoute()
const router = useRouter()
const showOnboarding = ref(false)

const navItems = [
  { to: '/', name: 'home', label: '首页' },
  { to: '/analyze', name: 'analyze', label: '分析' },
  { to: '/settings', name: 'settings', label: '设置' },
]

const onboardStore = new LazyStore('settings.json')

function onKeyDown(e: KeyboardEvent) {
  // Ctrl+F / Cmd+F：聚焦搜索框（分析页关键词 / 首页 Issue 输入）
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
    const el = document.querySelector<HTMLElement>('[data-shortcut="search"]')
    if (el) {
      e.preventDefault()
      el.focus()
    }
  }
  // Esc：在分析页返回首页
  if (e.key === 'Escape' && route.name === 'analyze') {
    router.push({ name: 'home' })
  }
}

onMounted(async () => {
  window.addEventListener('keydown', onKeyDown)
  await loadSettings()
  // 首次启动：未标记完成且配置不完整时弹出引导
  const onboarded = (await onboardStore.get<boolean>('onboarded')) ?? false
  if (!onboarded && !isSettingsComplete()) {
    showOnboarding.value = true
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
})

async function closeOnboarding() {
  await onboardStore.set('onboarded', true)
  await onboardStore.save()
  showOnboarding.value = false
}
</script>

<template>
  <div class="app">
    <header class="app-header">
      <div class="app-brand">
        <h1 class="app-title">灵鉴</h1>
        <span class="app-subtitle">日志分析工具</span>
      </div>
      <nav class="app-nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.name"
          :to="item.to"
          :class="['nav-link', { active: route.name === item.name }]"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
    </header>
    <main class="app-main">
      <RouterView />
    </main>
    <Onboarding v-if="showOnboarding" @done="closeOnboarding" @skip="closeOnboarding" />
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
  align-items: center;
  justify-content: space-between;
  padding: 0 1.5rem;
  height: 48px;
  background-color: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  -webkit-user-select: none;
  user-select: none;
}

.app-brand {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
}

.app-title {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--color-text-bright);
  margin: 0;
}

.app-subtitle {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.app-nav {
  display: flex;
  gap: 0.25rem;
}

.nav-link {
  padding: 0.375rem 0.75rem;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--color-text-muted);
  border-radius: var(--radius-sm);
  transition: color var(--transition-fast), background-color var(--transition-fast);
}

.nav-link:hover {
  color: var(--color-text);
  background-color: var(--color-surface-alt);
}

.nav-link.active {
  color: var(--color-primary);
  background-color: rgba(59, 130, 246, 0.1);
}

.app-main {
  flex: 1;
  overflow: auto;
  padding: 1.5rem;
}
</style>
