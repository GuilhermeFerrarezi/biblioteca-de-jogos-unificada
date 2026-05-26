import { useDeferredValue, useMemo } from 'react'
import { INSTALL_STATUS, PLATFORM_LABELS, QUICK_FILTER_IDS, SORT_MODE_IDS } from '../constants/libraryConstants.js'
import { getAchievementProgress, getPlaytimeHours } from '../adapters/libraryEntryAdapter.js'

const titleCollator = new Intl.Collator('pt-BR', {
  numeric: true,
  sensitivity: 'base',
})

const normalizeSearchValue = (value) =>
  String(value ?? '')
    .trim()
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
    .replace(/\s+/g, ' ')

const QUICK_FILTER_GROUPS = Object.freeze({
  favorites: new Set([QUICK_FILTER_IDS.FAVORITES]),
  status: new Set([QUICK_FILTER_IDS.INSTALLED, QUICK_FILTER_IDS.NOT_INSTALLED]),
  platform: new Set([QUICK_FILTER_IDS.STEAM, QUICK_FILTER_IDS.XBOX, QUICK_FILTER_IDS.LOCAL]),
})

function matchesQuickFilter(entry, quickFilterId) {
  switch (quickFilterId) {
    case QUICK_FILTER_IDS.FAVORITES:
      return isFavoriteEntry(entry)
    case QUICK_FILTER_IDS.INSTALLED:
      return entry.installStatus === INSTALL_STATUS.INSTALLED
    case QUICK_FILTER_IDS.NOT_INSTALLED:
      return entry.installStatus === INSTALL_STATUS.NOT_INSTALLED
    case QUICK_FILTER_IDS.STEAM:
    case QUICK_FILTER_IDS.XBOX:
    case QUICK_FILTER_IDS.LOCAL:
      return entry.platformIds?.includes(quickFilterId) || entry.primaryPlatformId === quickFilterId
    default:
      return true
  }
}

function getQuickFilterGroup(quickFilterId) {
  if (QUICK_FILTER_GROUPS.favorites.has(quickFilterId)) {
    return 'favorites'
  }

  if (QUICK_FILTER_GROUPS.status.has(quickFilterId)) {
    return 'status'
  }

  if (QUICK_FILTER_GROUPS.platform.has(quickFilterId)) {
    return 'platform'
  }

  return 'other'
}

function matchesQuickFilterGroup(entry, quickFilterIds) {
  return quickFilterIds.some((quickFilterId) => matchesQuickFilter(entry, quickFilterId))
}

const getEntryTitle = (entry) => String(entry?.game?.sortTitle ?? entry?.game?.title ?? '').trim()

const compareByTitle = (leftEntry, rightEntry, direction = 1) => {
  const titleComparison = titleCollator.compare(getEntryTitle(leftEntry), getEntryTitle(rightEntry))

  if (titleComparison !== 0) {
    return titleComparison * direction
  }

  return String(leftEntry?.id ?? '').localeCompare(String(rightEntry?.id ?? '')) * direction
}

const getPlaytimeMinutes = (entry) => Number(entry?.game?.playtime?.totalMinutes ?? 0)

export const isFavoriteEntry = (entry) => {
  if (entry?.isFavorite === true || entry?.is_favorite === true) {
    return true
  }

  return entry?.memberEntries?.some((memberEntry) => isFavoriteEntry(memberEntry)) ?? false
}

const getAchievementProgressValue = (entry) => {
  const progress = getAchievementProgress(entry)

  return progress.hasData ? progress.percentage : null
}

const compareByAchievementProgress = (leftEntry, rightEntry, direction) => {
  const leftProgress = getAchievementProgressValue(leftEntry)
  const rightProgress = getAchievementProgressValue(rightEntry)
  const leftHasData = leftProgress !== null
  const rightHasData = rightProgress !== null

  if (leftHasData !== rightHasData) {
    return leftHasData ? -1 : 1
  }

  if (!leftHasData && !rightHasData) {
    return compareByTitle(leftEntry, rightEntry)
  }

  return (leftProgress - rightProgress) * direction || compareByTitle(leftEntry, rightEntry)
}

