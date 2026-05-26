import { DEFAULT_ACCENT_COLOR, INSTALL_STATUS, LAUNCH_ACTION_KIND } from '../constants/libraryConstants.js'

export const emptyManualGameForm = Object.freeze({
  title: '',
  genre: '',
  installStatus: INSTALL_STATUS.NOT_INSTALLED,
  launchTarget: '',
})

export const getPlaytimeHours = (minutes) => Math.floor(minutes / 60)

export const getAchievementProgress = (entry) => {
  const candidates = [
    entry?.achievementProgress,
    entry?.achievement_progress,
    entry?.achievements,
    entry?.game?.achievementProgress,
    entry?.game?.achievement_progress,
    entry?.game?.achievements,
    ...(entry?.memberEntries?.flatMap((memberEntry) => [
      memberEntry?.achievementProgress,
      memberEntry?.achievement_progress,
      memberEntry?.achievements,
      memberEntry?.game?.achievementProgress,
      memberEntry?.game?.achievement_progress,
      memberEntry?.game?.achievements,
    ]) ?? []),
  ]

  return candidates.map(normalizeAchievementProgress).find((progress) => progress.hasData) ?? normalizeAchievementProgress(null)
}

export const getAchievementSummaryLabel = (entry) => {
  const progress = getAchievementProgress(entry)

  if (!progress.hasData) {
    return 'Sem dados'
  }

  return `${progress.unlocked}/${progress.total} conquistas`
}

export const getAchievementKey = (achievement, index = 0) =>
  String(achievement?.apiName ?? achievement?.api_name ?? achievement?.id ?? `achievement-${index}`)

export const getAchievementDisplayState = (achievement, revealedSecretAchievements = new Set(), index = 0) => {
  const apiName = getAchievementKey(achievement, index)
  const isHidden = achievement?.hidden === true
  const isAchieved = achievement?.achieved === true
  const isSecretLocked = isHidden && !isAchieved
  const isRevealed = isSecretLocked && revealedSecretAchievements.has(apiName)
  const shouldMask = isSecretLocked && !isRevealed
  const name = shouldMask ? 'Conquista secreta' : achievement?.name || apiName || 'Conquista'
  const description = shouldMask
    ? 'Conteudo oculto ate voce revelar manualmente.'
    : achievement?.description || (isAchieved ? 'Conquista desbloqueada.' : 'Ainda bloqueada.')
  const iconUrl = shouldMask
    ? achievement?.lockedIconUrl ?? achievement?.locked_icon_url ?? achievement?.iconUrl ?? achievement?.icon_url
    : achievement?.iconUrl ?? achievement?.icon_url ?? achievement?.lockedIconUrl ?? achievement?.locked_icon_url

  return {
    apiName,
    description,
    iconUrl,
    isAchieved,
    isHidden,
    isRevealed,
    isSecretLocked,
    name,
    shouldMask,
    visibleText: `${name} ${description}`.trim(),
  }
}

export const sortAchievementItems = (items, revealedSecretAchievements = new Set()) =>
  [...(Array.isArray(items) ? items : [])]
    .map((achievement, index) => ({
      achievement,
      index,
      display: getAchievementDisplayState(achievement, revealedSecretAchievements, index),
    }))
    .sort((left, right) => {
      if (left.display.isAchieved !== right.display.isAchieved) {
        return left.display.isAchieved ? -1 : 1
      }

      const leftSecretRank = left.display.isSecretLocked ? 1 : 0
      const rightSecretRank = right.display.isSecretLocked ? 1 : 0

      if (leftSecretRank !== rightSecretRank) {
        return leftSecretRank - rightSecretRank
      }

      const textComparison = left.display.name.localeCompare(right.display.name, 'pt-BR', {
        sensitivity: 'base',
      })

      return textComparison !== 0 ? textComparison : left.index - right.index
    })
    .map(({ achievement }) => achievement)

export const filterAchievementItems = (items, searchTerm, revealedSecretAchievements = new Set()) => {
  const query = String(searchTerm ?? '').trim().toLocaleLowerCase('pt-BR')

  if (!query) {
    return Array.isArray(items) ? items : []
  }

  return (Array.isArray(items) ? items : []).filter((achievement, index) =>
    getAchievementDisplayState(achievement, revealedSecretAchievements, index)
      .visibleText.toLocaleLowerCase('pt-BR')
      .includes(query),
  )
}

function normalizeAchievementProgress(value) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return {
      hasData: true,
      unlocked: 0,
      total: 0,
      percentage: Math.max(0, value),
      items: [],
    }
  }

  if (!value || typeof value !== 'object') {
    return {
      hasData: false,
      unlocked: 0,
      total: 0,
      percentage: 0,
      items: [],
    }
  }

  const unlocked = Number(value.unlocked ?? value.earned ?? value.completed ?? 0)
  const total = Number(value.total ?? value.available ?? value.count ?? 0)
  const directPercentage = Number(
    value.progress ?? value.percentage ?? value.percent ?? value.completionPercent ?? value.completion_percent ?? value.ratio,
  )
  const percentage = Number.isFinite(directPercentage)
    ? directPercentage
    : Number.isFinite(unlocked) && Number.isFinite(total) && total > 0
      ? (unlocked / total) * 100
      : 0
  const items = Array.isArray(value.items) ? value.items : []
  const hasData = total > 0 || items.length > 0 || Number.isFinite(directPercentage)

  return {
    hasData,
    unlocked: Number.isFinite(unlocked) ? unlocked : 0,
    total: Number.isFinite(total) ? total : items.length,
    percentage: Math.max(0, percentage),
    items,
  }
}

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
    isFavorite: existingEntry?.isFavorite ?? false,
  }
}

export const getSelectedEntryIdForEntries = (nextEntries, currentSelectedEntryId) =>
  nextEntries.some((entry) => entry.id === currentSelectedEntryId)
    ? currentSelectedEntryId
    : nextEntries[0]?.id ?? ''
