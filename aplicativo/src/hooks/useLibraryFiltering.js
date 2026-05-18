import { useDeferredValue, useMemo } from 'react'
import { INSTALL_STATUS, PLATFORM_LABELS, QUICK_FILTER_IDS } from '../constants/libraryConstants'
import { getPlaytimeHours } from '../adapters/libraryEntryAdapter'

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
  status: new Set([QUICK_FILTER_IDS.INSTALLED, QUICK_FILTER_IDS.NOT_INSTALLED]),
  platform: new Set([QUICK_FILTER_IDS.STEAM, QUICK_FILTER_IDS.XBOX, QUICK_FILTER_IDS.LOCAL]),
})

function matchesQuickFilter(entry, quickFilterId) {
  switch (quickFilterId) {
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

export function useLibraryFiltering(entries, searchTerm, quickFilters) {
  const deferredSearchTerm = useDeferredValue(searchTerm)
  const activeQuickFilters = useMemo(
    () => quickFilters.filter((quickFilterId) => quickFilterId !== QUICK_FILTER_IDS.ALL),
    [quickFilters],
  )
  const quickFilterGroups = useMemo(
    () =>
      activeQuickFilters.reduce((groups, quickFilterId) => {
        const groupId = getQuickFilterGroup(quickFilterId)
        const currentGroup = groups[groupId] ?? []

        return {
          ...groups,
          [groupId]: [...currentGroup, quickFilterId],
        }
      }, {}),
    [activeQuickFilters],
  )

  const filteredEntries = useMemo(() => {
    const normalizedSearch = normalizeSearchValue(deferredSearchTerm.trim())

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
  }, [deferredSearchTerm, entries, quickFilterGroups])

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
