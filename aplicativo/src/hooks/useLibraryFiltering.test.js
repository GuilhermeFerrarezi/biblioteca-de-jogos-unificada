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
