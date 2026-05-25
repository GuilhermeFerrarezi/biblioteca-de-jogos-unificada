const sanitizeFeedbackText = (value, maxLength = 180) => {
  if (value === null || value === undefined) {
    return ''
  }

  const normalized = String(value).replace(/\s+/g, ' ').trim()

  if (!normalized) {
    return ''
  }

  if (normalized.length <= maxLength) {
    return normalized
  }

  return `${normalized.slice(0, Math.max(0, maxLength - 1)).trimEnd()}...`
}

const normalizeFeedbackDetail = (detail, index) => {
  if (!detail) {
    return null
  }

  if (typeof detail === 'string' || typeof detail === 'number' || typeof detail === 'boolean') {
    const value = sanitizeFeedbackText(detail, 180)
    return value ? { label: index === 0 ? 'Detalhe tecnico' : `Detalhe ${index + 1}`, value } : null
  }

  if (typeof detail !== 'object') {
    return null
  }

  const label = sanitizeFeedbackText(detail.label ?? detail.name ?? detail.title ?? '', 48)
  const value = sanitizeFeedbackText(detail.value ?? detail.text ?? detail.message ?? detail.summary ?? '', 180)

  if (!value) {
    return null
  }

  return {
    label: label || (index === 0 ? 'Detalhe tecnico' : `Detalhe ${index + 1}`),
    value,
  }
}

const tryParseStructuredErrorPayload = (value) => {
  if (typeof value !== 'string') {
    return null
  }

  const trimmedValue = value.trim()
  if (!trimmedValue || (!trimmedValue.startsWith('{') && !trimmedValue.startsWith('['))) {
    return null
  }

  try {
    const parsedValue = JSON.parse(trimmedValue)
    return parsedValue && typeof parsedValue === 'object' ? parsedValue : null
  } catch {
    return null
  }
}

export const normalizeProviderErrorFeedback = (error, fallbackMessage, contextLabel = '') => {
  const fallbackSummary = sanitizeFeedbackText(fallbackMessage) || 'Nao foi possivel concluir a operacao.'

  if (!error) {
    return { message: fallbackSummary, details: [] }
  }

  const structuredSource =
    typeof error === 'object' && error !== null
      ? (error.data && typeof error.data === 'object' ? error.data : error)
      : null
  const parsedStringSource =
    tryParseStructuredErrorPayload(
      (typeof error === 'object' && error !== null && typeof error.data === 'string' && error.data) ||
        (structuredSource && typeof structuredSource.message === 'string' && structuredSource.message) ||
        (typeof error === 'string' ? error : error?.message),
    )
  const normalizedStructuredSource = parsedStringSource ?? structuredSource
  const structuredMessage = sanitizeFeedbackText(normalizedStructuredSource?.message ?? '', 180)

  const existingDetails = Array.isArray(normalizedStructuredSource?.details)
    ? normalizedStructuredSource.details
    : Array.isArray(error?.details)
      ? error.details
      : []

  if (existingDetails.length > 0) {
    const normalizedDetails = existingDetails
      .map((detail, index) => normalizeFeedbackDetail(detail, index))
      .filter(Boolean)
      .slice(0, 3)

    const details = []
    const pushDetail = (label, value, maxLength = 180) => {
      const normalizedValue = sanitizeFeedbackText(value, maxLength)

      if (normalizedValue && details.length < 3) {
        details.push({ label, value: normalizedValue })
      }
    }

    if (contextLabel) {
      pushDetail('Contexto', contextLabel, 96)
    }

    pushDetail('Codigo', normalizedStructuredSource?.code ?? error.code ?? '', 64)
    pushDetail('Etapa', normalizedStructuredSource?.phase ?? error.phase ?? '', 64)

    normalizedDetails.forEach((detail) => {
      if (details.length < 3) {
        details.push(detail)
      }
    })

    return {
      message: structuredMessage || fallbackSummary,
      details: details.slice(0, 3),
    }
  }

  const details = []
  const pushDetail = (label, value, maxLength = 180) => {
    const normalizedValue = sanitizeFeedbackText(value, maxLength)

    if (normalizedValue) {
      details.push({ label, value: normalizedValue })
    }
  }

  if (contextLabel) {
    pushDetail('Contexto', contextLabel, 96)
  }

  const technicalMessage = normalizedStructuredSource
    ? ''
    : typeof error === 'string'
      ? sanitizeFeedbackText(error, 180)
      : sanitizeFeedbackText(error.message ?? '', 180)

  if (technicalMessage && technicalMessage !== fallbackSummary) {
    pushDetail('Mensagem tecnica', technicalMessage)
  }

  pushDetail('Codigo', normalizedStructuredSource?.code ?? error.code ?? '', 64)
  pushDetail('Etapa', normalizedStructuredSource?.phase ?? error.phase ?? '', 64)

  const sanitizedDetails =
    normalizedStructuredSource?.detailsSanitized ??
    normalizedStructuredSource?.diagnostic ??
    normalizedStructuredSource?.summary ??
    normalizedStructuredSource?.detail ??
    (typeof error?.data === 'string' ? error.data : '') ??
    error.detailsSanitized ??
    error.diagnostic ??
    error.summary ??
    error.detail ??
    ''

  pushDetail('Resumo tecnico', sanitizedDetails, 180)

  if (details.length === 0 && technicalMessage) {
    pushDetail('Mensagem tecnica', technicalMessage)
  }

  return {
    message: structuredMessage || fallbackSummary,
    details: details.slice(0, 3),
  }
}
