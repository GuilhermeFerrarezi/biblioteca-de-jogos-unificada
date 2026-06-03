import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import assert from 'node:assert/strict'
import { describe, test } from 'vitest'
import GameDetailsPanel from '../GameDetailsPanel.jsx'
import { INSTALL_STATUS, LAUNCH_ACTION_KIND } from '../../constants/libraryConstants.js'

const noop = () => {}

function buildSteamEntry({ achievements = buildAchievements(), fetchedAt = '2026-05-27T11:30:00.000Z' } = {}) {
  const unlocked = achievements.filter((achievement) => achievement.achieved).length

  return {
    id: 'entry-steam-test',
    primaryPlatformId: 'steam',
    platformSummary: 'Steam',
    installStatus: INSTALL_STATUS.INSTALLED,
    isArchived: false,
    isFavorite: false,
    lastPlayedLabel: 'Nunca',
    game: {
      title: 'Steam Fixture',
      playtime: { totalMinutes: 180 },
      installLocations: ['C:\\Games\\Steam Fixture'],
      artwork: { accentColor: '#2563eb' },
      genres: ['Teste'],
      launchActions: [
        {
          id: 'launch-steam-test',
          platformId: 'steam',
          kind: LAUNCH_ACTION_KIND.URI,
          label: 'Steam',
          target: 'steam://rungameid/123',
          isPrimary: true,
        },
      ],
      achievements: {
        providerId: 'steam',
        appId: '123',
        unlocked,
        total: achievements.length,
        percentage: (unlocked / achievements.length) * 100,
        fetchedAt,
        items: achievements,
      },
    },
  }
}

function buildSteamNotInstalledEntry() {
  const entry = buildSteamEntry()

  return {
    ...entry,
    installStatus: INSTALL_STATUS.NOT_INSTALLED,
    game: {
      ...entry.game,
      sources: [{ platformId: 'steam', externalId: '123' }],
      installLocations: [],
    },
  }
}

function buildManualEntryWithoutLaunchAction() {
  return {
    id: 'entry-manual-empty',
    primaryPlatformId: 'manual',
    platformSummary: 'Manual',
    installStatus: INSTALL_STATUS.NOT_INSTALLED,
    isArchived: false,
    isFavorite: false,
    lastPlayedLabel: 'Nunca',
    game: {
      title: 'Manual Sem Acao',
      playtime: { totalMinutes: 0 },
      installLocations: [],
      artwork: { accentColor: '#475569' },
      genres: ['Teste'],
      launchActions: [],
    },
  }
}

function buildAchievements() {
  return [
    {
      apiName: 'LOCKED_SECRET_ZEBRA',
      name: 'Dragon Room',
      description: 'Find the hidden dragon.',
      hidden: true,
      achieved: false,
      iconUrl: 'https://example.test/secret-zebra.png',
      lockedIconUrl: 'https://example.test/secret-zebra-locked.png',
    },
    {
      apiName: 'LOCKED_VISIBLE_BETA',
      name: 'Beta Locked',
      description: 'Visible locked beta.',
      hidden: false,
      achieved: false,
    },
    {
      apiName: 'ACHIEVED_VISIBLE_FIRST',
      name: 'First Step',
      description: 'Start the journey.',
      hidden: false,
      achieved: true,
      unlockTime: 1_777_777_777,
    },
    {
      apiName: 'LOCKED_VISIBLE_ALPHA',
      name: 'Alpha Locked',
      description: 'Visible locked alpha.',
      hidden: false,
      achieved: false,
    },
    {
      apiName: 'ACHIEVED_HIDDEN_WINNER',
      name: 'Hidden Winner',
      description: 'Win quietly.',
      hidden: true,
      achieved: true,
      unlockTime: 1_777_777_778,
    },
    {
      apiName: 'LOCKED_SECRET_ALPHA',
      name: 'Ancient Door',
      description: '',
      hidden: true,
      achieved: false,
    },
  ]
}

