import {
  Clock3,
  CircleDot,
  Download,
  FolderPlus,
  Gamepad2,
  HardDrive,
  Library,
  LayoutGrid,
  List,
  Play,
  Search,
  Settings,
  SlidersHorizontal,
  X,
} from 'lucide-react'
import { type FormEvent, useEffect, useMemo, useState } from 'react'
import './App.css'
import { libraryEntries, platformLabels } from './data/mockLibrary'
import type { InstallStatus, LibraryEntry, LaunchAction } from './domain'
import { addPersistedManualGame, launchLibraryEntry, listPersistedManualGames } from './services/libraryApi'

type ViewMode = 'list' | 'grid'
type QuickFilter = 'all' | 'installed' | 'steam' | 'local'

type ManualGameFormState = {
  title: string
  genre: string
  installStatus: InstallStatus
  launchTarget: string
}

const emptyManualGameForm: ManualGameFormState = {
  title: '',
  genre: '',
  installStatus: 'not_installed',
  launchTarget: '',
}

const getPlaytimeHours = (minutes: number) => Math.floor(minutes / 60)

const quickFilterLabels: Record<QuickFilter, string> = {
  all: 'Todos',
  installed: 'Instalados',
  steam: 'Steam',
  local: 'Locais',
}

const createSlug = (value: string) =>
  value
    .trim()
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'jogo-manual'

const getDeterministicAccentColor = (value: string) => {
  const palette = ['#0d9488', '#2563eb', '#7c3aed', '#be123c', '#c2410c', '#15803d', '#9333ea', '#b45309']
  const hash = [...value].reduce((total, char) => total + char.charCodeAt(0), 0)

  return palette[hash % palette.length]
}

const getLaunchActionKind = (target: string): LaunchAction['kind'] => {
  if (!target) {
    return 'manual'
  }

  return target.includes('://') ? 'uri' : 'executable'
}

const createManualLibraryEntry = (form: ManualGameFormState): LibraryEntry => {
  const title = form.title.trim()
  const genre = form.genre.trim()
  const launchTarget = form.launchTarget.trim()
  const slug = createSlug(title)
  const timestamp = new Date().toISOString()
  const launchAction: LaunchAction = {
    id: `launch-manual-${slug}`,
    platformId: 'manual',
    kind: getLaunchActionKind(launchTarget),
    label: launchTarget || 'Sem acao configurada',
    target: launchTarget,
    isPrimary: true,
  }

  return {
    id: `entry-manual-${slug}-${Date.now()}`,
    primaryPlatformId: 'manual',
    installStatus: form.installStatus,
    lastPlayedLabel: 'Nunca',
    addedAt: timestamp,
    updatedAt: timestamp,
    game: {
      internalId: `game-manual-${slug}`,
      title,
      sortTitle: title,
      platforms: ['manual'],
      sources: [{ platformId: 'manual', externalId: `manual-${slug}` }],
      installed: form.installStatus === 'installed',
      installLocations: [],
      launchActions: [launchAction],
      playtime: { totalMinutes: 0 },
      artwork: { accentColor: getDeterministicAccentColor(title) },
      genres: genre ? [genre] : ['Sem genero'],
      tags: [],
      userOverrides: {},
    },
  }
}

