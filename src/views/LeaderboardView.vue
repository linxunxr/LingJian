<script setup lang="ts">
defineOptions({ name: 'LeaderboardView' })
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { useLeaderboard } from '@/composables/useLeaderboard'
import { isSettingsComplete } from '@/composables/useSettings'
import { formatTime } from '@/utils/format'

const { state, loadLeaderboard, exportLeaderboardCsv } = useLeaderboard()

const settingsReady = computed(() => isSettingsComplete())
/** 导出成功提示（短暂展示后自动清除） */
const exportedPath = ref<string | null>(null)
let exportedTimer: number | undefined

/** 玩家身份来源徽标：steam 前缀可作发奖凭据，device 仅为设备兜底 */
function idBadge(playerId: string): string {
  return playerId.startsWith('steam:') ? 'Steam' : '设备'
}

async function onExport() {
  exportedPath.value = null
  const path = await exportLeaderboardCsv(state.entries, state.unidentified)
  if (path) {
    exportedPath.value = path
    window.clearTimeout(exportedTimer)
    exportedTimer = window.setTimeout(() => (exportedPath.value = null), 5000)
  }
}

onMounted(() => {
  // 配置完整才拉取；未配置时视图显示引导提示
  if (settingsReady.value) loadLeaderboard()
})
</script>

<template>
  <div class="leaderboard">
    <div class="toolbar">
      <div class="toolbar-text">
        <h2 class="title">反馈排行榜</h2>
        <p class="subtitle">
          按玩家聚合全部反馈 Issue，可用于 alpha/beta 测试奖励发放。Steam 身份可直接对应账号，设备身份仅统计参考。
        </p>
      </div>
      <div class="toolbar-actions">
        <button class="btn" :disabled="state.loading || !settingsReady" @click="loadLeaderboard()">
          {{ state.loading ? '加载中…' : '刷新' }}
        </button>
        <button
          class="btn btn-primary"
          :disabled="state.loading || state.entries.length === 0"
          @click="onExport"
        >
          导出 CSV
        </button>
      </div>
    </div>

    <p v-if="exportedPath" class="exported-tip">已导出到：{{ exportedPath }}</p>
    <p v-if="state.error" class="error">{{ state.error }}</p>

    <div v-if="!settingsReady" class="empty">
      未配置 SCF 端点，请先到<RouterLink :to="{ name: 'settings' }" class="empty-link">设置页</RouterLink>填写后再查看排行榜。
    </div>

    <template v-else>
      <div class="summary">
        <div class="summary-item">
          <span class="summary-value">{{ state.totalIssues }}</span>
          <span class="summary-label">反馈总数</span>
        </div>
        <div class="summary-item">
          <span class="summary-value">{{ state.entries.length }}</span>
          <span class="summary-label">参与玩家</span>
        </div>
        <div class="summary-item">
          <span class="summary-value">{{ state.unidentified }}</span>
          <span class="summary-label">未标识（老数据）</span>
        </div>
        <div v-if="state.loadedAt" class="summary-item">
          <span class="summary-value summary-time">{{ formatTime(state.loadedAt) }}</span>
          <span class="summary-label">数据截至</span>
        </div>
      </div>

      <div v-if="state.loading && state.entries.length === 0" class="empty">正在拉取全部反馈…</div>
      <div v-else-if="!state.loading && state.entries.length === 0" class="empty">
        暂无带玩家标识的反馈（身份标识自游戏 v0.10.14 起随上报携带，之前的反馈无法归属到人）。
      </div>

      <div v-else class="table-wrap">
        <table class="lb-table">
          <thead>
            <tr>
              <th class="col-rank">排名</th>
              <th>玩家</th>
              <th class="col-count">反馈数</th>
              <th class="col-time">首次上报</th>
              <th class="col-time">最近上报</th>
              <th>版本跨度</th>
              <th>Issue</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(entry, i) in state.entries" :key="entry.playerId">
              <td class="col-rank" :class="{ 'rank-top': i < 3 }">{{ i + 1 }}</td>
              <td>
                <div class="player">
                  <span class="player-name">{{ entry.playerName || '（未留名）' }}</span>
                  <span class="player-id">
                    <span class="badge" :class="{ 'badge-steam': idBadge(entry.playerId) === 'Steam' }">
                      {{ idBadge(entry.playerId) }}
                    </span>
                    {{ entry.playerId }}
                  </span>
                </div>
              </td>
              <td class="col-count count">{{ entry.count }}</td>
              <td class="col-time muted">{{ formatTime(entry.firstAt) }}</td>
              <td class="col-time muted">{{ formatTime(entry.lastAt) }}</td>
              <td class="muted versions">{{ entry.versions.join(' / ') }}</td>
              <td class="issues">
                <a
                  v-for="(url, j) in entry.issueUrls"
                  :key="url"
                  :href="url"
                  target="_blank"
                  rel="noopener"
                  class="issue-link"
                >#{{ entry.issueNumbers[j] }}</a>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>

