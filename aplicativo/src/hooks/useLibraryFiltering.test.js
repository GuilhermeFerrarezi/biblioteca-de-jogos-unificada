import assert from 'node:assert/strict'
import test from 'node:test'
import { QUICK_FILTER_IDS, SORT_MODE_IDS } from '../constants/libraryConstants.js'
import {
  filterLibraryEntries,
  getVisibleLibraryEntries,
  sortLibraryEntries,
} from './useLibraryFiltering.js'

const makeEntry = ({
  id,
  title,
  minutes = 0,
  isFavorite,
  is_favorite,
  achievementProgress,
  achievements,
  personalRating,
  primaryPlatformId = 'steam',
  installStatus = 'not_installed',
}) => ({
  id,
  isFavorite,
  is_favorite,
  achievementProgress,
  primaryPlatformId,
  installStatus,
  game: {
    title,
    sortTitle: title,
    genres: ['RPG'],
    playtime: { totalMinutes: minutes },
    achievements,
    personalRating,
  },
})

const makeGroupedEntry = ({ id, title, memberEntries }) => ({
  ...memberEntries.find((entry) => entry.installStatus === 'installed'),
  id,
  isGroupedCrossPlatform: true,
  memberEntries,
  memberEntryIds: memberEntries.map((entry) => entry.id),
  platformIds: memberEntries.map((entry) => entry.primaryPlatformId),
  platformSummary: memberEntries.map((entry) => entry.primaryPlatformId).join(' + '),
  installStatus: memberEntries.some((entry) => entry.installStatus === 'installed') ? 'installed' : 'not_installed',
  game: {
    ...memberEntries[0].game,
    title,
    sortTitle: title,
  },
})

const getTitles = (entries) => entries.map((entry) => entry.game.title)

test('filterLibraryEntries shows only favorites and treats missing favorite flags as false', () => {
  const entries = [
    makeEntry({ id: '1', title: 'Zelda', isFavorite: true }),
    makeEntry({ id: '2', title: 'Alpha' }),
    makeEntry({ id: '3', title: 'Hades', is_favorite: true }),
  ]

  const filteredEntries = filterLibraryEntries(entries, '', [QUICK_FILTER_IDS.FAVORITES])

  assert.deepEqual(getTitles(filteredEntries), ['Zelda', 'Hades'])
})

test('sortLibraryEntries orders favorites first and keeps each group alphabetical', () => {
  const entries = [
    makeEntry({ id: '1', title: 'Zelda' }),
    makeEntry({ id: '2', title: 'Hades', isFavorite: true }),
    makeEntry({ id: '3', title: 'Alpha', is_favorite: true }),
    makeEntry({ id: '4', title: 'Bastion' }),
  ]

  const sortedEntries = sortLibraryEntries(entries, SORT_MODE_IDS.FAVORITES_FIRST)

  assert.deepEqual(getTitles(sortedEntries), ['Alpha', 'Hades', 'Bastion', 'Zelda'])
})

test('sortLibraryEntries orders by played hours with alphabetical fallback', () => {
  const entries = [
    makeEntry({ id: '1', title: 'Short', minutes: 30 }),
    makeEntry({ id: '2', title: 'Long', minutes: 600 }),
    makeEntry({ id: '3', title: 'Also Long', minutes: 600 }),
  ]

  const descendingEntries = sortLibraryEntries(entries, SORT_MODE_IDS.PLAYTIME_DESC)
  const ascendingEntries = sortLibraryEntries(entries, SORT_MODE_IDS.PLAYTIME_ASC)

  assert.deepEqual(getTitles(descendingEntries), ['Also Long', 'Long', 'Short'])
  assert.deepEqual(getTitles(ascendingEntries), ['Short', 'Also Long', 'Long'])
})

