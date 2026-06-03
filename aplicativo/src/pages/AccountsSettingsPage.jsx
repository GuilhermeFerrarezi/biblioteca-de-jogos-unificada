import {
  ArrowLeft,
  AlertCircle,
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  Clock3,
  Cloud,
  FolderOpen,
  KeyRound,
  LogIn,
  RefreshCw,
  Save,
  Store,
  Trash2,
  X,
} from 'lucide-react'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useEffect, useMemo, useRef, useState } from 'react'
import SteamIcon from '../components/icons/SteamIcon'
import XboxIcon from '../components/icons/XboxIcon'
import {
  deleteSteamApiKey,
  getSteamEnrichmentRetrySummary,
  getSteamAccountConfig,
  getEpicLibraryRoots,
  getSteamApiKeyStatus,
  getSteamLibraryRoots,
  getXboxAccountConfig,
  getXboxLiveAuthState,
  getXboxLibraryRoots,
  saveSteamAccountConfig,
  saveEpicLibraryRoots,
  saveSteamApiKey,
  saveSteamLibraryRoots,
  saveXboxLibraryRoots,
  startSteamLogin,
  startXboxLiveLogin,
} from '../services/libraryCommands'
import { saveLibrarySettings } from '../services/librarySettings'
import { normalizeProviderErrorFeedback } from '../services/providerErrorFeedback'

const emptySteamApiKeyForm = Object.freeze({
  apiKey: '',
})

const waitForUiFeedback = () =>
  new Promise((resolve) => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(resolve)
    })
  })

const emptySteamAccountForm = Object.freeze({
  steamId64: '',
})

const emptySteamLibraryRootsForm = Object.freeze({
  rootsText: '',
})

const emptyXboxLibraryRootsForm = Object.freeze({
  rootsText: '',
})

const emptyEpicLibraryRootsForm = Object.freeze({
  rootsText: '',
})

const accountProviders = Object.freeze([
  {
    id: 'steam',
    name: 'Steam',
    icon: SteamIcon,
    state: 'Sync local e por conta',
    tone: 'ready',
    detail: 'Manifestos instalados continuam disponiveis sem credencial.',
    nextStep: 'A conta usa SteamID64 e AuthVault configurado.',
  },
  {
    id: 'xbox',
    name: 'Xbox / Game Pass',
    icon: XboxIcon,
    state: 'Descoberta local',
    tone: 'ready',
    detail: 'Instalados entram no app. O historico de titulos pode revelar jogos com progresso antes da descoberta local.',
    nextStep: 'Login publico da Microsoft; sem client secret no fluxo final.',
  },
  {
    id: 'epic',
    name: 'Epic Games',
    icon: Store,
    state: 'Descoberta local',
    tone: 'ready',
    detail: 'Lê manifestos instalados do Epic Games Launcher neste computador.',
    nextStep: 'Sem login, API remota, token ou sessao de navegador.',
  },
])

const XBOX_LIVE_LOGIN_COMPLETE_EVENT = 'xbox-live-login-complete'

const normalizeSteamApiKeyStatus = (status) => {
  if (typeof status === 'boolean') {
    return status
  }

  if (!status || typeof status !== 'object') {
    return false
  }

  return Boolean(status.configured)
}

const normalizeXboxIdentityStatus = (status) => {
  if (typeof status === 'boolean') {
    return status
  }

  if (!status || typeof status !== 'object') {
    return false
  }

  if (typeof status.configured === 'boolean') {
    return status.configured
  }

  return Boolean(status.xuid || status.identityId || status.gamertag)
}

const validateSteamApiKeyInput = (apiKey) => {
  const trimmedApiKey = apiKey.trim()

  if (!trimmedApiKey) {
    return 'Informe a credencial Steam Web API antes de salvar.'
  }

  if (!/^[a-fA-F0-9]{32}$/.test(trimmedApiKey)) {
    return 'A credencial Steam Web API deve ter 32 caracteres hexadecimais.'
  }

  return ''
}

const validateSteamId64Input = (steamId64) => {
  const trimmedSteamId64 = steamId64.trim()

  if (!trimmedSteamId64) {
    return 'Informe o SteamID64 antes de sincronizar a conta.'
  }

  if (!/^\d{17}$/.test(trimmedSteamId64)) {
    return 'SteamID64 deve ter 17 digitos numericos.'
  }

  return ''
}

const validateMicrosoftClientIdInput = (clientId) => {
  const trimmedClientId = clientId.trim()

  if (!trimmedClientId) {
    return ''
  }

  if (
    !/^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/.test(trimmedClientId)
  ) {
    return 'O Microsoft client ID deve ser um GUID valido.'
  }

  return ''
}

const parseSteamLibraryRootsText = (value) =>
  String(value ?? '')
    .split(/\r?\n|;/)
    .map((root) => root.trim())
    .filter(Boolean)

function ProviderFeedback({ defaultMessage, errorFeedback, id, statusMessage }) {
  const [isDetailsOpen, setIsDetailsOpen] = useState(false)
  const detailsRef = useRef(null)
  const detailsId = `${id}-details`
  const hasErrorFeedback = Boolean(errorFeedback)
  const isStructuredFeedback = Boolean(errorFeedback && typeof errorFeedback === 'object' && !Array.isArray(errorFeedback))
  const summaryMessage = isStructuredFeedback
    ? errorFeedback.message || defaultMessage
    : errorFeedback || statusMessage || defaultMessage
  const details = isStructuredFeedback && Array.isArray(errorFeedback.details) ? errorFeedback.details : []

  useEffect(() => {
    setIsDetailsOpen(false)
  }, [errorFeedback])

  useEffect(() => {
    if (isDetailsOpen) {
      detailsRef.current?.focus()
    }
  }, [isDetailsOpen])

  const handleToggleDetails = () => {
    setIsDetailsOpen((currentValue) => !currentValue)
  }

  return (
    <div
      id={id}
      className={hasErrorFeedback ? 'steam-api-key-feedback error provider-feedback' : 'steam-api-key-feedback provider-feedback'}
      role="status"
      aria-live="polite"
    >
      <span className="provider-feedback-summary">{summaryMessage}</span>

      {details.length > 0 ? (
        <>
          <button
            className="provider-feedback-toggle"
            type="button"
            aria-expanded={isDetailsOpen}
            aria-controls={detailsId}
            onClick={handleToggleDetails}
          >
            {isDetailsOpen ? 'Ocultar detalhes tecnicos' : 'Ver detalhes tecnicos'}
          </button>
          <div
            id={detailsId}
            ref={detailsRef}
            className="provider-feedback-details"
            tabIndex={-1}
            hidden={!isDetailsOpen}
          >
            <dl className="provider-feedback-list">
              {details.map((detail, index) => (
                <div className="provider-feedback-item" key={`${detail.label}-${index}`}>
                  <dt>{detail.label}</dt>
                  <dd>{detail.value}</dd>
                </div>
              ))}
            </dl>
          </div>
        </>
      ) : null}
    </div>
  )
}

