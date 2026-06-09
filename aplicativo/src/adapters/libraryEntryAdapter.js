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
  const normalizedCandidates = candidates.map(normalizeAchievementProgress)

  return normalizedCandidates.find((progress) => progress.hasData) ??
    normalizedCandidates.find((progress) => progress.hasCache) ??
    normalizeAchievementProgress(null)
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
  const name = shouldMask ? 'Conquista secreta' : achievement?.name || apiName || 'Conquista sem nome'
  const description = shouldMask
    ? 'Conteudo oculto ate voce revelar manualmente.'
    : achievement?.description || 'A Steam não disponibilizou a descrição desta conquista secreta.'
  const iconUrl = shouldMask
    ? achievement?.lockedIconUrl ?? achievement?.locked_icon_url ?? achievement?.iconUrl ?? achievement?.icon_url
    : achievement?.iconUrl ?? achievement?.icon_url ?? achievement?.lockedIconUrl ?? achievement?.locked_icon_url
  const visibleText = `${name} ${description} ${isHidden ? 'secreta' : ''}`.trim()

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
    visibleText,
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

      const leftSortName = left.display.isSecretLocked ? 'Conquista secreta' : left.display.name
      const rightSortName = right.display.isSecretLocked ? 'Conquista secreta' : right.display.name
      const textComparison = leftSortName.localeCompare(rightSortName, 'pt-BR', {
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

export const buildAchievementObservability = (progress, steamEnrichmentStatus = null, now = new Date()) => {
  if (steamEnrichmentStatus?.rateLimited) {
    return {
      tone: 'warning',
      text: 'Limite da Steam atingido',
      detail: steamEnrichmentStatus.detail || 'Nova tentativa depois, sem bloquear a biblioteca.',
      expandable: true,
    }
  }

  if (steamEnrichmentStatus?.phase === 'running') {
    return {
      tone: 'running',
      text: 'Sincronizando conquistas...',
      detail: steamEnrichmentStatus.detail || 'O enrichment Steam roda em background.',
      expandable: false,
    }
  }

  if (steamEnrichmentStatus?.phase === 'failed') {
    return {
      tone: 'warning',
      text: steamEnrichmentStatus.recoverable === false ? 'Falha no enrichment' : 'Falha temporaria',
      detail: steamEnrichmentStatus.detail || 'O enrichment Steam falhou sem expor payload bruto.',
      expandable: true,
    }
  }

  if (progress?.hasCache && progress?.hasData) {
    return {
      tone: 'ready',
      text: formatAchievementCacheAge(progress.fetchedAt, now),
      detail: `Cache Steam atualizado em ${formatAchievementCacheDate(progress.fetchedAt)}.`,
      expandable: false,
    }
  }

  if (progress?.hasCache) {
    return {
      tone: 'muted',
      text: 'Sem dados da Steam',
      detail: 'A Steam não disponibilizou dados para este jogo. Pode ser privacidade, indisponibilidade ou ausencia de conquistas conhecidas.',
      expandable: true,
    }
  }

  return {
    tone: 'pending',
    text: 'Dados ainda não sincronizados',
    detail: 'As conquistas e metadados Steam serao preenchidos pelo enrichment em background quando disponiveis.',
    expandable: false,
  }
}

function normalizeAchievementProgress(value) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return {
      hasData: true,
      hasCache: true,
      fetchedAt: '',
      unlocked: 0,
      total: 0,
      percentage: Math.max(0, value),
      items: [],
    }
  }

  if (!value || typeof value !== 'object') {
    return {
      hasData: false,
      hasCache: false,
      fetchedAt: '',
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
  const fetchedAt = String(value.fetchedAt ?? value.fetched_at ?? value.cachedAt ?? value.cached_at ?? '').trim()
  const hasData = total > 0 || items.length > 0 || Number.isFinite(directPercentage)
  const hasCache = hasData || Boolean(fetchedAt)

  return {
    hasData,
    hasCache,
    fetchedAt,
    unlocked: Number.isFinite(unlocked) ? unlocked : 0,
    total: Number.isFinite(total) ? total : items.length,
    percentage: Math.max(0, percentage),
    items,
  }
}

function formatAchievementCacheAge(fetchedAt, now) {
  const fetchedDate = parseDate(fetchedAt)
  const currentDate = now instanceof Date ? now : parseDate(now)

  if (!fetchedDate || !currentDate) {
    return 'Cache atualizado'
  }

  const diffMinutes = Math.max(0, Math.floor((currentDate.getTime() - fetchedDate.getTime()) / 60000))

  if (diffMinutes < 1) {
    return 'Atualizado agora'
  }

  if (diffMinutes < 60) {
    return `Atualizado ha ${diffMinutes} min`
  }

  const diffHours = Math.floor(diffMinutes / 60)

  if (diffHours < 24) {
    return `Atualizado ha ${diffHours}h`
  }

  const diffDays = Math.floor(diffHours / 24)
  return `Atualizado ha ${diffDays}d`
}

function formatAchievementCacheDate(fetchedAt) {
  const fetchedDate = parseDate(fetchedAt)

  if (!fetchedDate) {
    return 'data desconhecida'
  }

  return new Intl.DateTimeFormat('pt-BR', {
    dateStyle: 'short',
    timeStyle: 'short',
  }).format(fetchedDate)
}

function parseDate(value) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? null : date
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
      personalRating: existingEntry?.game.personalRating ?? null,
      personalReview: existingEntry?.game.personalReview ?? null,
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
