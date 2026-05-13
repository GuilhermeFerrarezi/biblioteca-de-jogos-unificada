import { useDeferredValue, useMemo } from 'react'
import { INSTALL_STATUS, PLATFORM_LABELS } from '../constants/libraryConstants'
import { getPlaytimeHours } from '../adapters/libraryEntryAdapter'

export function useLibraryFiltering(entries, searchTerm, quickFilter) {
  const deferredSearchTerm = useDeferredValue(searchTerm)

  const filteredEntries = useMemo(() => {
    const normalizedSearch = deferredSearchTerm.trim().toLowerCase()

    return entries.filter((entry) => {
      const matchesSearch = normalizedSearch
        ? [
            entry.game.title,
            PLATFORM_LABELS[entry.primaryPlatformId] ?? entry.primaryPlatformId,
            entry.game.genres?.join(' ') ?? '',
            entry.installStatus === INSTALL_STATUS.INSTALLED ? 'instalado' : 'nao instalado',
          ].some((value) => value.toLowerCase().includes(normalizedSearch))
        : true

      const matchesQuickFilter =
        quickFilter === 'all' ||
        (quickFilter === 'installed' && entry.installStatus === INSTALL_STATUS.INSTALLED) ||
        quickFilter === entry.primaryPlatformId

      return matchesSearch && matchesQuickFilter
    })
  }, [deferredSearchTerm, entries, quickFilter])

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