function renderPanel({
  selectedEntry = buildSteamEntry(),
  steamEnrichmentStatus = null,
  selectedLaunchPlatformId = 'steam',
} = {}) {
  return render(
    <GameDetailsPanel
      launchFeedback={null}
      launchMessage=""
      selectedEntry={selectedEntry}
      selectedLaunchPlatformId={selectedLaunchPlatformId}
      showLibraryLoading={false}
      steamEnrichmentStatus={steamEnrichmentStatus}
      onArchiveEntry={noop}
      onEditEntry={noop}
      onLaunchEntry={noop}
      onLaunchPlatformChange={noop}
      onToggleFavoriteEntry={noop}
    />,
  )
}

async function openAchievementsModal(user) {
  await user.click(screen.getByRole('button', { name: /ver todas as 6 conquistas steam/i }))

  const dialog = await screen.findByRole('dialog', { name: /steam fixture/i })
  const searchInput = within(dialog).getByRole('searchbox', { name: /buscar conquistas/i })

  await waitFor(() => {
    assert.equal(document.activeElement, searchInput)
  })

  return { dialog, searchInput }
}

function getModalItemText() {
  return Array.from(document.querySelectorAll('.achievement-list.modal-list .achievement-item')).map((item) =>
    item.textContent.replace(/\s+/g, ' ').trim(),
  )
}

function getOrderTokens(items) {
  return items.map((text) => {
    if (text.includes('First Step')) {
      return 'First Step'
    }

    if (text.includes('Hidden Winner')) {
      return 'Hidden Winner'
    }

    if (text.includes('Alpha Locked')) {
      return 'Alpha Locked'
    }

    if (text.includes('Beta Locked')) {
      return 'Beta Locked'
    }

    if (text.includes('Dragon Room')) {
      return 'Dragon Room'
    }

    if (text.includes('Ancient Door')) {
      return 'Ancient Door'
    }

    if (text.includes('Conquista secreta')) {
      return 'Conquista secreta'
    }

    return text
  })
}

