import assert from 'node:assert/strict'
import test from 'node:test'
import { normalizeSteamEnrichmentStatus } from './steamEnrichmentStatus.js'

test('normalizeSteamEnrichmentStatus shows cumulative Steam progress', () => {
  const status = normalizeSteamEnrichmentStatus('running', {
    completed: 150,
    total_candidates: 420,
    batch_completed: 50,
    batch_total: 50,
    batches_completed: 3,
    fetched_artwork: true,
  })

  assert.equal(status.text, 'Steam 150/420')
  assert.equal(status.phase, 'running')
  assert.equal(status.rateLimited, false)
  assert.equal(status.detail.includes('150/420 candidatos processados.'), true)
  assert.equal(status.detail.includes('3 lotes concluidos'), true)
  assert.equal(status.detail.includes('artes'), true)
})

test('normalizeSteamEnrichmentStatus exposes Steam rate limit state', () => {
  const status = normalizeSteamEnrichmentStatus('running', {
    completed: 50,
    total: 100,
    rate_limited: true,
  })

  assert.equal(status.text, 'Steam 50/100')
  assert.equal(status.rateLimited, true)
  assert.equal(status.detail.includes('Aguardando limite de requisicoes da Steam.'), true)
})
