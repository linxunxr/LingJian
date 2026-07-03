import { reactive } from 'vue'
import { LazyStore } from '@tauri-apps/plugin-store'

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
 * scfUrl + apiKey 均存 tauri-plugin-store（settings.json）。
 * 此前 apiKey 走系统钥匙串（keyring），但在部分 Windows 环境下
 * keyring 写入会静默失败（凭据管理器无对应条目），导致保存成功
 * 却读不回来。本地桌面工具的 API Key 与 scfUrl 同级，统一存
 * settings.json 更简单可靠。
 */
export async function loadSettings(): Promise<void> {
  try {
    settings.scfUrl = (await store.get<string>('scfUrl')) ?? ''
    settings.apiKey = (await store.get<string>('apiKey')) ?? ''
  } catch (e) {
    console.warn('加载设置失败:', e)
  }
}

/** 将当前内存设置持久化 */
export async function saveSettings(): Promise<void> {
  await store.set('scfUrl', settings.scfUrl)
  await store.set('apiKey', settings.apiKey)
  await store.save()
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
