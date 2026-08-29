export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR' | 'FATAL'

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
  /** Issue 状态：open / closed（旧版 SCF 不返回时为空串） */
  state?: string
  /** 当前标签列表（旧版 SCF 不返回时为空数组） */
  labels?: string[]
  /** 上报环境信息（由 SCF 从 Issue body 环境表格提取，可选） */
  appVersion?: string
  platform?: string
  realm?: string
  /** 用户反馈文本（SCF 从 Issue body「用户描述」小节提取；旧版 SCF 不返回） */
  userDescription?: string
  /** 游玩时长（秒，字符串形式；旧版 SCF 不返回） */
  playTime?: string
}

/** 问题列表项（首页列表展示用） */
export interface IssueListItem {
  number: number
  reportId: string
  title: string
  /** Issue 状态：open / closed */
  state: string
  issueUrl: string
  /** 创建时间（ISO 8601） */
  createdAt: string
  owner: string
  repo: string
  appVersion?: string
  platform?: string
  realm?: string
  /** 当前标签列表 */
  labels?: string[]
}

/** SCF /issues 端点的完整响应 */
export interface IssueList {
  issues: IssueListItem[]
  page: number
  hasMore: boolean
}

/** SCF /issue/:number/action 端点的响应 */
export interface IssueActionResponse {
  ok: boolean
  state?: string
  labels?: string[]
}

export interface Report {
  reportId: string
  issueNumber?: number
  issueTitle?: string
  /** 来源应用名（本地导入的鸿蒙 App 日志标注；Issue 流程为空） */
  appName?: string
  appVersion?: string
  platform?: string
  realm?: string
  playTime?: number
  userDescription?: string
  reportTime: string
  logCount: number
  downloadedAt: string
}

export interface ImportResult {
  reportId: string
  logCount: number
  fileSize: number
  /** 识别出的日志格式：hlog-text / json */
  format: string
  /** 从文件名推断的应用名（baseName） */
  appName?: string | null
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
  fatal: number
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

/** 上报上下文卡片数据（Issue 流程取 IssueInfo，本地导入/最近分析取落库 Report） */
export interface ReportMeta {
  title: string | null
  /** 用户反馈文本 */
  userDescription: string | null
  /** 来源应用名（本地导入的鸿蒙日志有值） */
  appName: string | null
  appVersion: string | null
  platform: string | null
  realm: string | null
  /** 游玩时长（秒） */
  playTime: number | null
  reportTime: string | null
}
