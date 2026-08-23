import { beforeEach, describe, expect, it, vi } from 'vitest'

// LazyStore 在模块加载时即实例化，必须整体 mock 掉
const storeData = vi.hoisted(() => new Map<string, unknown>())
vi.mock('@tauri-apps/plugin-store', () => ({
  LazyStore: class {
    get = vi.fn(async (key: string) => storeData.get(key))
    set = vi.fn(async (key: string, value: unknown) => {
      storeData.set(key, value)
    })
    save = vi.fn(async () => {})
  },
}))

import { isSettingsComplete, loadSettings, saveSettings, settings } from './useSettings'

beforeEach(() => {
  storeData.clear()
  settings.scfUrl = ''
  settings.apiKey = ''
})

describe('loadSettings', () => {
  it('从持久化存储读取 scfUrl 与 apiKey', async () => {
    storeData.set('scfUrl', 'http://scf.example')
    storeData.set('apiKey', 'key-1')

    await loadSettings()

    expect(settings.scfUrl).toBe('http://scf.example')
    expect(settings.apiKey).toBe('key-1')
  })

  it('缺失的键回退为空串', async () => {
    await loadSettings()

    expect(settings.scfUrl).toBe('')
    expect(settings.apiKey).toBe('')
  })
})

describe('saveSettings', () => {
  it('将内存设置写回存储', async () => {
    settings.scfUrl = 'http://scf.example'
    settings.apiKey = 'key-2'

    await saveSettings()

    expect(storeData.get('scfUrl')).toBe('http://scf.example')
    expect(storeData.get('apiKey')).toBe('key-2')
  })
})

describe('isSettingsComplete', () => {
  it('任一项为空即未完成', () => {
    settings.scfUrl = ''
    settings.apiKey = 'key'
    expect(isSettingsComplete()).toBe(false)

    settings.scfUrl = 'http://scf.example'
    settings.apiKey = ''
    expect(isSettingsComplete()).toBe(false)
  })

  it('纯空白字符视为未填', () => {
    settings.scfUrl = '   '
    settings.apiKey = 'key'
    expect(isSettingsComplete()).toBe(false)
  })

  it('两项均非空即完成', () => {
    settings.scfUrl = 'http://scf.example'
    settings.apiKey = 'key'
    expect(isSettingsComplete()).toBe(true)
  })
})
