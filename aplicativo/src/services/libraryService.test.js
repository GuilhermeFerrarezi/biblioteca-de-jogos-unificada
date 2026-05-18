import assert from 'node:assert/strict'
import test from 'node:test'
import { normalizeProviderErrorFeedback } from './libraryService.js'

test('normalizeProviderErrorFeedback parses JSON error payloads returned as strings', () => {
  const payload = {
    code: 'xbox_discovery_unavailable',
    message: 'Nao foi possivel consultar o inventario local do Xbox no Windows.',
    recoverable: true,
    providerId: 'xbox',
    phase: 'discovery',
    detailsSanitized: 'powershell.exe returned exit code 9009',
  }

  const feedback = normalizeProviderErrorFeedback(JSON.stringify(payload), 'Falha no Xbox.', 'Sincronizacao Xbox local')

  assert.equal(feedback.message, 'Falha no Xbox.')
  assert.equal(feedback.details.length >= 3, true)
  assert.equal(feedback.details[0]?.label, 'Contexto')
  assert.equal(
    feedback.details.some((detail) => detail.label === 'Codigo' && detail.value === payload.code),
    true,
  )
  assert.equal(
    feedback.details.some((detail) => detail.label === 'Etapa' && detail.value === payload.phase),
    true,
  )
})

test('normalizeProviderErrorFeedback parses JSON payloads returned inside error.data strings', () => {
  const payload = {
    code: 'xbox_discovery_unavailable',
    message: 'Nao foi possivel consultar o inventario local do Xbox no Windows.',
    recoverable: true,
    providerId: 'xbox',
    phase: 'discovery',
    detailsSanitized: 'pwsh.exe returned exit code 9009',
  }

  const feedback = normalizeProviderErrorFeedback(
    { data: JSON.stringify(payload) },
    'Falha no Xbox.',
    'Sincronizacao Xbox local',
  )

  assert.equal(feedback.details.some((detail) => detail.label === 'Codigo' && detail.value === payload.code), true)
  assert.equal(feedback.details.some((detail) => detail.label === 'Etapa' && detail.value === payload.phase), true)
})
