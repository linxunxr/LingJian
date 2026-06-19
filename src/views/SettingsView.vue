<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useSettings, saveSettings } from '@/composables/useSettings'

const { settings, loadSettings } = useSettings()

const saving = ref(false)
const saved = ref(false)
const error = ref<string | null>(null)

// GitHub 验证状态
const verifyingGithub = ref(false)
const githubResult = ref<{ ok: boolean; msg: string } | null>(null)

// SCF 验证状态
const verifyingScf = ref(false)
const scfResult = ref<{ ok: boolean; msg: string } | null>(null)

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

async function verifyGithub() {
  verifyingGithub.value = true
  githubResult.value = null
  try {
    const login = await invoke<string>('verify_github_token', {
      githubToken: settings.githubToken,
    })
    githubResult.value = { ok: true, msg: `验证通过，账号: ${login}` }
  } catch (e) {
    githubResult.value = { ok: false, msg: typeof e === 'string' ? e : String(e) }
  } finally {
    verifyingGithub.value = false
  }
}

async function verifyScf() {
  verifyingScf.value = true
  scfResult.value = null
  try {
    await invoke('test_scf_endpoint', {
      scfUrl: settings.scfUrl,
      apiKey: settings.apiKey,
    })
    scfResult.value = { ok: true, msg: '端点连通，鉴权配置正确' }
  } catch (e) {
    scfResult.value = { ok: false, msg: typeof e === 'string' ? e : String(e) }
  } finally {
    verifyingScf.value = false
  }
}

onMounted(loadSettings)
</script>

<template>
  <div class="settings">
    <h2 class="settings-title">设置</h2>
    <p class="settings-hint">
      Token 和 API Key 加密存储于系统钥匙串，SCF URL 存于本地配置文件
    </p>

    <section class="card">
      <h3 class="card-title">GitHub 配置</h3>
      <div class="field">
        <label class="field-label">Token</label>
        <input v-model="settings.githubToken" type="password" class="field-input" placeholder="ghp_..." />
        <p class="field-hint">用于通过 GitHub API 获取 Issue 并解析 reportId（仅需 issues: read 权限）</p>
      </div>
      <div class="verify-row">
        <button class="verify-btn" :disabled="verifyingGithub" @click="verifyGithub">
          {{ verifyingGithub ? '验证中...' : '验证连接' }}
        </button>
        <span v-if="githubResult" :class="['verify-result', githubResult.ok ? 'ok' : 'fail']">
          {{ githubResult.ok ? '✓' : '✗' }} {{ githubResult.msg }}
        </span>
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
      <div class="verify-row">
        <button class="verify-btn" :disabled="verifyingScf" @click="verifyScf">
          {{ verifyingScf ? '测试中...' : '测试下载' }}
        </button>
        <span v-if="scfResult" :class="['verify-result', scfResult.ok ? 'ok' : 'fail']">
          {{ scfResult.ok ? '✓' : '✗' }} {{ scfResult.msg }}
        </span>
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
  margin-bottom: 0.25rem;
}

.settings-hint {
  font-size: 0.7rem;
  color: var(--color-text-muted);
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

.verify-row {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  margin-top: 0.625rem;
}

.verify-btn {
  padding: 0.375rem 0.875rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.75rem;
  font-weight: 500;
}

.verify-btn:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.verify-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.verify-result {
  font-size: 0.75rem;
}

.verify-result.ok {
  color: var(--color-success);
}

.verify-result.fail {
  color: var(--color-danger);
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
