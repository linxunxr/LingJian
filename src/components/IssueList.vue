<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useIssues } from '@/composables/useIssues'
import { formatTime } from '@/utils/format'

const emit = defineEmits<{
  /** 点击某个 Issue（行本身），传出编号 */
  select: [number: string]
}>()

const { state, loadIssues, switchState, loadMore, actOnIssue, clearActionError } = useIssues()

/** 预设标签（一期硬编码，后续可配置化） */
const PRESET_LABELS = ['已修复', '无法复现', '高优先级', '待验证'] as const

/** 当前展开菜单的 Issue 编号（null = 全部收起） */
const openMenu = ref<number | null>(null)

/** 对话框仅需 number 标识目标，避免持有 readonly 列表项 */
const commentTarget = ref<{ number: number } | null>(null)
const commentText = ref('')

const labelTarget = ref<{ number: number } | null>(null)
const labelDraft = ref<string[]>([])

function onSelect(number: number) {
  if (openMenu.value !== null) {
    // 菜单展开时，行点击不触发分析（避免误跳转）
    openMenu.value = null
    return
  }
  emit('select', String(number))
}

function toggleMenu(number: number) {
  openMenu.value = openMenu.value === number ? null : number
}

function closeMenu() {
  openMenu.value = null
}

/** 关闭/重开 */
async function onToggleState(number: number, issueState: string) {
  closeMenu()
  const action = issueState === 'closed' ? 'reopen' : 'close'
  await actOnIssue(number, action)
}

/** 打开评论对话框 */
function openComment(number: number) {
  closeMenu()
  commentTarget.value = { number }
  commentText.value = ''
}

async function submitComment() {
  if (!commentTarget.value || !commentText.value.trim()) return
  const ok = await actOnIssue(commentTarget.value.number, 'comment', {
    body: commentText.value.trim(),
  })
  if (ok) {
    commentTarget.value = null
    commentText.value = ''
  }
}

/** 打开标签编辑（加载当前标签到 draft） */
function openLabels(number: number, labels: readonly string[] = []) {
  closeMenu()
  labelTarget.value = { number }
  // 当前标签拷贝一份（预设区只勾选匹配项，提交时整体替换）
  labelDraft.value = [...labels]
}

function togglePresetLabel(label: string) {
  const idx = labelDraft.value.indexOf(label)
  if (idx >= 0) {
    labelDraft.value.splice(idx, 1)
  } else {
    labelDraft.value.push(label)
  }
}

async function submitLabels() {
  if (!labelTarget.value) return
  const ok = await actOnIssue(labelTarget.value.number, 'setLabels', {
    labels: labelDraft.value,
  })
  if (ok) {
    labelTarget.value = null
    labelDraft.value = []
  }
}

/** 点页面其他位置关闭菜单 */
function onDocClick(e: MouseEvent) {
  if (openMenu.value === null) return
  const target = e.target as HTMLElement
  if (!target.closest('.action-cell')) {
    openMenu.value = null
  }
}

onMounted(() => document.addEventListener('click', onDocClick))
onUnmounted(() => document.removeEventListener('click', onDocClick))
</script>

