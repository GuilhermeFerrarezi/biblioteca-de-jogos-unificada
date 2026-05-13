export const INSTALL_STATUS = Object.freeze({
  INSTALLED: 'installed',
  NOT_INSTALLED: 'not_installed',
})

export const LAUNCH_ACTION_KIND = Object.freeze({
  MANUAL: 'manual',
  URI: 'uri',
  EXECUTABLE: 'executable',
})

export const PLATFORM_LABELS = Object.freeze({
  steam: 'Steam',
  xbox: 'Xbox',
  epic: 'Epic Games',
  gog: 'GOG',
  itch: 'itch.io',
  'battle-net': 'Battle.net',
  ubisoft: 'Ubisoft Connect',
  ea: 'EA App',
  local: 'Local',
  manual: 'Manual',
})

export const QUICK_FILTERS = Object.freeze([
  { id: 'all', label: 'Todos' },
  { id: 'installed', label: 'Instalados' },
  { id: 'steam', label: 'Steam' },
  { id: 'local', label: 'Locais' },
])

export const DEFAULT_ACCENT_COLOR = '#0d9488'
