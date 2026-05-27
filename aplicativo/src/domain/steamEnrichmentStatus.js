const STEAM_ENRICHMENT_DEFAULT_DETAIL = 'Enriquecendo metadados Steam em background.'

const getPayloadNumber = (payload, keys) => {
  if (!payload || typeof payload !== 'object') {
    return null
  }

  for (const key of keys) {
    const value = payload[key]

    if (value === null || value === undefined || value === '') {
      continue
    }

    const numberValue = typeof value === 'number' ? value : Number(value)

    if (Number.isFinite(numberValue)) {
      return numberValue
    }
  }

  return null
}

const hasPayloadValue = (payload, keys) => {
  if (!payload || typeof payload !== 'object') {
    return false
  }

  return keys.some((key) => payload[key] !== undefined && payload[key] !== null)
}

const getPayloadBoolean = (payload, keys) => {
  if (!payload || typeof payload !== 'object') {
    return false
  }

  for (const key of keys) {
    const value = payload[key]

    if (value === undefined || value === null || value === '') {
      continue
    }

    if (typeof value === 'string') {
      const normalizedValue = value.trim().toLowerCase()

      if (normalizedValue === 'true') {
        return true
      }

      if (normalizedValue === 'false') {
        return false
      }
    }

    return Boolean(value)
  }

  return false
}

const getPayloadText = (payload, keys) => {
  if (typeof payload === 'string' && payload.trim()) {
    return payload.trim()
  }

  if (!payload || typeof payload !== 'object') {
    return ''
  }

  for (const key of keys) {
    const value = payload[key]

    if (typeof value === 'string' && value.trim()) {
      return value.trim()
    }
  }

  return ''
}

const buildSteamEnrichmentDetail = ({
  detail,
  fallback,
  completed,
  total,
  batchCompleted,
  batchTotal,
  batchesCompleted,
  rateLimited,
  fetchedArtwork,
  fetchedAchievementSchemas,
  fetchedPlayerAchievements,
}) => {
  const details = []

  if (detail) {
    details.push(detail)
  } else if (fallback) {
    details.push(fallback)
  }

  if (completed !== null && total) {
    details.push(`${completed}/${total} candidatos processados.`)
  }

  if (batchCompleted !== null && batchTotal) {
    const batchSuffix = batchesCompleted !== null ? `; ${batchesCompleted} lotes concluidos` : ''
    details.push(`Lote atual: ${batchCompleted}/${batchTotal}${batchSuffix}.`)
  } else if (batchesCompleted !== null) {
    details.push(`${batchesCompleted} lotes concluidos.`)
  }

  if (rateLimited) {
    details.push('Aguardando limite de requisicoes da Steam.')
  }

  const fetchedResources = [
    fetchedArtwork ? 'artes' : '',
    fetchedAchievementSchemas ? 'schemas de conquistas' : '',
    fetchedPlayerAchievements ? 'conquistas do jogador' : '',
  ].filter(Boolean)

  if (fetchedResources.length > 0) {
    details.push(`Atualizados: ${fetchedResources.join(', ')}.`)
  }

  return details.join(' ')
}

