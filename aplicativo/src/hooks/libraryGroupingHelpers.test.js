import assert from 'node:assert/strict'
import test from 'node:test'
import { groupLibraryEntries } from './libraryGroupingHelpers.js'

const steamEntry = {
  id: 'steam-entry',
  primaryPlatformId: 'steam',
  installStatus: 'installed',
  lastPlayedLabel: 'Ontem',
  game: {
    title: 'Hollow Knight',
    sortTitle: 'Hollow Knight',
    platforms: ['steam'],
    sources: [{ platformId: 'steam', externalId: 'steam-10' }],
    installLocations: ['C:/Games/Steam/HollowKnight'],
    launchActions: [
      {
        isPrimary: true,
        kind: 'uri',
        target: 'steam://rungameid/10',
        label: 'Jogar na Steam',
      },
    ],
    playtime: { totalMinutes: 180 },
    artwork: { accentColor: '#123456' },
    genres: ['Aventura'],
    tags: ['metroidvania'],
    userOverrides: { steam: true },
  },
}

const xboxEntry = {
  id: 'xbox-entry',
  primaryPlatformId: 'xbox',
  installStatus: 'not_installed',
  lastPlayedLabel: 'Nunca',
  game: {
    title: 'Hollow Knight',
    sortTitle: 'Hollow Knight',
    platforms: ['xbox'],
    sources: [{ platformId: 'xbox', externalId: '1234567890' }],
    installLocations: ['C:/Xbox/HollowKnight'],
    launchActions: [],
    playtime: { totalMinutes: 60 },
    artwork: { accentColor: '#654321' },
    genres: ['Aventura'],
    tags: ['metroidvania'],
    userOverrides: { xbox: true },
  },
}

const manualEntry = {
  id: 'manual-entry',
  primaryPlatformId: 'manual',
  installStatus: 'not_installed',
  lastPlayedLabel: 'Nunca',
  game: {
    title: 'Curse of the Dead Gods',
    sortTitle: 'Curse of the Dead Gods',
    platforms: ['manual'],
    sources: [{ platformId: 'manual', externalId: 'manual-1' }],
    installLocations: [],
    launchActions: [],
    playtime: { totalMinutes: 0 },
    artwork: { accentColor: '#777777' },
    genres: ['Roguelike'],
    tags: [],
    userOverrides: {},
  },
}

test('groupLibraryEntries merges Steam and Xbox records with the same title', () => {
  const groupedEntries = groupLibraryEntries([steamEntry, xboxEntry, manualEntry])

  assert.equal(groupedEntries.length, 2)

  const mergedEntry = groupedEntries[0]

  assert.equal(mergedEntry.isGroupedCrossPlatform, true)
  assert.deepEqual(mergedEntry.memberEntryIds, ['steam-entry', 'xbox-entry'])
  assert.equal(mergedEntry.platformSummary, 'Steam + Xbox')
  assert.equal(mergedEntry.installStatus, 'installed')
  assert.equal(mergedEntry.game.playtime.totalMinutes, 240)
  assert.equal(mergedEntry.game.installed, true)
  assert.equal(mergedEntry.game.platforms.length, 2)
  assert.deepEqual(mergedEntry.platformIds, ['steam', 'xbox'])
  assert.equal(mergedEntry.game.sources.length, 2)
})

test('groupLibraryEntries keeps non cross-platform entries separate', () => {
  const groupedEntries = groupLibraryEntries([manualEntry])

  assert.equal(groupedEntries.length, 1)
  assert.equal(groupedEntries[0].id, 'manual-entry')
  assert.equal(groupedEntries[0].isGroupedCrossPlatform, undefined)
})

test('groupLibraryEntries merges Steam and Xbox records even when casing differs', () => {
  const groupedEntries = groupLibraryEntries([
    {
      ...steamEntry,
      game: {
        ...steamEntry.game,
        title: 'A Little to the Left',
        sortTitle: 'A Little to the Left',
      },
    },
    {
      ...xboxEntry,
      game: {
        ...xboxEntry.game,
        title: 'A Little To The Left',
        sortTitle: 'A Little To The Left',
      },
    },
  ])

  assert.equal(groupedEntries.length, 1)
  assert.equal(groupedEntries[0].isGroupedCrossPlatform, true)
  assert.deepEqual(groupedEntries[0].memberEntryIds, ['steam-entry', 'xbox-entry'])
})
