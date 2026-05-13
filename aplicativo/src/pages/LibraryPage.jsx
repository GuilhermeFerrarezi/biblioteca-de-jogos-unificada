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
