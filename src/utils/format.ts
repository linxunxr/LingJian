import type { LogLevel } from '@/types'

/** 格式化 ISO 时间戳为可读字符串 */
export function formatTime(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

/** 级别对应的 CSS 颜色变量名 */
export function levelColorVar(level: LogLevel): string {
  switch (level) {
    case 'DEBUG':
      return 'var(--color-text-muted)'
    case 'INFO':
      return 'var(--color-primary)'
    case 'WARN':
      return 'var(--color-warning)'
    case 'ERROR':
    case 'FATAL':
      return 'var(--color-danger)'
  }
}

/** 级别对应的主题色类名（用于 class 绑定） */
export function levelClass(level: LogLevel): string {
  return `level-${level.toLowerCase()}`
}

/** 字节数格式化 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

/** 秒数格式化为可读时长（如 3600 → 1小时，3720 → 1小时2分，90 → 1分30秒） */
export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '-'
  const s = Math.floor(seconds)
  if (s < 60) return `${s}秒`
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const rest = s % 60
  if (h > 0) return m > 0 ? `${h}小时${m}分` : `${h}小时`
  return rest > 0 ? `${m}分${rest}秒` : `${m}分`
}
