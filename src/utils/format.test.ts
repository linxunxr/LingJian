import { describe, expect, it } from 'vitest'

import { formatBytes, formatDuration, formatTime, levelClass, levelColorVar } from './format'

describe('formatTime', () => {
  it('格式化本地时间 ISO 字符串为可读格式', () => {
    expect(formatTime('2026-08-23T10:05:03')).toBe('2026-08-23 10:05:03')
  })

  it('补零个位数的月/日/时/分/秒', () => {
    expect(formatTime('2026-01-02T03:04:05')).toBe('2026-01-02 03:04:05')
  })

  it('无法解析的输入原样返回', () => {
    expect(formatTime('not-a-date')).toBe('not-a-date')
  })
})

describe('levelColorVar', () => {
  it('各级别映射到对应主题色变量', () => {
    expect(levelColorVar('DEBUG')).toBe('var(--color-text-muted)')
    expect(levelColorVar('INFO')).toBe('var(--color-primary)')
    expect(levelColorVar('WARN')).toBe('var(--color-warning)')
  })

  it('ERROR 与 FATAL 共用 danger 色', () => {
    expect(levelColorVar('ERROR')).toBe('var(--color-danger)')
    expect(levelColorVar('FATAL')).toBe('var(--color-danger)')
  })
})

describe('levelClass', () => {
  it('级别转为小写 class 名', () => {
    expect(levelClass('ERROR')).toBe('level-error')
    expect(levelClass('WARN')).toBe('level-warn')
  })
})

describe('formatBytes', () => {
  it('小于 1KB 以 B 显示', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(1023)).toBe('1023 B')
  })

  it('KB 档保留一位小数', () => {
    expect(formatBytes(1024)).toBe('1.0 KB')
    expect(formatBytes(1536)).toBe('1.5 KB')
  })

  it('MB 档保留两位小数', () => {
    expect(formatBytes(1024 * 1024)).toBe('1.00 MB')
    expect(formatBytes(2.5 * 1024 * 1024)).toBe('2.50 MB')
  })
})

describe('formatDuration', () => {
  it('不足一分钟以秒显示', () => {
    expect(formatDuration(0)).toBe('0秒')
    expect(formatDuration(59)).toBe('59秒')
  })

  it('分钟档带剩余秒', () => {
    expect(formatDuration(60)).toBe('1分')
    expect(formatDuration(90)).toBe('1分30秒')
  })

  it('小时档带分钟，整小时不拖尾', () => {
    expect(formatDuration(3600)).toBe('1小时')
    expect(formatDuration(3720)).toBe('1小时2分')
  })

  it('非法输入显示占位符', () => {
    expect(formatDuration(-1)).toBe('-')
    expect(formatDuration(Number.NaN)).toBe('-')
  })
})
