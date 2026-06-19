<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useSettings, saveSettings, isSettingsComplete } from '@/composables/useSettings'

const emit = defineEmits<{
  done: []
  skip: []
}>()

const { settings } = useSettings()

const step = ref<1 | 2 | 3>(1)
const saving = ref(false)
const error = ref<string | null>(null)

// GitHub 验证状态
const verifyingGithub = ref(false)
const githubResult = ref<{ ok: boolean; msg: string } | null>(null)
// SCF 验证状态
const verifyingScf = ref(false)
const scfResult = ref<{ ok: boolean; msg: string } | null>(null)

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

async function finish() {
  if (!isSettingsComplete()) {
    error.value = '请填写完整的三项配置'
    return
  }
  saving.value = true
  error.value = null
  try {
    await saveSettings()
    emit('done')
  } catch (e) {
    error.value = typeof e === 'string' ? e : String(e)
  } finally {
    saving.value = false
  }
}

function skip() {
  emit('skip')
}
</script>

<template>
  <div class="onboarding-overlay" @click.self="skip">
    <div class="onboarding-modal">
      <div class="modal-header">
        <h2 class="modal-title">欢迎使用灵鉴</h2>
        <button class="close-btn" @click="skip" aria-label="关闭">×</button>
      </div>

      <div class="step-indicator">
        <span :class="['step-dot', { active: step >= 1 }, { current: step === 1 }]">1</span>
        <span class="step-line" />
        <span :class="['step-dot', { active: step >= 2 }, { current: step === 2 }]">2</span>
        <span class="step-line" />
        <span :class="['step-dot', { active: step >= 3 }, { current: step === 3 }]">3</span>
      </div>

      <!-- 步骤 1：GitHub Token -->
      <div v-if="step === 1" class="step-body">
        <h3 class="step-title">配置 GitHub Token</h3>
        <p class="step-desc">
          灵鉴通过 GitHub API 获取 Issue 并解析上报编号。创建一个 Fine-grained Token，
          仅需目标仓库的 Issues 读取权限。
        </p>
        <label class="field-label">GitHub Token</label>
        <input v-model="settings.githubToken" type="password" class="field-input" placeholder="ghp_..." />
        <div class="verify-row">
          <button class="verify-btn" :disabled="verifyingGithub || !settings.githubToken" @click="verifyGithub">
            {{ verifyingGithub ? '验证中...' : '验证连接' }}
          </button>
          <span v-if="githubResult" :class="['verify-result', githubResult.ok ? 'ok' : 'fail']">
            {{ githubResult.ok ? '✓' : '✗' }} {{ githubResult.msg }}
          </span>
        </div>
        <div class="step-actions">
          <button class="ghost-btn" @click="skip">稍后配置</button>
          <button class="primary-btn" @click="step = 2">下一步</button>
        </div>
      </div>

      <!-- 步骤 2：SCF 端点 -->
      <div v-else-if="step === 2" class="step-body">
        <h3 class="step-title">配置 SCF 下载端点</h3>
        <p class="step-desc">
          日志压缩包存储在腾讯云 COS，通过 SCF 函数 URL 下载。填写端点地址和鉴权密钥。
        </p>
        <label class="field-label">SCF URL</label>
        <input v-model="settings.scfUrl" type="text" class="field-input" placeholder="https://xxxx.tencentscf.com" />
        <label class="field-label">API Key</label>
        <input v-model="settings.apiKey" type="password" class="field-input" placeholder="下载端点鉴权密钥" />
        <div class="verify-row">
          <button class="verify-btn" :disabled="verifyingScf || !settings.scfUrl" @click="verifyScf">
            {{ verifyingScf ? '测试中...' : '测试下载' }}
          </button>
          <span v-if="scfResult" :class="['verify-result', scfResult.ok ? 'ok' : 'fail']">
            {{ scfResult.ok ? '✓' : '✗' }} {{ scfResult.msg }}
          </span>
        </div>
        <div class="step-actions">
          <button class="ghost-btn" @click="step = 1">上一步</button>
          <button class="primary-btn" @click="step = 3">下一步</button>
        </div>
      </div>

      <!-- 步骤 3：完成 -->
      <div v-else class="step-body">
        <h3 class="step-title">配置完成</h3>
        <p class="step-desc">
          一切就绪！现在可以在首页输入 Issue URL 或编号开始分析日志。
          凭证已加密存储于系统钥匙串。
        </p>
        <p v-if="error" class="error-msg">{{ error }}</p>
        <div class="step-actions">
          <button class="ghost-btn" @click="step = 2">上一步</button>
          <button class="primary-btn" :disabled="saving" @click="finish">
            {{ saving ? '保存中...' : '开始使用' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.onboarding-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.onboarding-modal {
  width: 520px;
  max-width: 90vw;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--color-border);
}

.modal-title {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--color-text-bright);
}

.close-btn {
  background: none;
  border: none;
  color: var(--color-text-muted);
  font-size: 1.5rem;
  line-height: 1;
  padding: 0 0.25rem;
}

.close-btn:hover {
  color: var(--color-text);
}

.step-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 1rem 1.5rem;
}

.step-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background-color: var(--color-surface-alt);
  color: var(--color-text-muted);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 600;
}

.step-dot.active {
  background-color: var(--color-primary);
  color: #fff;
}

.step-dot.current {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}

.step-line {
  width: 40px;
  height: 1px;
  background-color: var(--color-border);
}

.step-body {
  padding: 0.5rem 1.5rem 1.5rem;
}

.step-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.5rem;
}

.step-desc {
  font-size: 0.8125rem;
  color: var(--color-text-muted);
  line-height: 1.6;
  margin-bottom: 1rem;
}

.field-label {
  display: block;
  font-size: 0.75rem;
  color: var(--color-text-muted);
  margin-bottom: 0.25rem;
}

.field-label + .field-label {
  margin-top: 0.75rem;
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
  margin-bottom: 0.75rem;
}

.field-input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.verify-row {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  margin: 0.5rem 0 1rem;
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

.step-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.625rem;
  margin-top: 1rem;
}

.ghost-btn {
  padding: 0.5rem 1rem;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.8125rem;
}

.ghost-btn:hover {
  color: var(--color-text);
}

.primary-btn {
  padding: 0.5rem 1.25rem;
  background-color: var(--color-primary);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 0.8125rem;
  font-weight: 500;
}

.primary-btn:hover:not(:disabled) {
  background-color: var(--color-primary-hover);
}

.primary-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.error-msg {
  padding: 0.5rem 0.75rem;
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger);
  font-size: 0.75rem;
  margin-bottom: 0.5rem;
}
</style>
