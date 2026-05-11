import {
  Archive,
  Clock3,
  CircleDot,
  Download,
  FolderPlus,
  Gamepad2,
  HardDrive,
  Library,
  LayoutGrid,
  List,
  Pencil,
  Play,
  RefreshCw,
  Search,
  Settings,
  SlidersHorizontal,
  X,
} from 'lucide-react'
import { useDeferredValue, useEffect, useMemo, useState } from 'react'
import './App.css'
import { platformLabels } from './data/mockLibrary'
import {
  addPersistedManualGame,
  launchLibraryEntry,
  listLibraryEntries,
  syncLocalGames,
  updatePersistedManualGame,
  setLibraryEntryArchived,
} from './services/libraryApi'
import { listen } from '@tauri-apps/api/event'

const LIBRARY_BOOTSTRAP_COMPLETE_EVENT = 'library-bootstrap-complete'

const hasTauriRuntime = () =>
  typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

const emptyManualGameForm = {
  title: '',
  genre: '',
  installStatus: 'not_installed',
  launchTarget: '',
}

const getPlaytimeHours = (minutes) => Math.floor(minutes / 60)

const quickFilterLabels = {
  all: 'Todos',
  installed: 'Instalados',
  steam: 'Steam',
  local: 'Locais',
}

const createSlug = (value) =>
  value
    .trim()
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'jogo-manual'

const getDeterministicAccentColor = (value) => {
  const palette = ['#0d9488', '#2563eb', '#7c3aed', '#be123c', '#c2410c', '#15803d', '#9333ea', '#b45309']
  const hash = [...value].reduce((total, char) => total + char.charCodeAt(0), 0)

  return palette[hash % palette.length]
}

const getLaunchActionKind = (target) => {
  if (!target) {
    return 'manual'
  }

  return target.includes('://') ? 'uri' : 'executable'
}

const getPrimaryLaunchAction = (entry) =>
  entry?.game.launchActions.find((action) => action.isPrimary) ?? entry?.game.launchActions[0] ?? null

const getManualGameFormFromEntry = (entry) => ({
  title: entry?.game.title ?? '',
  genre: entry?.game.genres?.[0] ?? '',
  installStatus: entry?.installStatus ?? 'not_installed',
  launchTarget: getPrimaryLaunchAction(entry)?.target ?? '',
})

const buildManualLibraryEntry = (form, existingEntry = null) => {
  const title = form.title.trim()
  const genre = form.genre.trim()
  const launchTarget = form.launchTarget.trim()
  const slug = createSlug(title)
  const timestamp = new Date().toISOString()
  const existingLaunchAction = getPrimaryLaunchAction(existingEntry)
  const launchAction = {
    id: existingLaunchAction?.id ?? `launch-manual-${slug}`,
    platformId: existingLaunchAction?.platformId ?? 'manual',
    kind: getLaunchActionKind(launchTarget),
    label: launchTarget || 'Sem acao configurada',
    target: launchTarget,
    isPrimary: true,
  }

  return {
    id: existingEntry?.id ?? `entry-manual-${slug}-${Date.now()}`,
    primaryPlatformId: 'manual',
    installStatus: form.installStatus,
    lastPlayedLabel: existingEntry?.lastPlayedLabel ?? 'Nunca',
    addedAt: existingEntry?.addedAt ?? timestamp,
    updatedAt: timestamp,
    game: {
      internalId: existingEntry?.game.internalId ?? `game-manual-${slug}`,
      title,
      sortTitle: title,
      platforms: existingEntry?.game.platforms ?? ['manual'],
      sources:
        existingEntry?.game.sources ?? [{ platformId: 'manual', externalId: `manual-${slug}` }],
      installed: form.installStatus === 'installed',
      installLocations: existingEntry?.game.installLocations ?? [],
      launchActions: [launchAction],
      playtime: existingEntry?.game.playtime ?? { totalMinutes: 0 },
      artwork: { accentColor: getDeterministicAccentColor(title) },
      genres: genre ? [genre] : ['Sem genero'],
      tags: existingEntry?.game.tags ?? [],
      userOverrides: existingEntry?.game.userOverrides ?? {},
    },
    isArchived: existingEntry?.isArchived ?? false,
  }
}

