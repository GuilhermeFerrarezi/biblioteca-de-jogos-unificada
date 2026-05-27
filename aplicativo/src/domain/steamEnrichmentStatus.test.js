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

test('normalizeSteamEnrichmentStatus exposes sanitized failed event details', () => {
  const status = normalizeSteamEnrichmentStatus('failed', {
    error: {
      code: 'steam_web_api_rate_limited',
      phase: 'achievement_schema',
      recoverable: true,
      message: 'A Steam limitou as requisicoes.',
    },
    partialSummary: {
      completed: 25,
      totalCandidates: 100,
      rateLimited: true,
    },
  })

  assert.equal(status.phase, 'failed')
  assert.equal(status.code, 'steam_web_api_rate_limited')
  assert.equal(status.errorPhase, 'achievement_schema')
  assert.equal(status.recoverable, true)
  assert.equal(status.rateLimited, true)
  assert.equal(status.detail.includes('A Steam limitou as requisicoes.'), true)
  assert.equal(status.detail.includes('Codigo: steam_web_api_rate_limited.'), true)
  assert.equal(status.detail.includes('25/100 candidatos processados.'), true)
})

test('normalizeSteamEnrichmentStatus reports completed enrichment as non-limited', () => {
  const status = normalizeSteamEnrichmentStatus('completed', {
    completed: 100,
    totalCandidates: 100,
    fetchedAchievementSchemas: true,
    fetchedPlayerAchievements: true,
    rateLimited: true,
  })

  assert.equal(status.text, 'Steam atualizada')
  assert.equal(status.rateLimited, false)
  assert.equal(status.detail.includes('Aguardando limite'), false)
  assert.equal(status.detail.includes('schemas de conquistas'), true)
  assert.equal(status.detail.includes('conquistas do jogador'), true)
})
