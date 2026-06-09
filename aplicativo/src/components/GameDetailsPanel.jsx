import { Archive, Clock3, Download, Eye, Heart, LockKeyhole, Pencil, Play, Search, Star, Store, Trophy, X } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import {
  buildAchievementObservability,
  filterAchievementItems,
  getAchievementDisplayState,
  getAchievementKey,
  getAchievementProgress,
  getPlaytimeHours,
  sortAchievementItems,
} from '../adapters/libraryEntryAdapter'
import { DEFAULT_ACCENT_COLOR, INSTALL_STATUS, LAUNCH_ACTION_KIND, PLATFORM_LABELS } from '../constants/libraryConstants'
import { getDetailsEntryForSelectedPlatform, getLaunchActionState, getLaunchChoices, isMicrosoftStoreUri, isSteamInstallUri } from '../domain/libraryLaunch'
import StatusDisclosure from './StatusDisclosure'

const WINDOWS_EXECUTABLE_PATH_PATTERN = /^[a-zA-Z]:[\\/].+\.exe$/i
const WINDOWS_EXPLORER_PATH_PATTERN = /(?:^|[\\/])explorer\.exe$/i

function getParentDirectory(path) {
  const normalizedPath = String(path ?? '').trim()

  if (!WINDOWS_EXECUTABLE_PATH_PATTERN.test(normalizedPath)) {
    return ''
  }

  const lastSeparatorIndex = Math.max(normalizedPath.lastIndexOf('\\'), normalizedPath.lastIndexOf('/'))

  return lastSeparatorIndex > 0 ? normalizedPath.slice(0, lastSeparatorIndex) : ''
}

function resolveInstalledGamePath(selectedEntry, primaryLaunchAction) {
  if (selectedEntry?.installStatus !== INSTALL_STATUS.INSTALLED) {
    return ''
  }

  const workingDirectory = String(primaryLaunchAction?.workingDirectory ?? '').trim()

  if (workingDirectory) {
    return workingDirectory
  }

  if (primaryLaunchAction?.kind === LAUNCH_ACTION_KIND.EXECUTABLE) {
    if (WINDOWS_EXPLORER_PATH_PATTERN.test(String(primaryLaunchAction.target ?? ''))) {
      return ''
    }

    return getParentDirectory(primaryLaunchAction.target)
  }

  return String(selectedEntry?.game?.installLocations?.[0] ?? '').trim()
}

function resolveDisplayArtwork(selectedEntry, detailsEntry) {
  const steamEntry = Array.isArray(selectedEntry?.memberEntries)
    ? selectedEntry.memberEntries.find((entry) => entry.primaryPlatformId === 'steam')
    : null

  return steamEntry?.game?.artwork ?? detailsEntry?.game?.artwork ?? selectedEntry?.game?.artwork
}

