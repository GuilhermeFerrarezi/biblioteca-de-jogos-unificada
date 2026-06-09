import assert from 'node:assert/strict'
import test from 'node:test'
import { normalizeLibraryGridSize, normalizeLibrarySettings } from './librarySettings.js'

test('normalizeLibrarySettings preserves a trimmed Microsoft client id', () => {
  const settings = normalizeLibrarySettings({
    preferredStoreId: 'xbox',
    gridSize: 'large',
    localScanMode: 'selected_only',
    localScanRoots: ['C:/Games'],
    localScanExcludedRoots: ['D:/Temp'],
    microsoftClientId: '  11111111-2222-3333-4444-555555555555  ',
  })

  assert.equal(settings.preferredStoreId, 'xbox')
  assert.equal(settings.gridSize, 'large')
  assert.equal(settings.localScanMode, 'selected_only')
  assert.equal(settings.microsoftClientId, '11111111-2222-3333-4444-555555555555')
})

test('normalizeLibraryGridSize accepts only supported grid sizes', () => {
  assert.equal(normalizeLibraryGridSize(' compact '), 'compact')
  assert.equal(normalizeLibraryGridSize('default'), 'default')
  assert.equal(normalizeLibraryGridSize('LARGE'), 'large')
  assert.equal(normalizeLibraryGridSize('wide'), 'default')
  assert.equal(normalizeLibraryGridSize(null), 'default')
})

test('normalizeLibrarySettings falls back to default grid size for invalid values', () => {
  assert.equal(normalizeLibrarySettings({ gridSize: 'tiny' }).gridSize, 'default')
  assert.equal(normalizeLibrarySettings({ gridSize: 42 }).gridSize, 'default')
  assert.equal(normalizeLibrarySettings(null).gridSize, 'default')
})
