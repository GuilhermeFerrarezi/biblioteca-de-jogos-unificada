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

export const QUICK_FILTER_IDS = Object.freeze({
  ALL: 'all',
  INSTALLED: 'installed',
  NOT_INSTALLED: 'not_installed',
  STEAM: 'steam',
  XBOX: 'xbox',
  LOCAL: 'local',
})

export const QUICK_FILTERS = Object.freeze([
  { id: QUICK_FILTER_IDS.ALL, label: 'Todos' },
  { id: QUICK_FILTER_IDS.INSTALLED, label: 'Instalados' },
  { id: QUICK_FILTER_IDS.NOT_INSTALLED, label: 'Nao instalados' },
  { id: QUICK_FILTER_IDS.STEAM, label: 'Steam' },
  { id: QUICK_FILTER_IDS.XBOX, label: 'Xbox' },
  { id: QUICK_FILTER_IDS.LOCAL, label: 'Locais' },
])

export const DEFAULT_ACCENT_COLOR = '#0d9488'
