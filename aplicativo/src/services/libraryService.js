import { invoke } from '@tauri-apps/api/core'
import { validateManualGameInput } from '../adapters/libraryEntryAdapter.js'

const hasTauriRuntime = () =>
  typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

const hasLocalStorage = () => typeof window !== 'undefined' && typeof window.localStorage !== 'undefined'
const LIBRARY_SETTINGS_KEY = 'biblioteca-jogos-unificada.library-settings'
const LIBRARY_SCAN_MODE_AUTOMATIC = 'automatic'
const LIBRARY_SCAN_MODE_SELECTED_ONLY = 'selected_only'
const LIBRARY_SCAN_MODE_AUTOMATIC_PLUS_EXTRA = 'automatic_plus_extra'
const DEFAULT_MICROSOFT_CLIENT_ID = String(import.meta.env?.VITE_XBOX_CLIENT_ID ?? '').trim()

const defaultLibrarySettings = Object.freeze({
  preferredStoreId: 'steam',
  localScanMode: LIBRARY_SCAN_MODE_AUTOMATIC,
  localScanRoots: [],
  localScanExcludedRoots: [],
  microsoftClientId: DEFAULT_MICROSOFT_CLIENT_ID,
})

const sanitizeFeedbackText = (value, maxLength = 180) => {
  if (value === null || value === undefined) {
    return ''
  }

  const normalized = String(value).replace(/\s+/g, ' ').trim()

  if (!normalized) {
    return ''
  }

  if (normalized.length <= maxLength) {
    return normalized
  }

  return `${normalized.slice(0, Math.max(0, maxLength - 1)).trimEnd()}...`
}

const normalizeFeedbackDetail = (detail, index) => {
  if (!detail) {
    return null
  }

  if (typeof detail === 'string' || typeof detail === 'number' || typeof detail === 'boolean') {
    const value = sanitizeFeedbackText(detail, 180)
    return value ? { label: index === 0 ? 'Detalhe tecnico' : `Detalhe ${index + 1}`, value } : null
  }

  if (typeof detail !== 'object') {
    return null
  }

  const label = sanitizeFeedbackText(detail.label ?? detail.name ?? detail.title ?? '', 48)
  const value = sanitizeFeedbackText(detail.value ?? detail.text ?? detail.message ?? detail.summary ?? '', 180)

  if (!value) {
    return null
  }

  return {
    label: label || (index === 0 ? 'Detalhe tecnico' : `Detalhe ${index + 1}`),
    value,
  }
}

const tryParseStructuredErrorPayload = (value) => {
  if (typeof value !== 'string') {
    return null
  }

  const trimmedValue = value.trim()
  if (!trimmedValue || (!trimmedValue.startsWith('{') && !trimmedValue.startsWith('['))) {
    return null
  }

  try {
    const parsedValue = JSON.parse(trimmedValue)
    return parsedValue && typeof parsedValue === 'object' ? parsedValue : null
  } catch {
    return null
  }
}

export const normalizeProviderErrorFeedback = (error, fallbackMessage, contextLabel = '') => {
  const fallbackSummary = sanitizeFeedbackText(fallbackMessage) || 'Nao foi possivel concluir a operacao.'

  if (!error) {
    return { message: fallbackSummary, details: [] }
  }

  const structuredSource =
    typeof error === 'object' && error !== null
      ? (error.data && typeof error.data === 'object' ? error.data : error)
      : null
  const parsedStringSource =
    tryParseStructuredErrorPayload(
      (typeof error === 'object' && error !== null && typeof error.data === 'string' && error.data) ||
        (structuredSource && typeof structuredSource.message === 'string' && structuredSource.message) ||
        (typeof error === 'string' ? error : error?.message),
    )
  const normalizedStructuredSource = parsedStringSource ?? structuredSource
  const structuredMessage = sanitizeFeedbackText(normalizedStructuredSource?.message ?? '', 180)

  const existingDetails = Array.isArray(normalizedStructuredSource?.details)
    ? normalizedStructuredSource.details
    : Array.isArray(error?.details)
      ? error.details
      : []

  if (existingDetails.length > 0) {
    const normalizedDetails = existingDetails
      .map((detail, index) => normalizeFeedbackDetail(detail, index))
      .filter(Boolean)
      .slice(0, 3)

    const details = []
    const pushDetail = (label, value, maxLength = 180) => {
      const normalizedValue = sanitizeFeedbackText(value, maxLength)

      if (normalizedValue && details.length < 3) {
        details.push({ label, value: normalizedValue })
      }
    }

    if (contextLabel) {
      pushDetail('Contexto', contextLabel, 96)
    }

    pushDetail('Codigo', normalizedStructuredSource?.code ?? error.code ?? '', 64)
    pushDetail('Etapa', normalizedStructuredSource?.phase ?? error.phase ?? '', 64)

    normalizedDetails.forEach((detail) => {
      if (details.length < 3) {
        details.push(detail)
      }
    })

    return {
      message: structuredMessage || fallbackSummary,
      details: details.slice(0, 3),
    }
  }

  const details = []
  const pushDetail = (label, value, maxLength = 180) => {
    const normalizedValue = sanitizeFeedbackText(value, maxLength)

    if (normalizedValue) {
      details.push({ label, value: normalizedValue })
    }
  }

  if (contextLabel) {
    pushDetail('Contexto', contextLabel, 96)
  }

  const technicalMessage = normalizedStructuredSource
    ? ''
    : typeof error === 'string'
      ? sanitizeFeedbackText(error, 180)
      : sanitizeFeedbackText(error.message ?? '', 180)

  if (technicalMessage && technicalMessage !== fallbackSummary) {
    pushDetail('Mensagem tecnica', technicalMessage)
  }

  pushDetail('Codigo', normalizedStructuredSource?.code ?? error.code ?? '', 64)
  pushDetail('Etapa', normalizedStructuredSource?.phase ?? error.phase ?? '', 64)

  const sanitizedDetails =
    normalizedStructuredSource?.detailsSanitized ??
    normalizedStructuredSource?.diagnostic ??
    normalizedStructuredSource?.summary ??
    normalizedStructuredSource?.detail ??
    (typeof error?.data === 'string' ? error.data : '') ??
    error.detailsSanitized ??
    error.diagnostic ??
    error.summary ??
    error.detail ??
    ''

  pushDetail('Resumo tecnico', sanitizedDetails, 180)

  if (details.length === 0 && technicalMessage) {
    pushDetail('Mensagem tecnica', technicalMessage)
  }

  return {
    message: structuredMessage || fallbackSummary,
    details: details.slice(0, 3),
  }
}

