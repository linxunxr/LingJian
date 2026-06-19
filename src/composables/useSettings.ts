import { reactive } from 'vue'
import { LazyStore } from '@tauri-apps/plugin-store'

export interface AppSettings {
  githubToken: string
  scfUrl: string
  apiKey: string
}

const STORE_FILE = 'settings.json'

const store = new LazyStore(STORE_FILE)

/** 全局共享的设置状态（模块级单例） */
export const settings = reactive<AppSettings>({
  githubToken: '',
  scfUrl: '',
  apiKey: '',
})

/** 标记是否已从磁盘加载 */
let loaded = false

/** 从持久化存储加载设置到内存 */
export async function loadSettings(): Promise<void> {
  if (loaded) return
  settings.githubToken = (await store.get<string>('githubToken')) ?? ''
  settings.scfUrl = (await store.get<string>('scfUrl')) ?? ''
  settings.apiKey = (await store.get<string>('apiKey')) ?? ''
  loaded = true
}

/** 将当前内存设置持久化到磁盘 */
export async function saveSettings(): Promise<void> {
  await store.set('githubToken', settings.githubToken)
  await store.set('scfUrl', settings.scfUrl)
  await store.set('apiKey', settings.apiKey)
  await store.save()
}

/** 设置是否完整（用于判断能否发起分析） */
export function isSettingsComplete(): boolean {
  return (
    settings.githubToken.trim() !== '' &&
    settings.scfUrl.trim() !== '' &&
    settings.apiKey.trim() !== ''
  )
}

/** 提供响应式 settings 引用（组合式 API 入口） */
export function useSettings() {
  return { settings, loadSettings, saveSettings, isSettingsComplete }
}
