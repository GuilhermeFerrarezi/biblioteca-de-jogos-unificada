import { useState } from 'react'
import GameDetailsPanel from '../components/GameDetailsPanel'
import LibraryBrowser from '../components/LibraryBrowser'
import ManualGameModal from '../components/ManualGameModal'
import Sidebar from '../components/Sidebar'
import StatsGrid from '../components/StatsGrid'
import Topbar from '../components/Topbar'
import { useLibraryPageState } from '../hooks/useLibraryPageState'
import AccountsSettingsPage from './AccountsSettingsPage'
import '../styles/library.css'

function LibraryPage() {
  const library = useLibraryPageState()
  const [activeSection, setActiveSection] = useState('library')

  const handleLibraryFilterChange = (filter) => {
    setActiveSection('library')
    library.handleNavigationFilter(filter)
  }

  return (
    <main className="app-shell">
      <Sidebar
        activeSection={activeSection}
        quickFilter={library.quickFilter}
        onFilterChange={handleLibraryFilterChange}
        onAccountsClick={() => setActiveSection('accounts')}
      />

      <section className="workspace">
        {activeSection === 'accounts' ? (
          <AccountsSettingsPage
            feedbackMessage={library.launchMessage}
            isSteamSyncing={library.isSteamSyncing}
            onBackToLibrary={() => setActiveSection('library')}
            onSyncSteamGames={library.handleSyncSteamGames}
          />
        ) : (
          <>
            <Topbar
              entriesCount={library.entries.length}
              isLocalSyncing={library.isLocalSyncing}
              isSteamSyncing={library.isSteamSyncing}
              onAddManualGame={library.openManualGameModal}
              onFilterClick={library.handleClearLibraryFilters}
              onSyncLocalGames={library.handleSyncLocalGames}
              onSyncSteamGames={library.handleSyncSteamGames}
            />

            <StatsGrid
              entriesCount={library.entries.length}
              installedCount={library.installedCount}
              totalHours={library.totalHours}
            />

            <div className="library-layout">
              <LibraryBrowser
                filteredEntries={library.filteredEntries}
                quickFilter={library.quickFilter}
                searchTerm={library.searchTerm}
                selectedEntry={library.selectedEntry}
                showLibraryLoading={library.showLibraryLoading}
                viewMode={library.viewMode}
                onFilterChange={library.setQuickFilter}
                onSearchChange={library.setSearchTerm}
                onSelectEntry={library.handleSelectEntry}
                onViewModeChange={library.setViewMode}
              />

              <GameDetailsPanel
                launchMessage={library.launchMessage}
                selectedEntry={library.selectedEntry}
                showLibraryLoading={library.showLibraryLoading}
                onArchiveEntry={library.handleArchiveSelectedEntry}
                onEditEntry={library.handleEditSelectedEntry}
                onInstallAction={library.handleInstallAction}
                onLaunchEntry={library.handleLaunchSelectedEntry}
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