test('getVisibleLibraryEntries sorts by achievement progress when available and falls back safely', () => {
  const entries = [
    makeEntry({ id: '1', title: 'No Data' }),
    makeEntry({ id: '2', title: 'Half Done', achievementProgress: { unlocked: 5, total: 10 } }),
    makeEntry({ id: '3', title: 'Complete', achievements: { percent: 100 } }),
  ]

  const sortedEntries = getVisibleLibraryEntries(entries, '', [], SORT_MODE_IDS.ACHIEVEMENTS_DESC)

  assert.deepEqual(getTitles(sortedEntries), ['Complete', 'Half Done', 'No Data'])
})

test('filterLibraryEntries recognizes the Epic platform quick filter', () => {
  const entries = [
    makeEntry({ id: '1', title: 'Steam Game', primaryPlatformId: 'steam' }),
    makeEntry({ id: '2', title: 'Epic Game', primaryPlatformId: 'epic' }),
  ]

  const filteredEntries = filterLibraryEntries(entries, '', [QUICK_FILTER_IDS.EPIC])

  assert.deepEqual(getTitles(filteredEntries), ['Epic Game'])
})

test('filterLibraryEntries matches combined provider and status filters against the same grouped member', () => {
  const groupedEntry = makeGroupedEntry({
    id: 'group-dual-provider',
    title: 'Dual Provider Game',
    memberEntries: [
      makeEntry({
        id: 'steam-member',
        title: 'Dual Provider Game',
        primaryPlatformId: 'steam',
        installStatus: 'not_installed',
      }),
      makeEntry({
        id: 'xbox-member',
        title: 'Dual Provider Game',
        primaryPlatformId: 'xbox',
        installStatus: 'installed',
      }),
    ],
  })
  const epicEntry = makeEntry({
    id: 'epic-installed',
    title: 'Epic Installed Game',
    primaryPlatformId: 'epic',
    installStatus: 'installed',
  })

  assert.deepEqual(
    getTitles(filterLibraryEntries([groupedEntry, epicEntry], '', [QUICK_FILTER_IDS.STEAM, QUICK_FILTER_IDS.INSTALLED])),
    [],
  )
  assert.deepEqual(
    getTitles(filterLibraryEntries([groupedEntry, epicEntry], '', [QUICK_FILTER_IDS.XBOX, QUICK_FILTER_IDS.INSTALLED])),
    ['Dual Provider Game'],
  )
  assert.deepEqual(
    getTitles(filterLibraryEntries([groupedEntry, epicEntry], '', [QUICK_FILTER_IDS.STEAM, QUICK_FILTER_IDS.NOT_INSTALLED])),
    ['Dual Provider Game'],
  )
  assert.deepEqual(
    getTitles(filterLibraryEntries([groupedEntry, epicEntry], '', [QUICK_FILTER_IDS.EPIC, QUICK_FILTER_IDS.INSTALLED])),
    ['Epic Installed Game'],
  )
})

test('sortLibraryEntries keeps games without achievement data at the end in both achievement modes', () => {
  const entries = [
    makeEntry({ id: '1', title: 'No Data' }),
    makeEntry({ id: '2', title: 'Low', achievements: { unlocked: 1, total: 10 } }),
    makeEntry({ id: '3', title: 'High', achievements: { unlocked: 8, total: 10 } }),
  ]

  const descendingEntries = sortLibraryEntries(entries, SORT_MODE_IDS.ACHIEVEMENTS_DESC)
  const ascendingEntries = sortLibraryEntries(entries, SORT_MODE_IDS.ACHIEVEMENTS_ASC)

  assert.deepEqual(getTitles(descendingEntries), ['High', 'Low', 'No Data'])
  assert.deepEqual(getTitles(ascendingEntries), ['Low', 'High', 'No Data'])
})