function AccountsSettingsPage({
  feedbackMessage,
  feedbackDetails,
  isLibrarySettingsLoading,
  isLibrarySettingsSaving,
  isSteamAccountSyncing,
  isSteamSyncing,
  isXboxSyncing,
  isEpicSyncing,
  preferredStoreId,
  localScanMode,
  localScanRootsText,
  localScanExcludedRootsText,
  microsoftClientId,
  onBackToLibrary,
  onPreferredStoreChange,
  onLocalScanModeChange,
  onLocalScanRootsChange,
  onLocalScanRootsSelect,
  onLocalScanExcludedRootsChange,
  onLocalScanExcludedRootsSelect,
  onMicrosoftClientIdChange,
  onSaveLibrarySettings,
  onSyncXboxTitleHistory,
  onSyncSteamAccountGames,
  onSyncSteamGames,
  onSyncXboxGames,
  onSyncEpicGames,
}) {
  const [isSteamPanelOpen, setIsSteamPanelOpen] = useState(false)
  const [steamApiKeyForm, setSteamApiKeyForm] = useState(emptySteamApiKeyForm)
  const [steamAccountForm, setSteamAccountForm] = useState(emptySteamAccountForm)
  const [steamLibraryRootsForm, setSteamLibraryRootsForm] = useState(emptySteamLibraryRootsForm)
  const [xboxLibraryRootsForm, setXboxLibraryRootsForm] = useState(emptyXboxLibraryRootsForm)
  const [epicLibraryRootsForm, setEpicLibraryRootsForm] = useState(emptyEpicLibraryRootsForm)
  const [steamApiKeyConfigured, setSteamApiKeyConfigured] = useState(false)
  const [steamApiKeyStatusMessage, setSteamApiKeyStatusMessage] = useState('')
  const [steamApiKeyError, setSteamApiKeyError] = useState(null)
  const [steamAccountStatusMessage, setSteamAccountStatusMessage] = useState('')
  const [steamAccountError, setSteamAccountError] = useState(null)
  const [steamEnrichmentRetrySummary, setSteamEnrichmentRetrySummary] = useState(null)
  const [isSteamEnrichmentRetryModalOpen, setIsSteamEnrichmentRetryModalOpen] = useState(false)
  const [isSteamEnrichmentRetryChecking, setIsSteamEnrichmentRetryChecking] = useState(false)
  const [steamLibraryRootsStatusMessage, setSteamLibraryRootsStatusMessage] = useState('')
  const [steamLibraryRootsError, setSteamLibraryRootsError] = useState(null)
  const [xboxLibraryRootsStatusMessage, setXboxLibraryRootsStatusMessage] = useState('')
  const [xboxLibraryRootsError, setXboxLibraryRootsError] = useState(null)
  const [epicLibraryRootsStatusMessage, setEpicLibraryRootsStatusMessage] = useState('')
  const [epicLibraryRootsError, setEpicLibraryRootsError] = useState(null)
  const [isSteamAccountConnected, setIsSteamAccountConnected] = useState(false)
  const [isSteamLoginStarting, setIsSteamLoginStarting] = useState(false)
  const [isSteamApiKeyLoading, setIsSteamApiKeyLoading] = useState(true)
  const [isSteamApiKeySaving, setIsSteamApiKeySaving] = useState(false)
  const [isSteamApiKeyDeleting, setIsSteamApiKeyDeleting] = useState(false)
  const [isSteamLibraryRootsLoading, setIsSteamLibraryRootsLoading] = useState(true)
  const [isSteamLibraryRootsSaving, setIsSteamLibraryRootsSaving] = useState(false)
  const [isXboxLibraryRootsLoading, setIsXboxLibraryRootsLoading] = useState(true)
  const [isXboxLibraryRootsSaving, setIsXboxLibraryRootsSaving] = useState(false)
  const [isEpicLibraryRootsLoading, setIsEpicLibraryRootsLoading] = useState(true)
  const [isEpicLibraryRootsSaving, setIsEpicLibraryRootsSaving] = useState(false)
  const [xboxLiveAuthStatus, setXboxLiveAuthStatus] = useState({
    configured: false,
    providerId: 'xbox',
    storage: 'auth_vault',
  })
  const [isXboxLiveAuthLoading, setIsXboxLiveAuthLoading] = useState(true)
  const [isXboxLoginStarting, setIsXboxLoginStarting] = useState(false)
  const [xboxLiveAuthError, setXboxLiveAuthError] = useState(null)
  const [xboxLiveAuthStatusMessage, setXboxLiveAuthStatusMessage] = useState('')
  const [xboxMicrosoftClientIdError, setXboxMicrosoftClientIdError] = useState(null)
  const [xboxMicrosoftClientIdStatusMessage, setXboxMicrosoftClientIdStatusMessage] = useState('')
  const [isXboxTitleHistoryModalOpen, setIsXboxTitleHistoryModalOpen] = useState(false)
  const [isXboxTitleHistoryScanning, setIsXboxTitleHistoryScanning] = useState(false)
  const [xboxTitleHistoryError, setXboxTitleHistoryError] = useState(null)
  const [xboxTitleHistoryStatusMessage, setXboxTitleHistoryStatusMessage] = useState('')
  const [xboxIdentityStatus, setXboxIdentityStatus] = useState({
    connected: false,
    xuid: null,
  })
  const [isXboxIdentityLoading, setIsXboxIdentityLoading] = useState(true)
  const [isXboxPanelOpen, setIsXboxPanelOpen] = useState(false)
  const [isEpicPanelOpen, setIsEpicPanelOpen] = useState(false)

  const steamPanelRef = useRef(null)
  const steamToggleRef = useRef(null)
  const previousSteamPanelOpen = useRef(isSteamPanelOpen)
  const xboxPanelRef = useRef(null)
  const xboxToggleRef = useRef(null)
  const previousXboxPanelOpen = useRef(isXboxPanelOpen)
  const epicPanelRef = useRef(null)
  const epicToggleRef = useRef(null)
  const previousEpicPanelOpen = useRef(isEpicPanelOpen)

  const steamApiKeyStatusLabel = useMemo(() => {
    if (!steamApiKeyConfigured) {
      return 'Cofre nao configurado'
    }

    return 'Cofre configurado'
  }, [steamApiKeyConfigured])

  const isSteamApiKeyBusy = isSteamApiKeyLoading || isSteamApiKeySaving || isSteamApiKeyDeleting
  const steamId64Error = validateSteamId64Input(steamAccountForm.steamId64)
  const canSyncSteamAccount = steamApiKeyConfigured && isSteamAccountConnected && !steamId64Error
  const steamPanelContentId = 'steam-panel-content'
  const steamAccountDisabledReason = !steamApiKeyConfigured
    ? 'Configure o AuthVault para sincronizar a conta.'
    : !isSteamAccountConnected
      ? 'Conecte a conta Steam para sincronizar.'
      : steamId64Error || ''
  const xboxPanelContentId = 'xbox-panel-content'
  const epicPanelContentId = 'epic-panel-content'
  const xboxIdentityConfigured = normalizeXboxIdentityStatus(xboxIdentityStatus)
  const xboxLiveAuthConfigured = normalizeXboxIdentityStatus(xboxLiveAuthStatus)
  const normalizedMicrosoftClientId = String(microsoftClientId ?? '').trim()
  const xboxTitleHistoryHint = isXboxIdentityLoading
    ? 'Consultando identidade Xbox/XUID.'
    : isXboxLiveAuthLoading
      ? 'Consultando o status do Xbox Live.'
      : xboxLiveAuthConfigured
      ? 'O historico de achievements mostra progresso, nao posse. Os titulos importados abrem a Microsoft Store antes do sync local.'
      : 'Conecte a conta Microsoft para importar titulos com progresso.'
  const canSyncXboxTitleHistory = xboxLiveAuthConfigured && typeof onSyncXboxTitleHistory === 'function'
  const xboxTitleHistoryDisabledReason = !xboxLiveAuthConfigured
    ? 'Conecte a conta Microsoft antes de importar titulos.'
    : typeof onSyncXboxTitleHistory !== 'function'
      ? 'Importacao de titulos ainda nao conectada ao backend.'
      : ''

  useEffect(() => {
    let isMounted = true

    const loadXboxIdentityStatus = async () => {
      setIsXboxIdentityLoading(true)

      try {
        const status = await getXboxAccountConfig()

        if (!isMounted) {
          return
        }

        setXboxIdentityStatus({
          connected: Boolean(status?.connected),
          xuid: typeof status?.xuid === 'string' && status.xuid.trim() ? status.xuid.trim() : null,
        })
      } catch {
        if (isMounted) {
          setXboxIdentityStatus({
            connected: false,
            xuid: null,
          })
        }
      } finally {
        if (isMounted) {
          setIsXboxIdentityLoading(false)
        }
      }
    }

    void loadXboxIdentityStatus()

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    let isMounted = true

    const loadSteamAccountConfig = async () => {
      try {
        const accountConfig = await getSteamAccountConfig()

        if (!isMounted) {
          return
        }

        if (accountConfig?.connected && /^\d{17}$/.test(accountConfig.steamId64 ?? '')) {
          setSteamAccountForm({ steamId64: accountConfig.steamId64 })
          setIsSteamAccountConnected(true)
          setSteamAccountStatusMessage('Conta Steam conectada neste dispositivo.')
        }
      } catch (error) {
        if (isMounted) {
          setSteamAccountError(
            normalizeProviderErrorFeedback(
              error,
              'Nao foi possivel consultar a conta Steam conectada.',
              'Consulta da conta Steam',
            ),
          )
        }
      }
    }

    void loadSteamAccountConfig()

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    let isMounted = true

    const loadSteamLibraryRoots = async () => {
      setIsSteamLibraryRootsLoading(true)
      setSteamLibraryRootsError(null)

      try {
        const config = await getSteamLibraryRoots()

        if (!isMounted) {
          return
        }

        const roots = Array.isArray(config?.roots) ? config.roots : []
        setSteamLibraryRootsForm({ rootsText: roots.join('\n') })
        setSteamLibraryRootsStatusMessage(
          roots.length > 0
            ? `${roots.length} ${roots.length === 1 ? 'biblioteca adicional configurada.' : 'bibliotecas adicionais configuradas.'}`
            : 'Nenhuma biblioteca adicional configurada.',
        )
      } catch (error) {
        if (isMounted) {
          setSteamLibraryRootsError(
            normalizeProviderErrorFeedback(
              error,
              'Nao foi possivel consultar as pastas Steam adicionais.',
              'Consulta de bibliotecas Steam adicionais',
            ),
          )
        }
      } finally {
        if (isMounted) {
          setIsSteamLibraryRootsLoading(false)
        }
      }
    }

    void loadSteamLibraryRoots()

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    let isMounted = true

    const loadXboxLibraryRoots = async () => {
      setIsXboxLibraryRootsLoading(true)
      setXboxLibraryRootsError(null)

      try {
        const config = await getXboxLibraryRoots()

        if (!isMounted) {
          return
        }

        const roots = Array.isArray(config?.roots) ? config.roots : []
        setXboxLibraryRootsForm({ rootsText: roots.join('\n') })
        setXboxLibraryRootsStatusMessage(
          roots.length > 0
            ? `${roots.length} ${roots.length === 1 ? 'pasta Xbox adicional configurada.' : 'pastas Xbox adicionais configuradas.'}`
            : 'Nenhuma pasta Xbox adicional configurada.',
        )
      } catch (error) {
        if (isMounted) {
          setXboxLibraryRootsError(
            normalizeProviderErrorFeedback(
              error,
              'Nao foi possivel consultar as pastas Xbox adicionais.',
              'Consulta de pastas Xbox adicionais',
            ),
          )
        }
      } finally {
        if (isMounted) {
          setIsXboxLibraryRootsLoading(false)
        }
      }
    }

    void loadXboxLibraryRoots()

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    let isMounted = true

    const loadEpicLibraryRoots = async () => {
      setIsEpicLibraryRootsLoading(true)
      setEpicLibraryRootsError(null)

      try {
        const config = await getEpicLibraryRoots()

        if (!isMounted) {
          return
        }

        const roots = Array.isArray(config?.roots) ? config.roots : []
        setEpicLibraryRootsForm({ rootsText: roots.join('\n') })
        setEpicLibraryRootsStatusMessage(
          roots.length > 0
            ? `${roots.length} ${roots.length === 1 ? 'pasta Epic adicional configurada.' : 'pastas Epic adicionais configuradas.'}`
            : 'Nenhuma pasta Epic adicional configurada.',
        )
      } catch (error) {
        if (isMounted) {
          setEpicLibraryRootsError(
            normalizeProviderErrorFeedback(
              error,
              'Nao foi possivel consultar as pastas Epic adicionais.',
              'Consulta de pastas Epic adicionais',
            ),
          )
        }
      } finally {
        if (isMounted) {
          setIsEpicLibraryRootsLoading(false)
        }
      }
    }

    void loadEpicLibraryRoots()

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    let isMounted = true

    const loadXboxLiveAuthState = async () => {
      setIsXboxLiveAuthLoading(true)

      try {
        const status = await getXboxLiveAuthState()

        if (!isMounted) {
          return
        }

        const isConfigured = status?.configured === true

        setXboxLiveAuthStatus({
          configured: isConfigured,
          providerId: typeof status?.providerId === 'string' ? status.providerId : 'xbox',
          storage: typeof status?.storage === 'string' ? status.storage : 'auth_vault',
        })
        setXboxLiveAuthError(null)
        setXboxLiveAuthStatusMessage(
          isConfigured ? 'Xbox Live conectado neste dispositivo.' : 'Xbox Live nao conectado neste dispositivo.',
        )
      } catch (error) {
        if (isMounted) {
          setXboxLiveAuthStatus({
            configured: false,
            providerId: 'xbox',
            storage: 'auth_vault',
          })
          setXboxLiveAuthError(
            normalizeProviderErrorFeedback(
              error,
              'Nao foi possivel consultar o status do Xbox Live.',
              'Consulta do AuthVault Xbox Live',
            ),
          )
        }
      } finally {
        if (isMounted) {
          setIsXboxLiveAuthLoading(false)
        }
      }
    }

    void loadXboxLiveAuthState()

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    let unlisten = null
    let isMounted = true

    const subscribeToXboxLiveLogin = async () => {
      try {
        unlisten = await listen(XBOX_LIVE_LOGIN_COMPLETE_EVENT, (event) => {
          if (!isMounted) {
            return
          }

          setIsXboxLoginStarting(false)

          const payload = event?.payload ?? {}
          const isSuccess = Boolean(payload?.success)

          if (isSuccess) {
            setXboxLiveAuthStatus({
              configured: true,
              providerId: typeof payload?.providerId === 'string' ? payload.providerId : 'xbox',
              storage: 'auth_vault',
            })
            setXboxLiveAuthStatusMessage(
              typeof payload?.message === 'string' && payload.message.trim()
                ? payload.message
                : 'Xbox Live conectado neste dispositivo.',
            )
            setXboxLiveAuthError(null)

            const xuid = typeof payload?.xuid === 'string' && payload.xuid.trim() ? payload.xuid.trim() : null
            setXboxIdentityStatus({
              connected: Boolean(xuid),
              xuid,
            })
            return
          }

          setXboxLiveAuthStatus({
            configured: false,
            providerId: 'xbox',
            storage: 'auth_vault',
          })
          setXboxLiveAuthError(
            normalizeProviderErrorFeedback(
              payload,
              'Nao foi possivel concluir o login publico do Xbox Live.',
              'Login publico do Xbox Live',
            ),
          )
          setXboxLiveAuthStatusMessage('')
        })
      } catch {
        if (isMounted) {
          setIsXboxLoginStarting(false)
        }
      }
    }

    void subscribeToXboxLiveLogin()

    return () => {
      isMounted = false
      if (unlisten) {
        unlisten()
      }
    }
  }, [])

  useEffect(() => {
    let unlisten = null
    let isMounted = true

    const subscribeToSteamLogin = async () => {
      try {
        unlisten = await listen('steam-openid-login-complete', (event) => {
          if (!isMounted) {
            return
          }

          const payload = event.payload ?? {}
          setIsSteamLoginStarting(false)

          if (payload.success && /^\d{17}$/.test(payload.steamId64 ?? '')) {
            setSteamAccountForm({ steamId64: payload.steamId64 })
            setIsSteamAccountConnected(true)
            setSteamAccountError(null)
            setSteamAccountStatusMessage(payload.message || 'Conta Steam conectada neste dispositivo.')
            return
          }

          setIsSteamAccountConnected(false)
          setSteamAccountError(
            normalizeProviderErrorFeedback(
              payload.error ?? payload,
              'Nao foi possivel concluir o login Steam.',
              'Login oficial da Steam',
            ),
          )
        })
      } catch {
        if (isMounted) {
          setSteamAccountError(
            normalizeProviderErrorFeedback(
              null,
              'Nao foi possivel acompanhar o retorno do login Steam.',
              'Login oficial da Steam',
            ),
          )
        }
      }
    }

    void subscribeToSteamLogin()

    return () => {
      isMounted = false
      if (unlisten) {
        unlisten()
      }
    }
  }, [])

  useEffect(() => {
    const loadSteamApiKeyStatus = async () => {
      setIsSteamApiKeyLoading(true)
      setSteamApiKeyError(null)

      try {
        const status = await getSteamApiKeyStatus()

        const isConfigured = normalizeSteamApiKeyStatus(status)

        setSteamApiKeyConfigured(isConfigured)
        setSteamApiKeyStatusMessage(isConfigured ? 'Cofre configurado.' : 'Cofre nao configurado.')
      } catch (error) {
        setSteamApiKeyError(
          normalizeProviderErrorFeedback(
            error,
            'Nao foi possivel consultar o status do cofre Steam Web API.',
            'Consulta do AuthVault Steam Web API',
          ),
        )
      } finally {
        setIsSteamApiKeyLoading(false)
      }
    }

    void loadSteamApiKeyStatus()
  }, [])

  useEffect(() => {
    const wasOpen = previousSteamPanelOpen.current

    if (isSteamPanelOpen && !wasOpen) {
      const firstFocusable = steamPanelRef.current?.querySelector(
        'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
      )

      if (firstFocusable instanceof HTMLElement) {
        firstFocusable.focus()
      }
    }

    if (!isSteamPanelOpen && wasOpen) {
      steamToggleRef.current?.focus()
    }

    previousSteamPanelOpen.current = isSteamPanelOpen
  }, [isSteamPanelOpen])

  useEffect(() => {
    const wasOpen = previousXboxPanelOpen.current

    if (isXboxPanelOpen && !wasOpen) {
      const firstFocusable = xboxPanelRef.current?.querySelector(
        'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
      )

      if (firstFocusable instanceof HTMLElement) {
        firstFocusable.focus()
      }
    }

    if (!isXboxPanelOpen && wasOpen) {
      xboxToggleRef.current?.focus()
    }

    previousXboxPanelOpen.current = isXboxPanelOpen
  }, [isXboxPanelOpen])

  useEffect(() => {
    const wasOpen = previousEpicPanelOpen.current

    if (isEpicPanelOpen && !wasOpen) {
      const firstFocusable = epicPanelRef.current?.querySelector(
        'button:not(:disabled), textarea:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
      )

      if (firstFocusable instanceof HTMLElement) {
        firstFocusable.focus()
      }
    }

    if (!isEpicPanelOpen && wasOpen) {
      epicToggleRef.current?.focus()
    }

    previousEpicPanelOpen.current = isEpicPanelOpen
  }, [isEpicPanelOpen])

  const handleSteamApiKeyChange = (event) => {
    setSteamApiKeyForm({ apiKey: event.target.value })
    setSteamApiKeyError(null)
  }

  const handleSteamId64Change = (event) => {
    const steamId64 = event.target.value.replace(/\D/g, '').slice(0, 17)

    setSteamAccountForm({ steamId64 })
    setSteamAccountError(null)
    setIsSteamAccountConnected(false)

    if (/^\d{17}$/.test(steamId64)) {
      setSteamAccountStatusMessage('SteamID64 pronto para salvar neste dispositivo.')
      return
    }

    setSteamAccountStatusMessage('')
  }

  const handleSteamAccountSave = async () => {
    const validationError = validateSteamId64Input(steamAccountForm.steamId64)

    if (validationError) {
      setSteamAccountError(validationError)
      return
    }

    try {
      const accountConfig = await saveSteamAccountConfig(steamAccountForm.steamId64)
      setSteamAccountForm({ steamId64: accountConfig.steamId64 ?? steamAccountForm.steamId64.trim() })
      setIsSteamAccountConnected(true)
      setSteamAccountError(null)
      setSteamAccountStatusMessage('SteamID64 salvo no banco local.')
    } catch (error) {
      setIsSteamAccountConnected(false)
      setSteamAccountError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel salvar o SteamID64 no banco local.',
          'Salvar SteamID64 no banco local',
        ),
      )
    }
  }

  const handleSteamLoginStart = async () => {
    setIsSteamLoginStarting(true)
    setSteamAccountError(null)
    setSteamAccountStatusMessage('Abrindo login oficial da Steam no navegador.')

    try {
      await startSteamLogin()
      setSteamAccountStatusMessage('Conclua o login no navegador para conectar a conta.')
    } catch (error) {
      setIsSteamLoginStarting(false)
      setSteamAccountError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel iniciar o login oficial da Steam.',
          'Login oficial da Steam',
        ),
      )
      setSteamAccountStatusMessage('')
    }
  }

  const runSteamAccountSync = async ({ retryMarkedEnrichment = false } = {}) => {
    setSteamAccountError(null)
    setSteamAccountStatusMessage(
      retryMarkedEnrichment
        ? 'Sincronizacao por conta em andamento, tentando novamente jogos adiados.'
        : 'Sincronizacao por conta em andamento.',
    )
    await waitForUiFeedback()

    try {
      await onSyncSteamAccountGames({ retryMarkedEnrichment })
      setSteamAccountStatusMessage('Sincronizacao por conta finalizada.')
      setSteamEnrichmentRetrySummary(null)
      setIsSteamEnrichmentRetryModalOpen(false)
    } catch (error) {
      setSteamAccountError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel sincronizar a conta Steam.',
          'Sincronizacao da conta Steam',
        ),
      )
      setSteamAccountStatusMessage('')
    }
  }

  const handleSteamAccountSync = async () => {
    const validationError = validateSteamId64Input(steamAccountForm.steamId64)

    if (validationError) {
      setSteamAccountError(validationError)
      return
    }

    if (!steamApiKeyConfigured) {
      setSteamAccountError(
        normalizeProviderErrorFeedback(
          null,
          'Configure o AuthVault antes de sincronizar a conta.',
          'Sincronizacao da conta Steam',
        ),
      )
      return
    }

    if (!isSteamAccountConnected) {
      setSteamAccountError(
        normalizeProviderErrorFeedback(
          null,
          'Conecte a conta Steam antes de sincronizar pela Web API.',
          'Sincronizacao da conta Steam',
        ),
      )
      return
    }

    setIsSteamEnrichmentRetryChecking(true)
    setSteamAccountError(null)
    setSteamAccountStatusMessage('Verificando jogos Steam adiados pelo enrichment.')
    await waitForUiFeedback()

    try {
      const retrySummary = await getSteamEnrichmentRetrySummary()
      const markedGames = Number(retrySummary?.markedGames ?? 0)

      if (markedGames > 0) {
        setSteamEnrichmentRetrySummary(retrySummary)
        setIsSteamEnrichmentRetryModalOpen(true)
        setSteamAccountStatusMessage('')
        return
      }

      await runSteamAccountSync({ retryMarkedEnrichment: false })
    } catch (error) {
      setSteamAccountError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel verificar os jogos adiados da Steam.',
          'Verificar enrichment Steam',
        ),
      )
      setSteamAccountStatusMessage('')
    } finally {
      setIsSteamEnrichmentRetryChecking(false)
    }
  }

  const handleSteamEnrichmentRetryModalClose = () => {
    if (isSteamAccountSyncing) {
      return
    }

    setIsSteamEnrichmentRetryModalOpen(false)
  }

  const handleSteamEnrichmentRetrySkip = async () => {
    if (isSteamAccountSyncing) {
      return
    }

    await runSteamAccountSync({ retryMarkedEnrichment: false })
  }

  const handleSteamEnrichmentRetryConfirm = async () => {
    if (isSteamAccountSyncing) {
      return
    }

    await runSteamAccountSync({ retryMarkedEnrichment: true })
  }

  const handleSteamApiKeySubmit = async (event) => {
    event.preventDefault()

    const validationError = validateSteamApiKeyInput(steamApiKeyForm.apiKey)

    if (validationError) {
      setSteamApiKeyError(validationError)
      return
    }

    setIsSteamApiKeySaving(true)
    setSteamApiKeyError(null)
    setSteamApiKeyStatusMessage('Salvando credencial Steam Web API no AuthVault.')

    try {
      const status = await saveSteamApiKey(steamApiKeyForm.apiKey.trim())
      const isConfigured = normalizeSteamApiKeyStatus(status)
      setSteamApiKeyForm(emptySteamApiKeyForm)
      setSteamApiKeyConfigured(isConfigured)
      setSteamApiKeyStatusMessage(isConfigured ? 'AuthVault configurado.' : 'Cofre nao configurado.')
    } catch (error) {
      setSteamApiKeyError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel configurar o AuthVault.',
          'Salvar credencial Steam Web API',
        ),
      )
      setSteamApiKeyStatusMessage('')
    } finally {
      setIsSteamApiKeySaving(false)
    }
  }

  const handleSteamApiKeyDelete = async () => {
    setIsSteamApiKeyDeleting(true)
    setSteamApiKeyError(null)
    setSteamApiKeyStatusMessage('Removendo credencial Steam Web API do AuthVault.')

    try {
      await deleteSteamApiKey()
      setSteamApiKeyForm(emptySteamApiKeyForm)
      setSteamApiKeyConfigured(false)
      setSteamApiKeyStatusMessage('AuthVault nao configurado.')
    } catch (error) {
      setSteamApiKeyError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel limpar o AuthVault.',
          'Remover credencial Steam Web API',
        ),
      )
      setSteamApiKeyStatusMessage('')
    } finally {
      setIsSteamApiKeyDeleting(false)
    }
  }

  const handleSteamLibraryRootsChange = (event) => {
    setSteamLibraryRootsForm({ rootsText: event.target.value })
    setSteamLibraryRootsError(null)
    setSteamLibraryRootsStatusMessage('Pastas prontas para salvar.')
  }

  const handleSteamLibraryRootSelect = async () => {
    setSteamLibraryRootsError(null)

    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Selecionar biblioteca Steam',
      })

      if (typeof selectedPath !== 'string' || !selectedPath.trim()) {
        return
      }

      setSteamLibraryRootsForm((currentForm) => {
        const currentRoots = parseSteamLibraryRootsText(currentForm.rootsText)
        const hasRoot = currentRoots.some((root) => root.toLowerCase() === selectedPath.trim().toLowerCase())
        const nextRoots = hasRoot ? currentRoots : [...currentRoots, selectedPath.trim()]

        return { rootsText: nextRoots.join('\n') }
      })
      setSteamLibraryRootsStatusMessage('Pasta adicionada. Salve para usar na proxima sincronizacao local.')
    } catch (error) {
      setSteamLibraryRootsError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel abrir o seletor de pastas.',
          'Selecionar biblioteca Steam',
        ),
      )
    }
  }

  const handleSteamLibraryRootsSubmit = async (event) => {
    event.preventDefault()

    const roots = parseSteamLibraryRootsText(steamLibraryRootsForm.rootsText)
    setIsSteamLibraryRootsSaving(true)
    setSteamLibraryRootsError(null)
    setSteamLibraryRootsStatusMessage('Salvando bibliotecas Steam adicionais.')

    try {
      const config = await saveSteamLibraryRoots(roots)
      const savedRoots = Array.isArray(config?.roots) ? config.roots : roots
      setSteamLibraryRootsForm({ rootsText: savedRoots.join('\n') })
      setSteamLibraryRootsStatusMessage(
        savedRoots.length > 0
          ? `${savedRoots.length} ${savedRoots.length === 1 ? 'biblioteca adicional salva.' : 'bibliotecas adicionais salvas.'}`
          : 'Lista de bibliotecas adicionais limpa.',
      )
    } catch (error) {
      setSteamLibraryRootsError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel salvar as pastas Steam. Confirme se cada caminho existe e contem a pasta steamapps.',
          'Salvar bibliotecas Steam adicionais',
        ),
      )
      setSteamLibraryRootsStatusMessage('')
    } finally {
      setIsSteamLibraryRootsSaving(false)
    }
  }

  const handleXboxLibraryRootsChange = (event) => {
    setXboxLibraryRootsForm({ rootsText: event.target.value })
    setXboxLibraryRootsError(null)
    setXboxLibraryRootsStatusMessage('Pastas Xbox prontas para salvar.')
  }

  const handleMicrosoftClientIdChange = (event) => {
    onMicrosoftClientIdChange(event.target.value)
    setXboxMicrosoftClientIdError(null)
    setXboxMicrosoftClientIdStatusMessage(
      event.target.value.trim()
        ? 'Microsoft client ID interno pronto para salvar.'
        : 'Configuracao interna opcional, pode ficar em branco.',
    )
  }

  const handleXboxLibraryRootSelect = async () => {
    setXboxLibraryRootsError(null)

    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Selecionar pasta Xbox',
      })

      if (typeof selectedPath !== 'string' || !selectedPath.trim()) {
        return
      }

      setXboxLibraryRootsForm((currentForm) => {
        const currentRoots = parseSteamLibraryRootsText(currentForm.rootsText)
        const hasRoot = currentRoots.some((root) => root.toLowerCase() === selectedPath.trim().toLowerCase())
        const nextRoots = hasRoot ? currentRoots : [...currentRoots, selectedPath.trim()]

        return { rootsText: nextRoots.join('\n') }
      })
      setXboxLibraryRootsStatusMessage('Pasta adicionada. Salve para usar na proxima sincronizacao local.')
    } catch (error) {
      setXboxLibraryRootsError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel abrir o seletor de pastas.',
          'Selecionar pasta Xbox',
        ),
      )
    }
  }

  const handleXboxLibraryRootsSubmit = async (event) => {
    event.preventDefault()

    const roots = parseSteamLibraryRootsText(xboxLibraryRootsForm.rootsText)
    setIsXboxLibraryRootsSaving(true)
    setXboxLibraryRootsError(null)
    setXboxLibraryRootsStatusMessage('Salvando pastas Xbox adicionais.')

    try {
      const config = await saveXboxLibraryRoots(roots)
      const savedRoots = Array.isArray(config?.roots) ? config.roots : roots
      setXboxLibraryRootsForm({ rootsText: savedRoots.join('\n') })
      setXboxLibraryRootsStatusMessage(
        savedRoots.length > 0
          ? `${savedRoots.length} ${savedRoots.length === 1 ? 'pasta Xbox adicional salva.' : 'pastas Xbox adicionais salvas.'}`
          : 'Lista de pastas Xbox adicionais limpa.',
      )
    } catch (error) {
      setXboxLibraryRootsError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel salvar as pastas Xbox. Confirme se cada caminho existe e contem a pasta XboxGames.',
          'Salvar pastas Xbox adicionais',
        ),
      )
      setXboxLibraryRootsStatusMessage('')
    } finally {
      setIsXboxLibraryRootsSaving(false)
    }
  }

  const handleEpicLibraryRootsChange = (event) => {
    setEpicLibraryRootsForm({ rootsText: event.target.value })
    setEpicLibraryRootsError(null)
    setEpicLibraryRootsStatusMessage('Pastas Epic prontas para salvar.')
  }

  const handleEpicLibraryRootSelect = async () => {
    setEpicLibraryRootsError(null)

    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Selecionar pasta de manifestos Epic',
      })

      if (typeof selectedPath !== 'string' || !selectedPath.trim()) {
        return
      }

      setEpicLibraryRootsForm((currentForm) => {
        const currentRoots = parseSteamLibraryRootsText(currentForm.rootsText)
        const hasRoot = currentRoots.some((root) => root.toLowerCase() === selectedPath.trim().toLowerCase())
        const nextRoots = hasRoot ? currentRoots : [...currentRoots, selectedPath.trim()]

        return { rootsText: nextRoots.join('\n') }
      })
      setEpicLibraryRootsStatusMessage('Pasta adicionada. Salve para usar na proxima sincronizacao Epic local.')
    } catch (error) {
      setEpicLibraryRootsError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel abrir o seletor de pastas.',
          'Selecionar pasta Epic',
        ),
      )
    }
  }

  const handleEpicLibraryRootsSubmit = async (event) => {
    event.preventDefault()

    const roots = parseSteamLibraryRootsText(epicLibraryRootsForm.rootsText)
    setIsEpicLibraryRootsSaving(true)
    setEpicLibraryRootsError(null)
    setEpicLibraryRootsStatusMessage('Salvando pastas Epic adicionais.')

    try {
      const config = await saveEpicLibraryRoots(roots)
      const savedRoots = Array.isArray(config?.roots) ? config.roots : roots
      setEpicLibraryRootsForm({ rootsText: savedRoots.join('\n') })
      setEpicLibraryRootsStatusMessage(
        savedRoots.length > 0
          ? `${savedRoots.length} ${savedRoots.length === 1 ? 'pasta Epic adicional salva.' : 'pastas Epic adicionais salvas.'}`
          : 'Lista de pastas Epic adicionais limpa.',
      )
    } catch (error) {
      setEpicLibraryRootsError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel salvar as pastas Epic. Confirme se cada caminho existe e aponta para manifestos do Epic Games Launcher.',
          'Salvar pastas Epic adicionais',
        ),
      )
      setEpicLibraryRootsStatusMessage('')
    } finally {
      setIsEpicLibraryRootsSaving(false)
    }
  }

  const handleXboxLiveLoginStart = async () => {
    setIsXboxLoginStarting(true)
    setXboxLiveAuthError(null)
    setXboxLiveAuthStatusMessage('Abrindo o login publico da Microsoft para Xbox no navegador.')

    try {
      await startXboxLiveLogin()
      setXboxLiveAuthStatusMessage('Conclua o login com sua conta Microsoft para conectar o Xbox Live.')
    } catch (error) {
      setIsXboxLoginStarting(false)
      setXboxLiveAuthError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel iniciar o login publico do Xbox Live.',
          'Login publico do Xbox Live',
        ),
      )
      setXboxLiveAuthStatusMessage('')
    }
  }

  const handleMicrosoftClientIdSubmit = async (event) => {
    event.preventDefault()

    const validationError = validateMicrosoftClientIdInput(normalizedMicrosoftClientId)

    if (validationError) {
      setXboxMicrosoftClientIdError(validationError)
      return
    }

    setXboxMicrosoftClientIdError(null)
    setXboxMicrosoftClientIdStatusMessage('Salvando configuracao interna de Xbox Live nas configuracoes da biblioteca.')

    try {
      await saveLibrarySettings({
        preferredStoreId,
        localScanMode,
        localScanRoots: localScanRootsText.split(/\r?\n|;/),
        localScanExcludedRoots: localScanExcludedRootsText.split(/\r?\n|;/),
        microsoftClientId: normalizedMicrosoftClientId,
      })

      setXboxMicrosoftClientIdStatusMessage('Configuracao interna de Xbox Live salva nas configuracoes da biblioteca.')
    } catch (error) {
      setXboxMicrosoftClientIdError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel salvar a configuracao interna do Xbox Live.',
          'Salvar configuracao interna do Xbox Live',
        ),
      )
      setXboxMicrosoftClientIdStatusMessage('')
    }
  }

  const handleSteamPanelToggle = () => {
    setIsSteamPanelOpen((currentValue) => !currentValue)
  }

  const handleXboxPanelToggle = () => {
    setIsXboxPanelOpen((currentValue) => !currentValue)
  }

  const handleEpicPanelToggle = () => {
    setIsEpicPanelOpen((currentValue) => !currentValue)
  }

  const handleXboxTitleHistoryModalOpen = () => {
    if (!canSyncXboxTitleHistory) {
      setXboxTitleHistoryError(
        normalizeProviderErrorFeedback(
          null,
          xboxTitleHistoryDisabledReason || 'Importacao de titulos indisponivel no momento.',
          'Importacao de titulos Xbox',
        ),
      )
      setXboxTitleHistoryStatusMessage('')
      return
    }

    setXboxTitleHistoryError(null)
    setXboxTitleHistoryStatusMessage('')
    setIsXboxTitleHistoryModalOpen(true)
  }

  const handleXboxTitleHistoryModalClose = () => {
    if (isXboxTitleHistoryScanning) {
      return
    }

    setIsXboxTitleHistoryModalOpen(false)
  }

  const handleXboxTitleHistoryConfirm = async () => {
    if (!canSyncXboxTitleHistory || isXboxTitleHistoryScanning) {
      return
    }

    setIsXboxTitleHistoryScanning(true)
    setXboxTitleHistoryError(null)
    setXboxTitleHistoryStatusMessage('Importando titulos descobertos pelo historico do Xbox.')

    try {
      await onSyncXboxTitleHistory()
      setXboxTitleHistoryStatusMessage(
        'Importacao concluida. As entradas novas devem abrir a Microsoft Store ao serem acionadas.',
      )
      setIsXboxTitleHistoryModalOpen(false)
    } catch (error) {
      setXboxTitleHistoryError(
        normalizeProviderErrorFeedback(
          error,
          'Nao foi possivel importar os titulos do Xbox.',
          'Importacao de titulos Xbox',
        ),
      )
      setXboxTitleHistoryStatusMessage('')
    } finally {
      setIsXboxTitleHistoryScanning(false)
    }
  }

  return (
    <section className="accounts-page" aria-labelledby="accounts-title">
      <header className="topbar accounts-topbar">
        <div>
          <h1 id="accounts-title">Contas e integracoes</h1>
          <p>Providers preparados para sincronizacao da biblioteca.</p>
        </div>
        <button className="secondary-button" type="button" onClick={onBackToLibrary}>
          <ArrowLeft size={18} aria-hidden="true" />
          Biblioteca
        </button>
      </header>

      <div className="accounts-layout">
        <section className="accounts-panel" aria-label="Contas conectadas">
          {accountProviders.map((provider) => (
            <ProviderAccountRow
              canSyncSteamAccount={canSyncSteamAccount}
              isSteamAccountConnected={isSteamAccountConnected}
              isSteamAccountSyncing={isSteamAccountSyncing}
              isSteamLoginStarting={isSteamLoginStarting}
              isSteamPanelOpen={isSteamPanelOpen}
              isSteamSyncing={isSteamSyncing}
              isEpicPanelOpen={isEpicPanelOpen}
              isEpicSyncing={isEpicSyncing}
              isXboxPanelOpen={isXboxPanelOpen}
              isXboxIdentityConfigured={xboxLiveAuthConfigured}
              isXboxTitleHistoryScanning={isXboxTitleHistoryScanning}
              isXboxSyncing={isXboxSyncing}
              xboxTitleHistoryErrorFeedback={xboxTitleHistoryError}
              xboxTitleHistoryDisabledReason={xboxTitleHistoryDisabledReason}
              xboxTitleHistoryHint={xboxTitleHistoryHint}
              xboxTitleHistoryStatusMessage={xboxTitleHistoryStatusMessage}
              key={provider.id}
              panelContent={
                provider.id === 'steam' && isSteamPanelOpen ? (
                  <SteamProviderPanel
                    canSyncSteamAccount={canSyncSteamAccount}
                    errorFeedback={steamAccountError}
                    form={steamAccountForm}
                    isSteamAccountConnected={isSteamAccountConnected}
                    isSteamAccountSyncing={isSteamAccountSyncing}
                    isSteamApiKeyBusy={isSteamApiKeyBusy}
                    isSteamApiKeyConfigured={steamApiKeyConfigured}
                    isSteamApiKeyDeleting={isSteamApiKeyDeleting}
                    isSteamApiKeyLoading={isSteamApiKeyLoading}
                    isSteamApiKeySaving={isSteamApiKeySaving}
                    isSteamEnrichmentRetryChecking={isSteamEnrichmentRetryChecking}
                    isSteamLibraryRootsLoading={isSteamLibraryRootsLoading}
                    isSteamLibraryRootsSaving={isSteamLibraryRootsSaving}
                    isSteamLoginStarting={isSteamLoginStarting}
                    isSteamSyncing={isSteamSyncing}
                    panelRef={steamPanelRef}
                    steamAccountDisabledReason={steamAccountDisabledReason}
                    steamAccountStatusMessage={steamAccountStatusMessage}
                    steamApiKeyError={steamApiKeyError}
                    steamApiKeyForm={steamApiKeyForm}
                    steamApiKeyStatusLabel={steamApiKeyStatusLabel}
                    steamApiKeyStatusMessage={steamApiKeyStatusMessage}
                    steamLibraryRootsError={steamLibraryRootsError}
                    steamLibraryRootsForm={steamLibraryRootsForm}
                    steamLibraryRootsStatusMessage={steamLibraryRootsStatusMessage}
                    onStartSteamLogin={handleSteamLoginStart}
                    onSteamAccountChange={handleSteamId64Change}
                    onSteamAccountSave={handleSteamAccountSave}
                    onSteamApiKeyChange={handleSteamApiKeyChange}
                    onSteamApiKeyDelete={handleSteamApiKeyDelete}
                    onSteamApiKeySubmit={handleSteamApiKeySubmit}
                    onSteamLibraryRootsChange={handleSteamLibraryRootsChange}
                    onSteamLibraryRootSelect={handleSteamLibraryRootSelect}
                    onSteamLibraryRootsSubmit={handleSteamLibraryRootsSubmit}
                    onSyncSteamAccountGames={handleSteamAccountSync}
                    onSyncSteamGames={onSyncSteamGames}
                    panelId={steamPanelContentId}
                  />
                ) : provider.id === 'xbox' ? (
                  isXboxPanelOpen ? (
                    <XboxLibraryRootsPanel
                      errorFeedback={xboxLibraryRootsError}
                      form={xboxLibraryRootsForm}
                      isBusy={
                        isXboxLibraryRootsLoading ||
                        isXboxLibraryRootsSaving ||
                        isXboxSyncing
                      }
                      isXboxLiveAuthConfigured={xboxLiveAuthConfigured}
                      isXboxLiveAuthLoading={isXboxLiveAuthLoading}
                      isXboxLoginStarting={isXboxLoginStarting}
                      isXboxIdentityConfigured={xboxIdentityConfigured}
                      isLoading={isXboxLibraryRootsLoading}
                      isSaving={isXboxLibraryRootsSaving}
                      microsoftClientId={normalizedMicrosoftClientId}
                      xboxLiveAuthErrorFeedback={xboxLiveAuthError}
                      xboxLiveAuthStatusMessage={xboxLiveAuthStatusMessage}
                      xboxMicrosoftClientIdErrorFeedback={xboxMicrosoftClientIdError}
                      xboxMicrosoftClientIdStatusMessage={xboxMicrosoftClientIdStatusMessage}
                      panelId={xboxPanelContentId}
                      panelRef={xboxPanelRef}
                      statusMessage={xboxLibraryRootsStatusMessage}
                      onMicrosoftClientIdChange={handleMicrosoftClientIdChange}
                      onMicrosoftClientIdSubmit={handleMicrosoftClientIdSubmit}
                      onStartXboxLiveLogin={handleXboxLiveLoginStart}
                      onChange={handleXboxLibraryRootsChange}
                      onSelectRoot={handleXboxLibraryRootSelect}
                      onSubmit={handleXboxLibraryRootsSubmit}
                    />
                  ) : null
                ) : provider.id === 'epic' ? (
                  isEpicPanelOpen ? (
                    <EpicLibraryRootsPanel
                      errorFeedback={epicLibraryRootsError}
                      form={epicLibraryRootsForm}
                      isBusy={isEpicLibraryRootsLoading || isEpicLibraryRootsSaving || isEpicSyncing}
                      isLoading={isEpicLibraryRootsLoading}
                      isSaving={isEpicLibraryRootsSaving}
                      panelId={epicPanelContentId}
                      panelRef={epicPanelRef}
                      statusMessage={epicLibraryRootsStatusMessage}
                      onChange={handleEpicLibraryRootsChange}
                      onSelectRoot={handleEpicLibraryRootSelect}
                      onSubmit={handleEpicLibraryRootsSubmit}
                    />
                  ) : null
                ) : null
              }
              provider={provider}
              epicPanelContentId={epicPanelContentId}
              epicToggleRef={epicToggleRef}
              steamPanelContentId={steamPanelContentId}
              steamToggleRef={steamToggleRef}
              xboxPanelContentId={xboxPanelContentId}
              xboxToggleRef={xboxToggleRef}
              onOpenXboxTitleHistoryModal={handleXboxTitleHistoryModalOpen}
              onSyncXboxGames={onSyncXboxGames}
              onSyncEpicGames={onSyncEpicGames}
              onToggleSteamPanel={handleSteamPanelToggle}
              onToggleXboxPanel={handleXboxPanelToggle}
              onToggleEpicPanel={handleEpicPanelToggle}
            />
          ))}
        </section>

        <aside className="accounts-summary" aria-label="Estado das integracoes">
          <LibraryDefaultsCard
            isLoading={isLibrarySettingsLoading}
            isSaving={isLibrarySettingsSaving}
            preferredStoreId={preferredStoreId}
            localScanMode={localScanMode}
            localScanRootsText={localScanRootsText}
            localScanExcludedRootsText={localScanExcludedRootsText}
            onPreferredStoreChange={onPreferredStoreChange}
            onLocalScanModeChange={onLocalScanModeChange}
            onLocalScanRootsChange={onLocalScanRootsChange}
            onLocalScanRootsSelect={onLocalScanRootsSelect}
            onLocalScanExcludedRootsChange={onLocalScanExcludedRootsChange}
            onLocalScanExcludedRootsSelect={onLocalScanExcludedRootsSelect}
            onSaveLibrarySettings={onSaveLibrarySettings}
          />
          <div>
            <span className="summary-kicker">Seguranca</span>
            <strong>AuthVault protege as configuracoes internas</strong>
            <p>O fluxo final nao exige client secret e a interface nao pede segredo para conectar a conta.</p>
          </div>
          <div className="summary-row">
            <KeyRound size={18} aria-hidden="true" />
            <span>Steam Web API: {steamApiKeyStatusLabel}</span>
          </div>
          <div className="summary-row">
            <Cloud size={18} aria-hidden="true" />
            <span>Sync por conta: {canSyncSteamAccount ? 'pronta' : 'pendente'}</span>
          </div>
          {feedbackMessage ? (
            <ProviderFeedback
              defaultMessage={feedbackMessage}
              errorFeedback={feedbackDetails}
              id="account-sync-feedback"
              statusMessage=""
            />
          ) : null}
        </aside>
      </div>

      {isXboxTitleHistoryModalOpen ? (
        <XboxTitleHistoryModal
          errorFeedback={xboxTitleHistoryError}
          isBusy={isXboxTitleHistoryScanning}
          isConfigured={xboxLiveAuthConfigured}
          statusMessage={xboxTitleHistoryStatusMessage}
          onClose={handleXboxTitleHistoryModalClose}
          onConfirm={handleXboxTitleHistoryConfirm}
        />
      ) : null}

      {isSteamEnrichmentRetryModalOpen ? (
        <SteamEnrichmentRetryModal
          isBusy={isSteamAccountSyncing}
          onClose={handleSteamEnrichmentRetryModalClose}
          onConfirm={handleSteamEnrichmentRetryConfirm}
          onSkip={handleSteamEnrichmentRetrySkip}
          summary={steamEnrichmentRetrySummary}
        />
      ) : null}
    </section>
  )
}

