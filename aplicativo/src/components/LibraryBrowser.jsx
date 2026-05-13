import { LayoutGrid, List, Search } from 'lucide-react'
import { getPlaytimeHours } from '../adapters/libraryEntryAdapter'
import { platformLabels } from '../data/mockLibrary'
import { quickFilterLabels } from '../hooks/useLibraryPageState'

function LibraryBrowser({
  filteredEntries,
  quickFilter,
  searchTerm,
  selectedEntry,
  showLibraryLoading,
  viewMode,
  onFilterChange,
  onSearchChange,
  onSelectEntry,
  onViewModeChange,
}) {
  return (
    <section className="library-list" aria-label="Lista de jogos">
      <label className="search-box">
        <Search size={18} aria-hidden="true" />
        <input
          type="search"
          placeholder="Buscar por nome, plataforma ou genero"
          value={searchTerm}
          onChange={(event) => onSearchChange(event.target.value)}
        />
      </label>

      <div className="filter-row" aria-label="Filtros rapidos">
        {Object.keys(quickFilterLabels).map((filter) => (
          <button
            className={quickFilter === filter ? 'filter-chip active' : 'filter-chip'}
            type="button"
            key={filter}
            onClick={() => onFilterChange(filter)}
          >
            {quickFilterLabels[filter]}
          </button>
        ))}
        <div className="view-toggle" aria-label="Modo de visualizacao">
          <button
            className={viewMode === 'list' ? 'active' : ''}
            type="button"
            aria-label="Mostrar jogos em lista"
            title="Lista"
            onClick={() => onViewModeChange('list')}
          >
            <List size={17} aria-hidden="true" />
          </button>
          <button
            className={viewMode === 'grid' ? 'active' : ''}
            type="button"
            aria-label="Mostrar capas dos jogos"
            title="Capas"
            onClick={() => onViewModeChange('grid')}
          >
            <LayoutGrid size={17} aria-hidden="true" />
          </button>
        </div>
      </div>

      {showLibraryLoading ? (
        <div className="empty-state">
          <strong>Carregando biblioteca</strong>
          <span>Consultando a listagem unificada local.</span>
        </div>
      ) : viewMode === 'list' ? (
        <div className="game-table">
          {filteredEntries.map((entry) => (
            <GameRow
              entry={entry}
              isSelected={selectedEntry?.id === entry.id}
              key={entry.id}
              onSelectEntry={onSelectEntry}
            />
          ))}
        </div>
      ) : (
        <div className="game-cover-grid">
          {filteredEntries.map((entry) => (
            <GameCoverCard
              entry={entry}
              isSelected={selectedEntry?.id === entry.id}
              key={entry.id}
              onSelectEntry={onSelectEntry}
            />
          ))}
        </div>
      )}
      {!showLibraryLoading && filteredEntries.length === 0 ? (
        <div className="empty-state">
          <strong>Nenhum jogo encontrado</strong>
          <span>Ajuste a busca ou troque o filtro ativo.</span>
        </div>
      ) : null}
    </section>
  )
}

function GameRow({ entry, isSelected, onSelectEntry }) {
  return (
    <article
      className={isSelected ? 'game-row selected' : 'game-row'}
      role="button"
      tabIndex={0}
      onClick={() => onSelectEntry(entry.id)}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          onSelectEntry(entry.id)
        }
      }}
    >
      <div className="cover" style={{ background: entry.game.artwork.accentColor }}>
        {entry.game.title.slice(0, 1)}
      </div>
      <div className="game-info">
        <strong>{entry.game.title}</strong>
        <span>{platformLabels[entry.primaryPlatformId]} / {entry.game.genres[0]}</span>
      </div>
      <div className="status-pill" data-status={entry.installStatus}>
        {entry.installStatus === 'installed' ? 'Instalado' : 'Nao instalado'}
      </div>
      <span className="playtime">{getPlaytimeHours(entry.game.playtime.totalMinutes)}h</span>
    </article>
  )
}

function GameCoverCard({ entry, isSelected, onSelectEntry }) {
  return (
    <article
      className={isSelected ? 'game-cover-card selected' : 'game-cover-card'}
      role="button"
      tabIndex={0}
      onClick={() => onSelectEntry(entry.id)}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          onSelectEntry(entry.id)
        }
      }}
    >
      <div className="poster" style={{ background: entry.game.artwork.accentColor }}>
        <span>{entry.game.title}</span>
      </div>
      <strong>{entry.game.title}</strong>
      <span>{platformLabels[entry.primaryPlatformId]}</span>
    </article>
  )
}

export default LibraryBrowser
