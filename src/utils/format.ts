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