describe('GameDetailsPanel Steam achievements UI', () => {
  test('uses one contextual primary action and removes the secondary install button', () => {
    const { rerender } = renderPanel()

    assert.ok(screen.getByRole('button', { name: /^jogar$/i }))
    assert.equal(screen.queryByRole('button', { name: /instalar ou localizar arquivos/i }), null)

    rerender(
      <GameDetailsPanel
        launchFeedback={null}
        launchMessage=""
        selectedEntry={buildSteamNotInstalledEntry()}
        selectedLaunchPlatformId="steam"
        showLibraryLoading={false}
        steamEnrichmentStatus={null}
        onArchiveEntry={noop}
        onEditEntry={noop}
        onLaunchEntry={noop}
        onLaunchPlatformChange={noop}
        onToggleFavoriteEntry={noop}
      />,
    )

    assert.ok(screen.getByRole('button', { name: /^instalar$/i }))
    assert.equal(screen.queryByRole('button', { name: /^jogar$/i }), null)
    assert.equal(screen.queryByRole('button', { name: /instalar ou localizar arquivos/i }), null)

    rerender(
      <GameDetailsPanel
        launchFeedback={null}
        launchMessage=""
        selectedEntry={buildManualEntryWithoutLaunchAction()}
        selectedLaunchPlatformId="manual"
        showLibraryLoading={false}
        steamEnrichmentStatus={null}
        onArchiveEntry={noop}
        onEditEntry={noop}
        onLaunchEntry={noop}
        onLaunchPlatformChange={noop}
        onToggleFavoriteEntry={noop}
      />,
    )

    assert.equal(screen.getByRole('button', { name: /^jogar$/i }).disabled, true)
    assert.ok(screen.getByText(/ainda nao tem acao de lancamento configurada/i))
    assert.equal(screen.queryByRole('button', { name: /instalar ou localizar arquivos/i }), null)
  })

  test('renders preview progress, updated cache status and keeps locked hidden achievements masked', () => {
    renderPanel()

    assert.ok(screen.getByText('2/6'))
    assert.ok(screen.getByText(/Atualizado ha/))
    assert.ok(screen.getByText('Conquista secreta'))
    assert.equal(screen.queryByText('Dragon Room'), null)
    assert.equal(screen.queryByText('Find the hidden dragon.'), null)
  })

  test('opens the portal modal, focuses search and closes by Escape or close button', async () => {
    const user = userEvent.setup()

    renderPanel()

    const { dialog } = await openAchievementsModal(user)

    assert.ok(dialog)
    assert.equal(document.body.classList.contains('achievement-modal-open'), true)

    await user.keyboard('{Escape}')

    await waitFor(() => {
      assert.equal(screen.queryByRole('dialog', { name: /steam fixture/i }), null)
    })

    assert.equal(document.body.classList.contains('achievement-modal-open'), false)
    assert.equal(document.activeElement, screen.getByRole('button', { name: /ver todas as 6 conquistas steam/i }))

    await user.click(screen.getByRole('button', { name: /ver todas as 6 conquistas steam/i }))
    await user.click(await screen.findByRole('button', { name: /fechar conquistas/i }))

    await waitFor(() => {
      assert.equal(screen.queryByRole('dialog', { name: /steam fixture/i }), null)
    })
  })

  test('searches safely, reveals hidden achievements by click and keyboard, and keeps specific fallback text', async () => {
    const user = userEvent.setup()

    renderPanel()

    const { dialog, searchInput } = await openAchievementsModal(user)

    await user.type(searchInput, 'dragon')
    assert.ok(within(dialog).getByText('Nenhuma conquista encontrada.'))

    await user.clear(searchInput)
    await user.type(searchInput, 'secreta')
    assert.equal(within(dialog).queryByText('Dragon Room'), null)

    const revealButtons = within(dialog).getAllByRole('button', { name: /revelar conquista secreta/i })
    await user.click(revealButtons[0])

    assert.ok(await within(dialog).findByText('Dragon Room'))
    assert.ok(within(dialog).getByText('Find the hidden dragon.'))
    assert.ok(within(dialog).getAllByText('(secreta)').length >= 2)

    await user.clear(searchInput)
    await user.type(searchInput, 'dragon')
    assert.ok(within(dialog).getByText('Dragon Room'))

    await user.clear(searchInput)
    await user.type(searchInput, 'secreta')

    const remainingRevealButton = within(dialog).getByRole('button', { name: /revelar conquista secreta/i })
    remainingRevealButton.focus()
    await user.keyboard(' ')

    assert.ok(await within(dialog).findByText('Ancient Door'))
    assert.ok(within(dialog).getByText(/Steam .* disponibilizou .* descri.*conquista secreta/i))
  })

  test('preserves modal order through filtering and hidden reveal', async () => {
    const user = userEvent.setup()

    renderPanel()

    const { dialog, searchInput } = await openAchievementsModal(user)
    const initialOrder = getModalItemText()

    assert.deepEqual(getOrderTokens(initialOrder), [
      'First Step',
      'Hidden Winner',
      'Alpha Locked',
      'Beta Locked',
      'Conquista secreta',
      'Conquista secreta',
    ])

    await user.type(searchInput, 'locked')
    assert.deepEqual(
      getModalItemText().map((text) => text.match(/Alpha Locked|Beta Locked/)?.[0]).filter(Boolean),
      ['Alpha Locked', 'Beta Locked'],
    )

    await user.clear(searchInput)
    await user.type(searchInput, 'secreta')
    within(dialog).getAllByRole('button', { name: /revelar conquista secreta/i })[0].focus()
    await user.keyboard('{Enter}')

    await within(dialog).findByText('Dragon Room')
    const revealedOrder = getModalItemText()

    assert.equal(revealedOrder.findIndex((text) => text.includes('Dragon Room')) > revealedOrder.findIndex((text) => text.includes('Beta Locked')), true)
    assert.equal(revealedOrder.findIndex((text) => text.includes('Ancient Door')), -1)
  })

  test('does not leak modal reveal state back into the preview', async () => {
    const user = userEvent.setup()

    renderPanel()

    const { dialog } = await openAchievementsModal(user)

    await user.click(within(dialog).getAllByRole('button', { name: /revelar conquista secreta/i })[0])
    assert.ok(await within(dialog).findByText('Dragon Room'))

    await user.click(within(dialog).getByRole('button', { name: /fechar conquistas/i }))

    await waitFor(() => {
      assert.equal(screen.queryByRole('dialog', { name: /steam fixture/i }), null)
    })

    assert.ok(screen.getByText('Conquista secreta'))
    assert.equal(screen.queryByText('Dragon Room'), null)
  })

  test('renders non-fatal empty cache and not-yet-synced states', () => {
    const emptyCachedEntry = buildSteamEntry({ achievements: [], fetchedAt: '2026-05-27T11:30:00.000Z' })
    const missingCacheEntry = buildSteamEntry()
    delete missingCacheEntry.game.achievements

    const { rerender } = renderPanel({ selectedEntry: emptyCachedEntry })

    assert.ok(screen.getByText('Sem dados da Steam'))
    assert.ok(screen.getByText(/Steam .* disponibilizou dados para este jogo/i))

    rerender(
      <GameDetailsPanel
        launchFeedback={null}
        launchMessage=""
        selectedEntry={missingCacheEntry}
        selectedLaunchPlatformId="steam"
        showLibraryLoading={false}
        steamEnrichmentStatus={null}
        onArchiveEntry={noop}
        onEditEntry={noop}
        onLaunchEntry={noop}
        onLaunchPlatformChange={noop}
        onToggleFavoriteEntry={noop}
      />,
    )

    assert.ok(screen.getByText(/Dados ainda n.o sincronizados/))
  })

  test('renders enrichment progress, rate limit and recoverable error chips with expandable sanitized details', async () => {
    const user = userEvent.setup()
    const { rerender } = renderPanel({
      steamEnrichmentStatus: {
        phase: 'running',
        detail: 'Processando lote Steam em background.',
      },
    })

    assert.ok(screen.getByText('Sincronizando conquistas...'))

    rerender(
      <GameDetailsPanel
        launchFeedback={null}
        launchMessage=""
        selectedEntry={buildSteamEntry()}
        selectedLaunchPlatformId="steam"
        showLibraryLoading={false}
        steamEnrichmentStatus={{
          phase: 'failed',
          rateLimited: true,
          detail: 'Limite temporario da Steam; nova tentativa depois.',
        }}
        onArchiveEntry={noop}
        onEditEntry={noop}
        onLaunchEntry={noop}
        onLaunchPlatformChange={noop}
        onToggleFavoriteEntry={noop}
      />,
    )

    assert.ok(screen.getByText('Limite da Steam atingido'))
    await user.click(screen.getByRole('button', { name: /detalhes/i }))
    assert.ok(screen.getByText('Limite temporario da Steam; nova tentativa depois.'))

    rerender(
      <GameDetailsPanel
        launchFeedback={null}
        launchMessage=""
        selectedEntry={buildSteamEntry()}
        selectedLaunchPlatformId="steam"
        showLibraryLoading={false}
        steamEnrichmentStatus={{
          phase: 'failed',
          recoverable: true,
          detail: 'Codigo: steam_web_api_network_unavailable. Recuperavel: sim.',
        }}
        onArchiveEntry={noop}
        onEditEntry={noop}
        onLaunchEntry={noop}
        onLaunchPlatformChange={noop}
        onToggleFavoriteEntry={noop}
      />,
    )

    assert.ok(screen.getByText('Falha temporaria'))
    assert.equal(screen.queryByText(/api key|token|headers|payload bruto/i), null)
  })
})
