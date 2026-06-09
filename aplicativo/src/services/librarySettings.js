import { invoke } from '@tauri-apps/api/core'
import { hasLocalStorage, hasTauriRuntime } from './tauriRuntime.js'

const LIBRARY_SETTINGS_KEY = 'biblioteca-jogos-unificada.library-settings'
const LIBRARY_SCAN_MODE_AUTOMATIC = 'automatic'
const LIBRARY_SCAN_MODE_SELECTED_ONLY = 'selected_only'
const LIBRARY_SCAN_MODE_AUTOMATIC_PLUS_EXTRA = 'automatic_plus_extra'
export const LIBRARY_GRID_SIZE_COMPACT = 'compact'
export const LIBRARY_GRID_SIZE_DEFAULT = 'default'
export const LIBRARY_GRID_SIZE_LARGE = 'large'
const DEFAULT_MICROSOFT_CLIENT_ID = String(import.meta.env?.VITE_XBOX_CLIENT_ID ?? '').trim()

const defaultLibrarySettings = Object.freeze({
  preferredStoreId: 'steam',
  gridSize: LIBRARY_GRID_SIZE_DEFAULT,
  localScanMode: LIBRARY_SCAN_MODE_AUTOMATIC,
  localScanRoots: [],
  localScanExcludedRoots: [],
  microsoftClientId: DEFAULT_MICROSOFT_CLIENT_ID,
})

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

export const normalizeLibraryGridSize = (value) => {
  switch (String(value ?? '').trim().toLowerCase()) {
    case LIBRARY_GRID_SIZE_COMPACT:
      return LIBRARY_GRID_SIZE_COMPACT
    case LIBRARY_GRID_SIZE_LARGE:
      return LIBRARY_GRID_SIZE_LARGE
    default:
      return LIBRARY_GRID_SIZE_DEFAULT
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

export const normalizeLibrarySettings = (settings) => {
  if (!settings || typeof settings !== 'object') {
    return { ...defaultLibrarySettings }
  }

  const preferredStoreId = normalizePreferredStoreId(settings.preferredStoreId)
  const gridSize = normalizeLibraryGridSize(settings.gridSize)
  const localScanMode = normalizeLocalScanMode(settings.localScanMode)
  const localScanRoots = normalizePathList(settings.localScanRoots)
  const localScanExcludedRoots = normalizePathList(settings.localScanExcludedRoots)
  const microsoftClientId = normalizeMicrosoftClientId(
    settings.microsoftClientId ?? settings.microsoftClientID ?? settings.xboxLiveClientId,
  )

  return {
    preferredStoreId,
    gridSize,
    localScanMode,
    localScanRoots,
    localScanExcludedRoots,
    microsoftClientId,
  }
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
