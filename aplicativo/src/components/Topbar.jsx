import { FolderPlus, RefreshCw, SlidersHorizontal } from 'lucide-react'

function Topbar({ entriesCount, isLocalSyncing, onAddManualGame, onSyncLocalGames }) {
  return (
    <header className="topbar">
      <div>
        <h1>Biblioteca de jogos</h1>
        <p>{entriesCount} jogos catalogados para o MVP inicial</p>
      </div>
      <div className="toolbar">
        <button className="icon-button" type="button" aria-label="Filtrar biblioteca" title="Filtrar biblioteca">
          <SlidersHorizontal size={18} aria-hidden="true" />
        </button>
        <button
          className="icon-button"
          type="button"
          aria-label="Sincronizar jogos locais"
          title="Sincronizar jogos locais"
          onClick={onSyncLocalGames}
          disabled={isLocalSyncing}
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