const loadDevelopmentLibraryEntries = async () => {
  if (!import.meta.env.DEV) {
    return []
  }

  const { libraryEntries } = await import('../data/mockLibrary')
  return libraryEntries
}

export const normalizeLibrarySettings = (settings) => {
  if (!settings || typeof settings !== 'object') {
    return { ...defaultLibrarySettings }
  }

  const preferredStoreId = normalizePreferredStoreId(settings.preferredStoreId)
  const localScanMode = normalizeLocalScanMode(settings.localScanMode)
  const localScanRoots = normalizePathList(settings.localScanRoots)
  const localScanExcludedRoots = normalizePathList(settings.localScanExcludedRoots)
  const microsoftClientId = normalizeMicrosoftClientId(
    settings.microsoftClientId ?? settings.microsoftClientID ?? settings.xboxLiveClientId,
  )

  return {
    preferredStoreId,
    localScanMode,
    localScanRoots,
    localScanExcludedRoots,
    microsoftClientId,
  }
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

export const syncXboxGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_xbox_games')
}

export const syncXboxAchievementGames = async () => {
  if (!hasTauriRuntime()) {
    return null
  }

  return invoke('sync_xbox_achievement_games')
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

export const getLibrarySettings = async () => {
  const cachedSettings = readCachedLibrarySettings()

  if (cachedSettings) {
    return cachedSettings
  }

  if (!hasTauriRuntime()) {
    return { ...defaultLibrarySettings }
  }

  try {
    const settings = await invoke('get_library_settings')
    const normalizedSettings = normalizeLibrarySettings(settings)
    cacheLibrarySettings(normalizedSettings)
    return normalizedSettings
  } catch {
    return { ...defaultLibrarySettings }
  }
}

export const saveLibrarySettings = async (settingsOrPreferredStoreId) => {
  const normalizedSettings =
    typeof settingsOrPreferredStoreId === 'string'
      ? normalizeLibrarySettings({ preferredStoreId: settingsOrPreferredStoreId })
      : normalizeLibrarySettings(settingsOrPreferredStoreId)

  cacheLibrarySettings(normalizedSettings)

  if (!hasTauriRuntime()) {
    return normalizedSettings
  }

  await invoke('save_library_settings', { input: normalizedSettings })

  return normalizedSettings
}

const normalizePreferredStoreId = (value) =>
  String(value ?? '').trim().toLowerCase() === 'xbox' ? 'xbox' : 'steam'

const normalizeLocalScanMode = (value) => {
  switch (String(value ?? '').trim().toLowerCase()) {
    case LIBRARY_SCAN_MODE_SELECTED_ONLY:
      return LIBRARY_SCAN_MODE_SELECTED_ONLY
    case LIBRARY_SCAN_MODE_AUTOMATIC_PLUS_EXTRA:
      return LIBRARY_SCAN_MODE_AUTOMATIC_PLUS_EXTRA
    default:
      return LIBRARY_SCAN_MODE_AUTOMATIC
  }
}

const normalizePathList = (values) => {
  if (!Array.isArray(values)) {
    return []
  }

  const roots = []

  for (const value of values) {
    const normalizedValue = String(value ?? '').trim()

    if (!normalizedValue) {
      continue
    }

    if (!roots.some((root) => root.toLowerCase() === normalizedValue.toLowerCase())) {
      roots.push(normalizedValue)
    }
  }

  return roots
}

const normalizeMicrosoftClientId = (value) => {
  const normalizedValue = String(value ?? '').trim()

  if (!normalizedValue) {
    return DEFAULT_MICROSOFT_CLIENT_ID
  }

  return normalizedValue
}

const readCachedLibrarySettings = () => {
  if (!hasLocalStorage()) {
    return null
  }

  try {
    const rawSettings = window.localStorage.getItem(LIBRARY_SETTINGS_KEY)

    if (!rawSettings) {
      return null
    }

    const parsedSettings = JSON.parse(rawSettings)
    return normalizeLibrarySettings(parsedSettings)
  } catch {
    return null
  }
}

const cacheLibrarySettings = (settings) => {
  if (!hasLocalStorage()) {
    return
  }

  try {
    window.localStorage.setItem(LIBRARY_SETTINGS_KEY, JSON.stringify(normalizeLibrarySettings(settings)))
  } catch {
    // Ignore cache write failures.
  }
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
