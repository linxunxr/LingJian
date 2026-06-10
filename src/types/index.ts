export interface LogEntry {
  timestamp: string
  level: string
  module: string
  message: string
}

export interface IssueInfo {
  owner: string
  repo: string
  number: number
  report_id: string
  title: string
}

export interface DownloadResult {
  report_id: string
  log_count: number
  file_size: number
}

export interface AnalysisResult {
  total: number
  errors: number
  warnings: number
}
