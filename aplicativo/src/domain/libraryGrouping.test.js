import assert from 'node:assert/strict'
import test from 'node:test'
import { groupLibraryEntries } from './libraryGrouping.js'

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

const epicEntry = {
  id: 'epic-entry',
  primaryPlatformId: 'epic',
  installStatus: 'installed',
  lastPlayedLabel: 'Nunca',
  game: {
    title: 'Hollow Knight',
    sortTitle: 'Hollow Knight',
    platforms: ['epic'],
    sources: [{ platformId: 'epic', externalId: 'ns:item:artifact' }],
    installLocations: ['C:/Epic/HollowKnight'],
    launchActions: [
      {
        isPrimary: true,
        kind: 'uri',
        target: 'com.epicgames.launcher://apps/ns%3Aitem%3Aartifact?action=launch&silent=true',
        label: 'Epic Games',
      },
    ],
    playtime: { totalMinutes: 0 },
    artwork: { accentColor: '#0d9488' },
    genres: ['Aventura'],
    tags: ['metroidvania'],
    userOverrides: { epic: true },
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

test('groupLibraryEntries can merge Epic with another launcher for the same title', () => {
  const groupedEntries = groupLibraryEntries([steamEntry, epicEntry])

  assert.equal(groupedEntries.length, 1)
  assert.equal(groupedEntries[0].isGroupedCrossPlatform, true)
  assert.deepEqual(groupedEntries[0].memberEntryIds, ['steam-entry', 'epic-entry'])
  assert.equal(groupedEntries[0].platformSummary, 'Steam + Epic Games')
  assert.deepEqual(groupedEntries[0].platformIds, ['steam', 'epic'])
})

test('groupLibraryEntries marks a merged entry as favorite when any member is favorite', () => {
  const groupedEntries = groupLibraryEntries([
    { ...steamEntry, isFavorite: false },
    { ...xboxEntry, isFavorite: true },
  ])

  assert.equal(groupedEntries.length, 1)
  assert.equal(groupedEntries[0].isFavorite, true)
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

test('groupLibraryEntries prefers artwork with image URLs for merged entries', () => {
  const steamWithArtwork = {
    ...steamEntry,
    installStatus: 'not_installed',
    game: {
      ...steamEntry.game,
      artwork: {
        accentColor: '#123456',
        coverUrl: 'https://cdn.akamai.steamstatic.com/steam/apps/10/library_600x900.jpg',
        heroUrl: 'https://cdn.akamai.steamstatic.com/steam/apps/10/header.jpg',
        source: 'steam',
      },
    },
  }
  const installedXboxWithoutArtwork = {
    ...xboxEntry,
    installStatus: 'installed',
    game: {
      ...xboxEntry.game,
      artwork: { accentColor: '#654321' },
    },
  }

  const groupedEntries = groupLibraryEntries([installedXboxWithoutArtwork, steamWithArtwork])

  assert.equal(groupedEntries.length, 1)
  assert.equal(groupedEntries[0].game.artwork.source, 'steam')
  assert.equal(
    groupedEntries[0].game.artwork.coverUrl,
    'https://cdn.akamai.steamstatic.com/steam/apps/10/library_600x900.jpg',
  )
})
