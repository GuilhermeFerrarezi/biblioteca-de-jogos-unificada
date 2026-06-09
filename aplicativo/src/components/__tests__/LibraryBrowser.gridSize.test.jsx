import { render, screen } from '@testing-library/react'
import assert from 'node:assert/strict'
import { describe, test } from 'vitest'
import LibraryBrowser from '../LibraryBrowser.jsx'
import { INSTALL_STATUS, LAUNCH_ACTION_KIND, SORT_MODE_IDS } from '../../constants/libraryConstants.js'

const noop = () => {}

function buildEntry() {
  return {
    id: 'entry-grid-size',
    primaryPlatformId: 'steam',
    platformSummary: 'Steam',
    installStatus: INSTALL_STATUS.INSTALLED,
    isArchived: false,
    isFavorite: true,
    lastPlayedLabel: 'Hoje',
    game: {
      title: 'Grid Size Fixture',
      personalRating: 4,
      playtime: { totalMinutes: 120 },
      artwork: { accentColor: '#0f766e' },
      genres: ['Teste'],
      achievements: {
        unlocked: 3,
        total: 10,
        percentage: 30,
      },
      launchActions: [
        {
          id: 'launch-grid-size',
          platformId: 'steam',
          kind: LAUNCH_ACTION_KIND.URI,
          label: 'Steam',
          target: 'steam://rungameid/1',
          isPrimary: true,
        },
      ],
    },
  }
}

function renderBrowser({ gridSize = 'default', viewMode = 'grid' } = {}) {
  const entry = buildEntry()

  return render(
    <LibraryBrowser
      entriesCount={1}
      filteredEntries={[entry]}
      gridSize={gridSize}
      quickFilters={[]}
      searchTerm=""
      selectedEntry={entry}
      showLibraryLoading={false}
      sortMode={SORT_MODE_IDS.ALPHA_ASC}
      viewMode={viewMode}
      onFilterChange={noop}
      onSearchChange={noop}
      onSelectEntry={noop}
      onSortModeChange={noop}
      onViewModeChange={noop}
    />,
  )
}

describe('LibraryBrowser grid size', () => {
  test('applies the selected grid size only to the cover grid', () => {
    const { container, rerender } = renderBrowser({ gridSize: 'compact' })

    assert.equal(container.querySelector('.game-cover-grid')?.dataset.gridSize, 'compact')

    rerender(
      <LibraryBrowser
        entriesCount={1}
        filteredEntries={[buildEntry()]}
        gridSize="large"
        quickFilters={[]}
        searchTerm=""
        selectedEntry={buildEntry()}
        showLibraryLoading={false}
        sortMode={SORT_MODE_IDS.ALPHA_ASC}
        viewMode="grid"
        onFilterChange={noop}
        onSearchChange={noop}
        onSelectEntry={noop}
        onSortModeChange={noop}
        onViewModeChange={noop}
      />,
    )

    assert.equal(container.querySelector('.game-cover-grid')?.dataset.gridSize, 'large')
  })

  test('keeps list rendering independent from grid size', () => {
    const { container } = renderBrowser({ gridSize: 'large', viewMode: 'list' })

    assert.equal(container.querySelector('.game-cover-grid'), null)
    assert.ok(container.querySelector('.game-table'))
    assert.ok(screen.getByText('Grid Size Fixture'))
  })
})
