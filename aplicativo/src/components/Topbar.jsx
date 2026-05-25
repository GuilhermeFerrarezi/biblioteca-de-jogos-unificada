import { FolderPlus, RefreshCw, SlidersHorizontal } from 'lucide-react'
import { useSteamEnrichmentStatus } from '../hooks/events/useSteamEnrichmentStatus'
import SteamIcon from './icons/SteamIcon'

function Topbar({
  entriesCount,
  isLocalSyncing,
  isSteamSyncing,
  onAddManualGame,
  onFilterClick,
  onSyncLocalGames,
  onSyncSteamGames,
}) {
  const isSyncing = isLocalSyncing || isSteamSyncing
  const steamEnrichmentStatus = useSteamEnrichmentStatus()

  return (
    <header className="topbar" aria-busy={isSyncing}>
      <div>
        <h1>Biblioteca de jogos</h1>
        <p>{entriesCount} jogos catalogados para o MVP inicial</p>
      </div>
      <div className="toolbar">
        <button className="icon-button" type="button" aria-label="Limpar filtros" title="Limpar filtros" onClick={onFilterClick}>
          <SlidersHorizontal size={18} aria-hidden="true" />
        </button>
        <button
          className="icon-button"
          type="button"
          aria-label="Sincronizar Steam"
          title="Sincronizar Steam"
          onClick={onSyncSteamGames}
          disabled={isSyncing}
        >
          <SteamIcon size={18} className={isSteamSyncing ? 'spin-icon' : ''} />
        </button>
        {steamEnrichmentStatus ? (
          <div
            className="steam-enrichment-status"
            data-phase={steamEnrichmentStatus.phase}
            data-rate-limited={steamEnrichmentStatus.rateLimited ? 'true' : undefined}
            role="status"
            aria-live="polite"
            aria-label={`${steamEnrichmentStatus.text}. ${steamEnrichmentStatus.detail}`}
            title={steamEnrichmentStatus.detail}
          >
            <span className="steam-enrichment-status-dot" aria-hidden="true" />
            <span>{steamEnrichmentStatus.text}</span>
          </div>
        ) : null}
        <button
          className="icon-button"
          type="button"
          aria-label="Sincronizar jogos locais"
          title="Sincronizar jogos locais"
          onClick={onSyncLocalGames}
          disabled={isSyncing}
        >
          <RefreshCw size={18} aria-hidden="true" className={isLocalSyncing ? 'spin-icon' : ''} />
        </button>
        <button className="primary-button" type="button" onClick={onAddManualGame}>
          <FolderPlus size={18} aria-hidden="true" />
          Adicionar jogo
        </button>
      </div>
    </header>
  )
}

export default Topbar