const getSelectedEntryIdForEntries = (nextEntries, currentSelectedEntryId) =>
  nextEntries.some((entry) => entry.id === currentSelectedEntryId)
    ? currentSelectedEntryId
    : nextEntries[0]?.id ?? ''

function App() {
  const [viewMode, setViewMode] = useState('grid')
  const [entries, setEntries] = useState([])
  const [selectedEntryId, setSelectedEntryId] = useState('')
  const [searchTerm, setSearchTerm] = useState('')
  const [quickFilter, setQuickFilter] = useState('all')
  const [launchMessage, setLaunchMessage] = useState('')
  const [isLibraryLoading, setIsLibraryLoading] = useState(true)
  const [isBootstrapping, setIsBootstrapping] = useState(true)
  const [isLocalSyncing, setIsLocalSyncing] = useState(false)
  const [isManualModalOpen, setIsManualModalOpen] = useState(false)
  const [editingEntryId, setEditingEntryId] = useState('')
  const [manualGameForm, setManualGameForm] = useState(emptyManualGameForm)
  const [manualGameError, setManualGameError] = useState('')
  const deferredSearchTerm = useDeferredValue(searchTerm)
  const selectedEntry = entries.find((entry) => entry.id === selectedEntryId) ?? entries[0] ?? null
  const isEditingManualGame = editingEntryId !== ''
  const showLibraryLoading = isLibraryLoading || isBootstrapping

  useEffect(() => {
    let isMounted = true
    let unlistenBootstrapComplete = null
    let bootstrapTimeoutId = hasTauriRuntime()
      ? window.setTimeout(() => {
          if (isMounted) {
            void syncLibraryEntries().finally(() => {
              if (isMounted) {
                setIsBootstrapping(false)
              }
            })
          }
        }, 4000)
      : null

    const syncLibraryEntries = async () => {
      const libraryEntries = await listLibraryEntries().catch(() => null)

      if (!isMounted || !libraryEntries) {
        return
      }

      setEntries(libraryEntries)
      setSelectedEntryId((currentSelectedEntryId) =>
        getSelectedEntryIdForEntries(libraryEntries, currentSelectedEntryId),
      )

      if (!hasTauriRuntime() || libraryEntries.length > 0) {
        setIsBootstrapping(false)
      }
    }

    const registerBootstrapListener = async () => {
      if (!hasTauriRuntime()) {
        if (isMounted) {
          setIsBootstrapping(false)
        }
        return
      }

      try {
        unlistenBootstrapComplete = await listen(LIBRARY_BOOTSTRAP_COMPLETE_EVENT, async () => {
          if (!isMounted) {
            return
          }

          setIsBootstrapping(false)
          if (bootstrapTimeoutId !== null) {
            clearTimeout(bootstrapTimeoutId)
            bootstrapTimeoutId = null
          }
          await syncLibraryEntries()
        })
      } catch {
        if (isMounted) {
          setIsBootstrapping(false)
        }
      }
    }

    void registerBootstrapListener()
    const initialSyncPromise = syncLibraryEntries()
    initialSyncPromise
      .catch(() => {
        if (isMounted) {
          setLaunchMessage('Nao foi possivel carregar a biblioteca local.')
        }
      })
      .finally(() => {
        if (isMounted) {
          setIsLibraryLoading(false)
        }
      })

    return () => {
      isMounted = false
      if (bootstrapTimeoutId !== null) {
        clearTimeout(bootstrapTimeoutId)
      }
      if (unlistenBootstrapComplete) {
        void unlistenBootstrapComplete()
      }
    }
  }, [])
  const filteredEntries = useMemo(() => {
    const normalizedSearch = deferredSearchTerm.trim().toLowerCase()

    return entries.filter((entry) => {
      const matchesSearch = normalizedSearch
        ? [
            entry.game.title,
            platformLabels[entry.primaryPlatformId] ?? entry.primaryPlatformId,
            entry.game.genres?.join(' ') ?? '',
            entry.installStatus === 'installed' ? 'instalado' : 'nao instalado',
          ].some((value) => value.toLowerCase().includes(normalizedSearch))
        : true

      const matchesQuickFilter =
        quickFilter === 'all' ||
        (quickFilter === 'installed' && entry.installStatus === 'installed') ||
        quickFilter === entry.primaryPlatformId

      return matchesSearch && matchesQuickFilter
    })
  }, [deferredSearchTerm, entries, quickFilter])
  const installedCount = entries.filter((entry) => entry.installStatus === 'installed').length
  const totalHours = entries.reduce((sum, entry) => sum + getPlaytimeHours(entry.game.playtime.totalMinutes), 0)

  const closeManualModal = () => {
    setIsManualModalOpen(false)
    setEditingEntryId('')
    setManualGameForm(emptyManualGameForm)
    setManualGameError('')
  }

  const openManualGameModal = () => {
    setEditingEntryId('')
    setManualGameForm(emptyManualGameForm)
    setManualGameError('')
    setIsManualModalOpen(true)
  }

  const openManualGameEditor = (entry) => {
    setEditingEntryId(entry.id)
    setManualGameForm(getManualGameFormFromEntry(entry))
    setManualGameError('')
    setIsManualModalOpen(true)
  }

  const handleManualGameSubmit = async (event) => {
    event.preventDefault()

    if (!manualGameForm.title.trim()) {
      setManualGameError('Informe o titulo do jogo.')
      return
    }

    const input = {
      title: manualGameForm.title,
      genre: manualGameForm.genre,
      installStatus: manualGameForm.installStatus,
      launchTarget: manualGameForm.launchTarget,
    }

    try {
      if (isEditingManualGame) {
        const baseEntry = entries.find((entry) => entry.id === editingEntryId) ?? selectedEntry
        const persistedEntry = await updatePersistedManualGame(editingEntryId, input)
        const updatedEntry = persistedEntry ?? buildManualLibraryEntry(manualGameForm, baseEntry)

        setEntries((currentEntries) =>
          currentEntries.map((entry) => (entry.id === updatedEntry.id ? updatedEntry : entry)),
        )
        setSelectedEntryId(updatedEntry.id)
        setLaunchMessage('Jogo atualizado.')
        closeManualModal()
        return
      }

      const persistedEntry = await addPersistedManualGame(input)
      const newEntry = persistedEntry ?? buildManualLibraryEntry(manualGameForm)

      setEntries((currentEntries) => [newEntry, ...currentEntries])
      setSelectedEntryId(newEntry.id)
      setLaunchMessage('Jogo adicionado.')
      closeManualModal()
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setLaunchMessage(`Nao foi possivel salvar as alteracoes: ${message}`)
    }
  }

  const handleSelectEntry = (entryId) => {
    setSelectedEntryId(entryId)
    setLaunchMessage('')
  }

  const handleLaunchSelectedEntry = async () => {
    if (!selectedEntry) {
      setLaunchMessage('Nenhum jogo selecionado.')
      return
    }

    const primaryAction = selectedEntry.game.launchActions.find((action) => action.isPrimary) ?? selectedEntry.game.launchActions[0]

    if (!primaryAction || primaryAction.kind === 'manual' || !primaryAction.target) {
      setLaunchMessage(`Nenhuma acao de lancamento configurada para ${selectedEntry.game.title}.`)
      return
    }

    setLaunchMessage(`Tentando iniciar ${selectedEntry.game.title} por ${primaryAction.label}.`)

    if (primaryAction.kind === 'uri') {
      window.location.href = primaryAction.target
      return
    }

    if (selectedEntry.primaryPlatformId !== 'manual' && selectedEntry.primaryPlatformId !== 'local') {
      setLaunchMessage(`Execucao de executaveis para jogos importados sera ligada ao provider correspondente. Acao configurada: ${primaryAction.label}.`)
      return
    }

    const result = await launchLibraryEntry(selectedEntry.id).catch((error) => {
      const message = error instanceof Error ? error.message : String(error)
      return { started: false, message }
    })

    if (!result) {
      setLaunchMessage(`Execucao de executaveis locais esta disponivel apenas no aplicativo Tauri. Acao configurada: ${primaryAction.label}.`)
      return
    }

    setLaunchMessage(result.message)
  }

  const handleSyncLocalGames = async () => {
    if (isLocalSyncing) {
      return
    }

    setIsLocalSyncing(true)
    setLaunchMessage('Sincronizando jogos locais...')

    try {
      const summary = await syncLocalGames()

      if (!summary) {
        setLaunchMessage('Sincronizacao local disponivel apenas no aplicativo Tauri.')
        return
      }

      const refreshedEntries = await listLibraryEntries().catch(() => null)
      if (refreshedEntries) {
        setEntries(refreshedEntries)
        setSelectedEntryId((currentSelectedEntryId) =>
          getSelectedEntryIdForEntries(refreshedEntries, currentSelectedEntryId),
        )
      }

      setLaunchMessage(
        `Sincronizacao local concluida: ${summary.inserted} novos e ${summary.updated} atualizados em ${summary.discovered} itens encontrados.`,
      )
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setLaunchMessage(`Nao foi possivel sincronizar jogos locais: ${message}`)
    } finally {
      setIsLocalSyncing(false)
    }
  }

  const handleInstallAction = () => {
    if (!selectedEntry) {
      setLaunchMessage('Nenhum jogo selecionado.')
      return
    }

    setLaunchMessage(`Instalacao/localizacao de arquivos ainda sera implementada para ${selectedEntry.game.title}.`)
  }

  const handleArchiveSelectedEntry = async () => {
    if (!selectedEntry) {
      setLaunchMessage('Nenhum jogo selecionado.')
      return
    }

    const nextArchivedState = !selectedEntry.isArchived
    const result = await setLibraryEntryArchived(selectedEntry.id, nextArchivedState).catch(() => null)

    if (result === null) {
      setEntries((currentEntries) => {
        const nextEntries = currentEntries.filter((entry) => entry.id !== selectedEntry.id)
        setSelectedEntryId((currentSelectedEntryId) =>
          getSelectedEntryIdForEntries(nextEntries, currentSelectedEntryId),
        )
        return nextEntries
      })
      setLaunchMessage(nextArchivedState ? 'Jogo arquivado nesta sessao.' : 'Jogo reativado nesta sessao.')
      return
    }

    const refreshedEntries = await listLibraryEntries().catch(() => null)
    if (refreshedEntries) {
      setEntries(refreshedEntries)
      setSelectedEntryId((currentSelectedEntryId) =>
        getSelectedEntryIdForEntries(refreshedEntries, currentSelectedEntryId),
      )
    }

    setLaunchMessage(nextArchivedState ? 'Jogo arquivado.' : 'Jogo reativado.')
  }

  const handleEditSelectedEntry = () => {
    if (!selectedEntry || selectedEntry.primaryPlatformId !== 'manual') {
      return
    }

    openManualGameEditor(selectedEntry)
  }

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="Navegacao principal">
        <div className="brand">
          <div className="brand-mark">
            <Gamepad2 size={22} aria-hidden="true" />
          </div>
          <div>
            <strong>Biblioteca</strong>
            <span>Jogos Unificados</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Filtros da biblioteca">
          <button
            className={quickFilter === 'all' ? 'nav-item active' : 'nav-item'}
            type="button"
            onClick={() => {
              setQuickFilter('all')
              setSearchTerm('')
              setLaunchMessage('')
            }}
          >
            <Library size={18} aria-hidden="true" />
            Biblioteca
          </button>
          <button
            className={quickFilter === 'steam' ? 'nav-item active' : 'nav-item'}
            type="button"
            onClick={() => {
              setQuickFilter('steam')
              setSearchTerm('')
              setLaunchMessage('')
            }}
          >
            <CircleDot size={18} aria-hidden="true" />
            Steam
          </button>
          <button
            className={quickFilter === 'local' ? 'nav-item active' : 'nav-item'}
            type="button"
            onClick={() => {
              setQuickFilter('local')
              setSearchTerm('')
              setLaunchMessage('')
            }}
          >
            <HardDrive size={18} aria-hidden="true" />
            Locais
          </button>
          <button
            className="nav-item"
            type="button"
            onClick={() => setLaunchMessage('Gerenciamento de contas sera implementado na fase de integracoes.')}
          >
            <Settings size={18} aria-hidden="true" />
            Contas
          </button>
        </nav>

        <div className="sync-panel">
          <span>Ultima sincronizacao</span>
          <strong>Steam pendente</strong>
        </div>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <h1>Biblioteca de jogos</h1>
            <p>{entries.length} jogos catalogados para o MVP inicial</p>
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
              onClick={handleSyncLocalGames}
              disabled={isLocalSyncing}
            >
              <RefreshCw size={18} aria-hidden="true" className={isLocalSyncing ? 'spin-icon' : ''} />
            </button>
            <button className="primary-button" type="button" onClick={openManualGameModal}>
              <FolderPlus size={18} aria-hidden="true" />
              Adicionar jogo
            </button>
          </div>
        </header>

        <section className="stats-grid" aria-label="Resumo da biblioteca">
          <div className="metric">
            <span>Total</span>
            <strong>{entries.length}</strong>
          </div>
          <div className="metric">
            <span>Instalados</span>
            <strong>{installedCount}</strong>
          </div>
          <div className="metric">
            <span>Horas jogadas</span>
            <strong>{totalHours}h</strong>
          </div>
        </section>

        <div className="library-layout">
          <section className="library-list" aria-label="Lista de jogos">
            <label className="search-box">
              <Search size={18} aria-hidden="true" />
              <input
                type="search"
                placeholder="Buscar por nome, plataforma ou genero"
                value={searchTerm}
                onChange={(event) => setSearchTerm(event.target.value)}
              />
            </label>

            <div className="filter-row" aria-label="Filtros rapidos">
              {Object.keys(quickFilterLabels).map((filter) => (
                <button
                  className={quickFilter === filter ? 'filter-chip active' : 'filter-chip'}
                  type="button"
                  key={filter}
                  onClick={() => setQuickFilter(filter)}
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
                  onClick={() => setViewMode('list')}
                >
                  <List size={17} aria-hidden="true" />
                </button>
                <button
                  className={viewMode === 'grid' ? 'active' : ''}
                  type="button"
                  aria-label="Mostrar capas dos jogos"
                  title="Capas"
                  onClick={() => setViewMode('grid')}
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
                  <article
                    className={selectedEntry?.id === entry.id ? 'game-row selected' : 'game-row'}
                    key={entry.id}
                    role="button"
                    tabIndex={0}
                    onClick={() => handleSelectEntry(entry.id)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault()
                        handleSelectEntry(entry.id)
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
                ))}
              </div>
            ) : (
              <div className="game-cover-grid">
                {filteredEntries.map((entry) => (
                  <article
                    className={selectedEntry?.id === entry.id ? 'game-cover-card selected' : 'game-cover-card'}
                    key={entry.id}
                    role="button"
                    tabIndex={0}
                    onClick={() => handleSelectEntry(entry.id)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault()
                        handleSelectEntry(entry.id)
                      }
                    }}
                  >
                    <div className="poster" style={{ background: entry.game.artwork.accentColor }}>
                      <span>{entry.game.title}</span>
                    </div>
                    <strong>{entry.game.title}</strong>
                    <span>{platformLabels[entry.primaryPlatformId]}</span>
                  </article>
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

          <aside className="details-panel" aria-label="Detalhes do jogo selecionado">
            {selectedEntry ? (
              <>
                <div className="detail-cover" style={{ background: selectedEntry.game.artwork.accentColor }}>
                  <span>{selectedEntry.game.title}</span>
                </div>
                <div className="detail-content">
                  <span className="platform-label">{platformLabels[selectedEntry.primaryPlatformId] ?? selectedEntry.primaryPlatformId}</span>
                  <h2>{selectedEntry.game.title}</h2>
                  <div className="detail-actions">
                    <button className="play-button" type="button" onClick={handleLaunchSelectedEntry}>
                      <Play size={18} fill="currentColor" aria-hidden="true" />
                      Jogar
                    </button>
                    <button className="icon-button" type="button" aria-label="Instalar ou localizar arquivos" title="Instalar ou localizar arquivos" onClick={handleInstallAction}>
                      <Download size={18} aria-hidden="true" />
                    </button>
                    {selectedEntry.primaryPlatformId === 'manual' ? (
                      <button
                        className="icon-button"
                        type="button"
                        aria-label="Editar jogo"
                        title="Editar jogo"
                        onClick={handleEditSelectedEntry}
                      >
                        <Pencil size={18} aria-hidden="true" />
                      </button>
                    ) : null}
                    <button
                      className="icon-button"
                      type="button"
                      aria-label={selectedEntry.isArchived ? 'Reativar jogo' : 'Arquivar jogo'}
                      title={selectedEntry.isArchived ? 'Reativar jogo' : 'Arquivar jogo'}
                      onClick={handleArchiveSelectedEntry}
                    >
                      <Archive size={18} aria-hidden="true" />
                    </button>
                  </div>
                  <dl className="detail-list">
                    <div>
                      <dt>Status</dt>
                      <dd>{selectedEntry.installStatus === 'installed' ? 'Instalado' : 'Nao instalado'}</dd>
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
        </div>
      </section>

      {isManualModalOpen ? (
        <div className="modal-backdrop" role="presentation">
          <section className="modal-panel" role="dialog" aria-modal="true" aria-labelledby="manual-game-title">
            <header className="modal-header">
              <div>
                <span>Cadastro manual</span>
                <h2 id="manual-game-title">{isEditingManualGame ? 'Editar jogo' : 'Adicionar jogo'}</h2>
              </div>
              <button className="icon-button" type="button" aria-label="Fechar cadastro" title="Fechar" onClick={closeManualModal}>
                <X size={18} aria-hidden="true" />
              </button>
            </header>

            <form className="manual-game-form" onSubmit={handleManualGameSubmit}>
              <label>
                <span>Titulo</span>
                <input
                  type="text"
                  value={manualGameForm.title}
                  onChange={(event) => {
                    setManualGameForm((currentForm) => ({ ...currentForm, title: event.target.value }))
                    setManualGameError('')
                  }}
                  autoFocus
                />
              </label>

              <label>
                <span>Genero</span>
                <input
                  type="text"
                  value={manualGameForm.genre}
                  onChange={(event) => setManualGameForm((currentForm) => ({ ...currentForm, genre: event.target.value }))}
                />
              </label>

              <fieldset>
                <legend>Status</legend>
                <div className="segmented-control">
                  <label>
                    <input
                      type="radio"
                      name="installStatus"
                      value="installed"
                      checked={manualGameForm.installStatus === 'installed'}
                      onChange={() => setManualGameForm((currentForm) => ({ ...currentForm, installStatus: 'installed' }))}
                    />
                    Instalado
                  </label>
                  <label>
                    <input
                      type="radio"
                      name="installStatus"
                      value="not_installed"
                      checked={manualGameForm.installStatus === 'not_installed'}
                      onChange={() => setManualGameForm((currentForm) => ({ ...currentForm, installStatus: 'not_installed' }))}
                    />
                    Nao instalado
                  </label>
                </div>
              </fieldset>

              <label>
                <span>Acao de lancamento</span>
                <input
                  type="text"
                  value={manualGameForm.launchTarget}
                  onChange={(event) => setManualGameForm((currentForm) => ({ ...currentForm, launchTarget: event.target.value }))}
                />
              </label>

              {manualGameError ? <p className="form-error">{manualGameError}</p> : null}

              <div className="modal-actions">
                <button className="secondary-button" type="button" onClick={closeManualModal}>
                  Cancelar
                </button>
                <button className="primary-button" type="submit">
                  {isEditingManualGame ? 'Salvar alterações' : 'Salvar'}
                </button>
              </div>
            </form>
          </section>
        </div>
      ) : null}
    </main>
  )
}

export default App
