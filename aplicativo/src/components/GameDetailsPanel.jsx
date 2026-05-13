import { Archive, Clock3, Download, Pencil, Play } from 'lucide-react'
import { getPlaytimeHours } from '../adapters/libraryEntryAdapter'
import { DEFAULT_ACCENT_COLOR, INSTALL_STATUS, PLATFORM_LABELS } from '../constants/libraryConstants'

function GameDetailsPanel({
  launchMessage,
  selectedEntry,
  showLibraryLoading,
  onArchiveEntry,
  onEditEntry,
  onInstallAction,
  onLaunchEntry,
}) {
  return (
    <aside className="details-panel" aria-label="Detalhes do jogo selecionado">
      {selectedEntry ? (
        <>
          <div className="detail-cover" style={{ background: selectedEntry.game.artwork.accentColor ?? DEFAULT_ACCENT_COLOR }}>
            <span>{selectedEntry.game.title}</span>
          </div>
          <div className="detail-content">
            <span className="platform-label">{PLATFORM_LABELS[selectedEntry.primaryPlatformId] ?? selectedEntry.primaryPlatformId}</span>
            <h2>{selectedEntry.game.title}</h2>
            <div className="detail-actions">
              <button className="play-button" type="button" onClick={onLaunchEntry}>
                <Play size={18} fill="currentColor" aria-hidden="true" />
                Jogar
              </button>
              <button className="icon-button" type="button" aria-label="Instalar ou localizar arquivos" title="Instalar ou localizar arquivos" onClick={onInstallAction}>
                <Download size={18} aria-hidden="true" />
              </button>
              {selectedEntry.primaryPlatformId === 'manual' ? (
                <button
                  className="icon-button"
                  type="button"
                  aria-label="Editar jogo"
                  title="Editar jogo"
                  onClick={onEditEntry}
                >
                  <Pencil size={18} aria-hidden="true" />
                </button>
              ) : null}
              <button
                className="icon-button"
                type="button"
                aria-label={selectedEntry.isArchived ? 'Reativar jogo' : 'Arquivar jogo'}
                title={selectedEntry.isArchived ? 'Reativar jogo' : 'Arquivar jogo'}
                onClick={onArchiveEntry}
              >
                <Archive size={18} aria-hidden="true" />
              </button>
            </div>
            <dl className="detail-list">
              <div>
                <dt>Status</dt>
                <dd>{selectedEntry.installStatus === INSTALL_STATUS.INSTALLED ? 'Instalado' : 'Nao instalado'}</dd>
              </div>
              <div>
                <dt>Arquivo</dt>
                <dd>{selectedEntry.isArchived ? 'Arquivado' : 'Ativo'}</dd>
              </div>
              <div>
                <dt>Tempo</dt>
                <dd>{getPlaytimeHours(selectedEntry.game.playtime.totalMinutes)}h</dd>
              </div>
              <div>
                <dt>Ultima vez</dt>
                <dd>{selectedEntry.lastPlayedLabel}</dd>
              </div>
              <div>
                <dt>Acao</dt>
                <dd>{selectedEntry.game.launchActions[0]?.label ?? 'Sem acao configurada'}</dd>
              </div>
            </dl>
            <div className="timeline-note">
              <Clock3 size={16} aria-hidden="true" />
              Sincronizacao Steam sera a primeira integracao real.
            </div>
            {launchMessage ? <div className="launch-feedback">{launchMessage}</div> : null}
          </div>
        </>
      ) : (
        <div className="detail-content">
          <span className="platform-label">Biblioteca</span>
          <h2>Nenhum jogo selecionado</h2>
          <div className="timeline-note">
            <Clock3 size={16} aria-hidden="true" />
            {showLibraryLoading ? 'Carregando biblioteca local.' : 'Adicione ou selecione um jogo para ver detalhes.'}
          </div>
          {launchMessage ? <div className="launch-feedback">{launchMessage}</div> : null}
        </div>
      )}
    </aside>
  )
}

export default GameDetailsPanel
