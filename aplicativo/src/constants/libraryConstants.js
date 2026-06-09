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
  FAVORITES: 'favorites',
  INSTALLED: 'installed',
  NOT_INSTALLED: 'not_installed',
  RATED: 'rated',
  UNRATED: 'unrated',
  STEAM: 'steam',
  XBOX: 'xbox',
  EPIC: 'epic',
  LOCAL: 'local',
})

export const QUICK_FILTERS = Object.freeze([
  { id: QUICK_FILTER_IDS.ALL, label: 'Todos' },
  { id: QUICK_FILTER_IDS.FAVORITES, label: 'Favoritos' },
  { id: QUICK_FILTER_IDS.INSTALLED, label: 'Instalados' },
  { id: QUICK_FILTER_IDS.NOT_INSTALLED, label: 'Nao instalados' },
  { id: QUICK_FILTER_IDS.RATED, label: 'Avaliados' },
  { id: QUICK_FILTER_IDS.UNRATED, label: 'Não avaliados' },
  { id: QUICK_FILTER_IDS.STEAM, label: 'Steam' },
  { id: QUICK_FILTER_IDS.XBOX, label: 'Xbox' },
  { id: QUICK_FILTER_IDS.EPIC, label: 'Epic' },
  { id: QUICK_FILTER_IDS.LOCAL, label: 'Locais' },
])

export const SORT_MODE_IDS = Object.freeze({
  ALPHA_ASC: 'alpha_asc',
  ALPHA_DESC: 'alpha_desc',
  PLAYTIME_DESC: 'playtime_desc',
  PLAYTIME_ASC: 'playtime_asc',
  FAVORITES_FIRST: 'favorites_first',
  ACHIEVEMENTS_DESC: 'achievements_desc',
  ACHIEVEMENTS_ASC: 'achievements_asc',
  PERSONAL_RATING_DESC: 'personal_rating_desc',
  PERSONAL_RATING_ASC: 'personal_rating_asc',
})

export const SORT_MODE_OPTIONS = Object.freeze([
  { id: SORT_MODE_IDS.ALPHA_ASC, label: 'A-Z' },
  { id: SORT_MODE_IDS.ALPHA_DESC, label: 'Z-A' },
  { id: SORT_MODE_IDS.PLAYTIME_DESC, label: 'Horas: maior' },
  { id: SORT_MODE_IDS.PLAYTIME_ASC, label: 'Horas: menor' },
  { id: SORT_MODE_IDS.FAVORITES_FIRST, label: 'Favoritos primeiro' },
  { id: SORT_MODE_IDS.PERSONAL_RATING_DESC, label: 'Melhor avaliados' },
  { id: SORT_MODE_IDS.PERSONAL_RATING_ASC, label: 'Pior avaliados' },
  { id: SORT_MODE_IDS.ACHIEVEMENTS_DESC, label: 'Conquistas: maior' },
  { id: SORT_MODE_IDS.ACHIEVEMENTS_ASC, label: 'Conquistas: menor' },
])

export const DEFAULT_ACCENT_COLOR = '#0d9488'
