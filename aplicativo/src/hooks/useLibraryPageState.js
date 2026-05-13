import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useState } from 'react'
import {
  addPersistedManualGame,
  launchLibraryEntry,
  listLibraryEntries,
  setLibraryEntryArchived,
  syncLocalGames,
  syncSteamGames,
  updatePersistedManualGame,
} from '../services/libraryService'
import {
  buildManualLibraryEntry,
  emptyManualGameForm,
  getManualGameFormFromEntry,
  getSelectedEntryIdForEntries,
  validateManualGameInput,
} from '../adapters/libraryEntryAdapter'
import { useLibraryFiltering } from './useLibraryFiltering'

const LIBRARY_BOOTSTRAP_COMPLETE_EVENT = 'library-bootstrap-complete'

const hasTauriRuntime = () =>
  typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

export function useLibraryPageState() {
  const [viewMode, setViewMode] = useState('grid')
  const [entries, setEntries] = useState([])
  const [selectedEntryId, setSelectedEntryId] = useState('')
  const [searchTerm, setSearchTerm] = useState('')
  const [quickFilter, setQuickFilter] = useState('all')
  const [launchMessage, setLaunchMessage] = useState('')
  const [isLibraryLoading, setIsLibraryLoading] = useState(true)
  const [isBootstrapping, setIsBootstrapping] = useState(true)
  const [isLocalSyncing, setIsLocalSyncing] = useState(false)
  const [isSteamSyncing, setIsSteamSyncing] = useState(false)
  const [isManualModalOpen, setIsManualModalOpen] = useState(false)
  const [editingEntryId, setEditingEntryId] = useState('')
  const [manualGameForm, setManualGameForm] = useState(emptyManualGameForm)
  const [manualGameErrors, setManualGameErrors] = useState({})
  const selectedEntry = entries.find((entry) => entry.id === selectedEntryId) ?? entries[0] ?? null
  const isEditingManualGame = editingEntryId !== ''
  const showLibraryLoading = isLibraryLoading || isBootstrapping
  const { filteredEntries, installedCount, totalHours } = useLibraryFiltering(entries, searchTerm, quickFilter)

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
    syncLibraryEntries()
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

  const closeManualModal = useCallback(() => {
    setIsManualModalOpen(false)
    setEditingEntryId('')
    setManualGameForm(emptyManualGameForm)
    setManualGameErrors({})
  }, [])

  const openManualGameModal = useCallback(() => {
    setEditingEntryId('')
    setManualGameForm(emptyManualGameForm)
    setManualGameErrors({})
    setIsManualModalOpen(true)
  }, [])

  const openManualGameEditor = useCallback((entry) => {
    setEditingEntryId(entry.id)
    setManualGameForm(getManualGameFormFromEntry(entry))
    setManualGameErrors({})
    setIsManualModalOpen(true)
  }, [])

  const refreshEntries = async () => {
    const refreshedEntries = await listLibraryEntries().catch(() => null)

    if (refreshedEntries) {
      setEntries(refreshedEntries)
      setSelectedEntryId((currentSelectedEntryId) =>
        getSelectedEntryIdForEntries(refreshedEntries, currentSelectedEntryId),
      )
    }
  }

  const handleManualGameSubmit = async (event) => {
    event.preventDefault()

    const validation = validateManualGameInput(manualGameForm)

    if (!validation.isValid) {
      setManualGameErrors(validation.errors)
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

  const handleSelectEntry = useCallback((entryId) => {
    setSelectedEntryId(entryId)
    setLaunchMessage('')
  }, [])

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

      await refreshEntries()
      setLaunchMessage(
        `Sincronizacao local concluida: ${summary.inserted} novos, ${summary.updated} atualizados e ${summary.archived ?? 0} arquivados em ${summary.discovered} itens encontrados.`,
      )
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setLaunchMessage(`Nao foi possivel sincronizar jogos locais: ${message}`)
    } finally {
      setIsLocalSyncing(false)
    }
  }

  const handleSyncSteamGames = async () => {
    if (isSteamSyncing) {
      return
    }

    setIsSteamSyncing(true)
    setLaunchMessage('Sincronizando biblioteca Steam instalada...')

    try {
      const summary = await syncSteamGames()

      if (!summary) {
        setLaunchMessage('Sincronizacao Steam disponivel apenas no aplicativo Tauri.')
        return
      }

      await refreshEntries()
      setLaunchMessage(
        `Sincronizacao Steam concluida: ${summary.inserted} novos e ${summary.updated} atualizados em ${summary.discovered} manifestos encontrados.`,
      )
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setLaunchMessage(`Nao foi possivel sincronizar a Steam: ${message}`)
    } finally {
      setIsSteamSyncing(false)
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

    await refreshEntries()
    setLaunchMessage(nextArchivedState ? 'Jogo arquivado.' : 'Jogo reativado.')
  }

  const handleEditSelectedEntry = () => {
    if (!selectedEntry || selectedEntry.primaryPlatformId !== 'manual') {
      return
    }

    openManualGameEditor(selectedEntry)
  }

  const handleNavigationFilter = useCallback((filter) => {
    setQuickFilter(filter)
    setSearchTerm('')
    setLaunchMessage('')
  }, [])

  return {
    entries,
    filteredEntries,
    installedCount,
    totalHours,
    selectedEntry,
    showLibraryLoading,
    viewMode,
    setViewMode,
    searchTerm,
    setSearchTerm,
    quickFilter,
    setQuickFilter,
    launchMessage,
    setLaunchMessage,
    isLocalSyncing,
    isSteamSyncing,
    isManualModalOpen,
    manualGameForm,
    setManualGameForm,
    manualGameErrors,
    setManualGameErrors,
    isEditingManualGame,
    openManualGameModal,
    closeManualModal,
    handleArchiveSelectedEntry,
    handleEditSelectedEntry,
    handleInstallAction,
    handleLaunchSelectedEntry,
    handleManualGameSubmit,
    handleNavigationFilter,
    handleSelectEntry,
    handleSyncLocalGames,
    handleSyncSteamGames,
  }
}
