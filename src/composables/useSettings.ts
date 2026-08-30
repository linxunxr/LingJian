import { reactive } from 'vue'
import { LazyStore } from '@tauri-apps/plugin-store'
import { invoke } from '@tauri-apps/api/core'

export interface AppSettings {
  scfUrl: string
  apiKey: string
}

const STORE_FILE = 'settings.json'
const store = new LazyStore(STORE_FILE)

/** 全局共享的设置状态（模块级单例） */
export const settings = reactive<AppSettings>({
  scfUrl: '',
  apiKey: '',
})

/**
 * 从持久化存储加载设置到内存（每次调用都刷新，确保读到最新值）。
 *
 * - scfUrl：非敏感，存 tauri-plugin-store（settings.json）
 * - apiKey：敏感凭证，存系统钥匙串（Windows 凭据管理器 /
 *   macOS 钥匙串 / Linux Secret Service）
 *
 * 历史包袱：apiKey 曾两度换位置——最初就是钥匙串，7 月因旧版
 * keyring 在部分 Windows 环境写入静默失败降级为 settings.json
 * 明文（411d123）；现 keyring 3.x 验证可靠后回迁，load 时顺带
 * 调 migrate_api_key 把老版本留下的明文迁入钥匙串并擦除。
 */
export async function loadSettings(): Promise<void> {
  try {
    await invoke('migrate_api_key')
  } catch (e) {
    // 迁移失败不阻断启动（明文仍在，功能可用），下次启动重试
    console.warn('迁移明文 apiKey 失败:', e)
  }
  try {
    settings.scfUrl = (await store.get<string>('scfUrl')) ?? ''
    settings.apiKey = await invoke<string>('get_secret', { kind: 'scfApiKey' })
  } catch (e) {
    console.warn('加载设置失败:', e)
  }
}

/** 将当前内存设置持久化（apiKey 走钥匙串，settings.json 不落明文） */
export async function saveSettings(): Promise<void> {
  await store.set('scfUrl', settings.scfUrl)
  await store.delete('apiKey').catch(() => {})
  await store.save()
  await invoke('set_secret', { kind: 'scfApiKey', value: settings.apiKey })
}

/** 设置是否完整（用于判断能否发起分析） */
export function isSettingsComplete(): boolean {
  return (
    settings.scfUrl.trim() !== '' &&
    settings.apiKey.trim() !== ''
  )
}

/** 提供响应式 settings 引用（组合式 API 入口） */
export function useSettings() {
  return { settings, loadSettings, saveSettings, isSettingsComplete }
}
