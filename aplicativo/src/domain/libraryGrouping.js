import { PLATFORM_LABELS } from '../constants/libraryConstants.js'

const CROSS_PLATFORM_IDS = new Set(['steam', 'xbox', 'epic'])

const normalizeGroupingKey = (value) =>
  String(value ?? '')
    .trim()
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
    .replace(/\s+/g, '-')

const getEntryTitle = (entry) => String(entry?.game?.sortTitle ?? entry?.game?.title ?? '').trim()

const getPlatformLabel = (platformId) => PLATFORM_LABELS[platformId] ?? platformId

const uniqueBy = (items, getKey) => {
  const seen = new Set()

  return items.filter((item) => {
    const key = getKey(item)
    if (!key || seen.has(key)) {
      return false
    }

    seen.add(key)
    return true
  })
}

const mergeSources = (entries) =>
  uniqueBy(
    entries.flatMap((entry) => entry?.game?.sources ?? []),
    (source) => `${source?.platformId ?? ''}:${source?.externalId ?? ''}`,
  )

const mergeInstallLocations = (entries) =>
  uniqueBy(
    entries.flatMap((entry) => entry?.game?.installLocations ?? []),
    (location) => String(location ?? '').trim(),
  )

const mergePlatforms = (entries) =>
  uniqueBy(
    entries.flatMap((entry) => entry?.game?.platforms ?? [entry?.primaryPlatformId]),
    (platformId) => String(platformId ?? '').trim(),
  ).filter(Boolean)

const pickRepresentativeEntry = (entries) =>
  entries.find((entry) => entry?.installStatus === 'installed') ?? entries[0] ?? null

const pickPrimaryPlatformId = (entries) =>
  entries.find((entry) => entry?.primaryPlatformId === 'local')?.primaryPlatformId ??
  pickRepresentativeEntry(entries)?.primaryPlatformId ??
  'steam'

const pickInstallStatus = (entries) => entries.some((entry) => entry?.installStatus === 'installed') ? 'installed' : 'not_installed'

const pickLastPlayedLabel = (entries) =>
  entries.find((entry) => entry?.lastPlayedLabel && entry.lastPlayedLabel !== 'Nunca')?.lastPlayedLabel ?? entries[0]?.lastPlayedLabel ?? 'Nunca'

const pickArchivedState = (entries) => entries.every((entry) => entry?.isArchived === true)

const pickFavoriteState = (entries) => entries.some((entry) => entry?.isFavorite === true || entry?.is_favorite === true)

const hasArtworkImage = (artwork) => Boolean(artwork?.coverUrl || artwork?.heroUrl || artwork?.fallbackUrl)

const pickArtwork = (entries) =>
  entries.find((entry) => hasArtworkImage(entry?.game?.artwork))?.game?.artwork ??
  pickRepresentativeEntry(entries)?.game?.artwork ??
  entries[0]?.game?.artwork ??
  { accentColor: '#0d9488' }

const pickGenres = (entries) => {
  const genres = uniqueBy(
    entries.flatMap((entry) => entry?.game?.genres ?? []),
    (genre) => String(genre ?? '').trim().toLowerCase(),
  )

  return genres.length > 0 ? genres : ['Sem genero']
}

const pickTags = (entries) =>
  uniqueBy(
    entries.flatMap((entry) => entry?.game?.tags ?? []),
    (tag) => String(tag ?? '').trim().toLowerCase(),
  )

const pickUserOverrides = (entries) =>
  entries.reduce((merged, entry) => ({ ...merged, ...(entry?.game?.userOverrides ?? {}) }), {})

const pickAchievements = (entries) =>
  entries.find((entry) => entry?.primaryPlatformId === 'steam' && entry?.game?.achievements)?.game?.achievements ??
  entries.find((entry) => entry?.game?.achievements)?.game?.achievements ??
  null

const buildSyntheticId = (groupKey) => `group-${groupKey}`

export function groupLibraryEntries(entries) {
  const groupedByKey = new Map()

  entries.forEach((entry, index) => {
    const title = getEntryTitle(entry)
    const groupKey = normalizeGroupingKey(title)
    const platformId = String(entry?.primaryPlatformId ?? '').trim()

    if (!CROSS_PLATFORM_IDS.has(platformId) || !groupKey) {
      return
    }

    const currentGroup = groupedByKey.get(groupKey) ?? {
      kind: 'candidate',
      entries: [],
      firstIndex: index,
      platformIds: new Set(),
    }

    currentGroup.entries.push(entry)
    currentGroup.firstIndex = Math.min(currentGroup.firstIndex, index)
    currentGroup.platformIds.add(platformId)

    groupedByKey.set(groupKey, currentGroup)
  })

  const emittedGroupKeys = new Set()
  const result = []

  entries.forEach((entry, index) => {
    const title = getEntryTitle(entry)
    const groupKey = normalizeGroupingKey(title)
    const candidateGroup = groupedByKey.get(groupKey)

    if (candidateGroup?.kind === 'candidate' && candidateGroup.platformIds.size > 1) {
      if (candidateGroup.firstIndex === index && !emittedGroupKeys.has(groupKey)) {
        result.push(buildGroupedEntry(candidateGroup.entries, groupKey))
        emittedGroupKeys.add(groupKey)
      }

      return
    }

    result.push(entry)
  })

  return result
}

function buildGroupedEntry(entries, groupKey) {
  const representativeEntry = pickRepresentativeEntry(entries)
  const title = representativeEntry?.game?.title ?? entries[0]?.game?.title ?? 'Jogo'
  const platformIds = mergePlatforms(entries)

  return {
    ...representativeEntry,
    id: buildSyntheticId(groupKey),
    isGroupedCrossPlatform: true,
    memberEntryIds: entries.map((entry) => entry.id),
    memberEntries: entries,
    primaryPlatformId: pickPrimaryPlatformId(entries),
    installStatus: pickInstallStatus(entries),
    lastPlayedLabel: pickLastPlayedLabel(entries),
    isArchived: pickArchivedState(entries),
    isFavorite: pickFavoriteState(entries),
    platformIds,
    platformSummary: platformIds.map(getPlatformLabel).join(' + '),
    game: {
      ...representativeEntry?.game,
      title,
      sortTitle: representativeEntry?.game?.sortTitle ?? title,
      platforms: platformIds,
      sources: mergeSources(entries),
      installed: pickInstallStatus(entries) === 'installed',
      installLocations: mergeInstallLocations(entries),
      launchActions: representativeEntry?.game?.launchActions ?? [],
      playtime: {
        totalMinutes: entries.reduce((sum, entry) => sum + Number(entry?.game?.playtime?.totalMinutes ?? 0), 0),
      },
      artwork: pickArtwork(entries),
      achievements: pickAchievements(entries),
      genres: pickGenres(entries),
      tags: pickTags(entries),
      userOverrides: pickUserOverrides(entries),
    },
  }
}
