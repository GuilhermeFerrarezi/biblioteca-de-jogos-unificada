import { invoke } from '@tauri-apps/api/core'
import { validateManualGameInput } from '../adapters/libraryEntryAdapter'

const hasTauriRuntime = () =>
  typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

const loadDevelopmentLibraryEntries = async () => {
  if (!import.meta.env.DEV) {
    return []
  }

  const { libraryEntries } = await import('../data/mockLibrary')
  return libraryEntries
}

const validateManualInputOrThrow = (input) => {
  const validation = validateManualGameInput(input)

  if (!validation.isValid) {
    throw new Error(Object.values(validation.errors)[0])
  }
}

export const listLibraryEntries = async () => {
  if (!hasTauriRuntime()) {
    return loadDevelopmentLibraryEntries()
  }

  return invoke('list_library_entries')
}

export const addPersistedManualGame = async (input) => {
  validateManualInputOrThrow(input)

  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('add_manual_game', { input })
}

export const updatePersistedManualGame = async (entryId, input) => {
  validateManualInputOrThrow(input)

  if (!entryId) {
    throw new Error('Entrada de biblioteca invalida.')
  }

  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('update_manual_game', { entryId, input })
}

export const syncLocalGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_local_games')
}

export const syncSteamGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_steam_games')
}

export const syncSteamAccountGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_steam_account_games')
}

export const getSteamAccountConfig = async () => {
  if (!hasTauriRuntime()) {
    return { connected: false, steamId64: null }
  }

  return invoke('get_steam_account_config')
}

export const saveSteamAccountConfig = async (steamId64) => {
  const normalizedSteamId64 = String(steamId64 ?? '').trim()

  if (!/^\d{17}$/.test(normalizedSteamId64)) {
    throw new Error('Informe um SteamID64 valido antes de salvar a conta.')
  }

  if (!hasTauriRuntime()) {
    return { connected: true, steamId64: normalizedSteamId64 }
  }

  return invoke('save_steam_account_config', { input: { steamId64: normalizedSteamId64 } })
}

export const startSteamLogin = async () => {
  if (!hasTauriRuntime()) {
    return { pending: false, providerId: 'steam' }
  }

  return invoke('start_steam_openid_login')
}

export const getSteamApiKeyStatus = async () => {
  if (!hasTauriRuntime()) {
    return { configured: false, providerId: 'steam', storage: 'dev' }
  }

  return invoke('get_steam_web_api_key_state')
}

export const saveSteamApiKey = async (apiKey) => {
  if (!apiKey || typeof apiKey !== 'string') {
    throw new Error('Informe uma credencial Steam Web API valida.')
  }

  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('save_steam_web_api_key', { input: { apiKey } })
}

export const deleteSteamApiKey = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('disconnect_steam_web_api_key')
}

export const launchLibraryEntry = async (entryId) => {
  if (!entryId) {
    throw new Error('Entrada de biblioteca invalida.')
  }

  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('launch_library_entry', { entryId })
}

export const setLibraryEntryArchived = async (entryId, isArchived) => {
  if (!entryId) {
    throw new Error('Entrada de biblioteca invalida.')
  }

  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('set_library_entry_archived', { entryId, isArchived })
}