export function sortLibraryEntries(entries, sortMode = SORT_MODE_IDS.ALPHA_ASC) {
  const normalizedSortMode = Object.values(SORT_MODE_IDS).includes(sortMode) ? sortMode : SORT_MODE_IDS.ALPHA_ASC

  return [...entries].sort((leftEntry, rightEntry) => {
    switch (normalizedSortMode) {
      case SORT_MODE_IDS.ALPHA_DESC:
        return compareByTitle(leftEntry, rightEntry, -1)
      case SORT_MODE_IDS.PLAYTIME_DESC: {
        const playtimeComparison = getPlaytimeMinutes(rightEntry) - getPlaytimeMinutes(leftEntry)
        return playtimeComparison || compareByTitle(leftEntry, rightEntry)
      }
      case SORT_MODE_IDS.PLAYTIME_ASC: {
        const playtimeComparison = getPlaytimeMinutes(leftEntry) - getPlaytimeMinutes(rightEntry)
        return playtimeComparison || compareByTitle(leftEntry, rightEntry)
      }
      case SORT_MODE_IDS.FAVORITES_FIRST: {
        const favoriteComparison = Number(isFavoriteEntry(rightEntry)) - Number(isFavoriteEntry(leftEntry))
        return favoriteComparison || compareByTitle(leftEntry, rightEntry)
      }
      case SORT_MODE_IDS.ACHIEVEMENTS_DESC: {
        return compareByAchievementProgress(leftEntry, rightEntry, -1)
      }
      case SORT_MODE_IDS.ACHIEVEMENTS_ASC: {
        return compareByAchievementProgress(leftEntry, rightEntry, 1)
      }
      case SORT_MODE_IDS.ALPHA_ASC:
      default:
        return compareByTitle(leftEntry, rightEntry)
    }
  })
}

export function filterLibraryEntries(entries, searchTerm, quickFilters) {
  const normalizedSearch = normalizeSearchValue(String(searchTerm ?? '').trim())
  const activeQuickFilters = quickFilters.filter((quickFilterId) => quickFilterId !== QUICK_FILTER_IDS.ALL)
  const quickFilterGroups = activeQuickFilters.reduce((groups, quickFilterId) => {
    const groupId = getQuickFilterGroup(quickFilterId)
    const currentGroup = groups[groupId] ?? []

    return {
      ...groups,
      [groupId]: [...currentGroup, quickFilterId],
    }
  }, {})

  return entries.filter((entry) => {
    const matchesSearch = normalizedSearch
      ? [
          entry.game.title,
          entry.platformSummary ?? PLATFORM_LABELS[entry.primaryPlatformId] ?? entry.primaryPlatformId,
          entry.platformIds?.map((platformId) => PLATFORM_LABELS[platformId] ?? platformId).join(' ') ?? '',
          entry.game.genres?.join(' ') ?? '',
          entry.installStatus === INSTALL_STATUS.INSTALLED ? 'instalado' : 'nao instalado',
        ].some((value) => normalizeSearchValue(value).includes(normalizedSearch))
      : true

    const matchesQuickFilters = Object.values(quickFilterGroups).every((quickFilterIds) =>
      matchesQuickFilterGroup(entry, quickFilterIds),
    )

    return matchesSearch && matchesQuickFilters
  })
}

export function getVisibleLibraryEntries(entries, searchTerm, quickFilters, sortMode) {
  return sortLibraryEntries(filterLibraryEntries(entries, searchTerm, quickFilters), sortMode)
}

export function useLibraryFiltering(entries, searchTerm, quickFilters, sortMode = SORT_MODE_IDS.ALPHA_ASC) {
  const deferredSearchTerm = useDeferredValue(searchTerm)

  const filteredEntries = useMemo(() => {
    return getVisibleLibraryEntries(entries, deferredSearchTerm, quickFilters, sortMode)
  }, [deferredSearchTerm, entries, quickFilters, sortMode])

  const installedCount = useMemo(
    () => entries.filter((entry) => entry.installStatus === INSTALL_STATUS.INSTALLED).length,
    [entries],
  )
  const totalHours = useMemo(
    () => entries.reduce((sum, entry) => sum + getPlaytimeHours(entry.game.playtime.totalMinutes), 0),
    [entries],
  )

  return {
    filteredEntries,
    installedCount,
    totalHours,
  }
}