test('sortLibraryEntries orders by personal rating and keeps unrated games at the end', () => {
  const entries = [
    makeEntry({ id: '1', title: 'Unrated Alpha' }),
    makeEntry({ id: '2', title: 'Middle', personalRating: 3 }),
    makeEntry({ id: '3', title: 'Perfect', personalRating: 5 }),
    makeEntry({ id: '4', title: 'Nearly Perfect', personalRating: 4.5 }),
    makeEntry({ id: '5', title: 'Unrated Beta' }),
  ]

  const descendingEntries = sortLibraryEntries(entries, SORT_MODE_IDS.PERSONAL_RATING_DESC)
  const ascendingEntries = sortLibraryEntries(entries, SORT_MODE_IDS.PERSONAL_RATING_ASC)

  assert.deepEqual(getTitles(descendingEntries), ['Perfect', 'Nearly Perfect', 'Middle', 'Unrated Alpha', 'Unrated Beta'])
  assert.deepEqual(getTitles(ascendingEntries), ['Middle', 'Nearly Perfect', 'Perfect', 'Unrated Alpha', 'Unrated Beta'])
})

test('sortLibraryEntries falls back to title when personal ratings tie', () => {
  const entries = [
    makeEntry({ id: '1', title: 'Zelda', personalRating: 4.5 }),
    makeEntry({ id: '2', title: 'Alpha', personalRating: 4.5 }),
    makeEntry({ id: '3', title: 'Bastion', personalRating: 3 }),
  ]

  const sortedEntries = sortLibraryEntries(entries, SORT_MODE_IDS.PERSONAL_RATING_DESC)

  assert.deepEqual(getTitles(sortedEntries), ['Alpha', 'Zelda', 'Bastion'])
})

test('filterLibraryEntries shows rated and unrated games by personal rating presence', () => {
  const entries = [
    makeEntry({ id: '1', title: 'Explicit Null', personalRating: null }),
    makeEntry({ id: '2', title: 'Rated Five', personalRating: 5 }),
    makeEntry({ id: '3', title: 'Rated Half', personalRating: 4.5 }),
    makeEntry({ id: '4', title: 'Unrated' }),
  ]

  assert.deepEqual(
    getTitles(filterLibraryEntries(entries, '', [QUICK_FILTER_IDS.RATED])),
    ['Rated Five', 'Rated Half'],
  )
  assert.deepEqual(
    getTitles(filterLibraryEntries(entries, '', [QUICK_FILTER_IDS.UNRATED])),
    ['Explicit Null', 'Unrated'],
  )
})

test('filterLibraryEntries combines rated filter with provider and status filters on grouped entries', () => {
  const groupedEntry = makeGroupedEntry({
    id: 'group-rated-steam',
    title: 'Rated Steam Group',
    memberEntries: [
      makeEntry({
        id: 'steam-member',
        title: 'Rated Steam Group',
        personalRating: 4.5,
        primaryPlatformId: 'steam',
        installStatus: 'installed',
      }),
      makeEntry({
        id: 'xbox-member',
        title: 'Rated Steam Group',
        personalRating: 4.5,
        primaryPlatformId: 'xbox',
        installStatus: 'not_installed',
      }),
    ],
  })
  const unratedSteamEntry = makeEntry({
    id: 'steam-unrated',
    title: 'Unrated Steam',
    primaryPlatformId: 'steam',
    installStatus: 'installed',
  })

  assert.deepEqual(
    getTitles(filterLibraryEntries(
      [groupedEntry, unratedSteamEntry],
      '',
      [QUICK_FILTER_IDS.RATED, QUICK_FILTER_IDS.STEAM, QUICK_FILTER_IDS.INSTALLED],
    )),
    ['Rated Steam Group'],
  )
  assert.deepEqual(
    getTitles(filterLibraryEntries(
      [groupedEntry, unratedSteamEntry],
      '',
      [QUICK_FILTER_IDS.UNRATED, QUICK_FILTER_IDS.STEAM, QUICK_FILTER_IDS.INSTALLED],
    )),
    ['Unrated Steam'],
  )
})
