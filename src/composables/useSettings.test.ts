import { beforeEach, describe, expect, it, vi } from 'vitest'

// LazyStore 在模块加载时即实例化，必须整体 mock 掉
const storeData = vi.hoisted(() => new Map<string, unknown>())
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-store', () => ({
  LazyStore: class {
    get = vi.fn(async (key: string) => storeData.get(key))
    set = vi.fn(async (key: string, value: unknown) => {
      storeData.set(key, value)
    })
    delete = vi.fn(async (key: string) => storeData.delete(key))
    save = vi.fn(async () => {})
  },
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

import { isSettingsComplete, loadSettings, saveSettings, settings } from './useSettings'

const mockedInvoke = vi.mocked(invoke)

beforeEach(() => {
  storeData.clear()
  mockedInvoke.mockReset()
  settings.scfUrl = ''
  settings.apiKey = ''
})

describe('loadSettings', () => {
  it('scfUrl 读 store，apiKey 读钥匙串', async () => {
    storeData.set('scfUrl', 'http://scf.example')
    mockedInvoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'migrate_api_key') return { keyringReady: true, migrated: false }
      if (cmd === 'get_secret' && args?.kind === 'scfApiKey') return 'key-1'
      throw new Error(`unexpected command: ${cmd}`)
    })

    await loadSettings()

    expect(settings.scfUrl).toBe('http://scf.example')
    expect(settings.apiKey).toBe('key-1')
    expect(mockedInvoke).toHaveBeenCalledWith('migrate_api_key')
  })

  it('迁移命令失败不阻断加载（明文仍在的场景）', async () => {
    storeData.set('scfUrl', 'http://scf.example')
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'migrate_api_key') throw new Error('keyring unavailable')
      if (cmd === 'get_secret') return ''
      throw new Error(`unexpected command: ${cmd}`)
    })

    await loadSettings()

    expect(settings.scfUrl).toBe('http://scf.example')
    expect(settings.apiKey).toBe('')
  })

  it('缺失的键回退为空串', async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'migrate_api_key') return { keyringReady: false, migrated: false }
      if (cmd === 'get_secret') return ''
      throw new Error(`unexpected command: ${cmd}`)
    })

    await loadSettings()

    expect(settings.scfUrl).toBe('')
    expect(settings.apiKey).toBe('')
  })
})

describe('saveSettings', () => {
  it('scfUrl 写 store，apiKey 写钥匙串且 store 不落明文', async () => {
    settings.scfUrl = 'http://scf.example'
    settings.apiKey = 'key-2'

    await saveSettings()

    expect(storeData.get('scfUrl')).toBe('http://scf.example')
    expect(storeData.has('apiKey')).toBe(false)
    expect(mockedInvoke).toHaveBeenCalledWith('set_secret', {
      kind: 'scfApiKey',
      value: 'key-2',
    })
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
