export const emptyManualGameForm = {
  title: '',
  genre: '',
  installStatus: 'not_installed',
  launchTarget: '',
}

export const getPlaytimeHours = (minutes) => Math.floor(minutes / 60)

const createSlug = (value) =>
  value
    .trim()
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'jogo-manual'

const getDeterministicAccentColor = (value) => {
  const palette = ['#0d9488', '#2563eb', '#7c3aed', '#be123c', '#c2410c', '#15803d', '#9333ea', '#b45309']
  const hash = [...value].reduce((total, char) => total + char.charCodeAt(0), 0)

  return palette[hash % palette.length]
}

const getLaunchActionKind = (target) => {
  if (!target) {
    return 'manual'
  }

  return target.includes('://') ? 'uri' : 'executable'
}

export const getPrimaryLaunchAction = (entry) =>
  entry?.game.launchActions.find((action) => action.isPrimary) ?? entry?.game.launchActions[0] ?? null

export const getManualGameFormFromEntry = (entry) => ({
  title: entry?.game.title ?? '',
  genre: entry?.game.genres?.[0] ?? '',
  installStatus: entry?.installStatus ?? 'not_installed',
  launchTarget: getPrimaryLaunchAction(entry)?.target ?? '',
})

export const buildManualLibraryEntry = (form, existingEntry = null) => {
  const title = form.title.trim()
  const genre = form.genre.trim()
  const launchTarget = form.launchTarget.trim()
  const slug = createSlug(title)
  const timestamp = new Date().toISOString()
  const existingLaunchAction = getPrimaryLaunchAction(existingEntry)
  const launchAction = {
    id: existingLaunchAction?.id ?? `launch-manual-${slug}`,
    platformId: existingLaunchAction?.platformId ?? 'manual',
    kind: getLaunchActionKind(launchTarget),
    label: launchTarget || 'Sem acao configurada',
    target: launchTarget,
    isPrimary: true,
  }

  return {
    id: existingEntry?.id ?? `entry-manual-${slug}-${Date.now()}`,
    primaryPlatformId: 'manual',
    installStatus: form.installStatus,
    lastPlayedLabel: existingEntry?.lastPlayedLabel ?? 'Nunca',
    addedAt: existingEntry?.addedAt ?? timestamp,
    updatedAt: timestamp,
    game: {
      internalId: existingEntry?.game.internalId ?? `game-manual-${slug}`,
      title,
      sortTitle: title,
      platforms: existingEntry?.game.platforms ?? ['manual'],
      sources:
        existingEntry?.game.sources ?? [{ platformId: 'manual', externalId: `manual-${slug}` }],
      installed: form.installStatus === 'installed',
      installLocations: existingEntry?.game.installLocations ?? [],
      launchActions: [launchAction],
      playtime: existingEntry?.game.playtime ?? { totalMinutes: 0 },
      artwork: { accentColor: getDeterministicAccentColor(title) },
      genres: genre ? [genre] : ['Sem genero'],
      tags: existingEntry?.game.tags ?? [],
      userOverrides: existingEntry?.game.userOverrides ?? {},
    },
    isArchived: existingEntry?.isArchived ?? false,
  }
}

export const getSelectedEntryIdForEntries = (nextEntries, currentSelectedEntryId) =>
  nextEntries.some((entry) => entry.id === currentSelectedEntryId)
    ? currentSelectedEntryId
    : nextEntries[0]?.id ?? ''
