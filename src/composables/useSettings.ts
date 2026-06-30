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
 * - SCF URL：明文，存 tauri-plugin-store
 * - SCF API Key：敏感，存系统钥匙串（keyring）
 *
 * 说明：Issue 解析已改由 SCF 服务端用自身 GITHUB_TOKEN 代理，
 * 客户端不再需要配置 GitHub Token。
 */
export async function loadSettings(): Promise<void> {
  try {
    // SCF URL 走 store
    settings.scfUrl = (await store.get<string>('scfUrl')) ?? ''
    // 敏感凭证走 keyring
    settings.apiKey = await invoke<string>('get_secret', { kind: 'scfApiKey' })
  } catch (e) {
    // keyring 不可用时降级为空，不阻断启动
    console.warn('加载凭证失败:', e)
  }
}

/** 将当前内存设置持久化 */
export async function saveSettings(): Promise<void> {
  // SCF URL 走 store
  await store.set('scfUrl', settings.scfUrl)
  await store.save()
  // 敏感凭证走 keyring
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