export const normalizeSteamEnrichmentStatus = (phase, payload) => {
  const errorPayload = payload?.error && typeof payload.error === 'object' ? payload.error : null
  const partialSummary = payload?.partialSummary ?? payload?.partial_summary ?? null
  const totalCandidates = getPayloadNumber(payload, ['totalCandidates', 'total_candidates'])
  const legacyTotal = getPayloadNumber(payload, ['total', 'count', 'totalGames', 'total_games'])
  const total = totalCandidates ?? legacyTotal
  const completed = getPayloadNumber(payload, ['completed', 'processed', 'current', 'done'])
  const batchCompleted = getPayloadNumber(payload, ['batchCompleted', 'batch_completed'])
  const batchTotal = getPayloadNumber(payload, ['batchTotal', 'batch_total'])
  const batchesCompleted = getPayloadNumber(payload, ['batchesCompleted', 'batches_completed'])
  const rawPercent = getPayloadNumber(payload, ['percent', 'percentage', 'progress'])
  const percent = rawPercent === null
    ? null
    : Math.max(0, Math.min(100, Math.round(rawPercent <= 1 ? rawPercent * 100 : rawPercent)))
  const detail = getPayloadText(errorPayload, ['message', 'detail', 'details', 'detailsSanitized', 'details_sanitized']) ||
    getPayloadText(payload, ['message', 'detail', 'stage', 'title'])
  const recoverable = errorPayload ? getPayloadBoolean(errorPayload, ['recoverable']) : null
  const errorCode = getPayloadText(errorPayload, ['code'])
  const errorPhase = getPayloadText(errorPayload, ['phase'])
  const rateLimited = hasPayloadValue(payload, ['rateLimited', 'rate_limited'])
    ? getPayloadBoolean(payload, ['rateLimited', 'rate_limited'])
    : hasPayloadValue(partialSummary, ['rateLimited', 'rate_limited'])
      ? getPayloadBoolean(partialSummary, ['rateLimited', 'rate_limited'])
      : errorCode === 'steam_web_api_rate_limited'
  const failedSummaryDetail = buildFailedSummaryDetail({ errorCode, errorPhase, recoverable })
  const failedDetail = [detail, failedSummaryDetail].filter(Boolean).join(' ')
  const completedPayload = partialSummary && phase === 'failed' ? partialSummary : payload
  const failedCompleted = getPayloadNumber(completedPayload, ['completed', 'processed', 'current', 'done'])
  const failedTotal = getPayloadNumber(completedPayload, ['totalCandidates', 'total_candidates', 'total'])
  const failedProgress = failedCompleted !== null && failedTotal
    ? `${failedCompleted}/${failedTotal} candidatos processados.`
    : ''
  const fetchedArtwork = getPayloadBoolean(payload, ['fetchedArtwork', 'fetched_artwork'])
  const fetchedAchievementSchemas = getPayloadBoolean(payload, [
    'fetchedAchievementSchemas',
    'fetched_achievement_schemas',
  ])
  const fetchedPlayerAchievements = getPayloadBoolean(payload, [
    'fetchedPlayerAchievements',
    'fetched_player_achievements',
  ])
  const detailText = buildSteamEnrichmentDetail({
    detail,
    fallback: phase === 'completed'
      ? 'Metadados Steam atualizados em background.'
      : STEAM_ENRICHMENT_DEFAULT_DETAIL,
    completed,
    total,
    batchCompleted,
    batchTotal,
    batchesCompleted,
    rateLimited: phase === 'completed' ? false : rateLimited,
    fetchedArtwork,
    fetchedAchievementSchemas,
    fetchedPlayerAchievements,
  })

  if (phase === 'completed') {
    return {
      phase,
      rateLimited: false,
      text: 'Steam atualizada',
      detail: detailText,
    }
  }

  if (phase === 'failed') {
    return {
      phase,
      code: errorCode,
      errorPhase,
      recoverable,
      rateLimited,
      text: 'Falha ao enriquecer Steam',
      detail: [failedDetail || 'O enriquecimento Steam em background falhou.', failedProgress].filter(Boolean).join(' '),
    }
  }

  if (completed !== null && total) {
    return {
      phase,
      rateLimited,
      text: `Steam ${completed}/${total}`,
      detail: detailText,
    }
  }

  if (percent !== null) {
    return {
      phase,
      rateLimited,
      text: `Steam ${percent}%`,
      detail: detailText,
    }
  }

  if (batchCompleted !== null && batchTotal) {
    return {
      phase,
      rateLimited,
      text: `Steam lote ${batchCompleted}/${batchTotal}`,
      detail: detailText,
    }
  }

  return {
    phase,
    rateLimited,
    text: 'Enriquecendo Steam',
    detail: detailText,
  }
}

function buildFailedSummaryDetail({ errorCode, errorPhase, recoverable }) {
  const details = []

  if (errorCode) {
    details.push(`Codigo: ${errorCode}.`)
  }

  if (errorPhase) {
    details.push(`Fase: ${errorPhase}.`)
  }

  if (recoverable !== null) {
    details.push(`Recuperavel: ${recoverable ? 'sim' : 'nao'}.`)
  }

  return details.join(' ')
}