function GameDetailsPanel({
  launchFeedback,
  launchMessage,
  selectedLaunchPlatformId,
  selectedEntry,
  showLibraryLoading,
  steamEnrichmentStatus,
  onArchiveEntry,
  onEditEntry,
  onLaunchEntry,
  onLaunchPlatformChange,
  onSavePersonalReview,
  onToggleFavoriteEntry,
}) {
  const [isLaunchChooserOpen, setIsLaunchChooserOpen] = useState(false)
  const [isAchievementsModalOpen, setIsAchievementsModalOpen] = useState(false)
  const [achievementSearchTerm, setAchievementSearchTerm] = useState('')
  const achievementsModalTriggerRef = useRef(null)
  const launchChooserButtonRef = useRef(null)
  const launchChooserRef = useRef(null)
  const detailsEntry = getDetailsEntryForSelectedPlatform(selectedEntry, selectedLaunchPlatformId)
  const artwork = resolveDisplayArtwork(selectedEntry, detailsEntry)
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
  const isSteamInstallAction = isSteamInstallUri(primaryLaunchAction?.target)
  const primaryActionLabel = isMicrosoftStoreAction ? 'Abrir Microsoft Store' : isSteamInstallAction ? 'Instalar' : 'Jogar'
  const PrimaryActionIcon = isMicrosoftStoreAction ? Store : isSteamInstallAction ? Download : Play
  const installedGamePath = resolveInstalledGamePath(detailsEntry, primaryLaunchAction)
  const achievementProgress = getAchievementProgress(detailsEntry)
  const displayedPlatformLabel =
    detailsEntry && hasMultipleLaunchChoices
      ? PLATFORM_LABELS[detailsEntry.primaryPlatformId] ?? detailsEntry.primaryPlatformId
      : selectedEntry?.platformSummary ?? PLATFORM_LABELS[selectedEntry?.primaryPlatformId] ?? selectedEntry?.primaryPlatformId
  const selectedPlatformLabel = PLATFORM_LABELS[selectedLaunchPlatformId] ?? selectedLaunchPlatformId
  const isFavorite =
    selectedEntry?.isFavorite === true ||
    selectedEntry?.is_favorite === true ||
    selectedEntry?.memberEntries?.some((entry) => entry?.isFavorite === true || entry?.is_favorite === true) === true

  useEffect(() => {
    setIsLaunchChooserOpen(false)
    setIsAchievementsModalOpen(false)
    setAchievementSearchTerm('')
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
    if (!isAchievementsModalOpen) {
      return undefined
    }

    document.body.classList.add('achievement-modal-open')

    return () => {
      document.body.classList.remove('achievement-modal-open')
    }
  }, [isAchievementsModalOpen])

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
            imageUrls={[artwork?.heroUrl, artwork?.fallbackUrl, artwork?.coverUrl]}
            accentColor={artwork?.accentColor}
            fallbackText={selectedEntry.game.title}
            imageAlt=""
          />
          <div className="detail-content">
            <span className="platform-label">{displayedPlatformLabel}</span>
            <h2>{selectedEntry.game.title}</h2>
            {hasMultipleLaunchChoices ? (
              <div className="timeline-note launch-selection-note">
                <Store size={16} aria-hidden="true" />
                {`Biblioteca selecionada: ${selectedPlatformLabel}.`}
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
                <PrimaryActionIcon size={18} fill={isMicrosoftStoreAction || isSteamInstallAction ? 'none' : 'currentColor'} aria-hidden="true" />
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
              <button
                className={isFavorite ? 'icon-button favorite-button active' : 'icon-button favorite-button'}
                type="button"
                aria-label={isFavorite ? 'Remover dos favoritos' : 'Adicionar aos favoritos'}
                aria-pressed={isFavorite}
                title={isFavorite ? 'Remover dos favoritos' : 'Adicionar aos favoritos'}
                onClick={onToggleFavoriteEntry}
              >
                <Heart size={18} fill={isFavorite ? 'currentColor' : 'none'} aria-hidden="true" />
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
                <dd>{detailsEntry?.installStatus === INSTALL_STATUS.INSTALLED ? 'Instalado' : 'Nao instalado'}</dd>
              </div>
              <div>
                <dt>Arquivo</dt>
                <dd>{detailsEntry?.isArchived ? 'Arquivado' : 'Ativo'}</dd>
              </div>
              {installedGamePath ? (
                <div>
                  <dt>Caminho</dt>
                  <dd title={installedGamePath}>{installedGamePath}</dd>
                </div>
              ) : null}
              <div>
                <dt>Tempo</dt>
                <dd>{getPlaytimeHours(detailsEntry?.game.playtime.totalMinutes ?? 0)}h</dd>
              </div>
              <div>
                <dt>Ultima vez</dt>
                <dd>{detailsEntry?.lastPlayedLabel ?? 'Nunca'}</dd>
              </div>
              <div>
                <dt>Acao</dt>
                <dd>{primaryLaunchAction?.label ?? 'Sem acao configurada'}</dd>
              </div>
            </dl>
            {!canLaunchSelectedEntry && launchActionHint ? (
              <p className="action-hint" id="launch-action-hint">
                {launchActionHint}
              </p>
            ) : null}
            <PersonalReviewSection
              selectedEntry={selectedEntry}
              onSavePersonalReview={onSavePersonalReview}
            />
            {detailsEntry?.primaryPlatformId === 'xbox' ? (
              <div className="timeline-note">
                <Store size={16} aria-hidden="true" />
                {detailsEntry.installStatus === INSTALL_STATUS.INSTALLED
                  ? 'Xbox descoberto no Windows.'
                  : 'Xbox nao instalado: abrir Microsoft Store.'}
              </div>
            ) : null}
            <SteamAchievementsSection
              achievements={achievementProgress}
              gameTitle={selectedEntry.game.title}
              isSteamGame={detailsEntry?.primaryPlatformId === 'steam'}
              libraryLabel={displayedPlatformLabel}
              isModalOpen={isAchievementsModalOpen}
              modalTriggerRef={achievementsModalTriggerRef}
              searchTerm={achievementSearchTerm}
              steamEnrichmentStatus={steamEnrichmentStatus}
              onCloseModal={() => {
                setIsAchievementsModalOpen(false)
                setAchievementSearchTerm('')
                window.requestAnimationFrame(() => {
                  achievementsModalTriggerRef.current?.focus()
                })
              }}
              onOpenModal={() => {
                setIsAchievementsModalOpen(true)
              }}
              onSearchTermChange={setAchievementSearchTerm}
            />
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

const ACHIEVEMENT_PREVIEW_ACHIEVED_LIMIT = 3
const ACHIEVEMENT_PREVIEW_LOCKED_LIMIT = 3
const PERSONAL_RATING_OPTIONS = Object.freeze([0.5, 1, 1.5, 2, 2.5, 3, 3.5, 4, 4.5, 5])

function PersonalReviewSection({ selectedEntry, onSavePersonalReview }) {
  const savedRating = selectedEntry?.game?.personalRating ?? null
  const savedReview = selectedEntry?.game?.personalReview ?? ''
  const [rating, setRating] = useState(savedRating)
  const [review, setReview] = useState(savedReview)
  const [status, setStatus] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const normalizedReview = review.trim()
  const hasChanges = rating !== savedRating || normalizedReview !== String(savedReview ?? '').trim()

  useEffect(() => {
    setRating(savedRating)
    setReview(savedReview)
    setStatus('')
    setIsSaving(false)
  }, [selectedEntry?.id, savedRating, savedReview])

  const handleSubmit = async (event) => {
    event.preventDefault()
    setStatus('')
    setIsSaving(true)

    try {
      await onSavePersonalReview?.({
        rating,
        review: normalizedReview || null,
      })
      setStatus('Salvo.')
    } catch (error) {
      setStatus(error?.message || 'Nao foi possivel salvar.')
    } finally {
      setIsSaving(false)
    }
  }

  const handleClear = () => {
    setRating(null)
    setStatus('')
  }

  return (
    <form className="personal-review-panel" onSubmit={handleSubmit}>
      <div className="personal-review-heading">
        <div>
          <span className="section-kicker">Minha avaliacao</span>
          <strong>{rating ? `${rating} de 5 estrelas` : 'Sem nota'}</strong>
        </div>
        <button className="text-button" type="button" onClick={handleClear}>
          Limpar nota
        </button>
      </div>
      <div className="personal-rating-control" aria-label="Nota pessoal">
        {PERSONAL_RATING_OPTIONS.map((option) => (
          <button
            className={rating === option ? 'personal-rating-option active' : 'personal-rating-option'}
            key={option}
            type="button"
            aria-label={`Avaliar com ${option} estrelas`}
            aria-pressed={rating === option}
            title={`${option} estrelas`}
            onClick={() => setRating(option)}
          >
            <Star size={14} fill={rating !== null && option <= rating ? 'currentColor' : 'none'} aria-hidden="true" />
            <span>{option}</span>
          </button>
        ))}
      </div>
      <label className="personal-review-field">
        <span>Resenha pessoal</span>
        <textarea
          maxLength={4000}
          rows={4}
          value={review}
          onChange={(event) => setReview(event.target.value)}
        />
      </label>
      <div className="personal-review-footer">
        <span role="status">{status}</span>
        <button className="secondary-button" type="submit" disabled={isSaving || !hasChanges}>
          {isSaving ? 'Salvando...' : 'Salvar'}
        </button>
      </div>
    </form>
  )
}

function SteamAchievementsSection({
  achievements,
  gameTitle,
  isSteamGame,
  libraryLabel,
  isModalOpen,
  modalTriggerRef,
  searchTerm,
  steamEnrichmentStatus,
  onCloseModal,
  onOpenModal,
  onSearchTermChange,
}) {
  const sortedItems = useMemo(
    () => sortAchievementItems(achievements.items),
    [achievements.items],
  )
  const achievedPreviewItems = sortedItems
    .filter((achievement) => achievement.achieved === true)
    .slice(0, ACHIEVEMENT_PREVIEW_ACHIEVED_LIMIT)
  const lockedPreviewItems = sortedItems
    .filter((achievement) => achievement.achieved !== true)
    .slice(0, ACHIEVEMENT_PREVIEW_LOCKED_LIMIT)
  const previewItems = [...achievedPreviewItems, ...lockedPreviewItems]
  const remainingCount = Math.max(0, achievements.items.length - previewItems.length)
  const achievementLibraryTitle = `${libraryLabel || 'Biblioteca'} achievements`
  const achievementObservability = useMemo(
    () => isSteamGame
      ? buildAchievementObservability(achievements, steamEnrichmentStatus)
      : {
          tone: 'muted',
          text: 'Sem dados',
          detail: 'Sem dados de conquistas para este jogo.',
          expandable: false,
        },
    [achievements, isSteamGame, steamEnrichmentStatus],
  )

  return (
    <section className="achievement-panel" aria-label={`Conquistas ${libraryLabel || 'da biblioteca'}`}>
      <div className="achievement-panel-header">
        <div>
          <span>{achievementLibraryTitle}</span>
          <strong>{achievements.hasData ? `${achievements.unlocked}/${achievements.total}` : 'Sem dados'}</strong>
        </div>
        <div className="achievement-panel-status">
          {achievements.hasData ? (
            <span className="achievement-progress-value">{Math.round(achievements.percentage)}%</span>
          ) : null}
          <AchievementObservabilityChip status={achievementObservability} />
        </div>
      </div>
      {achievements.hasData ? (
        <>
          <div
            className="achievement-progress-bar"
            role="progressbar"
            aria-label="Progresso de conquistas Steam"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(achievements.percentage)}
            aria-valuetext={`${achievements.unlocked} de ${achievements.total} conquistas alcancadas`}
          >
            <span style={{ width: `${Math.min(100, Math.max(0, achievements.percentage))}%` }} />
          </div>
          <div className="achievement-preview-grid">
            <AchievementPreviewColumn
              emptyLabel="Nenhuma conquista alcancada ainda."
              items={achievedPreviewItems}
              label="Alcancadas"
            />
            <AchievementPreviewColumn
              emptyLabel="Sem conquistas pendentes no cache."
              items={lockedPreviewItems}
              label="Pendentes"
            />
          </div>
          <div className="achievement-panel-footer">
            <span>{remainingCount > 0 ? `+${remainingCount} conquistas no modal` : `${achievements.items.length} conquistas no total`}</span>
            <button
              ref={modalTriggerRef}
              className="secondary-button compact"
              type="button"
              aria-label={`Ver todas as ${achievements.items.length} conquistas Steam`}
              onClick={onOpenModal}
            >
              Ver todas
            </button>
          </div>
          {isModalOpen ? (
            <SteamAchievementsModal
              achievements={achievements}
              achievementLibraryTitle={achievementLibraryTitle}
              gameTitle={gameTitle}
              sourceItems={achievements.items}
              searchTerm={searchTerm}
              onClose={onCloseModal}
              onSearchTermChange={onSearchTermChange}
            />
          ) : null}
        </>
      ) : (
        <p className="achievement-empty-state">{achievementObservability.detail}</p>
      )}
    </section>
  )
}

function AchievementObservabilityChip({ status }) {
  const [isExpanded, setIsExpanded] = useState(false)

  if (!status) {
    return null
  }

  return (
    <div className="achievement-status-wrap" role="status" aria-live="polite">
      <span className="achievement-status-chip" data-tone={status.tone} title={status.detail}>
        <span className="achievement-status-dot" aria-hidden="true" />
        {status.text}
      </span>
      {status.expandable ? (
        <>
          <button
            className="achievement-status-toggle"
            type="button"
            aria-expanded={isExpanded}
            onClick={() => setIsExpanded((currentValue) => !currentValue)}
          >
            {isExpanded ? 'Ocultar' : 'Detalhes'}
          </button>
          {isExpanded ? <p className="achievement-status-detail">{status.detail}</p> : null}
        </>
      ) : null}
    </div>
  )
}

function AchievementPreviewColumn({ emptyLabel, items, label }) {
  return (
    <div className="achievement-preview-column">
      <span>{label}</span>
      {items.length > 0 ? (
        <div className="achievement-list preview">
          {items.map((achievement, index) => (
            <AchievementListItem
              achievement={achievement}
              isCompact
              key={getAchievementKey(achievement, index)}
            />
          ))}
        </div>
      ) : (
        <p className="achievement-empty-state">{emptyLabel}</p>
      )}
    </div>
  )
}

function SteamAchievementsModal({
  achievements,
  achievementLibraryTitle,
  gameTitle,
  sourceItems,
  searchTerm,
  onClose,
  onSearchTermChange,
}) {
  const searchInputRef = useRef(null)
  const [revealedSecretAchievements, setRevealedSecretAchievements] = useState(() => new Set())
  const orderedItems = useMemo(
    () => sortAchievementItems(sourceItems),
    [sourceItems],
  )
  const filteredItems = useMemo(
    () => filterAchievementItems(orderedItems, searchTerm, revealedSecretAchievements),
    [orderedItems, revealedSecretAchievements, searchTerm],
  )
  const handleRevealSecretAchievement = (apiName) => {
    setRevealedSecretAchievements((currentAchievements) => {
      const nextAchievements = new Set(currentAchievements)
      nextAchievements.add(apiName)
      return nextAchievements
    })
  }

  useEffect(() => {
    window.requestAnimationFrame(() => {
      searchInputRef.current?.focus()
    })
  }, [])

  useEffect(() => {
    const handleKeyDown = (event) => {
      if (event.key === 'Escape') {
        onClose()
      }
    }

    window.addEventListener('keydown', handleKeyDown)

    return () => {
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [onClose])

  const preventBackgroundScroll = (event) => {
    if (!event.target.closest('.achievement-list.modal-list')) {
      event.preventDefault()
    }
  }

  const modalContent = (
    <div
      className="modal-backdrop achievement-modal-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose()
        }
      }}
      onTouchMove={preventBackgroundScroll}
      onWheel={preventBackgroundScroll}
    >
      <section
        className="modal-panel achievement-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="steam-achievements-modal-title"
      >
        <header className="modal-header achievement-modal-header">
          <div>
            <span>{achievementLibraryTitle}</span>
            <h2 id="steam-achievements-modal-title">{gameTitle}</h2>
            <p>{achievements.unlocked}/{achievements.total} alcancadas</p>
          </div>
          <button className="icon-button" type="button" aria-label="Fechar conquistas" title="Fechar" onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>
        <div className="achievement-modal-toolbar">
          <div
            className="achievement-progress-bar"
            role="progressbar"
            aria-label="Progresso de conquistas Steam"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(achievements.percentage)}
            aria-valuetext={`${achievements.unlocked} de ${achievements.total} conquistas alcancadas`}
          >
            <span style={{ width: `${Math.min(100, Math.max(0, achievements.percentage))}%` }} />
          </div>
          <label className="achievement-search">
            <Search size={16} aria-hidden="true" />
            <span className="sr-only">Buscar conquistas</span>
            <input
              ref={searchInputRef}
              type="search"
              value={searchTerm}
              placeholder="Buscar conquistas"
              onChange={(event) => onSearchTermChange(event.target.value)}
            />
          </label>
          <span className="achievement-modal-count">
            {filteredItems.length} de {achievements.items.length}
          </span>
        </div>
        <div className="achievement-list modal-list">
          {filteredItems.length > 0 ? (
            filteredItems.map((achievement, index) => (
              <AchievementListItem
                achievement={achievement}
                key={getAchievementKey(achievement, index)}
                revealedSecretAchievements={revealedSecretAchievements}
                onRevealSecretAchievement={handleRevealSecretAchievement}
              />
            ))
          ) : (
            <p className="achievement-empty-state modal-empty-state">Nenhuma conquista encontrada.</p>
          )}
        </div>
      </section>
    </div>
  )

  return createPortal(modalContent, document.body)
}