function App() {
  const [viewMode, setViewMode] = useState<ViewMode>('grid')
  const [entries, setEntries] = useState<LibraryEntry[]>(libraryEntries)
  const [selectedEntryId, setSelectedEntryId] = useState(libraryEntries[0]?.id ?? '')
  const [searchTerm, setSearchTerm] = useState('')
  const [quickFilter, setQuickFilter] = useState<QuickFilter>('all')
  const [launchMessage, setLaunchMessage] = useState('')
  const [isManualModalOpen, setIsManualModalOpen] = useState(false)
  const [manualGameForm, setManualGameForm] = useState<ManualGameFormState>(emptyManualGameForm)
  const [manualGameError, setManualGameError] = useState('')
  const selectedEntry = entries.find((entry) => entry.id === selectedEntryId) ?? entries[0]

  useEffect(() => {
    let isMounted = true

    listPersistedManualGames()
      .then((persistedManualEntries) => {
        if (!isMounted || persistedManualEntries.length === 0) {
          return
        }

        setEntries((currentEntries) => {
          const persistedIds = new Set(persistedManualEntries.map((entry) => entry.id))
          const nextEntries = [
            ...persistedManualEntries,
            ...currentEntries.filter((entry) => !persistedIds.has(entry.id)),
          ]

          setSelectedEntryId((currentSelectedEntryId) =>
            nextEntries.some((entry) => entry.id === currentSelectedEntryId)
              ? currentSelectedEntryId
              : nextEntries[0]?.id ?? '',
          )

          return nextEntries
        })
      })
      .catch(() => {
        setLaunchMessage('Persistencia local indisponivel. Usando biblioteca em memoria nesta sessao.')
      })

    return () => {
      isMounted = false
    }
  }, [])
  const filteredEntries = useMemo(() => {
    const normalizedSearch = searchTerm.trim().toLowerCase()

    return entries.filter((entry) => {
      const matchesSearch = normalizedSearch
        ? [
            entry.game.title,
            platformLabels[entry.primaryPlatformId],
            entry.game.genres.join(' '),
            entry.installStatus === 'installed' ? 'instalado' : 'nao instalado',
          ].some((value) => value.toLowerCase().includes(normalizedSearch))
        : true

      const matchesQuickFilter =
        quickFilter === 'all' ||
        (quickFilter === 'installed' && entry.installStatus === 'installed') ||
        quickFilter === entry.primaryPlatformId

      return matchesSearch && matchesQuickFilter
    })
  }, [entries, quickFilter, searchTerm])
  const installedCount = entries.filter((entry) => entry.installStatus === 'installed').length
  const totalHours = entries.reduce((sum, entry) => sum + getPlaytimeHours(entry.game.playtime.totalMinutes), 0)

  const closeManualModal = () => {
    setIsManualModalOpen(false)
    setManualGameForm(emptyManualGameForm)
    setManualGameError('')
  }

  const handleManualGameSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    if (!manualGameForm.title.trim()) {
      setManualGameError('Informe o titulo do jogo.')
      return
    }

    const persistedEntry = await addPersistedManualGame({
      title: manualGameForm.title,
      genre: manualGameForm.genre,
      installStatus: manualGameForm.installStatus,
      launchTarget: manualGameForm.launchTarget,
    }).catch(() => {
      setLaunchMessage('Nao foi possivel salvar no banco local. O jogo foi mantido apenas em memoria.')
      return null
    })
    const newEntry = persistedEntry ?? createManualLibraryEntry(manualGameForm)

    setEntries((currentEntries) => [newEntry, ...currentEntries])
    setSelectedEntryId(newEntry.id)
    setLaunchMessage('')
    closeManualModal()
  }

  const handleSelectEntry = (entryId: string) => {
    setSelectedEntryId(entryId)
    setLaunchMessage('')
  }

  const handleLaunchSelectedEntry = async () => {
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

    if (selectedEntry.primaryPlatformId !== 'manual') {
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

  const handleInstallAction = () => {
    setLaunchMessage(`Instalacao/localizacao de arquivos ainda sera implementada para ${selectedEntry.game.title}.`)
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
            <button className="primary-button" type="button" onClick={() => setIsManualModalOpen(true)}>
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
              {(Object.keys(quickFilterLabels) as QuickFilter[]).map((filter) => (
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

            {viewMode === 'list' ? (
              <div className="game-table">
                {filteredEntries.map((entry) => (
                  <article
                    className={selectedEntry.id === entry.id ? 'game-row selected' : 'game-row'}
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
                    className={selectedEntry.id === entry.id ? 'game-cover-card selected' : 'game-cover-card'}
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
            {filteredEntries.length === 0 ? (
              <div className="empty-state">
                <strong>Nenhum jogo encontrado</strong>
                <span>Ajuste a busca ou troque o filtro ativo.</span>
              </div>
            ) : null}
          </section>

          <aside className="details-panel" aria-label="Detalhes do jogo selecionado">
            <div className="detail-cover" style={{ background: selectedEntry.game.artwork.accentColor }}>
              <span>{selectedEntry.game.title}</span>
            </div>
            <div className="detail-content">
              <span className="platform-label">{platformLabels[selectedEntry.primaryPlatformId]}</span>
              <h2>{selectedEntry.game.title}</h2>
              <div className="detail-actions">
                <button className="play-button" type="button" onClick={handleLaunchSelectedEntry}>
                  <Play size={18} fill="currentColor" aria-hidden="true" />
                  Jogar
                </button>
                <button className="icon-button" type="button" aria-label="Instalar ou localizar arquivos" title="Instalar ou localizar arquivos" onClick={handleInstallAction}>
                  <Download size={18} aria-hidden="true" />
                </button>
              </div>
              <dl className="detail-list">
                <div>
                  <dt>Status</dt>
                  <dd>{selectedEntry.installStatus === 'installed' ? 'Instalado' : 'Nao instalado'}</dd>
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
          </aside>
        </div>
      </section>

      {isManualModalOpen ? (
        <div className="modal-backdrop" role="presentation">
          <section className="modal-panel" role="dialog" aria-modal="true" aria-labelledby="manual-game-title">
            <header className="modal-header">
              <div>
                <span>Cadastro manual</span>
                <h2 id="manual-game-title">Adicionar jogo</h2>
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
                  Salvar
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
