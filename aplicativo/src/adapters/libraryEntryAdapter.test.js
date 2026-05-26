import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildManualLibraryEntry,
  filterAchievementItems,
  getAchievementDisplayState,
  sortAchievementItems,
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

test('sortAchievementItems orders achieved, visible locked and secret locked by visible text', () => {
  const achievements = [
    { apiName: 'LOCKED_SECRET_B', name: 'B hidden', description: 'Secret B', hidden: true, achieved: false },
    { apiName: 'LOCKED_VISIBLE_B', name: 'Beta', description: 'Visible B', hidden: false, achieved: false },
    { apiName: 'ACHIEVED_B', name: 'Bravo', description: 'Done B', hidden: false, achieved: true },
    { apiName: 'LOCKED_VISIBLE_A', name: 'Alpha', description: 'Visible A', hidden: false, achieved: false },
    { apiName: 'ACHIEVED_A', name: 'Alpha done', description: 'Done A', hidden: true, achieved: true },
    { apiName: 'LOCKED_SECRET_A', name: 'A hidden', description: 'Secret A', hidden: true, achieved: false },
  ]

  const sortedApiNames = sortAchievementItems(achievements).map((achievement) => achievement.apiName)

  assert.deepEqual(sortedApiNames, [
    'ACHIEVED_A',
    'ACHIEVED_B',
    'LOCKED_VISIBLE_A',
    'LOCKED_VISIBLE_B',
    'LOCKED_SECRET_B',
    'LOCKED_SECRET_A',
  ])
})

test('filterAchievementItems searches visible text without leaking unrevealed secret achievements', () => {
  const secret = { apiName: 'SECRET_ONE', name: 'Dragon Room', description: 'Find the hidden dragon.', hidden: true, achieved: false }
  const visible = { apiName: 'VISIBLE_ONE', name: 'Explorer', description: 'Find a map.', hidden: false, achieved: false }

  assert.deepEqual(filterAchievementItems([secret, visible], 'dragon'), [])
  assert.deepEqual(filterAchievementItems([secret, visible], 'secreta'), [secret])

  const revealed = new Set(['SECRET_ONE'])

  assert.deepEqual(filterAchievementItems([secret, visible], 'dragon', revealed), [secret])
})

test('filterAchievementItems keeps large achievement lists complete when there is no query', () => {
  const achievements = Array.from({ length: 637 }, (_, index) => ({
    apiName: `COOKIE_${index}`,
    name: `Cookie ${index}`,
    description: `Achievement ${index}`,
    hidden: false,
    achieved: index % 2 === 0,
  }))

  assert.equal(filterAchievementItems(achievements, '').length, 637)
})

test('getAchievementDisplayState marks unlocked hidden achievements without masking them', () => {
  const display = getAchievementDisplayState({
    apiName: 'SECRET_DONE',
    name: 'Hidden path',
    description: 'Unlock a secret path.',
    hidden: true,
    achieved: true,
  })

  assert.equal(display.shouldMask, false)
  assert.equal(display.isHidden, true)
  assert.equal(display.isAchieved, true)
  assert.equal(display.name, 'Hidden path')
})
