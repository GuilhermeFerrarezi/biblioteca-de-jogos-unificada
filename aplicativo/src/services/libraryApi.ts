import { invoke } from '@tauri-apps/api/core'
import type { InstallStatus, LibraryEntry } from '../domain'

export type ManualGameInput = {
  title: string
  genre?: string
  installStatus: InstallStatus
  launchTarget?: string
}

const hasTauriRuntime = () =>
  Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)

export const listPersistedManualGames = async (): Promise<LibraryEntry[]> => {
  if (!hasTauriRuntime()) {
    return []
  }

  return invoke<LibraryEntry[]>('list_manual_games')
}

export const addPersistedManualGame = async (input: ManualGameInput): Promise<LibraryEntry | null> => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke<LibraryEntry>('add_manual_game', { input })
}

export type LaunchLocalExecutableResult = {
  started: boolean
  message: string
}

export const launchLibraryEntry = async (entryId: string): Promise<LaunchLocalExecutableResult | null> => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke<LaunchLocalExecutableResult>('launch_library_entry', { entryId })
}
