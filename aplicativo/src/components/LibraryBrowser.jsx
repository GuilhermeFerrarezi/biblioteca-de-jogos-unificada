import { CircleDot, HardDrive, LayoutGrid, Library, List, Search } from 'lucide-react'
import { useEffect, useState } from 'react'
import { getPlaytimeHours } from '../adapters/libraryEntryAdapter'
import { DEFAULT_ACCENT_COLOR, INSTALL_STATUS, PLATFORM_LABELS, QUICK_FILTERS, QUICK_FILTER_IDS } from '../constants/libraryConstants'
import SteamIcon from './icons/SteamIcon'
import XboxIcon from './icons/XboxIcon'

const STATUS_FILTER_IDS = Object.freeze([QUICK_FILTER_IDS.INSTALLED, QUICK_FILTER_IDS.NOT_INSTALLED])
const PLATFORM_FILTER_IDS = Object.freeze([QUICK_FILTER_IDS.STEAM, QUICK_FILTER_IDS.XBOX, QUICK_FILTER_IDS.LOCAL])
const QUICK_FILTER_ICONS = Object.freeze({
  [QUICK_FILTER_IDS.ALL]: Library,
  [QUICK_FILTER_IDS.INSTALLED]: CircleDot,
  [QUICK_FILTER_IDS.NOT_INSTALLED]: HardDrive,
  [QUICK_FILTER_IDS.STEAM]: SteamIcon,
  [QUICK_FILTER_IDS.XBOX]: XboxIcon,
  [QUICK_FILTER_IDS.LOCAL]: HardDrive,
})

function LibraryBrowser({
  entriesCount,
  filteredEntries,
  quickFilters,
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
      <SearchBox searchTerm={searchTerm} onSearchChange={onSearchChange} />

      <div className="filter-row">
        <FilterChipsRow quickFilters={quickFilters} onFilterChange={onFilterChange} />
        <FilterSummary
          filteredEntriesCount={filteredEntries.length}
          totalEntriesCount={entriesCount}
          isLoading={showLibraryLoading}
        />
        <ViewModeToggle viewMode={viewMode} onViewModeChange={onViewModeChange} />
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
          <strong>{getEmptyStateTitle(entriesCount, searchTerm, quickFilters)}</strong>
          <span>{getEmptyStateDescription(entriesCount, searchTerm, quickFilters)}</span>
        </div>
      ) : null}
    </section>
  )
}

function getEmptyStateTitle(entriesCount, searchTerm, quickFilters) {
  if (entriesCount === 0) {
    return 'Biblioteca vazia'
  }

  if (searchTerm.trim() || quickFilters.length > 0) {
    return 'Nenhum jogo encontrado'
  }

  return 'Nenhum jogo encontrado'
}

function getEmptyStateDescription(entriesCount, searchTerm, quickFilters) {
  const hasSearch = Boolean(searchTerm.trim())
  const activeQuickFilters = quickFilters.filter((filterId) => filterId !== QUICK_FILTER_IDS.ALL)
  const hasFilters = activeQuickFilters.length > 0
  const statusFilters = activeQuickFilters.filter((filterId) => STATUS_FILTER_IDS.includes(filterId))
  const platformFilters = activeQuickFilters.filter((filterId) => PLATFORM_FILTER_IDS.includes(filterId))

  if (entriesCount === 0) {
    return 'Adicione um jogo manualmente ou sincronize um provider para começar.'
  }

  if (hasSearch && hasFilters) {
    return 'Nenhum jogo corresponde a esta busca e à combinação de filtros selecionada.'
  }

  if (hasSearch) {
    return 'Nenhum jogo corresponde ao termo pesquisado.'
  }

  if (statusFilters.length > 0 && platformFilters.length > 0) {
    return `Nenhum jogo corresponde à combinação de ${describeFilterGroup('status', statusFilters)} e ${describeFilterGroup('plataforma', platformFilters)}.`
  }

  if (statusFilters.length === 1) {
    return `Nenhum jogo corresponde ao status ${getFilterLabel(statusFilters[0]).toLowerCase()} selecionado.`
  }

  if (statusFilters.length > 1) {
    return `Nenhum jogo corresponde aos status ${getFilterLabels(statusFilters).join(' e ').toLowerCase()} selecionados.`
  }

  if (platformFilters.length === 1) {
    return `Nenhum jogo corresponde à plataforma ${getFilterLabel(platformFilters[0]).toLowerCase()} selecionada.`
  }

  if (platformFilters.length > 1) {
    return `Nenhum jogo corresponde às plataformas ${getFilterLabels(platformFilters).join(' e ').toLowerCase()} selecionadas.`
  }

  if (hasFilters) {
    return 'Nenhum jogo corresponde à combinação atual de filtros.'
  }

  return 'Ajuste a busca ou os filtros para refinar os resultados.'
}

function describeFilterGroup(groupLabel, filterIds) {
  const labels = getFilterLabels(filterIds)

  if (labels.length === 1) {
    return `${groupLabel} ${labels[0].toLowerCase()}`
  }

  return `${groupLabel}s ${labels.join(' e ').toLowerCase()}`
}

