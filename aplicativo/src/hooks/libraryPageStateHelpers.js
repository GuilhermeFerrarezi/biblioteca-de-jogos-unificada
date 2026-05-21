import { INSTALL_STATUS, PLATFORM_LABELS } from '../constants/libraryConstants.js'

const MICROSOFT_STORE_URI_PREFIX = 'ms-windows-store://pdp/?productid='
const MICROSOFT_STORE_SEARCH_URI_PREFIX = 'ms-windows-store://search/?query='
const MICROSOFT_STORE_PRODUCT_ID_PATTERN = /^[0-9a-z]{12}$/i
const MICROSOFT_STORE_SEARCH_SUFFIX_PATTERNS = [
  /\s*[-–—:]\s*(?:deluxe|ultimate|standard|game of the year|goty|anniversary|special|collector'?s|definitive|enhanced|remastered|reloaded|complete|bundle|edition)\s*$/i,
  /\s*\((?:deluxe|ultimate|standard|game of the year|goty|anniversary|special|collector'?s|definitive|enhanced|remastered|reloaded|complete|bundle|edition|launcher|app|demo|beta|trial)\)\s*$/i,
  /\s*(?:launcher|app|demo|beta|trial)\s*$/i,
]

export function buildMicrosoftStoreUri(productId) {
  const normalizedProductId = String(productId ?? '').trim()

  if (!normalizedProductId) {
    return ''
  }

  return `${MICROSOFT_STORE_URI_PREFIX}${encodeURIComponent(normalizedProductId)}`
}

export function buildMicrosoftStoreSearchUri(title) {
  const normalizedTitle = normalizeMicrosoftStoreSearchTitle(title)

  if (!normalizedTitle) {
    return ''
  }

  return `${MICROSOFT_STORE_SEARCH_URI_PREFIX}${encodeURIComponent(normalizedTitle)}`
}

function isLikelyMicrosoftStoreProductId(value) {
  return MICROSOFT_STORE_PRODUCT_ID_PATTERN.test(String(value ?? '').trim())
}

function normalizeMicrosoftStoreSearchTitle(title) {
  let normalizedTitle = String(title ?? '').replace(/\s+/g, ' ').replace(/[™®©]/g, '').trim()

  if (!normalizedTitle) {
    return ''
  }

  for (let index = 0; index < 3; index += 1) {
    const reducedTitle = MICROSOFT_STORE_SEARCH_SUFFIX_PATTERNS.reduce(
      (currentTitle, pattern) => currentTitle.replace(pattern, '').trim(),
      normalizedTitle,
    )

    if (reducedTitle === normalizedTitle) {
      break
    }

    normalizedTitle = reducedTitle
  }

  return normalizedTitle.length >= 3 ? normalizedTitle : String(title ?? '').replace(/\s+/g, ' ').trim()
}

export function isMicrosoftStoreUri(target) {
  const normalizedTarget = String(target ?? '').trim()

  if (!normalizedTarget) {
    return false
  }

  return /^(ms-windows-store:|https:\/\/(www\.)?microsoft\.com\/store\/|https:\/\/apps\.microsoft\.com\/store\/)/i.test(
    normalizedTarget,
  )
}

export function resolveMicrosoftStoreTarget(selectedEntry) {
  const directStoreTarget = [
    selectedEntry?.game?.storeTarget,
    selectedEntry?.game?.storeUri,
    selectedEntry?.game?.microsoftStoreTarget,
    selectedEntry?.game?.microsoftStoreUri,
  ].find((target) => isMicrosoftStoreUri(target))

  if (directStoreTarget) {
    return String(directStoreTarget).trim()
  }

  const primaryStoreAction = selectedEntry?.game?.launchActions?.find(
    (action) => action?.kind === 'uri' && isMicrosoftStoreUri(action.target),
  )

  if (primaryStoreAction?.target) {
    return String(primaryStoreAction.target).trim()
  }

  if (selectedEntry?.primaryPlatformId !== 'xbox') {
    return ''
  }

  const xboxSource = selectedEntry?.game?.sources?.find(
    (source) => source?.platformId === 'xbox' || source?.platformId === 'microsoft-store',
  )
  const productId =
    selectedEntry?.game?.microsoftStoreProductId ??
    selectedEntry?.game?.storeProductId ??
    (isLikelyMicrosoftStoreProductId(xboxSource?.externalId) ? xboxSource.externalId : '')

  if (productId) {
    return buildMicrosoftStoreUri(productId)
  }

  return buildMicrosoftStoreSearchUri(selectedEntry?.game?.title ?? '')
}

export function getVisibleSelectedEntry(filteredEntries, selectedEntryId) {
  return filteredEntries.find((entry) => entry.id === selectedEntryId) ?? filteredEntries[0] ?? null
}

export function getLaunchActionState(selectedEntry, preferredPlatformId = 'steam') {
  if (Array.isArray(selectedEntry?.memberEntries) && selectedEntry.memberEntries.length > 0) {
    const preferredChoice = getLaunchChoices(selectedEntry, preferredPlatformId)[0] ?? null

    if (preferredChoice) {
      return {
        primaryLaunchAction: preferredChoice.launchAction,
        canLaunch: preferredChoice.canLaunch,
        hint: '',
      }
    }
  }

  const primaryLaunchAction = selectedEntry
    ? selectedEntry.game.launchActions.find((action) => action.isPrimary) ?? selectedEntry.game.launchActions[0] ?? null
    : null
  const isXboxStoreEntry = selectedEntry?.primaryPlatformId === 'xbox' && selectedEntry?.installStatus !== INSTALL_STATUS.INSTALLED
  const microsoftStoreTarget = resolveMicrosoftStoreTarget(selectedEntry)

  if (isXboxStoreEntry) {
    return {
      primaryLaunchAction: {
        id: selectedEntry ? `launch-store-${selectedEntry.id}` : 'launch-store-xbox',
        platformId: 'xbox',
        kind: 'uri',
        label: 'Abrir Microsoft Store',
        target: microsoftStoreTarget,
        isPrimary: true,
      },
      canLaunch: Boolean(microsoftStoreTarget),
      hint: microsoftStoreTarget ? '' : 'A Microsoft Store sera usada quando o backend informar o link do jogo.',
    }
  }

  const canLaunch = Boolean(primaryLaunchAction && primaryLaunchAction.kind !== 'manual' && primaryLaunchAction.target)

  if (!selectedEntry || canLaunch) {
    return {
      primaryLaunchAction,
      canLaunch,
      hint: '',
    }
  }

  return {
    primaryLaunchAction,
    canLaunch,
    hint:
      selectedEntry.game.launchActions.length === 0
        ? 'Este jogo ainda nao tem acao de lancamento configurada.'
        : 'Este jogo nao possui uma acao executavel. Edite o cadastro para informar um destino valido.',
  }
}

const PREFERRED_PLATFORM_ORDER = ['steam', 'xbox']

const compareLaunchChoices = (left, right, preferredPlatformId) => {
  if (left.platformId === preferredPlatformId && right.platformId !== preferredPlatformId) {
    return -1
  }

  if (right.platformId === preferredPlatformId && left.platformId !== preferredPlatformId) {
    return 1
  }

  const leftIndex = PREFERRED_PLATFORM_ORDER.indexOf(left.platformId)
  const rightIndex = PREFERRED_PLATFORM_ORDER.indexOf(right.platformId)

  return (leftIndex === -1 ? Number.MAX_SAFE_INTEGER : leftIndex) - (rightIndex === -1 ? Number.MAX_SAFE_INTEGER : rightIndex)
}

export function getLaunchChoices(selectedEntry, preferredPlatformId = 'steam') {
  if (!selectedEntry) {
    return []
  }

  const launchEntries =
    Array.isArray(selectedEntry.memberEntries) && selectedEntry.memberEntries.length > 0
      ? selectedEntry.memberEntries
      : [selectedEntry]

  const choicesByPlatform = new Map()

  launchEntries
    .map((entry) => {
      const launchState = getLaunchActionState(entry)
      const primaryLaunchAction = launchState.primaryLaunchAction

      if (!primaryLaunchAction || !launchState.canLaunch) {
        return null
      }

      return {
        entryId: entry.id,
        platformId: entry.primaryPlatformId,
        platformLabel: PLATFORM_LABELS[entry.primaryPlatformId] ?? entry.primaryPlatformId,
        actionLabel: primaryLaunchAction.label,
        launchAction: {
          ...primaryLaunchAction,
          platformId: primaryLaunchAction.platformId ?? entry.primaryPlatformId,
        },
        canLaunch: launchState.canLaunch,
      }
    })
    .filter(Boolean)
    .sort((left, right) => compareLaunchChoices(left, right, preferredPlatformId))
    .forEach((choice) => {
      if (!choicesByPlatform.has(choice.platformId)) {
        choicesByPlatform.set(choice.platformId, choice)
      }
    })

  return Array.from(choicesByPlatform.values())
}

export function getPreferredLaunchEntryId(selectedEntry, preferredPlatformId = 'steam') {
  const launchChoices = getLaunchChoices(selectedEntry, preferredPlatformId)

  return launchChoices[0]?.entryId ?? selectedEntry?.id ?? ''
}
