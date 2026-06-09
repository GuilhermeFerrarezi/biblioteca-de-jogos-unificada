import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  addPersistedManualGame,
  launchLibraryEntry,
  listLibraryEntries,
  setLibraryEntryArchived,
  setLibraryEntryFavorite,
  setLibraryEntriesPersonalReview,
  syncSteamAccountGames,
  syncEpicGames,
  syncLocalGames,
  syncSteamGames,
  syncXboxAchievementGames,
  syncXboxGames,
  updatePersistedManualGame,
} from '../services/libraryCommands'
import { getLibrarySettings, normalizeLibrarySettings, saveLibrarySettings } from '../services/librarySettings'
import { normalizeProviderErrorFeedback } from '../services/providerErrorFeedback'
import {
  buildManualLibraryEntry,
  emptyManualGameForm,
  getManualGameFormFromEntry,
  getSelectedEntryIdForEntries,
  validateManualGameInput,
} from '../adapters/libraryEntryAdapter'
import { QUICK_FILTER_IDS, SORT_MODE_IDS } from '../constants/libraryConstants'
import { groupLibraryEntries } from '../domain/libraryGrouping'
import { markBootStep } from '../services/bootInstrumentation'
import {
  getLaunchActionState,
  getPreferredLaunchEntryId,
  getVisibleSelectedEntry,
  isMicrosoftStoreUri,
  isSteamInstallUri,
} from '../domain/libraryLaunch'
import { hasTauriRuntime } from '../services/tauriRuntime'
import { useLibraryFiltering } from './useLibraryFiltering'

const LIBRARY_BOOTSTRAP_COMPLETE_EVENT = 'library-bootstrap-complete'
const STEAM_ENRICHMENT_COMPLETED_EVENT = 'steam-enrichment-completed'
const XBOX_SYNC_FAILED_EVENT = 'xbox-sync-failed'
const XBOX_ACHIEVEMENTS_SYNC_FAILED_EVENT = 'xbox-achievements-sync-failed'
let bootstrapLibraryEntriesPromise = null

const isFavoriteEntry = (entry) =>
  entry?.isFavorite === true ||
  entry?.is_favorite === true ||
  entry?.memberEntries?.some((memberEntry) => isFavoriteEntry(memberEntry)) === true

const updateEntriesFavoriteState = (entries, targetIds, isFavorite) =>
  entries.map((entry) => (
    targetIds.includes(entry.id)
      ? {
          ...entry,
          isFavorite,
          is_favorite: isFavorite,
        }
      : entry
  ))

const updateEntriesPersonalReview = (entries, targetIds, input) =>
  entries.map((entry) => (
    targetIds.includes(entry.id)
      ? {
          ...entry,
          game: {
            ...entry.game,
            personalRating: input.rating,
            personalReview: input.review,
          },
        }
      : entry
  ))

