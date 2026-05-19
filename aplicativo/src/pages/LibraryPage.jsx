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
          <AccountsSettingsPage
            feedbackMessage={library.launchMessage}
            feedbackDetails={library.launchFeedback}
            isLibrarySettingsLoading={library.isLibrarySettingsLoading}
            isLibrarySettingsSaving={library.isLibrarySettingsSaving}
            isSteamAccountSyncing={library.isSteamAccountSyncing}
            isSteamSyncing={library.isSteamSyncing}
            isXboxSyncing={library.isXboxSyncing}
            preferredStoreId={library.preferredStoreId}
            xboxIdentityStatus={library.xboxIdentityStatus}
            onBackToLibrary={() => setActiveSection('library')}
            onPreferredStoreChange={library.handlePreferredStoreChange}
            onSyncSteamAccountGames={library.handleSyncSteamAccountGames}
            onSyncSteamGames={library.handleSyncSteamGames}
            onSyncXboxTitleHistory={library.handleSyncXboxTitleHistory}
            onSyncXboxGames={library.handleSyncXboxGames}
          />
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
                viewMode={library.viewMode}
                onFilterChange={library.handleQuickFilterChange}
                onSearchChange={library.setSearchTerm}
                onSelectEntry={library.handleSelectEntry}
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