function LibraryDefaultsCard({
  isLoading,
  isSaving,
  preferredStoreId,
  localScanMode,
  localScanRootsText,
  localScanExcludedRootsText,
  onPreferredStoreChange,
  onLocalScanModeChange,
  onLocalScanRootsChange,
  onLocalScanRootsSelect,
  onLocalScanExcludedRootsChange,
  onLocalScanExcludedRootsSelect,
  onSaveLibrarySettings,
}) {
  const normalizedPreferredStoreId = preferredStoreId === 'xbox' ? 'xbox' : 'steam'
  const normalizedLocalScanMode = ['automatic', 'selected_only', 'automatic_plus_extra'].includes(localScanMode)
    ? localScanMode
    : 'automatic'
  const countConfiguredPaths = (value) =>
    String(value ?? '')
      .split(/\r?\n|;/)
      .map((item) => item.trim())
      .filter(Boolean).length
  const localScanRootsCount = countConfiguredPaths(localScanRootsText)
  const localScanExcludedRootsCount = countConfiguredPaths(localScanExcludedRootsText)
  const scanModeDescription =
    normalizedLocalScanMode === 'selected_only'
      ? 'Somente as pastas escolhidas abaixo serao varridas.'
      : normalizedLocalScanMode === 'automatic_plus_extra'
        ? 'A busca automatica continua ativa e as pastas abaixo entram como extras.'
        : 'A busca automatica atual continua varrendo as pastas conhecidas e os drives disponiveis.'

  return (
    <section className="accounts-defaults-card" aria-labelledby="library-defaults-title" aria-busy={isLoading || isSaving}>
      <div className="library-defaults-header">
        <div>
          <div className="summary-kicker">Padroes da biblioteca</div>
          <strong id="library-defaults-title">Loja principal e scan local</strong>
          <p>Defina a loja preferida e como a varredura local deve procurar fora das bibliotecas Steam e Xbox.</p>
        </div>

        <div className="library-defaults-status" aria-label="Resumo das configuracoes atuais">
          <span className="library-defaults-pill">
            {normalizedPreferredStoreId === 'xbox' ? 'Xbox como padrao' : 'Steam como padrao'}
          </span>
          <span className="library-defaults-pill">
            {normalizedLocalScanMode === 'automatic'
              ? 'Scan automatico'
              : normalizedLocalScanMode === 'selected_only'
                ? 'Somente pastas escolhidas'
                : 'Automatico + extras'}
          </span>
        </div>
      </div>

      <div className="library-defaults-grid">
        <label className="library-defaults-field" htmlFor="preferred-store">
          <span>Loja padrao</span>
          <select
            id="preferred-store"
            value={normalizedPreferredStoreId}
            disabled={isLoading || isSaving}
            onChange={(event) => onPreferredStoreChange(event.target.value)}
          >
            <option value="steam">Steam</option>
            <option value="xbox">Xbox</option>
          </select>
        </label>

        <label className="library-defaults-field" htmlFor="local-scan-mode">
          <span>Modo de scan local</span>
          <select
            id="local-scan-mode"
            value={normalizedLocalScanMode}
            disabled={isLoading || isSaving}
            onChange={(event) => onLocalScanModeChange(event.target.value)}
          >
            <option value="automatic">Automatico atual</option>
            <option value="selected_only">Somente pastas escolhidas</option>
            <option value="automatic_plus_extra">Automatico + extras</option>
          </select>
        </label>
      </div>

      <p className="library-defaults-note">{scanModeDescription}</p>

      <div className="library-defaults-scan-section">
        <div className="library-defaults-section-header">
          <div>
            <span className="library-defaults-section-kicker">Pastas do scan</span>
            <strong>Raizes adicionais</strong>
          </div>
          <span className="library-defaults-section-count">
            {localScanRootsCount} {localScanRootsCount === 1 ? 'pasta' : 'pastas'}
          </span>
        </div>
        <label className="library-defaults-field" htmlFor="local-scan-roots">
          <span className="library-defaults-field-label">Pastas que entram na varredura</span>
          <textarea
            id="local-scan-roots"
            rows={3}
            value={localScanRootsText}
            spellCheck="false"
            placeholder={'D:\\Jogos\nE:\\Games'}
            disabled={isLoading || isSaving}
            onChange={(event) => onLocalScanRootsChange(event.target.value)}
          />
        </label>

        <div className="steam-api-key-actions">
          <button className="secondary-button" type="button" disabled={isLoading || isSaving} onClick={onLocalScanRootsSelect}>
            <FolderOpen size={16} aria-hidden="true" />
            Adicionar pasta
          </button>
        </div>
      </div>

      <div className="library-defaults-scan-section library-defaults-scan-section--danger">
        <div className="library-defaults-section-header">
          <div>
            <span className="library-defaults-section-kicker">Exclusoes</span>
            <strong>Pastas ignoradas</strong>
          </div>
          <span className="library-defaults-section-count">
            {localScanExcludedRootsCount} {localScanExcludedRootsCount === 1 ? 'pasta' : 'pastas'}
          </span>
        </div>
        <label className="library-defaults-field" htmlFor="local-scan-excluded-roots">
          <span className="library-defaults-field-label">Pastas que nunca devem ser varridas</span>
          <textarea
            id="local-scan-excluded-roots"
            rows={3}
            value={localScanExcludedRootsText}
            spellCheck="false"
            placeholder={'D:\\Jogos\\NaoVarrer\nE:\\Protótipos'}
            disabled={isLoading || isSaving}
            onChange={(event) => onLocalScanExcludedRootsChange(event.target.value)}
          />
        </label>

        <div className="steam-api-key-actions">
          <button
            className="secondary-button"
            type="button"
            disabled={isLoading || isSaving}
            onClick={onLocalScanExcludedRootsSelect}
          >
            <FolderOpen size={16} aria-hidden="true" />
            Excluir pasta
          </button>
        </div>
      </div>

      <div className="library-defaults-footer">
        <span className="steam-account-help">
          {normalizedPreferredStoreId === 'xbox'
            ? 'Quando houver copia duplicada, o Xbox vira a escolha prioritaria do botao Jogar.'
            : 'Quando houver copia duplicada, a Steam vira a escolha prioritaria do botao Jogar.'}
        </span>
        <button className="primary-button" type="button" disabled={isLoading || isSaving} onClick={onSaveLibrarySettings}>
          <Save size={16} aria-hidden="true" />
          {isSaving ? 'Salvando' : 'Salvar configuracoes'}
        </button>
      </div>
    </section>
  )
}

