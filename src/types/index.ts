export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'

export interface LogEntry {
  timestamp: string
  level: LogLevel
  /** 模块/功能标签 */
  tag: string
  message: string
  /** 附加结构化数据，可选 */
  data?: unknown
}

/** 从 Issue URL 解析出的信息 */
export interface ParsedIssue {
  owner: string
  repo: string
  number: number
}

/** Issue 完整信息（含 reportId） */
export interface IssueInfo extends ParsedIssue {
  reportId: string
  title: string
  /** 上报环境信息（由 SCF 从 Issue body 环境表格提取，可选） */
  appVersion?: string
  platform?: string
  realm?: string
}

export interface Report {
  reportId: string
  issueNumber?: number
  issueTitle?: string
  appVersion?: string
  platform?: string
  realm?: string
  playTime?: number
  userDescription?: string
  reportTime: string
  logCount: number
  downloadedAt: string
}

export interface DownloadResult {
  reportId: string
  logCount: number
  fileSize: number
}

/** 日志过滤条件 */
export interface LogFilter {
  levels: LogLevel[]
  tags: string[]
  keyword: string
}

export interface TimelinePoint {
  timestamp: string
  level: LogLevel
  message: string
}

export interface ErrorAggregate {
  message: string
  count: number
  firstSeen: string
  lastSeen: string
}

export interface LevelCounts {
  debug: number
  info: number
  warn: number
  error: number
}

export interface TagCount {
  tag: string
  count: number
}

export interface AnalysisResult {
  /** 过滤后保留的日志条目 */
  entries: LogEntry[]
  /** 全量日志条目数（应用过滤前） */
  total: number
  /** 各级别计数（基于全量日志） */
  levelCounts: LevelCounts
  /** 所有出现过的 tag 及其计数（基于全量日志） */
  tagCounts: TagCount[]
  /** ERROR/WARN 时间线（基于过滤后日志） */
  timeline: TimelinePoint[]
  /** 错误聚合（基于过滤后 ERROR 日志） */
  errorAggregates: ErrorAggregate[]
}
