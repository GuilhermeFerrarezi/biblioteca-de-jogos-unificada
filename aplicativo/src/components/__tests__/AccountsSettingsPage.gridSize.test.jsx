import { fireEvent, render, screen, within } from '@testing-library/react'
import assert from 'node:assert/strict'
import { describe, test, vi } from 'vitest'
import { LibraryDefaultsCard } from '../../pages/AccountsSettingsPage.jsx'

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

vi.mock('../../services/libraryCommands', () => ({
  deleteSteamApiKey: vi.fn(),
  getEpicLibraryRoots: vi.fn(),
  getSteamAccountConfig: vi.fn(),
  getSteamApiKeyStatus: vi.fn(),
  getSteamEnrichmentRetrySummary: vi.fn(),
  getSteamLibraryRoots: vi.fn(),
  getXboxAccountConfig: vi.fn(),
  getXboxLibraryRoots: vi.fn(),
  getXboxLiveAuthState: vi.fn(),
  saveEpicLibraryRoots: vi.fn(),
  saveSteamAccountConfig: vi.fn(),
  saveSteamApiKey: vi.fn(),
  saveSteamLibraryRoots: vi.fn(),
  saveXboxLibraryRoots: vi.fn(),
  startSteamLogin: vi.fn(),
  startXboxLiveLogin: vi.fn(),
}))

vi.mock('../../services/librarySettings', () => ({
  saveLibrarySettings: vi.fn(),
}))

const noop = () => {}

function renderDefaultsCard(props = {}) {
  return render(
    <LibraryDefaultsCard
      gridSize="default"
      isLoading={false}
      isSaving={false}
      localScanExcludedRootsText=""
      localScanMode="automatic"
      localScanRootsText=""
      preferredStoreId="steam"
      onGridSizeChange={noop}
      onLocalScanExcludedRootsChange={noop}
      onLocalScanExcludedRootsSelect={noop}
      onLocalScanModeChange={noop}
      onLocalScanRootsChange={noop}
      onLocalScanRootsSelect={noop}
      onPreferredStoreChange={noop}
      onSaveLibrarySettings={noop}
      {...props}
    />,
  )
}

describe('LibraryDefaultsCard grid size control', () => {
  test('shows product labels for grid size choices and preserves internal values', () => {
    const onGridSizeChange = vi.fn()
    renderDefaultsCard({ gridSize: 'default', onGridSizeChange })

    const group = screen.getByRole('group', { name: 'Tamanho da grid' })
    const buttons = within(group).getAllByRole('button')

    assert.deepEqual(
      buttons.map((button) => button.textContent),
      ['Compacta', 'Padrão', 'Grande'],
    )
    assert.equal(within(group).getByRole('button', { name: 'Padrão' }).getAttribute('aria-pressed'), 'true')

    fireEvent.click(within(group).getByRole('button', { name: 'Compacta' }))
    fireEvent.click(within(group).getByRole('button', { name: 'Padrão' }))
    fireEvent.click(within(group).getByRole('button', { name: 'Grande' }))

    assert.deepEqual(onGridSizeChange.mock.calls, [['compact'], ['default'], ['large']])
  })
})