<template>
  <section class="issues">
    <div class="issues-header">
      <h3 class="section-title">问题列表</h3>
      <div class="tabs">
        <button
          :class="['tab', { active: state.state === 'open' }]"
          :disabled="state.loading"
          @click="switchState('open')"
        >
          未处理
        </button>
        <button
          :class="['tab', { active: state.state === 'all' }]"
          :disabled="state.loading"
          @click="switchState('all')"
        >
          全部
        </button>
        <button class="refresh-btn" :disabled="state.loading" @click="loadIssues" title="刷新">
          ↻
        </button>
      </div>
    </div>

    <!-- 操作错误提示（独立于列表加载错误） -->
    <div v-if="state.actionError" class="action-error">
      {{ state.actionError }}
      <button class="dismiss" @click="clearActionError">×</button>
    </div>

    <!-- 加载中（首拉） -->
    <div v-if="state.loading" class="empty">加载中...</div>

    <!-- 列表加载错误 -->
    <div v-else-if="state.error" class="error-msg">{{ state.error }}</div>

    <!-- 空态 -->
    <div v-else-if="state.loaded && state.issues.length === 0" class="empty">暂无上报问题</div>

    <!-- 列表 -->
    <ul v-else-if="state.issues.length > 0" class="issue-list">
      <li
        v-for="issue in state.issues"
        :key="issue.number"
        class="issue-item"
        @click="onSelect(issue.number)"
      >
        <span class="issue-number">#{{ issue.number }}</span>
        <span class="issue-title">
          {{ issue.title || '(无标题)' }}
          <span v-if="issue.labels && issue.labels.length" class="label-chips">
            <span v-for="lab in issue.labels" :key="lab" class="label-chip">{{ lab }}</span>
          </span>
        </span>
        <span :class="['issue-state', issue.state]">
          {{ issue.state === 'closed' ? '已关闭' : '未处理' }}
        </span>
        <span class="issue-time">{{ formatTime(issue.createdAt) }}</span>

        <!-- 操作菜单 -->
        <div class="action-cell" @click.stop>
          <button
            class="action-btn"
            :disabled="state.actingNumber === issue.number"
            :title="state.actingNumber === issue.number ? '处理中...' : '操作'"
            @click="toggleMenu(issue.number)"
          >
            {{ state.actingNumber === issue.number ? '⋯' : '⋮' }}
          </button>
          <div v-if="openMenu === issue.number" class="action-menu">
            <button class="menu-item" @click="onToggleState(issue.number, issue.state)">
              {{ issue.state === 'closed' ? '↻ 重新打开' : '✓ 关闭 Issue' }}
            </button>
            <button class="menu-item" @click="openComment(issue.number)">💬 添加评论</button>
            <button class="menu-item" @click="openLabels(issue.number, issue.labels)">
              🏷 管理标签
            </button>
          </div>
        </div>
      </li>
    </ul>

    <!-- 加载更多 -->
    <div v-if="state.hasMore && !state.loading" class="load-more">
      <button class="load-more-btn" :disabled="state.loadingMore" @click="loadMore">
        {{ state.loadingMore ? '加载中...' : '加载更多' }}
      </button>
    </div>

    <!-- 评论对话框 -->
    <div v-if="commentTarget" class="dialog-overlay" @click.self="commentTarget = null">
      <div class="dialog">
        <h4 class="dialog-title">添加评论 · #{{ commentTarget.number }}</h4>
        <textarea
          v-model="commentText"
          class="dialog-textarea"
          placeholder="输入评论内容（支持 Markdown）..."
          rows="4"
          autofocus
        />
        <div class="dialog-actions">
          <button class="ghost-btn" @click="commentTarget = null">取消</button>
          <button class="primary-btn" :disabled="!commentText.trim()" @click="submitComment">
            提交
          </button>
        </div>
      </div>
    </div>

    <!-- 标签对话框 -->
    <div v-if="labelTarget" class="dialog-overlay" @click.self="labelTarget = null">
      <div class="dialog">
        <h4 class="dialog-title">管理标签 · #{{ labelTarget.number }}</h4>
        <div class="label-editor">
          <p class="label-section-title">快速切换</p>
          <div class="preset-labels">
            <label
              v-for="lab in PRESET_LABELS"
              :key="lab"
              :class="['preset-label', { checked: labelDraft.includes(lab) }]"
            >
              <input
                type="checkbox"
                :checked="labelDraft.includes(lab)"
                @change="togglePresetLabel(lab)"
              />
              {{ lab }}
            </label>
          </div>
          <p class="label-section-title">当前全部标签</p>
          <p class="label-current">{{ labelDraft.length ? labelDraft.join('、') : '（无）' }}</p>
          <p class="label-hint">预设标签为切换开关；当前列表为整体替换结果（提交后覆盖远端）</p>
        </div>
        <div class="dialog-actions">
          <button class="ghost-btn" @click="labelTarget = null">取消</button>
          <button class="primary-btn" @click="submitLabels">保存</button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.issues-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}

.section-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
}