export function useLibraryPageState() {
  const [viewMode, setViewMode] = useState('grid')
  const [entries, setEntries] = useState([])
  const [selectedEntryId, setSelectedEntryId] = useState('')
  const [searchTerm, setSearchTerm] = useState('')
  const [quickFilters, setQuickFilters] = useState([])
  const [sortMode, setSortMode] = useState(SORT_MODE_IDS.ALPHA_ASC)
  const [launchMessage, setLaunchMessage] = useState('')
  const [launchFeedback, setLaunchFeedback] = useState(null)
  const [isLibraryLoading, setIsLibraryLoading] = useState(true)
  const [isLocalSyncing, setIsLocalSyncing] = useState(false)
  const [isSteamSyncing, setIsSteamSyncing] = useState(false)
  const [isXboxSyncing, setIsXboxSyncing] = useState(false)
  const [isEpicSyncing, setIsEpicSyncing] = useState(false)
  const [isSteamAccountSyncing, setIsSteamAccountSyncing] = useState(false)
  const [preferredStoreId, setPreferredStoreId] = useState('steam')
  const [localScanMode, setLocalScanMode] = useState('automatic')
  const [localScanRootsText, setLocalScanRootsText] = useState('')
  const [localScanExcludedRootsText, setLocalScanExcludedRootsText] = useState('')
  const [microsoftClientId, setMicrosoftClientId] = useState('')
  const [selectedLaunchPlatformByEntryId, setSelectedLaunchPlatformByEntryId] = useState({})
  const [isLibrarySettingsLoading, setIsLibrarySettingsLoading] = useState(true)
  const [isLibrarySettingsSaving, setIsLibrarySettingsSaving] = useState(false)
  const [isManualModalOpen, setIsManualModalOpen] = useState(false)
  const [editingEntryId, setEditingEntryId] = useState('')
  const [manualGameForm, setManualGameForm] = useState(emptyManualGameForm)
  const [manualGameErrors, setManualGameErrors] = useState({})
  const hasMarkedUsableRef = useRef(false)
  const xboxSyncFailureHandledRef = useRef(false)
  const isEditingManualGame = editingEntryId !== ''
  const showLibraryLoading = isLibraryLoading
  const groupedEntries = useMemo(() => groupLibraryEntries(entries), [entries])
  const { filteredEntries, installedCount, totalHours } = useLibraryFiltering(groupedEntries, searchTerm, quickFilters, sortMode)
  const selectedEntry = getVisibleSelectedEntry(filteredEntries, selectedEntryId)
  const selectedLaunchPlatformId =
    selectedLaunchPlatformByEntryId[selectedEntry?.id] ?? preferredStoreId
  const { primaryLaunchAction: selectedLaunchAction, hint: selectedLaunchActionHint } =
    getLaunchActionState(selectedEntry, selectedLaunchPlatformId)

  useEffect(() => {
    let isMounted = true

    const loadLibrarySettings = async () => {
      try {
        markBootStep('frontend.provider_settings.start', { critical: false })
        const settings = await getLibrarySettings()

        if (!isMounted) {
          return
        }

        const normalizedSettings = normalizeLibrarySettings(settings)
        setPreferredStoreId(normalizedSettings.preferredStoreId)
        setLocalScanMode(normalizedSettings.localScanMode)
        setLocalScanRootsText(normalizedSettings.localScanRoots.join('\n'))
        setLocalScanExcludedRootsText(normalizedSettings.localScanExcludedRoots.join('\n'))
        setMicrosoftClientId(normalizedSettings.microsoftClientId ?? '')
      } catch {
        if (isMounted) {
          setPreferredStoreId('steam')
          setLocalScanMode('automatic')
          setLocalScanRootsText('')
          setLocalScanExcludedRootsText('')
          setMicrosoftClientId('')
        }
      } finally {
        if (isMounted) {
          setIsLibrarySettingsLoading(false)
          markBootStep('frontend.provider_settings.complete', { critical: false })
        }
      }
    }

    if (!isLibraryLoading || !hasTauriRuntime()) {
      void loadLibrarySettings()
    }

    return () => {
      isMounted = false
    }
  }, [isLibraryLoading])

  useEffect(() => {
    let isMounted = true
    let unlistenBootstrapComplete = null
    let unlistenSteamEnrichmentCompleted = null
    let unlistenXboxSyncFailed = null
    let unlistenXboxAchievementsSyncFailed = null
    let bootstrapTimeoutId = hasTauriRuntime()
      ? window.setTimeout(() => {
          if (isMounted) {
            void syncLibraryEntries()
              .catch(() => {
                setLaunchMessage('Nao foi possivel recarregar a biblioteca local.')
                setLaunchFeedback(null)
              })
          }
      }, 4000)
      : null

    const syncLibraryEntries = async () => {
      markBootStep('frontend.library_sync.start')

      if (!bootstrapLibraryEntriesPromise) {
        bootstrapLibraryEntriesPromise = listLibraryEntries().finally(() => {
          bootstrapLibraryEntriesPromise = null
        })
      }

      const libraryEntries = await bootstrapLibraryEntriesPromise
      markBootStep('frontend.library_sync.entries_received', { entries: libraryEntries.length })

      if (!isMounted) {
        return
      }

      setEntries(libraryEntries)
      markBootStep('frontend.library_sync.entries_committed', { entries: libraryEntries.length })
    }

    const registerBootstrapListener = async () => {
      if (!hasTauriRuntime()) {
        return
      }

      try {
        unlistenBootstrapComplete = await listen(LIBRARY_BOOTSTRAP_COMPLETE_EVENT, async () => {
          if (!isMounted) {
            return
          }

          markBootStep('frontend.bootstrap_seed.complete', { critical: false })
          if (bootstrapTimeoutId !== null) {
            clearTimeout(bootstrapTimeoutId)
            bootstrapTimeoutId = null
          }
          await syncLibraryEntries().catch(() => {
            if (isMounted) {
              setLaunchMessage('Nao foi possivel recarregar a biblioteca local.')
              setLaunchFeedback(null)
            }
          })
        })
      } catch (error) {
        void error
      }
    }

    const registerSteamEnrichmentCompletionListener = async () => {
      if (!hasTauriRuntime()) {
        return
      }

      try {
        unlistenSteamEnrichmentCompleted = await listen(STEAM_ENRICHMENT_COMPLETED_EVENT, async () => {
          if (!isMounted) {
            return
          }

          await syncLibraryEntries().catch(() => {})
        })
      } catch {
        if (isMounted) {
          setLaunchFeedback(null)
        }
      }
    }

    const registerXboxSyncFailureListener = async () => {
      if (!hasTauriRuntime()) {
        return
      }

      try {
        unlistenXboxSyncFailed = await listen(XBOX_SYNC_FAILED_EVENT, (event) => {
          if (!isMounted) {
            return
          }

          xboxSyncFailureHandledRef.current = true
          const feedback = normalizeProviderErrorFeedback(
            event?.payload,
            'Nao foi possivel sincronizar a descoberta local do Xbox.',
            'Sincronizacao Xbox local',
          )
          setIsXboxSyncing(false)
          setLaunchMessage(feedback.message)
          setLaunchFeedback(feedback)
        })
      } catch {
        if (isMounted) {
          setIsXboxSyncing(false)
        }
      }
    }

    const registerXboxAchievementsSyncFailureListener = async () => {
      if (!hasTauriRuntime()) {
        return
      }

      try {
        unlistenXboxAchievementsSyncFailed = await listen(
          XBOX_ACHIEVEMENTS_SYNC_FAILED_EVENT,
          (event) => {
            if (!isMounted) {
              return
            }

            const feedback = normalizeProviderErrorFeedback(
              event?.payload,
              'Nao foi possivel importar os titulos com progresso do Xbox.',
              'Importacao de titulos Xbox',
            )
            setLaunchMessage(feedback.message)
            setLaunchFeedback(feedback)
          },
        )
      } catch {
        if (isMounted) {
          setLaunchFeedback(null)
        }
      }
    }

    void registerBootstrapListener()
    void registerSteamEnrichmentCompletionListener()
    void registerXboxSyncFailureListener()
    void registerXboxAchievementsSyncFailureListener()
    markBootStep('frontend.mounted')
    syncLibraryEntries()
      .catch(() => {
        if (isMounted) {
          setLaunchMessage('Nao foi possivel carregar a biblioteca local.')
          setLaunchFeedback(null)
        }
      })
      .finally(() => {
        if (isMounted) {
          setIsLibraryLoading(false)
          markBootStep('frontend.library_loading.complete')
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
      if (unlistenSteamEnrichmentCompleted) {
        void unlistenSteamEnrichmentCompleted()
      }
      if (unlistenXboxSyncFailed) {
        void unlistenXboxSyncFailed()
      }
      if (unlistenXboxAchievementsSyncFailed) {
        void unlistenXboxAchievementsSyncFailed()
      }
    }
  }, [])

  useEffect(() => {
    if (showLibraryLoading || hasMarkedUsableRef.current) {
      return undefined
    }

    const frameId = window.requestAnimationFrame(() => {
      hasMarkedUsableRef.current = true
      markBootStep('frontend.library_usable.painted', {
        entries: groupedEntries.length,
        filteredEntries: filteredEntries.length,
        selected: Boolean(selectedEntry),
      })
    })

    return () => {
      window.cancelAnimationFrame(frameId)
    }
  }, [filteredEntries.length, groupedEntries.length, selectedEntry, showLibraryLoading])

  useEffect(() => {
    if (!selectedEntryId && filteredEntries.length > 0) {
      setSelectedEntryId(filteredEntries[0].id)
      return
    }

    if (selectedEntryId && filteredEntries.length > 0 && !filteredEntries.some((entry) => entry.id === selectedEntryId)) {
      setSelectedEntryId(filteredEntries[0].id)
    }
  }, [filteredEntries, selectedEntryId])

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
    const refreshedEntries = await listLibraryEntries()

    setEntries(refreshedEntries)
    const groupedRefreshedEntries = groupLibraryEntries(refreshedEntries)
    setSelectedEntryId((currentSelectedEntryId) =>
      getSelectedEntryIdForEntries(groupedRefreshedEntries, currentSelectedEntryId),
    )
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
        setLaunchFeedback(null)
        closeManualModal()
        return
      }

      const persistedEntry = await addPersistedManualGame(input)
      const newEntry = persistedEntry ?? buildManualLibraryEntry(manualGameForm)

      setEntries((currentEntries) => [newEntry, ...currentEntries])
      setSelectedEntryId(newEntry.id)
      setLaunchMessage('Jogo adicionado.')
      setLaunchFeedback(null)
      closeManualModal()
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setLaunchMessage(`Nao foi possivel salvar as alteracoes: ${message}`)
      setLaunchFeedback(
        normalizeProviderErrorFeedback(error, 'Nao foi possivel salvar as alteracoes.', 'Salvar jogo manual'),
      )
    }
  }

  const handleSelectEntry = useCallback((entryId) => {
    setSelectedEntryId(entryId)
    setLaunchMessage('')
    setLaunchFeedback(null)
  }, [])

  const handleLaunchSelectedEntry = async (entryId = null) => {
    if (!selectedEntry) {
      setLaunchMessage('Nenhum jogo selecionado.')
      setLaunchFeedback(null)
      return
    }

    const targetEntryId = entryId || getPreferredLaunchEntryId(selectedEntry, selectedLaunchPlatformId)
    const targetEntry = entries.find((entry) => entry.id === targetEntryId) ?? selectedEntry.memberEntries?.find((entry) => entry.id === targetEntryId) ?? selectedEntry
    const targetLaunchState = getLaunchActionState(targetEntry, selectedLaunchPlatformId)
    const primaryAction = targetLaunchState.primaryLaunchAction

    if (!primaryAction || primaryAction.kind === 'manual' || !primaryAction.target) {
      setLaunchMessage(targetLaunchState.hint || selectedLaunchActionHint || `Nenhuma acao de lancamento configurada para ${selectedEntry.game.title}.`)
      setLaunchFeedback(null)
      return
    }

    const isStoreAction = primaryAction.label === 'Abrir Microsoft Store' || isMicrosoftStoreUri(primaryAction.target)
    const isSteamInstallAction = primaryAction.label === 'Instalar' || isSteamInstallUri(primaryAction.target)

    setLaunchMessage(
      isStoreAction
      ? `Abrindo ${targetEntry.game.title} na Microsoft Store.`
      : isSteamInstallAction
        ? `Abrindo instalacao de ${targetEntry.game.title} na Steam.`
      : `Tentando iniciar ${targetEntry.game.title} por ${primaryAction.label}.`,
    )
    setLaunchFeedback(null)

    if (primaryAction.kind === 'uri') {
      window.location.href = primaryAction.target
      return
    }

    if (
      targetEntry.primaryPlatformId !== 'manual' &&
      targetEntry.primaryPlatformId !== 'local' &&
      targetEntry.primaryPlatformId !== 'xbox'
    ) {
      setLaunchMessage(`Execucao de executaveis para jogos importados sera ligada ao provider correspondente. Acao configurada: ${primaryAction.label}.`)
      setLaunchFeedback(null)
      return
    }

    const result = await launchLibraryEntry(targetEntry.id).catch((error) => {
      const message = error instanceof Error ? error.message : String(error)
      return { started: false, message }
    })

    if (!result) {
      setLaunchMessage(`Execucao de executaveis locais esta disponivel apenas no aplicativo Tauri. Acao configurada: ${primaryAction.label}.`)
      setLaunchFeedback(null)
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
    setLaunchFeedback(null)

    try {
      const summary = await syncLocalGames()

      if (!summary) {
        setLaunchMessage('Sincronizacao local disponivel apenas no aplicativo Tauri.')
        setLaunchFeedback(null)
        return
      }

      await refreshEntries()
      setLaunchMessage(
        `Sincronizacao local concluida: ${summary.inserted} novos, ${summary.updated} atualizados e ${summary.archived ?? 0} arquivados em ${summary.discovered} itens encontrados.`,
      )
      setLaunchFeedback(null)
    } catch (error) {
      const feedback = normalizeProviderErrorFeedback(
        error,
        'Nao foi possivel sincronizar jogos locais.',
        'Sincronizacao de jogos locais',
      )
      setLaunchMessage(feedback.message)
      setLaunchFeedback(feedback)
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
    setLaunchFeedback(null)

    try {
      const summary = await syncSteamGames()

      if (!summary) {
        setLaunchMessage('Sincronizacao Steam disponivel apenas no aplicativo Tauri.')
        setLaunchFeedback(null)
        return
      }

      await refreshEntries()
      setLaunchMessage(
        `Sincronizacao Steam concluida: ${summary.inserted} novos, ${summary.updated} atualizados, ${summary.archived ?? 0} arquivados e ${summary.unavailable ?? 0} indisponiveis em ${summary.discovered} manifestos encontrados.`,
      )
      setLaunchFeedback(null)
    } catch (error) {
      const feedback = normalizeProviderErrorFeedback(
        error,
        'Nao foi possivel sincronizar a Steam.',
        'Sincronizacao da biblioteca Steam',
      )
      setLaunchMessage(feedback.message)
      setLaunchFeedback(feedback)
    } finally {
      setIsSteamSyncing(false)
    }
  }

  const handleSyncXboxGames = async () => {
    if (isXboxSyncing) {
      return
    }

    xboxSyncFailureHandledRef.current = false
    setIsXboxSyncing(true)
    setLaunchMessage('Sincronizando descoberta local do Xbox...')
    setLaunchFeedback(null)

    try {
      const summary = await syncXboxGames()

      if (!summary) {
        setLaunchMessage('Sincronizacao Xbox disponivel apenas no aplicativo Tauri.')
        setLaunchFeedback(null)
        return
      }

      setLaunchMessage(
        `Sincronizacao Xbox concluida: ${summary.inserted} novos, ${summary.updated} atualizados, ${summary.archived ?? 0} arquivados e ${summary.unavailable ?? 0} indisponiveis em ${summary.discovered} itens encontrados.`,
      )
      setLaunchFeedback(null)

      try {
        await refreshEntries()
      } catch (refreshError) {
        const feedback = normalizeProviderErrorFeedback(
          refreshError,
          'O Xbox sincronizou, mas a recarga da biblioteca falhou.',
          'Recarregar biblioteca apos sync Xbox',
        )
        setLaunchMessage(feedback.message)
        setLaunchFeedback(feedback)
      }
    } catch (error) {
      if (xboxSyncFailureHandledRef.current) {
        return
      }

      const feedback = normalizeProviderErrorFeedback(
        error,
        'Nao foi possivel sincronizar a descoberta local do Xbox.',
        'Sincronizacao Xbox local',
      )
      setLaunchMessage(feedback.message)
      setLaunchFeedback(feedback)
    } finally {
      setIsXboxSyncing(false)
    }
  }

  const handleSyncEpicGames = async () => {
    if (isEpicSyncing) {
      return
    }

    setIsEpicSyncing(true)
    setLaunchMessage('Sincronizando manifestos locais da Epic Games...')
    setLaunchFeedback(null)

    try {
      const summary = await syncEpicGames()

      if (!summary) {
        setLaunchMessage('Sincronizacao Epic disponivel apenas no aplicativo Tauri.')
        setLaunchFeedback(null)
        return
      }

      await refreshEntries()
      setLaunchMessage(
        `Sincronizacao Epic concluida: ${summary.inserted} novos, ${summary.updated} atualizados, ${summary.archived ?? 0} arquivados e ${summary.unavailable ?? 0} indisponiveis em ${summary.discovered} manifestos encontrados.`,
      )
      setLaunchFeedback(null)
    } catch (error) {
      const feedback = normalizeProviderErrorFeedback(
        error,
        'Nao foi possivel sincronizar os manifestos locais da Epic.',
        'Sincronizacao Epic local',
      )
      setLaunchMessage(feedback.message)
      setLaunchFeedback(feedback)
    } finally {
      setIsEpicSyncing(false)
    }
  }

  const handleSyncXboxTitleHistory = async () => {
    setLaunchMessage('Importando titulos do historico do Xbox...')
    setLaunchFeedback(null)

    try {
      const summary = await syncXboxAchievementGames()

      if (!summary) {
        setLaunchMessage('Importacao de titulos do Xbox disponivel apenas no aplicativo Tauri.')
        setLaunchFeedback(null)
        return
      }

      await refreshEntries()
      setLaunchMessage(
        `Importacao de titulos do Xbox concluida: ${summary.inserted} novos, ${summary.updated} atualizados, ${summary.archived ?? 0} arquivados e ${summary.unavailable ?? 0} indisponiveis em ${summary.discovered} titulos encontrados.`,
      )
      setLaunchFeedback(null)
    } catch (error) {
      const feedback = normalizeProviderErrorFeedback(
        error,
        'Nao foi possivel importar os titulos do Xbox.',
        'Importacao de titulos Xbox',
      )
      setLaunchMessage(feedback.message)
      setLaunchFeedback(feedback)
      throw feedback
    }
  }

  const handleSyncSteamAccountGames = async ({ retryMarkedEnrichment = false } = {}) => {
    if (isSteamAccountSyncing || isSteamSyncing) {
      return
    }

    setIsSteamAccountSyncing(true)
    setIsSteamSyncing(true)
    setLaunchMessage('Sincronizando biblioteca da conta Steam e instalados locais...')
    setLaunchFeedback(null)

    try {
      const accountSummary = await syncSteamAccountGames({ retryMarkedEnrichment })

      if (!accountSummary) {
        setLaunchMessage('Sincronizacao por conta disponivel apenas no aplicativo Tauri.')
        setLaunchFeedback(null)
        return
      }

      const localSummary = await syncSteamGames()

      if (!localSummary) {
        setLaunchMessage('Sincronizacao local da Steam disponivel apenas no aplicativo Tauri.')
        setLaunchFeedback(null)
        return
      }

      await refreshEntries()
      setLaunchMessage(
        `Sincronizacao Steam concluida: conta com ${accountSummary.inserted} novos, ${accountSummary.updated} atualizados e ${accountSummary.unavailable ?? 0} indisponiveis em ${accountSummary.discovered} itens; instalados locais com ${localSummary.inserted} novos, ${localSummary.updated} atualizados, ${localSummary.archived ?? 0} arquivados e ${localSummary.unavailable ?? 0} indisponiveis em ${localSummary.discovered} manifestos.`,
      )
      setLaunchFeedback(null)
    } catch (error) {
      const feedback = normalizeProviderErrorFeedback(
        error,
        'Nao foi possivel sincronizar a conta Steam.',
        'Sincronizacao da conta Steam',
      )
      setLaunchMessage(feedback.message)
      setLaunchFeedback(feedback)
      throw feedback
    } finally {
      setIsSteamAccountSyncing(false)
      setIsSteamSyncing(false)
    }
  }

  const handleArchiveSelectedEntry = async () => {
    if (!selectedEntry) {
      setLaunchMessage('Nenhum jogo selecionado.')
      setLaunchFeedback(null)
      return
    }

    const nextArchivedState = !selectedEntry.isArchived
    const archiveTargetIds = selectedEntry.memberEntryIds ?? [selectedEntry.id]
    const archiveResults = await Promise.all(
      archiveTargetIds.map((entryId) => setLibraryEntryArchived(entryId, nextArchivedState).catch(() => null)),
    )
    const result = archiveResults.every(Boolean)

    if (!result) {
      setEntries((currentEntries) => {
        const nextEntries = currentEntries.filter((entry) => !archiveTargetIds.includes(entry.id))
        setSelectedEntryId((currentSelectedEntryId) =>
          getSelectedEntryIdForEntries(nextEntries, currentSelectedEntryId),
        )
        return nextEntries
      })
      setLaunchMessage(nextArchivedState ? 'Jogo arquivado nesta sessao.' : 'Jogo reativado nesta sessao.')
      setLaunchFeedback(null)
      return
    }

    await refreshEntries()
    setLaunchMessage(nextArchivedState ? 'Jogo arquivado.' : 'Jogo reativado.')
    setLaunchFeedback(null)
  }

  const handleToggleFavoriteSelectedEntry = async () => {
    if (!selectedEntry) {
      setLaunchMessage('Nenhum jogo selecionado.')
      setLaunchFeedback(null)
      return
    }

    const nextFavoriteState = !isFavoriteEntry(selectedEntry)
    const favoriteTargetIds = selectedEntry.memberEntryIds ?? [selectedEntry.id]
    const favoriteResults = await Promise.all(
      favoriteTargetIds.map((entryId) =>
        setLibraryEntryFavorite(entryId, nextFavoriteState)
          .then(() => true)
          .catch(() => false),
      ),
    )
    const didPersistEverywhere = favoriteResults.every(Boolean)

    setEntries((currentEntries) => updateEntriesFavoriteState(currentEntries, favoriteTargetIds, nextFavoriteState))

    if (hasTauriRuntime() && didPersistEverywhere) {
      await refreshEntries()
      setLaunchMessage(nextFavoriteState ? 'Jogo adicionado aos favoritos.' : 'Jogo removido dos favoritos.')
      setLaunchFeedback(null)
      return
    }

    setLaunchMessage(
      nextFavoriteState
        ? 'Jogo adicionado aos favoritos nesta sessao.'
        : 'Jogo removido dos favoritos nesta sessao.',
    )
    setLaunchFeedback(null)
  }

  const handleSaveSelectedEntryPersonalReview = async (input) => {
    if (!selectedEntry) {
      throw new Error('Nenhum jogo selecionado.')
    }

    const targetIds = selectedEntry.memberEntryIds ?? [selectedEntry.id]
    const nextReview = {
      rating: input?.rating ?? null,
      review: String(input?.review ?? '').trim() || null,
    }

    await setLibraryEntriesPersonalReview(targetIds, nextReview)
    setEntries((currentEntries) => updateEntriesPersonalReview(currentEntries, targetIds, nextReview))
    setLaunchMessage('Avaliacao pessoal salva.')
    setLaunchFeedback(null)

    if (hasTauriRuntime()) {
      await refreshEntries()
    }
  }

  const handleEditSelectedEntry = () => {
    if (!selectedEntry || selectedEntry.primaryPlatformId !== 'manual') {
      return
    }

    openManualGameEditor(selectedEntry)
  }

  const handleQuickFilterChange = useCallback((filterId) => {
    if (filterId === QUICK_FILTER_IDS.ALL) {
      setQuickFilters([])
      setLaunchMessage('')
      setLaunchFeedback(null)
      return
    }

    setQuickFilters((currentFilters) =>
      currentFilters.includes(filterId)
        ? currentFilters.filter((currentFilterId) => currentFilterId !== filterId)
        : [...currentFilters, filterId],
    )
    setLaunchMessage('')
    setLaunchFeedback(null)
  }, [])

  const handleClearLibraryFilters = useCallback(() => {
    setQuickFilters([])
    setSearchTerm('')
    setLaunchMessage('Filtros limpos.')
    setLaunchFeedback(null)
  }, [])

  const handleLaunchPlatformChange = useCallback((nextPlatformId) => {
    const normalizedPlatformId = String(nextPlatformId ?? '').trim().toLowerCase()

    if (!selectedEntry?.id || !['steam', 'xbox', 'epic'].includes(normalizedPlatformId)) {
      return
    }

    setSelectedLaunchPlatformByEntryId((currentSelections) => ({
      ...currentSelections,
      [selectedEntry.id]: normalizedPlatformId,
    }))
    const selectedPlatformLabel = normalizedPlatformId === 'xbox' ? 'Xbox' : normalizedPlatformId === 'epic' ? 'Epic Games' : 'Steam'
    setLaunchMessage(`Biblioteca selecionada: ${selectedPlatformLabel}.`)
    setLaunchFeedback(null)
  }, [selectedEntry?.id])

  const handlePreferredStoreChange = useCallback((nextPreferredStoreId) => {
    const normalizedPreferredStoreId = String(nextPreferredStoreId ?? '').trim().toLowerCase() === 'xbox' ? 'xbox' : 'steam'

    setPreferredStoreId(normalizedPreferredStoreId)
  }, [])

  const handleLocalScanModeChange = useCallback((nextLocalScanMode) => {
    const normalizedMode = String(nextLocalScanMode ?? '').trim().toLowerCase()

    if (['automatic', 'selected_only', 'automatic_plus_extra'].includes(normalizedMode)) {
      setLocalScanMode(normalizedMode)
      return
    }

    setLocalScanMode('automatic')
  }, [])

  const handleLocalScanRootsChange = useCallback((value) => {
    setLocalScanRootsText(value)
  }, [])

  const handleLocalScanExcludedRootsChange = useCallback((value) => {
    setLocalScanExcludedRootsText(value)
  }, [])

  const handleMicrosoftClientIdChange = useCallback((value) => {
    setMicrosoftClientId(value)
  }, [])

  const handleLocalScanRootsSelect = useCallback(async () => {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: 'Selecionar pasta do scan local',
    })

    if (typeof selectedPath !== 'string' || !selectedPath.trim()) {
      return
    }

    setLocalScanRootsText((currentValue) => {
      const roots = String(currentValue ?? '')
        .split(/\r?\n|;/)
        .map((root) => root.trim())
        .filter(Boolean)

      if (roots.some((root) => root.toLowerCase() === selectedPath.trim().toLowerCase())) {
        return roots.join('\n')
      }

      return [...roots, selectedPath.trim()].join('\n')
    })
  }, [])

  const handleLocalScanExcludedRootsSelect = useCallback(async () => {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: 'Selecionar pasta excluida do scan local',
    })

    if (typeof selectedPath !== 'string' || !selectedPath.trim()) {
      return
    }

    setLocalScanExcludedRootsText((currentValue) => {
      const roots = String(currentValue ?? '')
        .split(/\r?\n|;/)
        .map((root) => root.trim())
        .filter(Boolean)

      if (roots.some((root) => root.toLowerCase() === selectedPath.trim().toLowerCase())) {
        return roots.join('\n')
      }

      return [...roots, selectedPath.trim()].join('\n')
    })
  }, [])

  const handleSaveLibrarySettings = useCallback(async () => {
    setIsLibrarySettingsSaving(true)

    try {
      const savedSettings = await saveLibrarySettings({
        preferredStoreId,
        localScanMode,
        localScanRoots: localScanRootsText.split(/\r?\n|;/),
        localScanExcludedRoots: localScanExcludedRootsText.split(/\r?\n|;/),
        microsoftClientId,
      })

      setPreferredStoreId(savedSettings.preferredStoreId)
      setLocalScanMode(savedSettings.localScanMode)
      setLocalScanRootsText(savedSettings.localScanRoots.join('\n'))
      setLocalScanExcludedRootsText(savedSettings.localScanExcludedRoots.join('\n'))
      setMicrosoftClientId(savedSettings.microsoftClientId ?? '')
      setLaunchMessage('Configuracoes da biblioteca salvas.')
      setLaunchFeedback(null)
    } catch (error) {
      const feedback = normalizeProviderErrorFeedback(
        error,
        'Nao foi possivel salvar as configuracoes da biblioteca.',
        'Salvar configuracoes da biblioteca',
      )
      setLaunchMessage(feedback.message)
      setLaunchFeedback(feedback)
    } finally {
      setIsLibrarySettingsSaving(false)
    }
  }, [localScanExcludedRootsText, localScanMode, localScanRootsText, microsoftClientId, preferredStoreId])

  return {
    entries,
    groupedEntries,
    filteredEntries,
    preferredStoreId,
    localScanMode,
    localScanRootsText,
    localScanExcludedRootsText,
    microsoftClientId,
    selectedLaunchPlatformId,
    isLibrarySettingsLoading,
    isLibrarySettingsSaving,
    installedCount,
    totalHours,
    selectedEntry,
    selectedLaunchAction,
    selectedLaunchActionHint,
    showLibraryLoading,
    viewMode,
    setViewMode,
    searchTerm,
    setSearchTerm,
    quickFilters,
    sortMode,
    setSortMode,
    handleQuickFilterChange,
    launchMessage,
    launchFeedback,
    setLaunchMessage,
    isLocalSyncing,
    isSteamSyncing,
    isXboxSyncing,
    isEpicSyncing,
    isSteamAccountSyncing,
    isManualModalOpen,
    manualGameForm,
    setManualGameForm,
    manualGameErrors,
    setManualGameErrors,
    isEditingManualGame,
    openManualGameModal,
    closeManualModal,
    handleArchiveSelectedEntry,
    handleToggleFavoriteSelectedEntry,
    handleSaveSelectedEntryPersonalReview,
    handleEditSelectedEntry,
    handleLaunchSelectedEntry,
    handleLaunchPlatformChange,
    handleClearLibraryFilters,
    handlePreferredStoreChange,
    handleLocalScanModeChange,
    handleLocalScanRootsChange,
    handleLocalScanRootsSelect,
    handleLocalScanExcludedRootsChange,
    handleLocalScanExcludedRootsSelect,
    handleMicrosoftClientIdChange,
    handleSaveLibrarySettings,
    handleManualGameSubmit,
    handleNavigationFilter: handleQuickFilterChange,
    handleSelectEntry,
    handleSyncLocalGames,
    handleSyncSteamGames,
    handleSyncXboxTitleHistory,
    handleSyncXboxGames,
    handleSyncEpicGames,
    handleSyncSteamAccountGames,
  }
}
