import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import assert from 'node:assert/strict'
import { describe, test } from 'vitest'
import GameDetailsPanel from '../GameDetailsPanel.jsx'
import { INSTALL_STATUS, LAUNCH_ACTION_KIND } from '../../constants/libraryConstants.js'

const noop = () => {}

function buildEntryWithPersonalReview() {
  return {
    id: 'entry-personal-review',
    primaryPlatformId: 'manual',
    platformSummary: 'Manual',
    installStatus: INSTALL_STATUS.INSTALLED,
    isArchived: false,
    isFavorite: false,
    lastPlayedLabel: 'Nunca',
    game: {
      title: 'Review Fixture',
      personalRating: 4.5,
      personalReview: 'Review existente sobre o jogo.',
      playtime: { totalMinutes: 45 },
      installLocations: ['C:\\Games\\Review Fixture'],
      artwork: { accentColor: '#2563eb' },
      genres: ['Teste'],
      launchActions: [
        {
          id: 'launch-personal-review',
          platformId: 'manual',
          kind: LAUNCH_ACTION_KIND.MANUAL,
          label: 'Manual',
          target: '',
          isPrimary: true,
        },
      ],
    },
  }
}

function renderPanel({ onSavePersonalReview = noop } = {}) {
  return render(
    <GameDetailsPanel
      launchFeedback={null}
      launchMessage=""
      selectedEntry={buildEntryWithPersonalReview()}
      selectedLaunchPlatformId="manual"
      showLibraryLoading={false}
      steamEnrichmentStatus={null}
      onArchiveEntry={noop}
      onEditEntry={noop}
      onLaunchEntry={noop}
      onLaunchPlatformChange={noop}
      onSavePersonalReview={onSavePersonalReview}
      onToggleFavoriteEntry={noop}
    />,
  )
}

describe('GameDetailsPanel personal review UI', () => {
  test('clears only the star rating and preserves review text when saving', async () => {
    const user = userEvent.setup()
    const saveCalls = []

    renderPanel({
      onSavePersonalReview: async (input) => {
        saveCalls.push(input)
      },
    })

    const reviewField = screen.getByRole('textbox', { name: /resenha pessoal/i })

    assert.equal(reviewField.value, 'Review existente sobre o jogo.')
    assert.ok(screen.getByText('4.5 de 5 estrelas'))

    await user.click(screen.getByRole('button', { name: /limpar nota/i }))

    assert.equal(reviewField.value, 'Review existente sobre o jogo.')
    assert.ok(screen.getByText('Sem nota'))

    await user.click(screen.getByRole('button', { name: /^salvar$/i }))

    assert.deepEqual(saveCalls, [
      {
        rating: null,
        review: 'Review existente sobre o jogo.',
      },
    ])
  })
})
