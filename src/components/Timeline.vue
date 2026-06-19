<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import {
  Chart,
  ScatterController,
  PointElement,
  LinearScale,
  TimeScale,
  Tooltip,
  Legend,
  type ChartConfiguration,
  type ChartData,
} from 'chart.js'
import 'chartjs-adapter-date-fns'
import { zhCN } from 'date-fns/locale'

import type { TimelinePoint } from '@/types'
import { formatTime } from '@/utils/format'

Chart.register(ScatterController, PointElement, LinearScale, TimeScale, Tooltip, Legend)

const props = defineProps<{
  points: readonly TimelinePoint[]
}>()

const canvasRef = ref<HTMLCanvasElement | null>(null)
let chart: Chart | null = null

/** Y 轴层级到标签的映射 */
const Y_LABELS: Record<number, string> = {
  1: 'ERROR',
  2: 'WARN',
}

/** 散点数据，按级别分组为两个 dataset */
const chartData = computed<ChartData<'scatter'>>(() => {
  const errorPoints = props.points
    .filter((p) => p.level === 'ERROR')
    .map((p) => ({ x: new Date(p.timestamp).getTime(), y: 1, message: p.message }))
  const warnPoints = props.points
    .filter((p) => p.level === 'WARN')
    .map((p) => ({ x: new Date(p.timestamp).getTime(), y: 2, message: p.message }))

  return {
    datasets: [
      {
        label: 'ERROR',
        data: errorPoints,
        backgroundColor: '#EF4444',
        pointRadius: 5,
        pointHoverRadius: 7,
      },
      {
        label: 'WARN',
        data: warnPoints,
        backgroundColor: '#F59E0B',
        pointRadius: 4,
        pointHoverRadius: 6,
      },
    ],
  }
})

function buildConfig(): ChartConfiguration<'scatter'> {
  return {
    type: 'scatter',
    data: chartData.value,
    options: {
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      plugins: {
        legend: {
          labels: { color: '#94A3B8', font: { size: 11 } },
        },
        tooltip: {
          callbacks: {
            title: (items) => {
              const raw = items[0]?.raw as { x: number }
              return formatTime(new Date(raw.x).toISOString())
            },
            label: (item) => {
              const raw = item.raw as { message: string }
              return raw.message
            },
          },
        },
      },
      scales: {
        x: {
          type: 'time',
          time: {
            tooltipFormat: 'yyyy-MM-dd HH:mm:ss',
            displayFormats: { minute: 'HH:mm', hour: 'MM-dd HH:mm' },
          },
          adapters: { date: { locale: zhCN } },
          ticks: { color: '#94A3B8', font: { size: 10 } },
          grid: { color: 'rgba(148, 163, 184, 0.1)' },
        },
        y: {
          min: 0.5,
          max: 2.5,
          ticks: {
            color: '#94A3B8',
            font: { size: 10 },
            stepSize: 1,
            callback: (val) => Y_LABELS[Number(val)] ?? '',
          },
          grid: { color: 'rgba(148, 163, 184, 0.1)' },
        },
      },
    },
  }
}

function renderChart() {
  if (!canvasRef.value) return
  chart?.destroy()
  chart = new Chart(canvasRef.value, buildConfig())
}

watch(
  () => props.points,
  () => renderChart(),
  { deep: true },
)

onMounted(renderChart)
onUnmounted(() => chart?.destroy())
</script>

<template>
  <div class="timeline">
    <div class="timeline__header">
      <span class="timeline__title">错误时间线</span>
      <span v-if="points.length === 0" class="timeline__empty-hint">暂无 WARN/ERROR</span>
    </div>
    <div class="timeline__canvas-wrap">
      <canvas v-show="points.length > 0" ref="canvasRef" />
      <div v-if="points.length === 0" class="timeline__empty">
        暂无 WARN/ERROR 日志
      </div>
    </div>
  </div>
</template>

<style scoped>
.timeline {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.75rem 1rem;
}

.timeline__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}

.timeline__title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--color-text);
}

.timeline__empty-hint {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.timeline__canvas-wrap {
  position: relative;
  height: 180px;
}

.timeline__empty {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-muted);
  font-size: 0.8125rem;
}
</style>