function SteamProviderPanel({
  canSyncSteamAccount,
  errorFeedback,
  form,
  isSteamAccountConnected,
  isSteamAccountSyncing,
  isSteamApiKeyBusy,
  isSteamApiKeyConfigured,
  isSteamApiKeyDeleting,
  isSteamApiKeyLoading,
  isSteamApiKeySaving,
  isSteamEnrichmentRetryChecking,
  isSteamLibraryRootsLoading,
  isSteamLibraryRootsSaving,
  isSteamLoginStarting,
  isSteamSyncing,
  onSteamAccountChange,
  onSteamAccountSave,
  onSteamApiKeyChange,
  onSteamApiKeyDelete,
  onSteamApiKeySubmit,
  onSteamLibraryRootsChange,
  onSteamLibraryRootSelect,
  onSteamLibraryRootsSubmit,
  onStartSteamLogin,
  onSyncSteamAccountGames,
  onSyncSteamGames,
  panelId,
  panelRef,
  steamAccountDisabledReason,
  steamAccountStatusMessage,
  steamApiKeyError,
  steamApiKeyForm,
  steamApiKeyStatusLabel,
  steamApiKeyStatusMessage,
  steamLibraryRootsError,
  steamLibraryRootsForm,
  steamLibraryRootsStatusMessage,
}) {
  return (
    <div className="steam-panel-popover" id={panelId} ref={panelRef}>
      <div className="steam-panel-popover-actions">
        <button
          className={isSteamAccountConnected ? 'secondary-button' : 'primary-button'}
          type="button"
          aria-label="Entrar com Steam pelo navegador oficial"
          disabled={isSteamSyncing || isSteamAccountSyncing || isSteamLoginStarting}
          onClick={onStartSteamLogin}
        >
          <LogIn size={16} aria-hidden="true" />
          {isSteamLoginStarting ? 'Conectando' : isSteamAccountConnected ? 'Reconectar Steam' : 'Entrar com Steam'}
        </button>
        <button
          className="secondary-button"
          type="button"
          aria-label="Sincronizar Steam local por manifestos"
          disabled={isSteamSyncing || isSteamAccountSyncing}
          onClick={onSyncSteamGames}
        >
          {isSteamSyncing ? 'Sincronizando local' : 'Sincronizar local'}
        </button>
        <button
          className="primary-button"
          type="button"
          aria-label="Sincronizar conta Steam via Web API"
          aria-describedby={!canSyncSteamAccount ? 'steam-account-sync-requirements' : undefined}
          disabled={!canSyncSteamAccount || isSteamSyncing || isSteamAccountSyncing || isSteamEnrichmentRetryChecking}
          onClick={onSyncSteamAccountGames}
        >
          {isSteamEnrichmentRetryChecking
            ? 'Verificando'
            : isSteamAccountSyncing
              ? 'Sincronizando conta'
              : 'Sincronizar conta'}
        </button>
        {!canSyncSteamAccount ? (
          <span id="steam-account-sync-requirements" className="account-action-hint">
            {steamAccountDisabledReason}
          </span>
        ) : null}
      </div>

      <SteamAccountPanel
        errorFeedback={errorFeedback}
        form={form}
        isBusy={isSteamAccountSyncing}
        isConfigured={isSteamAccountConnected}
        statusMessage={steamAccountStatusMessage}
        onChange={onSteamAccountChange}
        onSave={onSteamAccountSave}
      />

      <SteamWebApiVaultPanel
        errorFeedback={steamApiKeyError}
        form={steamApiKeyForm}
        isBusy={isSteamApiKeyBusy}
        isDeleting={isSteamApiKeyDeleting}
        isLoading={isSteamApiKeyLoading}
        isSaving={isSteamApiKeySaving}
        isConfigured={isSteamApiKeyConfigured}
        statusLabel={steamApiKeyStatusLabel}
        statusMessage={steamApiKeyStatusMessage}
        onChange={onSteamApiKeyChange}
        onDelete={onSteamApiKeyDelete}
        onSubmit={onSteamApiKeySubmit}
      />

      <SteamLibraryRootsPanel
        errorFeedback={steamLibraryRootsError}
        form={steamLibraryRootsForm}
        isBusy={isSteamLibraryRootsLoading || isSteamLibraryRootsSaving || isSteamSyncing}
        isLoading={isSteamLibraryRootsLoading}
        isSaving={isSteamLibraryRootsSaving}
        statusMessage={steamLibraryRootsStatusMessage}
        onChange={onSteamLibraryRootsChange}
        onSelectRoot={onSteamLibraryRootSelect}
        onSubmit={onSteamLibraryRootsSubmit}
      />
    </div>
  )
}

