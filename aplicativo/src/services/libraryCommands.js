import { invoke } from '@tauri-apps/api/core'
import { validateManualGameInput } from '../adapters/libraryEntryAdapter.js'
import { markBootStep } from './bootInstrumentation.js'
import { hasTauriRuntime } from './tauriRuntime.js'

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
  markBootStep('frontend.list_library_entries.start')

  if (!hasTauriRuntime()) {
    const entries = await loadDevelopmentLibraryEntries()
    markBootStep('frontend.list_library_entries.complete', { entries: entries.length, runtime: 'web' })
    return entries
  }

  const entries = await invoke('list_library_entries')
  markBootStep('frontend.list_library_entries.complete', { entries: entries.length, runtime: 'tauri' })
  return entries
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

export const syncXboxGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_xbox_games')
}

export const syncEpicGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_epic_games')
}

export const syncXboxAchievementGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_xbox_achievement_games')
}

export const syncSteamAccountGames = async ({ retryMarkedEnrichment = false } = {}) => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_steam_account_games', { input: { retryMarkedEnrichment } })
}

export const getSteamEnrichmentRetrySummary = async () => {
  if (!hasTauriRuntime()) {
    return {
      markedGames: 0,
      markedAttempts: 0,
      artwork: 0,
      achievementSchema: 0,
      playerAchievements: 0,
    }
  }

  return invoke('get_steam_enrichment_retry_summary')
}

export const getSteamAccountConfig = async () => {
  if (!hasTauriRuntime()) {
    return { connected: false, steamId64: null }
  }

  return invoke('get_steam_account_config')
}

export const getXboxAccountConfig = async () => {
  if (!hasTauriRuntime()) {
    return { connected: false, xuid: null }
  }

  return invoke('get_xbox_account_config')
}

export const getSteamLibraryRoots = async () => {
  if (!hasTauriRuntime()) {
    return { providerId: 'steam', roots: [] }
  }

  return invoke('get_steam_library_roots')
}

export const getXboxLibraryRoots = async () => {
  if (!hasTauriRuntime()) {
    return { providerId: 'xbox', roots: [] }
  }

  return invoke('get_xbox_library_roots')
}

export const getEpicLibraryRoots = async () => {
  if (!hasTauriRuntime()) {
    return { providerId: 'epic', roots: [] }
  }

  return invoke('get_epic_library_roots')
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

export const saveSteamLibraryRoots = async (roots) => {
  const normalizedRoots = Array.isArray(roots)
    ? roots.map((root) => String(root ?? '').trim()).filter(Boolean)
    : []

  if (!hasTauriRuntime()) {
    return { providerId: 'steam', roots: normalizedRoots }
  }

  return invoke('save_steam_library_roots', { input: { roots: normalizedRoots } })
}

export const saveXboxLibraryRoots = async (roots) => {
  const normalizedRoots = Array.isArray(roots)
    ? roots.map((root) => String(root ?? '').trim()).filter(Boolean)
    : []

  if (!hasTauriRuntime()) {
    return { providerId: 'xbox', roots: normalizedRoots }
  }

  return invoke('save_xbox_library_roots', { input: { roots: normalizedRoots } })
}

export const saveEpicLibraryRoots = async (roots) => {
  const normalizedRoots = Array.isArray(roots)
    ? roots.map((root) => String(root ?? '').trim()).filter(Boolean)
    : []

  if (!hasTauriRuntime()) {
    return { providerId: 'epic', roots: normalizedRoots }
  }

  return invoke('save_epic_library_roots', { input: { roots: normalizedRoots } })
}

export const startSteamLogin = async () => {
  if (!hasTauriRuntime()) {
    return { pending: false, providerId: 'steam' }
  }

  return invoke('start_steam_openid_login')
}

export const startXboxLiveLogin = async () => {
  if (!hasTauriRuntime()) {
    return { pending: false, providerId: 'xbox' }
  }

  return invoke('start_xbox_live_login')
}

export const getSteamApiKeyStatus = async () => {
  if (!hasTauriRuntime()) {
    return { configured: false, providerId: 'steam', storage: 'dev' }
  }

  return invoke('get_steam_web_api_key_state')
}

export const getXboxLiveAuthState = async () => {
  if (!hasTauriRuntime()) {
    return { configured: false, providerId: 'xbox', storage: 'dev' }
  }

  return invoke('get_xbox_live_auth_state')
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

export const setLibraryEntryFavorite = async (entryId, isFavorite) => {
  if (!entryId) {
    throw new Error('Entrada de biblioteca invalida.')
  }

  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('set_library_entry_favorite', { entryId, isFavorite })
}

export const setLibraryEntriesPersonalReview = async (entryIds, input) => {
  const normalizedEntryIds = Array.isArray(entryIds)
    ? [...new Set(entryIds.map((entryId) => String(entryId ?? '').trim()).filter(Boolean))]
    : []
  const rating = input?.rating ?? null
  const review = String(input?.review ?? '').trim()

  if (normalizedEntryIds.length === 0) {
    throw new Error('Entrada de biblioteca invalida.')
  }

  if (rating !== null) {
    const numericRating = Number(rating)
    if (!Number.isFinite(numericRating) || numericRating < 0.5 || numericRating > 5 || numericRating * 2 !== Math.round(numericRating * 2)) {
      throw new Error('Escolha uma nota entre 0.5 e 5 estrelas.')
    }
  }

  if ([...review].length > 4000) {
    throw new Error('A resenha deve ter no maximo 4000 caracteres.')
  }

  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('set_library_entries_personal_review', {
    entryIds: normalizedEntryIds,
    input: {
      rating: rating === null ? null : Number(rating),
      review: review || null,
    },
  })
}
