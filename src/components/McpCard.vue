<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface McpStatus {
  enabled: boolean
  running: boolean
  port: number
  listeningUrl: string | null
}

const status = ref<McpStatus | null>(null)
const enabled = ref(false)
const port = ref(3920)
const applying = ref(false)
const message = ref<string | null>(null)
const messageType = ref<'success' | 'error'>('success')
const copied = ref(false)

const zcodeConfig = computed(
  () => `"lingjian": {\n  "type": "http",\n  "url": "http://127.0.0.1:${port.value}/mcp"\n}`,
)

async function refresh() {
  try {
    status.value = await invoke<McpStatus>('mcp_status')
    enabled.value = status.value.enabled
    port.value = status.value.port
  } catch (e) {
    showMessage(`状态查询失败: ${e}`, 'error')
  }
}

async function onApply() {
  applying.value = true
  message.value = null
  try {
    status.value = await invoke<McpStatus>('mcp_set_config', {
      enabled: enabled.value,
      port: port.value,
    })
    showMessage(
      status.value.running ? `已启动，监听 ${status.value.listeningUrl}` : 'MCP 已关闭',
      'success',
    )
  } catch (e) {
    showMessage(typeof e === 'string' ? e : String(e), 'error')
  } finally {
    applying.value = false
  }
}

async function onCopyConfig() {
  try {
    await navigator.clipboard.writeText(zcodeConfig.value)
    copied.value = true
    setTimeout(() => (copied.value = false), 2000)
  } catch {
    showMessage('复制失败，请手动选择下方文本', 'error')
  }
}

function showMessage(text: string, type: 'success' | 'error') {
  message.value = text
  messageType.value = type
  if (type !== 'error') {
    setTimeout(() => (message.value = null), 4000)
  }
}

onMounted(refresh)
</script>

<template>
  <section class="card mcp-card">
    <h3 class="card-title">MCP / AI 接入</h3>

    <div v-if="status" class="info-row">
      <span class="info-label">运行状态</span>
      <span :class="['mode-badge', status.running ? 'on' : 'off']">
        {{ status.running ? '运行中' : '已停止' }}
      </span>
      <span v-if="status.running" class="info-url">{{ status.listeningUrl }}</span>
    </div>

    <div class="field">
      <label class="switch-row">
        <input v-model="enabled" type="checkbox" class="switch-input" />
        <span class="switch-label">开放 MCP 服务（供 ZCode 等 AI 工具查询分析结果）</span>
      </label>
    </div>

    <div class="field">
      <label class="field-label">端口</label>
      <input
        v-model.number="port"
        type="number"
        min="1"
        max="65535"
        class="field-input port-input"
      />
      <p class="field-hint">仅监听本机 127.0.0.1，修改后点击应用立即生效</p>
    </div>

    <div class="actions">
      <button class="action-btn" :disabled="applying" @click="onApply">
        {{ applying ? '应用中...' : '应用配置' }}
      </button>
      <button class="action-btn" @click="onCopyConfig">
        {{ copied ? '✓ 已复制' : '复制 ZCode 配置片段' }}
      </button>
    </div>

    <p v-if="message" :class="['message', messageType]">{{ message }}</p>

    <p class="hint">
      将配置片段加入 ZCode 用户配置（~/.zcode/cli/config.json 的 mcp.servers）后，即可在任意
      ZCode 会话中查询灵鉴的分析结果。可用工具：list_issues / get_report / analyze_report /
      query_logs。
    </p>
  </section>
</template>

<style scoped>
.mcp-card {
  margin-bottom: 1rem;
}

.card-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.875rem;
}

.info-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.375rem 0;
  margin-bottom: 0.5rem;
}

.info-label {
  flex-shrink: 0;
  width: 70px;
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.info-url {
  font-size: 0.75rem;
  color: var(--color-text-muted);
  font-family: var(--font-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mode-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.7rem;
  font-weight: 600;
}

.mode-badge.on {
  background-color: rgba(34, 197, 94, 0.15);
  color: var(--color-success);
}

.mode-badge.off {
  background-color: var(--color-surface-alt);
  color: var(--color-text-muted);
}

.field {
  margin-bottom: 0.75rem;
}

.switch-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.switch-input {
  accent-color: var(--color-primary);
}

.switch-label {
  font-size: 0.8125rem;
  color: var(--color-text);
}

.field-label {
  display: block;
  font-size: 0.75rem;
  color: var(--color-text-muted);
  margin-bottom: 0.25rem;
}

.field-input {
  padding: 0.5rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.8125rem;
  font-family: var(--font-mono);
}

.port-input {
  width: 120px;
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
  gap: 0.5rem;
}

.action-btn {
  padding: 0.375rem 0.875rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.75rem;
  font-weight: 500;
}

.action-btn:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.message {
  margin-top: 0.625rem;
  font-size: 0.75rem;
}

.message.success {
  color: var(--color-success);
}

.message.error {
  color: var(--color-danger);
}

.hint {
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-border);
  font-size: 0.7rem;
  color: var(--color-text-muted);
  line-height: 1.6;
}
</style>