function SteamAccountPanel({ errorFeedback, form, isBusy, isConfigured, onChange, onSave, statusMessage }) {
  return (
    <article className="steam-account-panel" aria-labelledby="steam-account-title" aria-busy={isBusy}>
      <div className="steam-api-vault-heading">
        <div className="account-provider-icon" aria-hidden="true">
          <SteamIcon size={22} />
        </div>

        <div>
          <div className="account-provider-heading">
            <h2 id="steam-account-title">Conta Steam</h2>
            <span className="account-status" data-tone={isConfigured ? 'ready' : 'planned'}>
              {isConfigured ? <CheckCircle2 size={14} aria-hidden="true" /> : <AlertCircle size={14} aria-hidden="true" />}
              {isConfigured ? 'SteamID64 configurado' : 'SteamID64 pendente'}
            </span>
          </div>
          <p>Use o login oficial da Steam ou salve o SteamID64 manualmente no banco local.</p>
        </div>
      </div>

      <label className="steam-account-field" htmlFor="steam-id64">
        <span>SteamID64</span>
        <input
          id="steam-id64"
          type="text"
          value={form.steamId64}
          autoComplete="off"
          inputMode="numeric"
          pattern="[0-9]*"
          maxLength={17}
          aria-invalid={errorFeedback ? 'true' : 'false'}
          aria-describedby="steam-id64-help steam-id64-feedback"
          disabled={isBusy}
          onChange={onChange}
        />
      </label>
      <p id="steam-id64-help" className="steam-account-help">
        Usado apenas para consultar a biblioteca publica/permitida pela Web API.
      </p>
      <div className="steam-api-key-actions">
        <button className="secondary-button" type="button" disabled={isBusy} onClick={onSave}>
          <Save size={16} aria-hidden="true" />
          Salvar SteamID64
        </button>
      </div>
      <ProviderFeedback
        defaultMessage="Informe o SteamID64 para liberar a sincronizacao por conta."
        errorFeedback={errorFeedback}
        id="steam-id64-feedback"
        statusMessage={statusMessage}
      />
    </article>
  )
}