.tabs {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.tab {
  padding: 0.25rem 0.75rem;
  background-color: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.75rem;
  transition: all var(--transition-fast);
}

.tab:hover:not(:disabled):not(.active) {
  color: var(--color-text);
  border-color: var(--color-text-muted);
}

.tab.active {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
}

.tab:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.refresh-btn {
  margin-left: 0.375rem;
  padding: 0.25rem 0.5rem;
  background-color: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 0.85rem;
  line-height: 1;
  transition: all var(--transition-fast);
}

.refresh-btn:hover:not(:disabled) {
  color: var(--color-primary);
  border-color: var(--color-primary);
}

.refresh-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.issue-list {
  list-style: none;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: visible;
  margin: 0;
  padding: 0;
}

.issue-item {
  display: grid;
  grid-template-columns: 60px 1fr 70px 150px 36px;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  font-size: 0.8125rem;
  cursor: pointer;
  transition: background-color var(--transition-fast);
  border-bottom: 1px solid var(--color-border);
}

.issue-item:last-child {
  border-bottom: none;
}

.issue-item:hover {
  background-color: var(--color-surface-alt);
}

.issue-number {
  color: var(--color-primary);
  font-weight: 600;
  font-family: var(--font-mono);
}

.issue-title {
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.label-chips {
  display: inline-flex;
  gap: 0.25rem;
  flex-shrink: 0;
}

.label-chip {
  font-size: 0.65rem;
  padding: 0.0625rem 0.375rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
}

.issue-state {
  font-size: 0.7rem;
  text-align: center;
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  border: 1px solid var(--color-border);
}

.issue-state.open {
  color: var(--color-warning);
  border-color: var(--color-warning);
}

.issue-state.closed {
  color: var(--color-success);
  border-color: var(--color-success);
}

.issue-time {
  color: var(--color-text-muted);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  text-align: right;
}

/* 操作菜单 */
.action-cell {
  position: relative;
  display: flex;
  justify-content: center;
}

.action-btn {
  width: 28px;
  height: 28px;
  background-color: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: 1rem;
  line-height: 1;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.action-btn:hover:not(:disabled) {
  background-color: var(--color-surface);
  border-color: var(--color-border);
  color: var(--color-text);
}

.action-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.action-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 0.25rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  z-index: 50;
  min-width: 140px;
  padding: 0.25rem 0;
}

.menu-item {
  display: block;
  width: 100%;
  padding: 0.5rem 0.875rem;
  background: transparent;
  border: none;
  text-align: left;
  color: var(--color-text);
  font-size: 0.8125rem;
  cursor: pointer;
  transition: background-color var(--transition-fast);
}

.menu-item:hover {
  background-color: var(--color-surface-alt);
}

/* 操作错误提示 */
.action-error {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
  padding: 0.5rem 0.75rem;
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  font-size: 0.75rem;
}

.dismiss {
  background: none;
  border: none;
  color: var(--color-danger);
  font-size: 1rem;
  cursor: pointer;
  line-height: 1;
}

.empty {
  padding: 2rem 0;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 0.85rem;
}

.error-msg {
  padding: 0.625rem 0.875rem;
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  font-size: 0.8125rem;
}

.load-more {
  display: flex;
  justify-content: center;
  margin-top: 0.75rem;
}

.load-more-btn {
  padding: 0.375rem 1.25rem;
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.75rem;
  transition: all var(--transition-fast);
}

.load-more-btn:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.load-more-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 对话框 */
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

.dialog-textarea {
  width: 100%;
  padding: 0.625rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.8125rem;
  font-family: var(--font-mono);
  resize: vertical;
  min-height: 80px;
}

.dialog-textarea:focus {
  outline: none;
  border-color: var(--color-primary);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.625rem;
  margin-top: 1rem;
}

.ghost-btn {
  padding: 0.4rem 0.875rem;
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
  padding: 0.4rem 1rem;
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

/* 标签编辑器 */
.label-editor {
  font-size: 0.8125rem;
}

.label-section-title {
  font-size: 0.75rem;
  color: var(--color-text-muted);
  margin: 0.5rem 0 0.375rem;
}

.preset-labels {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.preset-label {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.625rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  color: var(--color-text-muted);
  cursor: pointer;
  user-select: none;
}

.preset-label.checked {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
}

.preset-label input[type='checkbox'] {
  margin: 0;
  accent-color: currentColor;
}

.label-current {
  font-size: 0.8125rem;
  color: var(--color-text);
  margin: 0.25rem 0;
}

.label-hint {
  font-size: 0.7rem;
  color: var(--color-text-muted);
  margin-top: 0.5rem;
  line-height: 1.5;
}
</style>
