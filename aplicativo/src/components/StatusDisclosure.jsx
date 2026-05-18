import { ChevronDown } from 'lucide-react'
import { useId, useMemo, useState } from 'react'

function StatusDisclosure({ className = 'launch-feedback', feedback, message }) {
  const detailsId = useId()
  const [isExpanded, setIsExpanded] = useState(false)

  const presentation = useMemo(() => buildStatusPresentation(feedback, message), [feedback, message])

  if (!presentation) {
    return null
  }

  const { summary, details } = presentation

  return (
    <div className={className} role="status" aria-live="polite">
      <div className="status-disclosure-summary-row">
        <span className="status-disclosure-summary">{summary}</span>
        {details.length > 0 ? (
          <button
            className="status-disclosure-toggle"
            type="button"
            aria-expanded={isExpanded}
            aria-controls={detailsId}
            onClick={() => setIsExpanded((currentValue) => !currentValue)}
          >
            {isExpanded ? 'Ocultar detalhes' : 'Ver detalhes'}
            <ChevronDown size={16} aria-hidden="true" className={isExpanded ? 'status-disclosure-icon open' : 'status-disclosure-icon'} />
          </button>
        ) : null}
      </div>

      {isExpanded && details.length > 0 ? (
        <dl id={detailsId} className="status-disclosure-details">
          {details.map((item) => (
            <div key={item.label}>
              <dt>{item.label}</dt>
              <dd>{item.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
    </div>
  )
}

function buildStatusPresentation(feedback, message) {
  const text = String(message ?? '').trim()

  if (feedback && typeof feedback === 'object') {
    const structuredMessage = String(feedback.message ?? text).trim()
    const structuredDetails = Array.isArray(feedback.details) ? feedback.details : []

    if (!structuredMessage) {
      return null
    }

    return {
      summary: structuredMessage,
      details: structuredDetails,
    }
  }

  if (!text) {
    return null
  }

  const splitIndex = text.indexOf(':')
  const prefix = splitIndex >= 0 ? text.slice(0, splitIndex).trim() : text
  const reason = splitIndex >= 0 ? text.slice(splitIndex + 1).trim() : ''

  const details = buildStatusDetails(prefix, reason)

  return {
    summary: buildStatusSummary(prefix, reason),
    details,
  }
}

function buildStatusSummary(prefix, reason) {
  if (!reason) {
    return prefix
  }

  if (
    prefix === 'Nao foi possivel sincronizar a conta Steam' ||
    prefix === 'Nao foi possivel sincronizar a Steam' ||
    prefix === 'Nao foi possivel sincronizar jogos locais' ||
    prefix === 'Nao foi possivel carregar a biblioteca local'
  ) {
    return `${prefix}.`
  }

  return `${prefix}: ${reason}`
}

function buildStatusDetails(prefix, reason) {
  if (!reason) {
    return []
  }

  if (prefix === 'Nao foi possivel sincronizar a conta Steam') {
    return buildSteamSyncDetails(reason, 'sync_account')
  }

  if (prefix === 'Nao foi possivel sincronizar a Steam') {
    return buildSteamSyncDetails(reason, 'sync_local')
  }

  if (prefix === 'Nao foi possivel sincronizar jogos locais') {
    return [
      { label: 'Codigo', value: 'local_sync_failed' },
      { label: 'Fase', value: 'scan' },
      { label: 'Recuperavel', value: 'sim' },
      { label: 'Detalhe tecnico', value: reason },
    ]
  }

  if (prefix === 'Nao foi possivel carregar a biblioteca local') {
    return [
      { label: 'Codigo', value: 'library_bootstrap_failed' },
      { label: 'Fase', value: 'bootstrap' },
      { label: 'Recuperavel', value: 'sim' },
      { label: 'Detalhe tecnico', value: reason },
    ]
  }

  return []
}

function buildSteamSyncDetails(reason, phase) {
  const code = inferSteamErrorCode(reason)

  return [
    { label: 'Codigo', value: code },
    { label: 'Provider', value: 'steam' },
    { label: 'Fase', value: phase },
    { label: 'Recuperavel', value: 'sim' },
    { label: 'Detalhe tecnico', value: reason },
  ]
}

function inferSteamErrorCode(reason) {
  const normalizedReason = reason.toLowerCase()

  if (normalizedReason.includes('steam web api key') && normalizedReason.includes('configure')) {
    return 'steam_web_api_key_missing'
  }

  if (normalizedReason.includes('steamid64')) {
    return 'steam_account_id_missing'
  }

  if (normalizedReason.includes('autenticar')) {
    return 'steam_web_api_auth_required'
  }

  if (normalizedReason.includes('conectar')) {
    return 'steam_web_api_network_unavailable'
  }

  if (normalizedReason.includes('limitou as requisicoes')) {
    return 'steam_web_api_rate_limited'
  }

  if (normalizedReason.includes('nao respondeu')) {
    return 'steam_web_api_platform_unavailable'
  }

  if (normalizedReason.includes('ler a resposta')) {
    return 'steam_web_api_parse_failed'
  }

  return 'steam_provider_error'
}

export default StatusDisclosure