function SteamLibraryRootsPanel({
  errorFeedback,
  form,
  isBusy,
  isLoading,
  isSaving,
  onChange,
  onSelectRoot,
  onSubmit,
  statusMessage,
}) {
  return (
    <article className="steam-library-roots-panel" aria-labelledby="steam-library-roots-title" aria-busy={isBusy}>
      <div className="steam-api-vault-heading">
        <div className="account-provider-icon" aria-hidden="true">
          <SteamIcon size={22} />
        </div>

        <div>
          <div className="account-provider-heading">
            <h2 id="steam-library-roots-title">Bibliotecas Steam</h2>
            <span className="account-status" data-tone="ready">
              <CheckCircle2 size={14} aria-hidden="true" />
              Pastas adicionais
            </span>
          </div>
          <p>Adicione uma pasta por linha para bibliotecas em outros armazenamentos.</p>
        </div>
      </div>

      <form className="steam-library-roots-form" onSubmit={onSubmit}>
        <label htmlFor="steam-library-roots">
          <span>Pastas Steam adicionais</span>
          <textarea
            id="steam-library-roots"
            value={form.rootsText}
            rows={4}
            spellCheck="false"
            placeholder={'D:\\SteamLibrary\nE:\\Jogos\\SteamLibrary'}
            aria-invalid={errorFeedback ? 'true' : 'false'}
            aria-describedby="steam-library-roots-help steam-library-roots-feedback"
            disabled={isBusy}
            onChange={onChange}
          />
        </label>
        <p id="steam-library-roots-help">
          Use a raiz da biblioteca ou a propria pasta steamapps. Cada caminho precisa existir neste computador.
        </p>
        <div className="steam-api-key-actions">
          <button className="secondary-button" type="button" disabled={isBusy} onClick={onSelectRoot}>
            <FolderOpen size={16} aria-hidden="true" />
            Selecionar pasta
          </button>
          <button className="primary-button" type="submit" disabled={isBusy}>
            <Save size={16} aria-hidden="true" />
            {isSaving ? 'Salvando' : 'Salvar pastas'}
          </button>
        </div>
      </form>
      <ProviderFeedback
        defaultMessage={isLoading ? 'Consultando bibliotecas adicionais.' : 'Informe pastas Steam fora do disco C quando necessario.'}
        errorFeedback={errorFeedback}
        id="steam-library-roots-feedback"
        statusMessage={statusMessage}
      />
    </article>
  )
}

