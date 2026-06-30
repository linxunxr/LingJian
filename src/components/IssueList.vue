<script setup lang="ts">
import { useIssues } from '@/composables/useIssues'
import { formatTime } from '@/utils/format'

const emit = defineEmits<{
  /** 点击某个 Issue，传出编号（字符串便于直接喂给 runAnalysis） */
  select: [number: string]
}>()

const { state, loadIssues, switchState, loadMore } = useIssues()

function onSelect(number: number) {
  emit('select', String(number))
}
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

    <!-- 加载中（首拉） -->
    <div v-if="state.loading" class="empty">加载中...</div>

    <!-- 错误态 -->
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
        <span class="issue-title">{{ issue.title || '(无标题)' }}</span>
        <span :class="['issue-state', issue.state]">
          {{ issue.state === 'closed' ? '已关闭' : '未处理' }}
        </span>
        <span class="issue-time">{{ formatTime(issue.createdAt) }}</span>
      </li>
    </ul>

    <!-- 加载更多 -->
    <div v-if="state.hasMore && !state.loading" class="load-more">
      <button class="load-more-btn" :disabled="state.loadingMore" @click="loadMore">
        {{ state.loadingMore ? '加载中...' : '加载更多' }}
      </button>
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
  overflow: hidden;
  margin: 0;
  padding: 0;
}

.issue-item {
  display: grid;
  grid-template-columns: 60px 1fr 70px 160px;
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
</style>
