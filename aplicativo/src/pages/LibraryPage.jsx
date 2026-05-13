import GameDetailsPanel from '../components/GameDetailsPanel'
import LibraryBrowser from '../components/LibraryBrowser'
import ManualGameModal from '../components/ManualGameModal'
import Sidebar from '../components/Sidebar'
import StatsGrid from '../components/StatsGrid'
import Topbar from '../components/Topbar'
import { useLibraryPageState } from '../hooks/useLibraryPageState'
import '../styles/library.css'

function LibraryPage() {
  const library = useLibraryPageState()

  return (
    <main className="app-shell">
      <Sidebar
        quickFilter={library.quickFilter}
        onFilterChange={library.handleNavigationFilter}
        onAccountsClick={() => library.setLaunchMessage('Gerenciamento de contas sera implementado na fase de integracoes.')}
      />

      <section className="workspace">
        <Topbar
          entriesCount={library.entries.length}
          isLocalSyncing={library.isLocalSyncing}
          onAddManualGame={library.openManualGameModal}
          onSyncLocalGames={library.handleSyncLocalGames}
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
      </section>

      {library.isManualModalOpen ? (
        <ManualGameModal
          form={library.manualGameForm}
          isEditing={library.isEditingManualGame}
          error={library.manualGameError}
          onChange={library.setManualGameForm}
          onClearError={library.setManualGameError}
          onClose={library.closeManualModal}
          onSubmit={library.handleManualGameSubmit}
        />
      ) : null}
    </main>
  )
}

export default LibraryPage