function XboxLibraryRootsPanel({
  errorFeedback,
  form,
  isBusy,
  isXboxLiveAuthConfigured,
  isXboxLiveAuthLoading,
  isXboxIdentityConfigured,
  isXboxLoginStarting,
  isLoading,
  isSaving,
  microsoftClientId,
  onChange,
  onMicrosoftClientIdChange,
  onMicrosoftClientIdSubmit,
  onSelectRoot,
  onStartXboxLiveLogin,
  onSubmit,
  panelId,
  panelRef,
  xboxLiveAuthErrorFeedback,
  xboxLiveAuthStatusMessage,
  xboxMicrosoftClientIdErrorFeedback,
  xboxMicrosoftClientIdStatusMessage,
  statusMessage,
}) {
  const isDevelopmentBuild = import.meta.env.DEV
  const hasMicrosoftClientId = Boolean(microsoftClientId?.trim())

  return (
    <article
      className="xbox-library-roots-panel"
      id={panelId}
      ref={panelRef}
      aria-labelledby="xbox-library-roots-title"
      aria-busy={isBusy || isXboxLiveAuthLoading || isXboxLoginStarting}
    >
      <div className="steam-api-vault-heading">
        <div className="account-provider-icon" aria-hidden="true">
          <XboxIcon size={22} />
        </div>

        <div>
          <div className="account-provider-heading">
            <h2 id="xbox-library-roots-title">Pastas Xbox</h2>
            <span className="account-status" data-tone="ready">
              <CheckCircle2 size={14} aria-hidden="true" />
              Descoberta local
            </span>
          </div>
          <p>Adicione uma pasta por linha para que o Xbox varra outros armazenamentos do PC.</p>
        </div>
      </div>

      <div className="xbox-live-auth-panel">
        <div className="account-provider-heading">
          <h2 id="xbox-live-auth-title">Xbox Live</h2>
          <span className="account-status" data-tone={isXboxLiveAuthConfigured ? 'ready' : 'planned'}>
            {isXboxLiveAuthConfigured ? (
              <CheckCircle2 size={14} aria-hidden="true" />
            ) : (
              <AlertCircle size={14} aria-hidden="true" />
            )}
            {isXboxLiveAuthLoading
              ? 'Consultando login'
              : isXboxLiveAuthConfigured
                ? isXboxIdentityConfigured
                  ? 'Conectado'
                  : 'Conectado sem XUID'
                : 'Nao conectado'}
          </span>
        </div>
        <p>Use o login oficial para vincular a conta e liberar a importacao de titulos com progresso.</p>
        <p className="account-action-hint">
          Fluxo de public client para desktop. O usuario final nao precisa preencher client secret para entrar.
        </p>

        {isDevelopmentBuild ? (
          <form className="steam-library-roots-form" onSubmit={onMicrosoftClientIdSubmit}>
            <label htmlFor="microsoft-client-id">
              <span>Microsoft client ID interno</span>
              <input
                id="microsoft-client-id"
                type="text"
                value={microsoftClientId}
                autoComplete="off"
                spellCheck="false"
                inputMode="text"
                aria-invalid={xboxMicrosoftClientIdErrorFeedback ? 'true' : 'false'}
                aria-describedby="microsoft-client-id-help microsoft-client-id-feedback"
                disabled={isBusy}
                onChange={onMicrosoftClientIdChange}
              />
            </label>
            <p id="microsoft-client-id-help">
              Campo de manutencao do projeto. O fluxo final nao exige client secret e este ID pode ficar salvo apenas para administracao da instancia.
            </p>
            <div className="steam-api-key-actions">
              <button className="primary-button" type="submit" disabled={isBusy}>
                <Save size={16} aria-hidden="true" />
                Salvar configuracao interna
              </button>
            </div>
            <ProviderFeedback
              defaultMessage={
                hasMicrosoftClientId
                  ? 'Configuracao interna pronta para o login Xbox Live.'
                  : 'O login publico do Xbox Live nao depende deste campo.'
              }
              errorFeedback={xboxMicrosoftClientIdErrorFeedback}
              id="microsoft-client-id-feedback"
              statusMessage={xboxMicrosoftClientIdStatusMessage}
            />
          </form>
        ) : (
          <div className="steam-library-roots-form">
            <div className="library-defaults-field">
              <span>Microsoft client ID interno</span>
              <strong>{hasMicrosoftClientId ? 'Configurado internamente' : 'Nao configurado'}</strong>
            </div>
            <p id="microsoft-client-id-help">
              O app final carrega este valor da build ou da configuracao interna. Nao ha edicao na interface do usuario.
            </p>
            <ProviderFeedback
              defaultMessage={
                hasMicrosoftClientId
                  ? 'Configuracao interna pronta para o login Xbox Live.'
                  : 'Configure o Application (client) ID na build antes de conectar o Xbox Live.'
              }
              errorFeedback={xboxMicrosoftClientIdErrorFeedback}
              id="microsoft-client-id-feedback"
              statusMessage={xboxMicrosoftClientIdStatusMessage}
            />
          </div>
        )}

        <div className="steam-api-key-actions">
          <button
            className="primary-button"
            type="button"
            disabled={isBusy || isXboxLoginStarting}
            onClick={onStartXboxLiveLogin}
          >
            <LogIn size={16} aria-hidden="true" />
            {isXboxLoginStarting ? 'Abrindo login' : isXboxLiveAuthConfigured ? 'Reconectar conta Microsoft' : 'Conectar conta Microsoft'}
          </button>
        </div>
        <ProviderFeedback
          defaultMessage={
            hasMicrosoftClientId
              ? 'O estado do login Microsoft/Xbox sera mostrado aqui.'
              : 'O login Microsoft/Xbox sera mostrado aqui mesmo sem configuracao interna.'
          }
          errorFeedback={xboxLiveAuthErrorFeedback}
          id="xbox-live-auth-feedback"
          statusMessage={xboxLiveAuthStatusMessage}
        />
      </div>

      <form className="steam-library-roots-form" onSubmit={onSubmit}>
        <label htmlFor="xbox-library-roots">
          <span>Pastas Xbox adicionais</span>
          <textarea
            id="xbox-library-roots"
            value={form.rootsText}
            rows={4}
            spellCheck="false"
            placeholder={'D:\\XboxLibrary\nE:\\Jogos\\XboxLibrary'}
            aria-invalid={errorFeedback ? 'true' : 'false'}
            aria-describedby="xbox-library-roots-help xbox-library-roots-feedback"
            disabled={isBusy}
            onChange={onChange}
          />
        </label>
        <p id="xbox-library-roots-help">
          Use a raiz do armazenamento ou a pasta XboxGames. Cada caminho precisa existir neste computador.
        </p>
        <div className="steam-api-key-actions">
          <button className="secondary-button" type="button" disabled={isBusy} onClick={onSelectRoot}>
            <FolderOpen size={16} aria-hidden="true" />
            Selecionar pasta
          </button>
          <button className="primary-button" type="submit" disabled={isBusy}>
            <Save size={16} aria-hidden="true" />
            {isSaving ? 'Salvando' : 'Salvar pastas'}
          </button>
        </div>
      </form>
      <ProviderFeedback
        defaultMessage={isLoading ? 'Consultando pastas Xbox adicionais.' : 'Informe outras pastas com XboxGames para ampliar a descoberta local.'}
        errorFeedback={errorFeedback}
        id="xbox-library-roots-feedback"
        statusMessage={statusMessage}
      />
    </article>
  )
}

function EpicLibraryRootsPanel({
  errorFeedback,
  form,
  isBusy,
  isLoading,
  isSaving,
  onChange,
  onSelectRoot,
  onSubmit,
  panelId,
  panelRef,
  statusMessage,
}) {
  return (
    <article
      className="xbox-library-roots-panel"
      id={panelId}
      ref={panelRef}
      aria-labelledby="epic-library-roots-title"
      aria-busy={isBusy}
    >
      <div className="steam-api-vault-heading">
        <div className="account-provider-icon" aria-hidden="true">
          <Store size={22} />
        </div>

        <div>
          <div className="account-provider-heading">
            <h2 id="epic-library-roots-title">Manifestos Epic</h2>
            <span className="account-status" data-tone="ready">
              <CheckCircle2 size={14} aria-hidden="true" />
              Descoberta local
            </span>
          </div>
          <p>Adicione uma pasta por linha contendo manifestos .item do Epic Games Launcher.</p>
        </div>
      </div>

      <form className="steam-library-roots-form" onSubmit={onSubmit}>
        <label htmlFor="epic-library-roots">
          <span>Pastas Epic adicionais</span>
          <textarea
            id="epic-library-roots"
            value={form.rootsText}
            rows={4}
            spellCheck="false"
            placeholder={'D:\\Epic\\Manifests\nE:\\EpicGamesLauncher\\Data\\Manifests'}
            aria-invalid={errorFeedback ? 'true' : 'false'}
            aria-describedby="epic-library-roots-help epic-library-roots-feedback"
            disabled={isBusy}
            onChange={onChange}
          />
        </label>
        <p id="epic-library-roots-help">
          O caminho padrao ProgramData da Epic ja e consultado. Use este campo apenas para manifestos adicionais neste computador.
        </p>
        <div className="steam-api-key-actions">
          <button className="secondary-button" type="button" disabled={isBusy} onClick={onSelectRoot}>
            <FolderOpen size={16} aria-hidden="true" />
            Selecionar pasta
          </button>
          <button className="primary-button" type="submit" disabled={isBusy}>
            <Save size={16} aria-hidden="true" />
            {isSaving ? 'Salvando' : 'Salvar pastas'}
          </button>
        </div>
      </form>
      <ProviderFeedback
        defaultMessage={isLoading ? 'Consultando pastas Epic adicionais.' : 'A sincronizacao usa apenas manifestos locais validos da Epic.'}
        errorFeedback={errorFeedback}
        id="epic-library-roots-feedback"
        statusMessage={statusMessage}
      />
    </article>
  )
}

function SteamWebApiVaultPanel({
  errorFeedback,
  form,
  isBusy,
  isConfigured,
  isDeleting,
  isLoading,
  isSaving,
  onChange,
  onDelete,
  onSubmit,
  statusLabel,
  statusMessage,
}) {
  return (
    <article className="steam-api-vault" aria-labelledby="steam-api-vault-title">
      <div className="steam-api-vault-heading">
        <div className="account-provider-icon" aria-hidden="true">
          <SteamIcon size={22} />
        </div>

        <div>
          <div className="account-provider-heading">
            <h2 id="steam-api-vault-title">Steam Web API</h2>
            <span className="account-status" data-tone={isConfigured ? 'ready' : 'planned'}>
              {isConfigured ? <CheckCircle2 size={14} aria-hidden="true" /> : <AlertCircle size={14} aria-hidden="true" />}
              {isLoading ? 'Consultando cofre' : statusLabel}
            </span>
          </div>
          <p>
            Prepare o AuthVault para chamadas da Web API. A sincronizacao por SteamID64 sera
            conectada em uma etapa separada.
          </p>
        </div>
      </div>

      <form className="steam-api-key-form" onSubmit={onSubmit} aria-busy={isBusy}>
        <label htmlFor="steam-api-key">
          <span>Credencial Web API</span>
          <input
            id="steam-api-key"
            type="password"
            value={form.apiKey}
            autoComplete="off"
            spellCheck="false"
            inputMode="text"
            aria-invalid={errorFeedback ? 'true' : 'false'}
            aria-describedby="steam-api-key-help steam-api-key-feedback"
            disabled={isBusy}
            onChange={onChange}
          />
        </label>
        <p id="steam-api-key-help">
          Cole um novo valor para salvar ou substituir. O valor ja salvo nao e exibido.
        </p>

        <div className="steam-api-key-actions">
          <button className="primary-button" type="submit" disabled={isBusy}>
            <Save size={16} aria-hidden="true" />
            {isSaving ? 'Salvando' : 'Salvar'}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={isBusy || !isConfigured}
            onClick={onDelete}
          >
            <Trash2 size={16} aria-hidden="true" />
            {isDeleting ? 'Removendo' : 'Remover'}
          </button>
        </div>

        <ProviderFeedback
          defaultMessage="O status do cofre sera mostrado aqui."
          errorFeedback={errorFeedback}
          id="steam-api-key-feedback"
          statusMessage={statusMessage}
        />
      </form>
    </article>
  )
}

