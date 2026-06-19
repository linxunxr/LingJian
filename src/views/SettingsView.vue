<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useSettings, saveSettings } from '@/composables/useSettings'

const { settings, loadSettings } = useSettings()

const saving = ref(false)
const saved = ref(false)
const error = ref<string | null>(null)

async function onSave() {
  saving.value = true
  saved.value = false
  error.value = null
  try {
    await saveSettings()
    saved.value = true
    setTimeout(() => (saved.value = false), 2000)
  } catch (e) {
    error.value = typeof e === 'string' ? e : String(e)
  } finally {
    saving.value = false
  }
}

onMounted(loadSettings)
</script>

<template>
  <div class="settings">
    <h2 class="settings-title">设置</h2>

    <section class="card">
      <h3 class="card-title">GitHub 配置</h3>
      <div class="field">
        <label class="field-label">Token</label>
        <input v-model="settings.githubToken" type="password" class="field-input" placeholder="ghp_..." />
        <p class="field-hint">用于通过 GitHub API 获取 Issue 并解析 reportId（仅需 issues: read 权限）</p>
      </div>
    </section>

    <section class="card">
      <h3 class="card-title">SCF 下载端点</h3>
      <div class="field">
        <label class="field-label">URL</label>
        <input v-model="settings.scfUrl" type="text" class="field-input" placeholder="https://xxxx.tencentscf.com" />
      </div>
      <div class="field">
        <label class="field-label">API Key</label>
        <input v-model="settings.apiKey" type="password" class="field-input" placeholder="下载端点鉴权密钥" />
      </div>
    </section>

    <div class="actions">
      <button class="save-btn" :disabled="saving" @click="onSave">
        {{ saving ? '保存中...' : '保存设置' }}
      </button>
      <span v-if="saved" class="saved-tip">✓ 已保存</span>
      <span v-else-if="error" class="error-tip">{{ error }}</span>
    </div>

    <section class="card about">
      <h3 class="card-title">关于</h3>
      <p>灵鉴 LingJian v0.1.0</p>
      <p class="muted">Path of Idle Immortals 日志分析工具</p>
    </section>
  </div>
</template>

<style scoped>
.settings {
  max-width: 600px;
  margin: 0 auto;
}

.settings-title {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--color-text-bright);
  margin-bottom: 1.25rem;
}

.card {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 1.125rem 1.25rem;
  margin-bottom: 1rem;
}

.card-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.875rem;
}

.field {
  margin-bottom: 0.75rem;
}

.field:last-child {
  margin-bottom: 0;
}

.field-label {
  display: block;
  font-size: 0.75rem;
  color: var(--color-text-muted);
  margin-bottom: 0.25rem;
}

.field-input {
  width: 100%;
  padding: 0.5rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.8125rem;
  font-family: var(--font-mono);
}

.field-input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.field-hint {
  margin-top: 0.25rem;
  font-size: 0.7rem;
  color: var(--color-text-muted);
}

.actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.save-btn {
  padding: 0.5rem 1.25rem;
  background-color: var(--color-primary);
  color: #fff;
  border: none;
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
  font-weight: 500;
}

.save-btn:hover:not(:disabled) {
  background-color: var(--color-primary-hover);
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.saved-tip {
  color: var(--color-success);
  font-size: 0.75rem;
}

.error-tip {
  color: var(--color-danger);
  font-size: 0.75rem;
}

.about {
  font-size: 0.8125rem;
  color: var(--color-text);
  line-height: 1.6;
}

.muted {
  color: var(--color-text-muted);
  font-size: 0.75rem;
}
</style>
