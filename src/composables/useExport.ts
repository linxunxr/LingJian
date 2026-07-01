import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'

export type ExportFormat = 'markdown' | 'json' | 'csv'

interface ExportResult {
  path: string
  bytes: number
}

const EXTENSIONS: Record<ExportFormat, string> = {
  markdown: 'md',
  json: 'json',
  csv: 'csv',
}

/**
 * 导出指定 report 为文件。
 * 先弹出系统保存对话框选择路径，再调用后端生成并写盘。
 */
export async function exportReport(reportId: string, format: ExportFormat): Promise<ExportResult | null> {
  const ext = EXTENSIONS[format]
  const filePath = await save({
    title: '导出日志分析',
    defaultPath: `${reportId}.${ext}`,
    filters: [
      {
        name: format.toUpperCase(),
        extensions: [ext],
      },
    ],
  })

  // 用户取消
  if (!filePath) return null

  const result = await invoke<ExportResult>('export_report', {
    reportId,
    format,
    path: filePath,
  })
  return result
}