function ProviderAccountRow({
  epicPanelContentId,
  epicToggleRef,
  isEpicPanelOpen,
  isEpicSyncing,
  isSteamAccountSyncing,
  isSteamPanelOpen,
  isSteamSyncing,
  isXboxIdentityConfigured,
  isXboxTitleHistoryScanning,
  isXboxSyncing,
  isXboxPanelOpen,
  xboxTitleHistoryErrorFeedback,
  xboxTitleHistoryDisabledReason,
  xboxTitleHistoryHint,
  xboxTitleHistoryStatusMessage,
  onSyncEpicGames,
  onSyncXboxGames,
  onOpenXboxTitleHistoryModal,
  onToggleEpicPanel,
  onToggleSteamPanel,
  onToggleXboxPanel,
  panelContent,
  provider,
  steamPanelContentId,
  steamToggleRef,
  xboxPanelContentId,
  xboxToggleRef,
}) {
  const Icon = provider.icon
  const isSteam = provider.id === 'steam'
  const isXbox = provider.id === 'xbox'
  const isEpic = provider.id === 'epic'
  const isPlanned = provider.tone === 'planned'

  return (
    <article
      className={isXbox ? 'account-row account-row--xbox' : 'account-row'}
      aria-busy={isSteam ? isSteamSyncing || isSteamAccountSyncing : isXbox ? isXboxSyncing : isEpic ? isEpicSyncing : undefined}
    >
      <div className="account-provider-icon" aria-hidden="true">
        <Icon size={22} />
      </div>

      <div className="account-provider-main">
        <div className="account-provider-heading">
          <h2>{provider.name}</h2>
          <span className="account-status" data-tone={provider.tone}>
            {provider.tone === 'ready' ? <CheckCircle2 size={14} aria-hidden="true" /> : <Clock3 size={14} aria-hidden="true" />}
            {provider.state}
          </span>
        </div>
        <p>{provider.detail}</p>
        <span>{provider.nextStep}</span>
      </div>

      {isSteam ? (
        <button
          ref={steamToggleRef}
          className="account-expand-toggle"
          type="button"
          aria-expanded={isSteamPanelOpen}
          aria-controls={steamPanelContentId}
          aria-label={isSteamPanelOpen ? 'Fechar opcoes da Steam' : 'Abrir opcoes da Steam'}
          onClick={onToggleSteamPanel}
        >
          <ChevronDown size={18} aria-hidden="true" className={isSteamPanelOpen ? 'expand-icon open' : 'expand-icon'} />
        </button>
      ) : isXbox ? (
        <div className="account-actions account-actions--xbox">
          <button className="secondary-button" type="button" disabled={isXboxSyncing} onClick={onSyncXboxGames}>
            {isXboxSyncing ? 'Sincronizando Xbox' : 'Sincronizar local'}
          </button>
          <button
            className="primary-button"
            type="button"
            disabled={!isXboxIdentityConfigured || isXboxTitleHistoryScanning}
            aria-describedby="xbox-title-history-hint"
            aria-label={xboxTitleHistoryDisabledReason || 'Abrir confirmacao para importar titulos do Xbox'}
            onClick={onOpenXboxTitleHistoryModal}
          >
            {isXboxTitleHistoryScanning ? (
              <>
                <RefreshCw size={16} aria-hidden="true" className="spin-icon" />
                Importando titulos
              </>
            ) : (
              'Importar titulos'
            )}
          </button>
          <button
            ref={xboxToggleRef}
            className="account-expand-toggle"
            type="button"
            aria-expanded={isXboxPanelOpen}
            aria-controls={xboxPanelContentId}
            aria-label={isXboxPanelOpen ? 'Fechar opcoes do Xbox' : 'Abrir opcoes do Xbox'}
            onClick={onToggleXboxPanel}
          >
            <ChevronDown size={18} aria-hidden="true" className={isXboxPanelOpen ? 'expand-icon open' : 'expand-icon'} />
          </button>
          <span id="xbox-title-history-hint" className="account-action-hint">
            {xboxTitleHistoryHint}
          </span>
          {xboxTitleHistoryErrorFeedback || xboxTitleHistoryStatusMessage ? (
            <ProviderFeedback
              defaultMessage="Confirme o aviso para importar titulos do Xbox."
              errorFeedback={xboxTitleHistoryErrorFeedback}
              id="xbox-title-history-row-feedback"
              statusMessage={xboxTitleHistoryStatusMessage}
            />
          ) : null}
        </div>
      ) : isEpic ? (
        <div className="account-actions account-actions--xbox">
          <button className="secondary-button" type="button" disabled={isEpicSyncing} onClick={onSyncEpicGames}>
            {isEpicSyncing ? 'Sincronizando Epic' : 'Sincronizar local'}
          </button>
          <button
            ref={epicToggleRef}
            className="account-expand-toggle"
            type="button"
            aria-expanded={isEpicPanelOpen}
            aria-controls={epicPanelContentId}
            aria-label={isEpicPanelOpen ? 'Fechar opcoes da Epic' : 'Abrir opcoes da Epic'}
            onClick={onToggleEpicPanel}
          >
            <ChevronDown size={18} aria-hidden="true" className={isEpicPanelOpen ? 'expand-icon open' : 'expand-icon'} />
          </button>
          <span className="account-action-hint">
            Manifest-only: sem login Epic, API remota ou credencial.
          </span>
        </div>
      ) : (
        <button className="secondary-button" type="button" disabled={isPlanned}>
          Em breve
        </button>
      )}

      {panelContent}
    </article>
  )
}

function useModalBodyScrollLock() {
  useEffect(() => {
    const previousBodyOverflow = document.body.style.overflow
    const previousDocumentOverflow = document.documentElement.style.overflow

    document.body.style.overflow = 'hidden'
    document.documentElement.style.overflow = 'hidden'

    return () => {
      document.body.style.overflow = previousBodyOverflow
      document.documentElement.style.overflow = previousDocumentOverflow
    }
  }, [])
}

function SteamEnrichmentRetryModal({ isBusy, onClose, onConfirm, onSkip, summary }) {
  const panelRef = useRef(null)
  const closeButtonRef = useRef(null)
  const previouslyFocusedRef = useRef(null)
  const markedGames = Number(summary?.markedGames ?? 0)

  useModalBodyScrollLock()

  useEffect(() => {
    previouslyFocusedRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null

    const focusFirstElement = () => {
      const focusableElements = panelRef.current?.querySelectorAll(
        'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
      )
      const firstFocusable = focusableElements?.[0] ?? closeButtonRef.current

      if (firstFocusable instanceof HTMLElement) {
        firstFocusable.focus()
      }
    }

    const handleKeyDown = (event) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onClose()
        return
      }

      if (event.key !== 'Tab') {
        return
      }

      const focusableElements = Array.from(
        panelRef.current?.querySelectorAll(
          'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
        ) ?? [],
      ).filter((element) => element instanceof HTMLElement)

      if (focusableElements.length === 0) {
        return
      }

      const firstFocusable = focusableElements[0]
      const lastFocusable = focusableElements[focusableElements.length - 1]

      if (event.shiftKey && document.activeElement === firstFocusable) {
        event.preventDefault()
        lastFocusable.focus()
        return
      }

      if (!event.shiftKey && document.activeElement === lastFocusable) {
        event.preventDefault()
        firstFocusable.focus()
      }
    }

    const focusTimeoutId = window.setTimeout(focusFirstElement, 0)
    window.addEventListener('keydown', handleKeyDown)

    return () => {
      window.clearTimeout(focusTimeoutId)
      window.removeEventListener('keydown', handleKeyDown)
      previouslyFocusedRef.current?.focus?.()
    }
  }, [onClose])

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        ref={panelRef}
        className="modal-panel steam-enrichment-retry-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="steam-enrichment-retry-title"
      >
        <header className="modal-header">
          <div>
            <span>Steam</span>
            <h2 id="steam-enrichment-retry-title">Tentar jogos adiados?</h2>
          </div>
          <button
            ref={closeButtonRef}
            className="icon-button"
            type="button"
            aria-label="Fechar aviso de enrichment Steam"
            title="Fechar"
            onClick={onClose}
            disabled={isBusy}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <div className="steam-enrichment-retry-content">
          <p className="steam-enrichment-retry-intro">
            Existem {markedGames} {markedGames === 1 ? 'jogo Steam adiado' : 'jogos Steam adiados'} pelo enrichment.
            Eles ja foram tentados recentemente e podem continuar sem capa ou conquistas disponiveis na Steam.
          </p>

          <div className="steam-enrichment-retry-warning" role="note" aria-label="Aviso sobre nova tentativa Steam">
            <AlertTriangle size={18} aria-hidden="true" />
            <p>
              Tentar novamente pode fazer mais requisicoes a Steam Web API. Se preferir manter os marcadores, a
              sincronizacao da conta continua normalmente para os demais jogos.
            </p>
          </div>

          <ul className="steam-enrichment-retry-list">
            <li>{markedGames} {markedGames === 1 ? 'jogo com marcador ativo' : 'jogos com marcador ativo'} no cache local.</li>
            <li>Escolher nao tentar novamente preserva os marcadores atuais.</li>
            <li>Escolher tentar novamente ignora os marcadores apenas nesta rodada.</li>
          </ul>
        </div>

        <div className="modal-actions">
          <button className="secondary-button" type="button" disabled={isBusy} onClick={onClose}>
            Cancelar
          </button>
          <button className="secondary-button" type="button" disabled={isBusy} onClick={onSkip}>
            Sincronizar sem tentar
          </button>
          <button className="primary-button" type="button" disabled={isBusy} onClick={onConfirm}>
            {isBusy ? 'Sincronizando' : 'Tentar novamente'}
          </button>
        </div>
      </section>
    </div>
  )
}

function XboxTitleHistoryModal({ errorFeedback, isBusy, isConfigured, onClose, onConfirm, statusMessage }) {
  const panelRef = useRef(null)
  const closeButtonRef = useRef(null)
  const previouslyFocusedRef = useRef(null)

  useModalBodyScrollLock()

  useEffect(() => {
    previouslyFocusedRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null

    const focusFirstElement = () => {
      const focusableElements = panelRef.current?.querySelectorAll(
        'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
      )
      const firstFocusable = focusableElements?.[0] ?? closeButtonRef.current

      if (firstFocusable instanceof HTMLElement) {
        firstFocusable.focus()
      }
    }

    const handleKeyDown = (event) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onClose()
        return
      }

      if (event.key !== 'Tab') {
        return
      }

      const focusableElements = Array.from(
        panelRef.current?.querySelectorAll(
          'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
        ) ?? [],
      ).filter((element) => element instanceof HTMLElement)

      if (focusableElements.length === 0) {
        return
      }

      const firstFocusable = focusableElements[0]
      const lastFocusable = focusableElements[focusableElements.length - 1]

      if (event.shiftKey && document.activeElement === firstFocusable) {
        event.preventDefault()
        lastFocusable.focus()
        return
      }

      if (!event.shiftKey && document.activeElement === lastFocusable) {
        event.preventDefault()
        firstFocusable.focus()
      }
    }

    const focusTimeoutId = window.setTimeout(focusFirstElement, 0)
    window.addEventListener('keydown', handleKeyDown)

    return () => {
      window.clearTimeout(focusTimeoutId)
      window.removeEventListener('keydown', handleKeyDown)
      previouslyFocusedRef.current?.focus?.()
    }
  }, [onClose])

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        ref={panelRef}
        className="modal-panel xbox-title-history-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="xbox-title-history-title"
      >
        <header className="modal-header">
          <div>
            <span>Xbox / Game Pass</span>
            <h2 id="xbox-title-history-title">Importar titulos com progresso</h2>
          </div>
          <button
            ref={closeButtonRef}
            className="icon-button"
            type="button"
            aria-label="Fechar importacao de titulos do Xbox"
            title="Fechar"
            onClick={onClose}
            disabled={isBusy}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <div className="xbox-title-history-content">
          <p className="xbox-title-history-intro">
            Esta leitura pega os titulos em que o usuario ja tem progresso em achievements e os adiciona a biblioteca,
            mesmo antes da descoberta local do Xbox encontrar a instalacao. Isso nao confirma posse do jogo.
          </p>

          <div className="xbox-title-history-warning" role="note" aria-label="Aviso sobre Microsoft Store">
            <AlertTriangle size={18} aria-hidden="true" />
            <p>
              As entradas importadas devem abrir na pagina do jogo na Microsoft Store. Instale por la e depois use
              <strong> Sincronizar local</strong> para o Xbox reconhecer o jogo instalado.
            </p>
          </div>

          <ul className="xbox-title-history-list">
            <li>Importa apenas titulos que aparecem no historico de progresso.</li>
            <li>Historico de achievements nao e prova de posse ou licenca do jogo.</li>
            <li>Mantem a descoberta local do Xbox separada do sync de instalados.</li>
            <li>Nao altera o comportamento atual do botao <strong>Sincronizar local</strong>.</li>
          </ul>

          <ProviderFeedback
            defaultMessage="Confirme o aviso antes de iniciar a importacao."
            errorFeedback={errorFeedback}
            id="xbox-title-history-feedback"
            statusMessage={statusMessage}
          />
        </div>

        <div className="modal-actions">
          <button className="secondary-button" type="button" disabled={isBusy} onClick={onClose}>
            Cancelar
          </button>
          <button className="primary-button" type="button" disabled={isBusy || !isConfigured} onClick={onConfirm}>
            {isBusy ? 'Importando' : 'Confirmar importacao'}
          </button>
        </div>
      </section>
    </div>
  )
}

export default AccountsSettingsPage
