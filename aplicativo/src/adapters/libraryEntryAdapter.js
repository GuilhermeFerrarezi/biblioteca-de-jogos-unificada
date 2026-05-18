import { DEFAULT_ACCENT_COLOR, INSTALL_STATUS, LAUNCH_ACTION_KIND } from '../constants/libraryConstants.js'

export const emptyManualGameForm = Object.freeze({
  title: '',
  genre: '',
  installStatus: INSTALL_STATUS.NOT_INSTALLED,
  launchTarget: '',
})

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

  return palette[hash % palette.length] ?? DEFAULT_ACCENT_COLOR
}

const getLaunchActionKind = (target) => {
  if (!target) {
    return LAUNCH_ACTION_KIND.MANUAL
  }

  return target.includes('://') ? LAUNCH_ACTION_KIND.URI : LAUNCH_ACTION_KIND.EXECUTABLE
}

export const getPrimaryLaunchAction = (entry) =>
  entry?.game.launchActions.find((action) => action.isPrimary) ?? entry?.game.launchActions[0] ?? null

export const getManualGameFormFromEntry = (entry) => ({
  title: entry?.game.title ?? '',
  genre: entry?.game.genres?.[0] ?? '',
  installStatus: entry?.installStatus ?? INSTALL_STATUS.NOT_INSTALLED,
  launchTarget: getPrimaryLaunchAction(entry)?.target ?? '',
})

const isValidUriTarget = (target) => {
  try {
    const url = new URL(target)
    return Boolean(url.protocol) && target.includes('://')
  } catch {
    return false
  }
}

const isValidExecutableTarget = (target) =>
  /^[a-zA-Z]:[\\/][^<>:"|?*]+\.exe$/i.test(target.trim())

export const validateManualGameInput = (input) => {
  const errors = {}
  const title = input?.title?.trim() ?? ''
  const installStatus = input?.installStatus ?? ''
  const launchTarget = input?.launchTarget?.trim() ?? ''

  if (!title) {
    errors.title = 'Informe o titulo do jogo.'
  }

  if (![INSTALL_STATUS.INSTALLED, INSTALL_STATUS.NOT_INSTALLED].includes(installStatus)) {
    errors.installStatus = 'Escolha um status valido.'
  }

  if (launchTarget && !isValidUriTarget(launchTarget) && !isValidExecutableTarget(launchTarget)) {
    errors.launchTarget = 'Use uma URI valida ou um caminho absoluto para arquivo .exe.'
  }

  return {
    errors,
    isValid: Object.keys(errors).length === 0,
  }
}

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
      installed: form.installStatus === INSTALL_STATUS.INSTALLED,
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