<style scoped>
.leaderboard {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.title {
  margin: 0;
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--color-text-bright);
}

.subtitle {
  margin: 0.25rem 0 0;
  font-size: 0.8125rem;
  color: var(--color-text-muted);
  max-width: 60ch;
}

.toolbar-actions {
  display: flex;
  gap: 0.5rem;
  flex-shrink: 0;
}

.btn {
  padding: 0.375rem 0.875rem;
  font-size: 0.8125rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background-color: var(--color-surface);
  color: var(--color-text);
  cursor: pointer;
  transition: background-color var(--transition-fast);
}

.btn:hover:not(:disabled) {
  background-color: var(--color-surface-alt);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--color-primary);
  opacity: 0.9;
}

.exported-tip {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-success);
}

.error {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-danger);
}

.empty {
  padding: 2rem;
  text-align: center;
  font-size: 0.875rem;
  color: var(--color-text-muted);
  background-color: var(--color-surface);
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
}

.empty-link {
  color: var(--color-primary);
}

.summary {
  display: flex;
  gap: 1rem;
}

.summary-item {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
  padding: 0.875rem 1rem;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.summary-value {
  font-size: 1.375rem;
  font-weight: 700;
  color: var(--color-text-bright);
}

.summary-time {
  font-size: 0.9375rem;
}

.summary-label {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.table-wrap {
  overflow-x: auto;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.lb-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.8125rem;
}

.lb-table th,
.lb-table td {
  padding: 0.5rem 0.75rem;
  text-align: left;
  border-bottom: 1px solid var(--color-border);
  vertical-align: middle;
}

.lb-table thead th {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-text-muted);
  background-color: var(--color-surface-alt);
  position: sticky;
  top: 0;
}

.lb-table tbody tr:last-child td {
  border-bottom: none;
}

.lb-table tbody tr:hover {
  background-color: var(--color-surface-alt);
}

.col-rank {
  width: 3rem;
  text-align: center;
  font-variant-numeric: tabular-nums;
  color: var(--color-text-muted);
}

.rank-top {
  color: var(--color-warning);
  font-weight: 700;
}

.col-count {
  width: 4.5rem;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.count {
  font-weight: 700;
  color: var(--color-text-bright);
}

.col-time {
  white-space: nowrap;
}

.muted {
  color: var(--color-text-muted);
}

.player {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.player-name {
  font-weight: 600;
  color: var(--color-text);
}

.player-id {
  font-size: 0.6875rem;
  font-family: var(--font-mono, monospace);
  color: var(--color-text-muted);
  word-break: break-all;
}

.badge {
  display: inline-block;
  padding: 0 0.375rem;
  margin-right: 0.25rem;
  font-size: 0.6875rem;
  line-height: 1.25;
  border-radius: var(--radius-sm);
  background-color: var(--color-surface-alt);
  border: 1px solid var(--color-border);
}

.badge-steam {
  color: var(--color-primary);
  border-color: var(--color-primary);
}

.versions {
  max-width: 14ch;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.issues {
  max-width: 18ch;
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
}

.issue-link {
  color: var(--color-primary);
  text-decoration: none;
}

.issue-link:hover {
  text-decoration: underline;
}
</style>
