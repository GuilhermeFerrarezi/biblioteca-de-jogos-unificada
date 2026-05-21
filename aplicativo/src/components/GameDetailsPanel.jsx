import { Archive, Clock3, Download, Pencil, Play, Store } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { getPlaytimeHours } from '../adapters/libraryEntryAdapter'
import { DEFAULT_ACCENT_COLOR, INSTALL_STATUS, PLATFORM_LABELS } from '../constants/libraryConstants'
import { getLaunchActionState, getLaunchChoices, isMicrosoftStoreUri } from '../hooks/libraryPageStateHelpers'
import StatusDisclosure from './StatusDisclosure'

function GameDetailsPanel({
  launchFeedback,
  launchMessage,
  selectedLaunchPlatformId,
  selectedEntry,
  showLibraryLoading,
  onArchiveEntry,
  onEditEntry,
  onInstallAction,
  onLaunchEntry,
  onLaunchPlatformChange,
}) {
  const [isLaunchChooserOpen, setIsLaunchChooserOpen] = useState(false)
  const launchChooserButtonRef = useRef(null)
  const launchChooserRef = useRef(null)
  const artwork = selectedEntry?.game.artwork
  const launchChoices = getLaunchChoices(selectedEntry, selectedLaunchPlatformId).slice().sort((left, right) => {
    const platformComparison = (left.platformLabel ?? '').localeCompare(right.platformLabel ?? '', 'pt-BR', {
      sensitivity: 'base',
    })

    if (platformComparison !== 0) {
      return platformComparison
    }

    return (left.actionLabel ?? '').localeCompare(right.actionLabel ?? '', 'pt-BR', {
      sensitivity: 'base',
    })
  })
  const { primaryLaunchAction, canLaunch: canLaunchSelectedEntry, hint: launchActionHint } = getLaunchActionState(
    selectedEntry,
    selectedLaunchPlatformId,
  )
  const hasMultipleLaunchChoices = launchChoices.length > 1
  const isMicrosoftStoreAction =
    primaryLaunchAction?.label === 'Abrir Microsoft Store' || isMicrosoftStoreUri(primaryLaunchAction?.target)
  const primaryActionLabel = isMicrosoftStoreAction ? 'Abrir Microsoft Store' : 'Jogar'
  const PrimaryActionIcon = isMicrosoftStoreAction ? Store : Play

  useEffect(() => {
    setIsLaunchChooserOpen(false)
  }, [selectedEntry?.id])

  useEffect(() => {
    if (!isLaunchChooserOpen) {
      return undefined
    }

    const handleKeyDown = (event) => {
      if (event.key === 'Escape') {
        setIsLaunchChooserOpen(false)
        window.requestAnimationFrame(() => {
          launchChooserButtonRef.current?.focus()
        })
      }
    }

    window.addEventListener('keydown', handleKeyDown)

    return () => {
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [isLaunchChooserOpen])

  useEffect(() => {
    if (!isLaunchChooserOpen) {
      return undefined
    }

    const handlePointerDown = (event) => {
      if (!launchChooserRef.current?.contains(event.target)) {
        setIsLaunchChooserOpen(false)
      }
    }

    window.addEventListener('pointerdown', handlePointerDown)

    return () => {
      window.removeEventListener('pointerdown', handlePointerDown)
    }
  }, [isLaunchChooserOpen])

  return (
    <aside className="details-panel" aria-label="Detalhes do jogo selecionado">
      {selectedEntry ? (
        <>
          <DetailArtworkFrame
            imageUrls={[artwork?.coverUrl, artwork?.heroUrl]}
            accentColor={artwork?.accentColor}
            fallbackText={selectedEntry.game.title}
            imageAlt=""
          />
          <div className="detail-content">
            <span className="platform-label">
              {selectedEntry.platformSummary ?? PLATFORM_LABELS[selectedEntry.primaryPlatformId] ?? selectedEntry.primaryPlatformId}
            </span>
            <h2>{selectedEntry.game.title}</h2>
            {hasMultipleLaunchChoices ? (
              <div className="timeline-note launch-selection-note">
                <Store size={16} aria-hidden="true" />
                {selectedLaunchPlatformId === 'xbox' ? 'Biblioteca selecionada: Xbox.' : 'Biblioteca selecionada: Steam.'}
              </div>
            ) : null}
            <div className="detail-actions">
              <button
                className="play-button"
                type="button"
                disabled={!canLaunchSelectedEntry}
                aria-describedby={!canLaunchSelectedEntry && launchActionHint ? 'launch-action-hint' : undefined}
                title={!canLaunchSelectedEntry ? launchActionHint : primaryActionLabel}
                onClick={onLaunchEntry}
              >
                <PrimaryActionIcon size={18} fill={isMicrosoftStoreAction ? 'none' : 'currentColor'} aria-hidden="true" />
                {primaryActionLabel}
              </button>
              {hasMultipleLaunchChoices ? (
                <div className="launch-chooser" ref={launchChooserRef}>
                  <button
                    ref={launchChooserButtonRef}
                    className="secondary-button"
                    type="button"
                    aria-haspopup="menu"
                    aria-expanded={isLaunchChooserOpen}
                    aria-controls="launch-choice-panel"
                    onClick={() => setIsLaunchChooserOpen((currentValue) => !currentValue)}
                  >
                    Selecionar launcher
                  </button>
                  {isLaunchChooserOpen ? (
                    <div className="launch-choice-panel" id="launch-choice-panel" role="menu" aria-label="Escolher plataforma de inicio">
                      {launchChoices.map((choice) => (
                        <button
                          key={choice.entryId}
                          className="launch-choice-button"
                          type="button"
                          role="menuitemradio"
                          aria-checked={choice.platformId === selectedLaunchPlatformId}
                          onClick={() => {
                            setIsLaunchChooserOpen(false)
                            window.requestAnimationFrame(() => {
                              launchChooserButtonRef.current?.focus()
                            })
                            onLaunchPlatformChange(choice.platformId)
                          }}
                        >
                          <strong>{choice.platformLabel}</strong>
                          <span>{choice.platformId === selectedLaunchPlatformId ? 'Selecionado' : choice.actionLabel}</span>
                        </button>
                      ))}
                    </div>
                  ) : null}
                </div>
              ) : null}
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
                <dd>{hasMultipleLaunchChoices ? 'Escolher plataforma' : primaryLaunchAction?.label ?? 'Sem acao configurada'}</dd>
              </div>
            </dl>
            {!canLaunchSelectedEntry && launchActionHint ? (
              <p className="action-hint" id="launch-action-hint">
                {launchActionHint}
              </p>
            ) : null}
            {selectedEntry.platformIds?.includes('xbox') || selectedEntry.primaryPlatformId === 'xbox' ? (
              <div className="timeline-note">
                <Store size={16} aria-hidden="true" />
                {selectedEntry.installStatus === INSTALL_STATUS.INSTALLED
                  ? 'Xbox descoberto no Windows.'
                  : 'Xbox nao instalado: abrir Microsoft Store.'}
              </div>
            ) : null}
            <div className="timeline-note">
              <Clock3 size={16} aria-hidden="true" />
              Steam local, jogos locais e cadastro manual ja podem ser sincronizados.
            </div>
            {launchMessage ? (
              <StatusDisclosure className="launch-feedback" feedback={launchFeedback} message={launchMessage} />
            ) : null}
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
          {launchMessage ? (
            <StatusDisclosure className="launch-feedback" feedback={launchFeedback} message={launchMessage} />
          ) : null}
        </div>
      )}
    </aside>
  )
}

function DetailArtworkFrame({ imageUrls, accentColor, fallbackText, imageAlt }) {
  const availableImageUrls = imageUrls.filter(Boolean)
  const imageUrlsKey = availableImageUrls.join('\n')
  const [imageIndex, setImageIndex] = useState(0)
  const imageUrl = availableImageUrls[imageIndex]
  const shouldShowImage = Boolean(imageUrl)

  useEffect(() => {
    setImageIndex(0)
  }, [imageUrlsKey])

  return (
    <div className="detail-cover" style={{ background: accentColor ?? DEFAULT_ACCENT_COLOR }}>
      {shouldShowImage ? (
        <img className="artwork-image" src={imageUrl} alt={imageAlt} onError={() => setImageIndex((currentIndex) => currentIndex + 1)} />
      ) : (
        <span className="artwork-fallback-text">{fallbackText}</span>
      )}
    </div>
  )
}

export default GameDetailsPanel