function AchievementListItem({ achievement, isCompact = false, revealedSecretAchievements = new Set(), onRevealSecretAchievement }) {
  const { apiName, description, iconUrl, isAchieved, isHidden, name, shouldMask } = getAchievementDisplayState(
    achievement,
    revealedSecretAchievements,
  )
  const unlockLabel = isAchieved ? formatUnlockTime(achievement.unlockTime ?? achievement.unlock_time) : 'Bloqueada'

  const content = (
    <>
      <AchievementIcon iconUrl={iconUrl} isMasked={shouldMask} fallbackIcon={shouldMask ? LockKeyhole : Trophy} />
      <span className="achievement-copy">
        <strong>
          {name}
          {isHidden && !shouldMask ? <small className="achievement-secret-label">(secreta)</small> : null}
        </strong>
        <span>{description}</span>
        <small>{unlockLabel}</small>
      </span>
      {shouldMask && onRevealSecretAchievement ? (
        <span className="achievement-reveal-hint">
          <Eye size={14} aria-hidden="true" />
          Clique para mostrar
        </span>
      ) : null}
    </>
  )

  if (shouldMask && onRevealSecretAchievement) {
    return (
      <button
        className={isCompact ? 'achievement-item compact secret' : 'achievement-item secret'}
        type="button"
        aria-label="Revelar conquista secreta"
        title="Clique para mostrar o conteudo"
        onClick={() => onRevealSecretAchievement(apiName)}
      >
        {content}
      </button>
    )
  }

  return (
    <div className={isCompact ? `achievement-item compact ${isAchieved ? 'achieved' : 'locked'}` : `achievement-item ${isAchieved ? 'achieved' : 'locked'}`}>
      {content}
    </div>
  )
}

