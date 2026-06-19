import { reactive } from 'vue'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { getVersion } from '@tauri-apps/api/app'

type UpdateStatus =
  | 'idle' // 空闲
  | 'checking' // 检查中
  | 'available' // 有新版本
  | 'not-available' // 已是最新
  | 'downloading' // 下载中
  | 'installed' // 已安装待重启
  | 'error' // 出错

interface UpdaterState {
  status: UpdateStatus
  /** 当前应用版本 */
  currentVersion: string
  /** 新版本号 */
  version: string | null
  /** 新版本更新说明 */
  body: string | null
  /** 发布日期 */
  date: string | null
  /** 下载进度 0-100 */
  progress: number
  /** 错误信息 */
  error: string | null
}

export const state = reactive<UpdaterState>({
  status: 'idle',
  currentVersion: '',
  version: null,
  body: null,
  date: null,
  progress: 0,
  error: null,
})

/** 检查更新（不自动下载） */
export async function checkForUpdate(): Promise<void> {
  state.status = 'checking'
  state.error = null
  try {
    if (!state.currentVersion) {
      state.currentVersion = await getVersion()
    }
    const update = await check()
    if (update) {
      state.version = update.version
      state.body = update.body ?? null
      state.date = update.date ?? null
      state.status = 'available'
    } else {
      state.status = 'not-available'
    }
  } catch (e) {
    state.status = 'error'
    state.error = typeof e === 'string' ? e : String(e)
  }
}

/** 下载并安装更新 */
export async function downloadAndInstall(): Promise<void> {
  if (state.status !== 'available') return
  state.status = 'downloading'
  state.progress = 0
  state.error = null
  try {
    const update = await check()
    if (!update) {
      state.status = 'not-available'
      return
    }

    let total = 0
    let downloaded = 0
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          total = event.data.contentLength ?? 0
          break
        case 'Progress':
          downloaded += event.data.chunkLength
          state.progress = total > 0 ? Math.round((downloaded / total) * 100) : 0
          break
        case 'Finished':
          state.progress = 100
          break
      }
    })

    state.status = 'installed'
    // 安装完成，重启应用（Windows passive 模式会自动重启）
    await relaunch()
  } catch (e) {
    state.status = 'error'
    state.error = typeof e === 'string' ? e : String(e)
  }
}

/** 重置状态 */
export function resetUpdater(): void {
  state.status = 'idle'
  state.version = null
  state.body = null
  state.date = null
  state.progress = 0
  state.error = null
}

export function useUpdater() {
  return { state, checkForUpdate, downloadAndInstall, resetUpdater }
}
