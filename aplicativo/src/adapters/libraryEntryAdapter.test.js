import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildManualLibraryEntry,
  validateManualGameInput,
} from './libraryEntryAdapter.js'
import { INSTALL_STATUS, LAUNCH_ACTION_KIND } from '../constants/libraryConstants.js'

test('validateManualGameInput requires title and valid install status', () => {
  const result = validateManualGameInput({
    title: '',
    installStatus: 'invalid',
    launchTarget: '',
  })

  assert.equal(result.isValid, false)
  assert.equal(result.errors.title, 'Informe o titulo do jogo.')
  assert.equal(result.errors.installStatus, 'Escolha um status valido.')
})

test('validateManualGameInput accepts steam uri and local executable targets', () => {
  const uriResult = validateManualGameInput({
    title: 'Steam Game',
    installStatus: INSTALL_STATUS.INSTALLED,
    launchTarget: 'steam://rungameid/1030300',
  })

  const executableResult = validateManualGameInput({
    title: 'Local Game',
    installStatus: INSTALL_STATUS.NOT_INSTALLED,
    launchTarget: 'C:\\Games\\Local\\Game.exe',
  })

  assert.equal(uriResult.isValid, true)
  assert.equal(executableResult.isValid, true)
})

test('buildManualLibraryEntry infers uri launch actions and preserves existing ids when editing', () => {
  const existingEntry = {
    id: 'entry-existing',
    lastPlayedLabel: 'Ontem',
    addedAt: '2026-05-18T10:00:00.000Z',
    isArchived: true,
    isFavorite: true,
    game: {
      internalId: 'game-existing',
      platforms: ['manual'],
      sources: [{ platformId: 'manual', externalId: 'manual-existing' }],
      playtime: { totalMinutes: 123 },
      installLocations: ['C:\\Games\\Existing'],
      tags: ['fav'],
      userOverrides: { note: 'keep' },
      launchActions: [
        {
          id: 'launch-existing',
          platformId: 'manual',
          kind: 'manual',
          label: 'Anterior',
          target: '',
          isPrimary: true,
        },
      ],
    },
  }

  const nextEntry = buildManualLibraryEntry(
    {
      title: 'Steam Manual',
      genre: 'RPG',
      installStatus: INSTALL_STATUS.INSTALLED,
      launchTarget: 'steam://rungameid/1030300',
    },
    existingEntry,
  )

  assert.equal(nextEntry.id, 'entry-existing')
  assert.equal(nextEntry.isArchived, true)
  assert.equal(nextEntry.isFavorite, true)
  assert.equal(nextEntry.game.internalId, 'game-existing')
  assert.equal(nextEntry.game.launchActions[0].id, 'launch-existing')
  assert.equal(nextEntry.game.launchActions[0].kind, LAUNCH_ACTION_KIND.URI)
  assert.equal(nextEntry.game.launchActions[0].target, 'steam://rungameid/1030300')
  assert.equal(nextEntry.game.installed, true)
  assert.deepEqual(nextEntry.game.tags, ['fav'])
  assert.deepEqual(nextEntry.game.userOverrides, { note: 'keep' })
})
