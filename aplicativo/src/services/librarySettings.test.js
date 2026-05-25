import assert from 'node:assert/strict'
import test from 'node:test'
import { normalizeLibrarySettings } from './librarySettings.js'

test('normalizeLibrarySettings preserves a trimmed Microsoft client id', () => {
  const settings = normalizeLibrarySettings({
    preferredStoreId: 'xbox',
    localScanMode: 'selected_only',
    localScanRoots: ['C:/Games'],
    localScanExcludedRoots: ['D:/Temp'],
    microsoftClientId: '  11111111-2222-3333-4444-555555555555  ',
  })

  assert.equal(settings.preferredStoreId, 'xbox')
  assert.equal(settings.localScanMode, 'selected_only')
  assert.equal(settings.microsoftClientId, '11111111-2222-3333-4444-555555555555')
})