function getFilterLabels(filterIds) {
  return filterIds.map((filterId) => getFilterLabel(filterId))
}

function getFilterLabel(filterId) {
  return QUICK_FILTERS.find((filter) => filter.id === filterId)?.label ?? filterId
}

function SearchBox({ searchTerm, onSearchChange }) {
  return (
    <label className="search-box">
      <Search size={18} aria-hidden="true" />
      <input
        type="search"
        placeholder="Buscar por nome, plataforma ou genero"
        value={searchTerm}
        onChange={(event) => onSearchChange(event.target.value)}
      />
    </label>
  )
}

function FilterChipsRow({ quickFilters, onFilterChange }) {
  return (
    <div className="filter-chips" aria-label="Filtros rapidos">
      {QUICK_FILTERS.map((filter) => {
        const Icon = QUICK_FILTER_ICONS[filter.id]

        return (
          <button
            className={isFilterActive(quickFilters, filter.id) ? 'filter-chip active' : 'filter-chip'}
            type="button"
            key={filter.id}
            aria-pressed={isFilterActive(quickFilters, filter.id)}
            onClick={() => onFilterChange(filter.id)}
          >
            {Icon ? <Icon size={16} /> : null}
            {filter.label}
          </button>
        )
      })}
    </div>
  )
}

function FilterSummary({ filteredEntriesCount, totalEntriesCount, isLoading }) {
  const label = isLoading
    ? 'Consultando filtros'
    : `de ${totalEntriesCount} ${totalEntriesCount === 1 ? 'jogo' : 'jogos'}`

  return (
    <div className="filter-summary" aria-live="polite">
      <strong>{isLoading ? '...' : filteredEntriesCount}</strong>
      <span>{label}</span>
    </div>
  )
}

function isFilterActive(quickFilters, filterId) {
  return filterId === QUICK_FILTER_IDS.ALL ? quickFilters.length === 0 : quickFilters.includes(filterId)
}

function ViewModeToggle({ viewMode, onViewModeChange }) {
  return (
    <div className="view-toggle" aria-label="Modo de visualizacao">
      <button
        className={viewMode === 'list' ? 'active' : ''}
        type="button"
        aria-label="Mostrar jogos em lista"
        aria-pressed={viewMode === 'list'}
        title="Lista"
        onClick={() => onViewModeChange('list')}
      >
        <List size={17} aria-hidden="true" />
      </button>
      <button
        className={viewMode === 'grid' ? 'active' : ''}
        type="button"
        aria-label="Mostrar capas dos jogos"
        aria-pressed={viewMode === 'grid'}
        title="Capas"
        onClick={() => onViewModeChange('grid')}
      >
        <LayoutGrid size={17} aria-hidden="true" />
      </button>
    </div>
  )
}

function GameRow({ entry, isSelected, onSelectEntry }) {
  const artwork = entry.game.artwork

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
      <ArtworkFrame
        className="cover"
        imageUrls={[artwork.coverUrl, artwork.heroUrl]}
        accentColor={artwork.accentColor}
        fallbackText={entry.game.title.slice(0, 1)}
        imageAlt=""
      />
      <div className="game-info">
        <strong>{entry.game.title}</strong>
        <span>{entry.platformSummary ?? PLATFORM_LABELS[entry.primaryPlatformId]} / {entry.game.genres[0]}</span>
      </div>
      <div className="status-pill" data-status={entry.installStatus}>
        {entry.installStatus === INSTALL_STATUS.INSTALLED ? 'Instalado' : 'Nao instalado'}
      </div>
      <span className="playtime">{getPlaytimeHours(entry.game.playtime.totalMinutes)}h</span>
    </article>
  )
}

function GameCoverCard({ entry, isSelected, onSelectEntry }) {
  const artwork = entry.game.artwork

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
      <ArtworkFrame
        className="poster"
        imageUrls={[artwork.coverUrl, artwork.heroUrl]}
        accentColor={artwork.accentColor}
        fallbackText={entry.game.title}
        imageAlt=""
      />
      <strong>{entry.game.title}</strong>
      <span>{entry.platformSummary ?? PLATFORM_LABELS[entry.primaryPlatformId]}</span>
    </article>
  )
}

function ArtworkFrame({ className, imageUrls, accentColor, fallbackText, imageAlt }) {
  const availableImageUrls = imageUrls.filter(Boolean)
  const imageUrlsKey = availableImageUrls.join('\n')
  const [imageIndex, setImageIndex] = useState(0)
  const imageUrl = availableImageUrls[imageIndex]
  const shouldShowImage = Boolean(imageUrl)

  useEffect(() => {
    setImageIndex(0)
  }, [imageUrlsKey])

  return (
    <div className={className} style={{ background: accentColor ?? DEFAULT_ACCENT_COLOR }}>
      {shouldShowImage ? (
        <img
          className="artwork-image"
          src={imageUrl}
          alt={imageAlt}
          loading="lazy"
          onError={() => setImageIndex((currentIndex) => currentIndex + 1)}
        />
      ) : (
        <span className="artwork-fallback-text">{fallbackText}</span>
      )}
    </div>
  )
}

export default LibraryBrowser
