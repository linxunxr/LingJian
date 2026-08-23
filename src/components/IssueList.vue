<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useIssues } from '@/composables/useIssues'
import { isSettingsComplete } from '@/composables/useSettings'
import { formatTime } from '@/utils/format'
import CommentDialog from './CommentDialog.vue'
import LabelDialog from './LabelDialog.vue'
import CloseIssueDialog from './CloseIssueDialog.vue'

const emit = defineEmits<{
  /** 点击某个 Issue（行本身），传出编号 */
  select: [number: string]
}>()

const { state, loadIssues, switchState, loadMore, actOnIssue, clearActionError } = useIssues()

/** SCF 端点是否已配置（未配置时列表区显示引导空态，而非整块空白） */
const configured = computed(() => isSettingsComplete())

/** 当前展开菜单的 Issue 编号（null = 全部收起） */
const openMenu = ref<number | null>(null)

/** 评论对话框目标（仅需编号，内容与提交在 CommentDialog 内） */
const commentTarget = ref<number | null>(null)

/** 标签/关闭确认对话框目标（需携带当前标签供草稿回填/版本标签追加） */
const labelTarget = ref<{ number: number; labels: readonly string[] } | null>(null)
const closeTarget = ref<{ number: number; labels: readonly string[] } | null>(null)

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

/** 关闭/重开：重开直接执行；关闭走 CloseIssueDialog 填版本号 */
function onToggleState(number: number, issueState: string, labels: readonly string[] = []) {
  closeMenu()
  if (issueState === 'closed') {
    actOnIssue(number, 'reopen')
    return
  }
  closeTarget.value = { number, labels }
}

function openComment(number: number) {
  closeMenu()
  commentTarget.value = number
}

function openLabels(number: number, labels: readonly string[] = []) {
  closeMenu()
  labelTarget.value = { number, labels }
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
          :class="['tab', { active: state.state === 'closed' }]"
          :disabled="state.loading"
          @click="switchState('closed')"
        >
          已处理
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

    <!-- 未配置 SCF：显示引导空态（loadIssues 静默跳过，四个常规分支都不会命中） -->
    <div v-else-if="!configured" class="empty empty--guide">
      <p class="empty-title">问题列表需要 SCF 端点配置</p>
      <p class="empty-desc">
        前往 <RouterLink :to="{ name: 'settings' }">设置页</RouterLink> 填写 URL 与 API Key 后即可拉取；
        仅分析本地日志可不配置，直接使用上方「导入本地日志」。
      </p>
    </div>

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
            <button class="menu-item" @click="onToggleState(issue.number, issue.state, issue.labels)">
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

    <!-- 已配置但尚未拉取（如刚保存配置返回首页）：引导手动刷新 -->
    <div v-else class="empty">配置已就绪，点击右上角 ↻ 拉取问题列表</div>

    <!-- 加载更多 -->
    <div v-if="state.hasMore && !state.loading" class="load-more">
      <button class="load-more-btn" :disabled="state.loadingMore" @click="loadMore">
        {{ state.loadingMore ? '加载中...' : '加载更多' }}
      </button>
    </div>

    <!-- 评论对话框 -->
    <CommentDialog
      v-if="commentTarget !== null"
      :issue-number="commentTarget"
      @close="commentTarget = null"
    />

    <!-- 标签对话框 -->
    <LabelDialog
      v-if="labelTarget"
      :issue-number="labelTarget.number"
      :labels="labelTarget.labels"
      @close="labelTarget = null"
    />

    <!-- 关闭确认对话框 -->
    <CloseIssueDialog
      v-if="closeTarget"
      :issue-number="closeTarget.number"
      :labels="closeTarget.labels"
      @close="closeTarget = null"
    />
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

/* 未配置引导空态：占满列表区域，避免大片空白 */
.empty--guide {
  padding: 2.5rem 1.5rem;
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
}

.empty-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 0.5rem;
}

.empty-desc {
  font-size: 0.8125rem;
  line-height: 1.7;
  max-width: 420px;
  margin: 0 auto;
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
</style>
