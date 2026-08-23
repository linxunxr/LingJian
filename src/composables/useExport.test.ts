import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))

import { exportReport, type ExportFormat } from './useExport'

const mockedInvoke = vi.mocked(invoke)
const mockedSave = vi.mocked(save)

beforeEach(() => {
  mockedInvoke.mockReset()
  mockedSave.mockReset()
})

describe('exportReport', () => {
  it('用户取消保存对话框时不调用后端并返回 null', async () => {
    mockedSave.mockResolvedValue(null)

    const result = await exportReport('rp-1', 'markdown')

    expect(result).toBeNull()
    expect(mockedInvoke).not.toHaveBeenCalled()
  })

  it.each([
    ['markdown', 'md'],
    ['json', 'json'],
    ['csv', 'csv'],
  ] as const)('%s 格式使用正确的默认扩展名', async (format: ExportFormat, ext: string) => {
    mockedSave.mockResolvedValue(`D:/out/rp-1.${ext}`)
    mockedInvoke.mockResolvedValue({ path: `D:/out/rp-1.${ext}`, bytes: 128 })

    const result = await exportReport('rp-1', format)

    expect(mockedSave).toHaveBeenCalledWith(
      expect.objectContaining({ defaultPath: `rp-1.${ext}` }),
    )
    expect(mockedInvoke).toHaveBeenCalledWith('export_report', {
      reportId: 'rp-1',
      format,
      path: `D:/out/rp-1.${ext}`,
    })
    expect(result).toEqual({ path: `D:/out/rp-1.${ext}`, bytes: 128 })
  })
})
