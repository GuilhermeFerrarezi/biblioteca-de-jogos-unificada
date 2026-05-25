import { Suspense, lazy, useState } from 'react'
import GameDetailsPanel from '../components/GameDetailsPanel'
import LibraryBrowser from '../components/LibraryBrowser'
import ManualGameModal from '../components/ManualGameModal'
import Sidebar from '../components/Sidebar'
import StatsGrid from '../components/StatsGrid'
import Topbar from '../components/Topbar'
import { useLibraryPageState } from '../hooks/useLibraryPageState'
import '../styles/library.css'

const AccountsSettingsPage = lazy(() => import('./AccountsSettingsPage'))

function LibraryPage() {
  const library = useLibraryPageState()
  const [activeSection, setActiveSection] = useState('library')

  const handleLibraryFilterChange = (filter) => {
    setActiveSection('library')
    library.handleQuickFilterChange(filter)
  }

  return (
    <main className="app-shell">
      <Sidebar
        activeSection={activeSection}
        quickFilters={library.quickFilters}
        onFilterChange={handleLibraryFilterChange}
        onAccountsClick={() => setActiveSection('accounts')}
      />

      <section className="workspace">
        {activeSection === 'accounts' ? (
          <Suspense
            fallback={
              <div className="empty-state">
                <strong>Carregando configuracoes de conta</strong>
                <span>Preparando Steam, Xbox e AuthVault sob demanda.</span>
              </div>
            }
          >
              <AccountsSettingsPage
                feedbackMessage={library.launchMessage}
                feedbackDetails={library.launchFeedback}
                isLibrarySettingsLoading={library.isLibrarySettingsLoading}
                isLibrarySettingsSaving={library.isLibrarySettingsSaving}
                isSteamAccountSyncing={library.isSteamAccountSyncing}
                isSteamSyncing={library.isSteamSyncing}
                isXboxSyncing={library.isXboxSyncing}
                preferredStoreId={library.preferredStoreId}
                localScanMode={library.localScanMode}
                localScanRootsText={library.localScanRootsText}
                localScanExcludedRootsText={library.localScanExcludedRootsText}
                microsoftClientId={library.microsoftClientId}
                onBackToLibrary={() => setActiveSection('library')}
                onPreferredStoreChange={library.handlePreferredStoreChange}
                onLocalScanModeChange={library.handleLocalScanModeChange}
                onLocalScanRootsChange={library.handleLocalScanRootsChange}
                onLocalScanRootsSelect={library.handleLocalScanRootsSelect}
                onLocalScanExcludedRootsChange={library.handleLocalScanExcludedRootsChange}
                onLocalScanExcludedRootsSelect={library.handleLocalScanExcludedRootsSelect}
                onMicrosoftClientIdChange={library.handleMicrosoftClientIdChange}
                onSaveLibrarySettings={library.handleSaveLibrarySettings}
                onSyncSteamAccountGames={library.handleSyncSteamAccountGames}
                onSyncSteamGames={library.handleSyncSteamGames}
                onSyncXboxTitleHistory={library.handleSyncXboxTitleHistory}
              onSyncXboxGames={library.handleSyncXboxGames}
            />
          </Suspense>
        ) : (
          <>
            <Topbar
              entriesCount={library.groupedEntries.length}
              isLocalSyncing={library.isLocalSyncing}
              isSteamSyncing={library.isSteamSyncing}
              onAddManualGame={library.openManualGameModal}
              onFilterClick={library.handleClearLibraryFilters}
              onSyncLocalGames={library.handleSyncLocalGames}
              onSyncSteamGames={library.handleSyncSteamGames}
            />

            <StatsGrid
              entriesCount={library.groupedEntries.length}
              installedCount={library.installedCount}
              totalHours={library.totalHours}
            />

            <div className="library-layout">
              <LibraryBrowser
                entriesCount={library.groupedEntries.length}
                filteredEntries={library.filteredEntries}
                quickFilters={library.quickFilters}
                searchTerm={library.searchTerm}
                selectedEntry={library.selectedEntry}
                showLibraryLoading={library.showLibraryLoading}
                sortMode={library.sortMode}
                viewMode={library.viewMode}
                onFilterChange={library.handleQuickFilterChange}
                onSearchChange={library.setSearchTerm}
                onSelectEntry={library.handleSelectEntry}
                onSortModeChange={library.setSortMode}
                onViewModeChange={library.setViewMode}
              />

              <GameDetailsPanel
                launchFeedback={library.launchFeedback}
                launchMessage={library.launchMessage}
                selectedLaunchPlatformId={library.selectedLaunchPlatformId}
                selectedEntry={library.selectedEntry}
                showLibraryLoading={library.showLibraryLoading}
                onArchiveEntry={library.handleArchiveSelectedEntry}
                onEditEntry={library.handleEditSelectedEntry}
                onInstallAction={library.handleInstallAction}
                onLaunchEntry={library.handleLaunchSelectedEntry}
                onLaunchPlatformChange={library.handleLaunchPlatformChange}
                onToggleFavoriteEntry={library.handleToggleFavoriteSelectedEntry}
              />
            </div>
          </>
        )}
      </section>

      {library.isManualModalOpen ? (
        <ManualGameModal
          form={library.manualGameForm}
          isEditing={library.isEditingManualGame}
          errors={library.manualGameErrors}
          onChange={library.setManualGameForm}
          onClearErrors={library.setManualGameErrors}
          onClose={library.closeManualModal}
          onSubmit={library.handleManualGameSubmit}
        />
      ) : null}
    </main>
  )
}

export default LibraryPage
