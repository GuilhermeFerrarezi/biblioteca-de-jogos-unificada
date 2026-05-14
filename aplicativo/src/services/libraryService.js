import { invoke } from '@tauri-apps/api/core'
import { validateManualGameInput } from '../adapters/libraryEntryAdapter'

const hasTauriRuntime = () =>
  typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

const STEAM_ACCOUNT_COMMANDS = Object.freeze({
  getConfig: 'list_steam_account_config',
  saveConfig: 'save_steam_account_config',
  disconnectConfig: 'disconnect_steam_account_config',
})

const STEAM_API_KEY_COMMANDS = Object.freeze({
  getStatus: 'get_steam_api_key_status',
  save: 'save_steam_api_key',
  delete: 'delete_steam_api_key',
})

const isMissingCommandError = (error) => {
  const message = error instanceof Error ? error.message : String(error)

  return /command.*not found|not found.*command|unknown.*command|unknown.*invoke/i.test(message)
}

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

export const getSteamAccountSettings = async () => {
  if (!hasTauriRuntime()) {
    return {
      authState: 'disconnected',
      isBackendAvailable: false,
      steamId64: '',
    }
  }

  try {
    const settings = await invoke(STEAM_ACCOUNT_COMMANDS.getConfig)

    return {
      authState: settings?.authState === 'configured' ? 'configured' : 'disconnected',
      isBackendAvailable: true,
      steamId64: typeof settings?.steamId64 === 'string' ? settings.steamId64 : '',
    }
  } catch (error) {
    if (isMissingCommandError(error)) {
      return {
        authState: 'disconnected',
        isBackendAvailable: false,
        steamId64: '',
      }
    }

    throw new Error('Nao foi possivel carregar a configuracao Steam.')
  }
}

export const saveSteamAccountSettings = async ({ steamId64 }) => {
  if (!hasTauriRuntime()) {
    return {
      authState: 'disconnected',
      isBackendAvailable: false,
      saved: false,
      steamId64: '',
    }
  }

  try {
    const settings = await invoke(STEAM_ACCOUNT_COMMANDS.saveConfig, { input: { steamId64 } })

    return {
      authState: settings?.authState === 'configured' ? 'configured' : 'disconnected',
      isBackendAvailable: true,
      saved: true,
      steamId64: typeof settings?.steamId64 === 'string' ? settings.steamId64 : '',
    }
  } catch (error) {
    if (isMissingCommandError(error)) {
      return {
        authState: 'disconnected',
        isBackendAvailable: false,
        saved: false,
        steamId64: '',
      }
    }

    throw new Error('Nao foi possivel salvar a configuracao Steam.')
  }
}

export const disconnectSteamAccountSettings = async () => {
  if (!hasTauriRuntime()) {
    return {
      authState: 'disconnected',
      disconnected: false,
      isBackendAvailable: false,
      steamId64: '',
    }
  }

  try {
    const settings = await invoke(STEAM_ACCOUNT_COMMANDS.disconnectConfig)

    return {
      authState: settings?.authState === 'configured' ? 'configured' : 'disconnected',
      disconnected: true,
      isBackendAvailable: true,
      steamId64: typeof settings?.steamId64 === 'string' ? settings.steamId64 : '',
    }
  } catch (error) {
    if (isMissingCommandError(error)) {
      return {
        authState: 'disconnected',
        disconnected: false,
        isBackendAvailable: false,
        steamId64: '',
      }
    }

    throw new Error('Nao foi possivel desconectar a configuracao Steam.')
  }
}

export const getSteamApiKeyStatus = async () => {
  if (!hasTauriRuntime()) {
    return {
      isBackendAvailable: false,
      isConfigured: false,
    }
  }

  try {
    const status = await invoke(STEAM_API_KEY_COMMANDS.getStatus)

    return {
      isBackendAvailable: true,
      isConfigured: Boolean(status?.isConfigured),
    }
  } catch (error) {
    if (isMissingCommandError(error)) {
      return {
        isBackendAvailable: false,
        isConfigured: false,
      }
    }

    throw new Error('Nao foi possivel consultar o cofre Steam.')
  }
}

export const saveSteamApiKey = async ({ apiKey }) => {
  if (!hasTauriRuntime()) {
    return {
      isBackendAvailable: false,
      isConfigured: false,
      saved: false,
    }
  }

  try {
    const status = await invoke(STEAM_API_KEY_COMMANDS.save, { input: { apiKey } })

    return {
      isBackendAvailable: true,
      isConfigured: Boolean(status?.isConfigured),
      saved: true,
    }
  } catch (error) {
    if (isMissingCommandError(error)) {
      return {
        isBackendAvailable: false,
        isConfigured: false,
        saved: false,
      }
    }

    throw new Error('Nao foi possivel salvar a chave Steam no cofre.')
  }
}

export const deleteSteamApiKey = async () => {
  if (!hasTauriRuntime()) {
    return {
      deleted: false,
      isBackendAvailable: false,
      isConfigured: false,
    }
  }

  try {
    const status = await invoke(STEAM_API_KEY_COMMANDS.delete)

    return {
      deleted: true,
      isBackendAvailable: true,
      isConfigured: Boolean(status?.isConfigured),
    }
  } catch (error) {
    if (isMissingCommandError(error)) {
      return {
        deleted: false,
        isBackendAvailable: false,
        isConfigured: false,
      }
    }

    throw new Error('Nao foi possivel remover a chave Steam do cofre.')
  }
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