function AchievementIcon({ iconUrl, isMasked, fallbackIcon: FallbackIcon }) {
  const [hasImageError, setHasImageError] = useState(false)

  if (iconUrl && !hasImageError) {
    return (
      <span className={isMasked ? 'achievement-icon masked' : 'achievement-icon'}>
        <img src={iconUrl} alt="" loading="lazy" onError={() => setHasImageError(true)} />
      </span>
    )
  }

  return (
    <span className={isMasked ? 'achievement-icon masked' : 'achievement-icon'}>
      <FallbackIcon size={18} aria-hidden="true" />
    </span>
  )
}

function formatUnlockTime(unlockTime) {
  const timestamp = Number(unlockTime)

  if (!Number.isFinite(timestamp) || timestamp <= 0) {
    return 'Desbloqueada'
  }

  return new Intl.DateTimeFormat('pt-BR', {
    dateStyle: 'short',
  }).format(new Date(timestamp * 1000))
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
        <>
          <img className="artwork-backdrop" src={imageUrl} alt="" aria-hidden="true" />
          <img className="artwork-image" src={imageUrl} alt={imageAlt} onError={() => setImageIndex((currentIndex) => currentIndex + 1)} />
        </>
      ) : (
        <span className="artwork-fallback-text">{fallbackText}</span>
      )}
    </div>
  )
}

export default GameDetailsPanel
