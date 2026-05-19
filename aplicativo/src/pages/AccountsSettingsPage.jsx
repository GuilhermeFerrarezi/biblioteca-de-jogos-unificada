import {
  ArrowLeft,
  AlertCircle,
  CheckCircle2,
  ChevronDown,
  Clock3,
  Cloud,
  FolderOpen,
  KeyRound,
  LogIn,
  Save,
  Store,
  Trash2,
} from 'lucide-react'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useEffect, useMemo, useRef, useState } from 'react'
import SteamIcon from '../components/icons/SteamIcon'
import XboxIcon from '../components/icons/XboxIcon'
import {
  deleteSteamApiKey,
  getSteamAccountConfig,
  getSteamApiKeyStatus,
  getSteamLibraryRoots,
  normalizeProviderErrorFeedback,
  saveSteamAccountConfig,
  saveSteamApiKey,
  saveSteamLibraryRoots,
  startSteamLogin,
} from '../services/libraryService'

const emptySteamApiKeyForm = Object.freeze({
  apiKey: '',
})

const emptySteamAccountForm = Object.freeze({
  steamId64: '',
})

const emptySteamLibraryRootsForm = Object.freeze({
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
    detail: 'Instalados entram no app. Achievements sao apenas indicio auxiliar.',
    nextStep: 'Nao instalado: abrir Microsoft Store.',
  },
  {
    id: 'epic',
    name: 'Epic Games',
    icon: Store,
    state: 'Planejado',
    tone: 'planned',
    detail: 'Provider reservado para integracao futura.',
    nextStep: 'Sem segredo salvo agora.',
  },
])

const normalizeSteamApiKeyStatus = (status) => {
  if (typeof status === 'boolean') {
    return status
  }

  if (!status || typeof status !== 'object') {
    return false
  }

  return Boolean(status.configured)
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
  preferredStoreId,
  onBackToLibrary,
  onPreferredStoreChange,
  onSyncSteamAccountGames,
  onSyncSteamGames,
  onSyncXboxGames,
}) {
  const [isSteamPanelOpen, setIsSteamPanelOpen] = useState(false)
  const [steamApiKeyForm, setSteamApiKeyForm] = useState(emptySteamApiKeyForm)
  const [steamAccountForm, setSteamAccountForm] = useState(emptySteamAccountForm)
  const [steamLibraryRootsForm, setSteamLibraryRootsForm] = useState(emptySteamLibraryRootsForm)
  const [steamApiKeyConfigured, setSteamApiKeyConfigured] = useState(false)
  const [steamApiKeyStatusMessage, setSteamApiKeyStatusMessage] = useState('')
  const [steamApiKeyError, setSteamApiKeyError] = useState(null)
  const [steamAccountStatusMessage, setSteamAccountStatusMessage] = useState('')
  const [steamAccountError, setSteamAccountError] = useState(null)
  const [steamLibraryRootsStatusMessage, setSteamLibraryRootsStatusMessage] = useState('')
  const [steamLibraryRootsError, setSteamLibraryRootsError] = useState(null)
  const [isSteamAccountConnected, setIsSteamAccountConnected] = useState(false)
  const [isSteamLoginStarting, setIsSteamLoginStarting] = useState(false)
  const [isSteamApiKeyLoading, setIsSteamApiKeyLoading] = useState(true)
  const [isSteamApiKeySaving, setIsSteamApiKeySaving] = useState(false)
  const [isSteamApiKeyDeleting, setIsSteamApiKeyDeleting] = useState(false)
  const [isSteamLibraryRootsLoading, setIsSteamLibraryRootsLoading] = useState(true)
  const [isSteamLibraryRootsSaving, setIsSteamLibraryRootsSaving] = useState(false)

  const steamPanelRef = useRef(null)
  const steamToggleRef = useRef(null)
  const previousSteamPanelOpen = useRef(isSteamPanelOpen)

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
    let isMounted = true

    const loadSteamApiKeyStatus = async () => {
      setIsSteamApiKeyLoading(true)
      setSteamApiKeyError(null)

      try {
        const status = await getSteamApiKeyStatus()

        if (!isMounted) {
          return
        }

        const isConfigured = normalizeSteamApiKeyStatus(status)

        setSteamApiKeyConfigured(isConfigured)
        setSteamApiKeyStatusMessage(isConfigured ? 'Cofre configurado.' : 'Cofre nao configurado.')
      } catch (error) {
        if (isMounted) {
          setSteamApiKeyError(
            normalizeProviderErrorFeedback(
              error,
              'Nao foi possivel consultar o status do cofre Steam Web API.',
              'Consulta do AuthVault Steam Web API',
            ),
          )
        }
      } finally {
        if (isMounted) {
          setIsSteamApiKeyLoading(false)
        }
      }
    }

    void loadSteamApiKeyStatus()

    return () => {
      isMounted = false
    }
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

    setSteamAccountError(null)
    setSteamAccountStatusMessage('Sincronizacao por conta em andamento.')

    try {
      await onSyncSteamAccountGames()
      setSteamAccountStatusMessage('Sincronizacao por conta finalizada.')
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

  const handleSteamPanelToggle = () => {
    setIsSteamPanelOpen((currentValue) => !currentValue)
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
              isXboxSyncing={isXboxSyncing}
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
                ) : null
              }
              provider={provider}
              steamPanelContentId={steamPanelContentId}
              steamToggleRef={steamToggleRef}
              onSyncXboxGames={onSyncXboxGames}
              onToggleSteamPanel={handleSteamPanelToggle}
            />
          ))}
        </section>

        <aside className="accounts-summary" aria-label="Estado das integracoes">
          <LibraryDefaultsCard
            isLoading={isLibrarySettingsLoading}
            isSaving={isLibrarySettingsSaving}
            preferredStoreId={preferredStoreId}
            onPreferredStoreChange={onPreferredStoreChange}
          />
          <div>
            <span className="summary-kicker">Seguranca</span>
            <strong>AuthVault protege o segredo</strong>
            <p>A credencial salva nunca volta preenchida para a interface.</p>
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
    </section>
  )
}

function LibraryDefaultsCard({ isLoading, isSaving, preferredStoreId, onPreferredStoreChange }) {
  const normalizedPreferredStoreId = preferredStoreId === 'xbox' ? 'xbox' : 'steam'

  return (
    <section className="accounts-defaults-card" aria-labelledby="library-defaults-title" aria-busy={isLoading || isSaving}>
      <div className="summary-kicker">Padroes da biblioteca</div>
      <strong id="library-defaults-title">Loja principal</strong>
      <p>Define qual launcher sera usado por padrao quando o mesmo jogo existir em mais de uma loja.</p>

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
      <span className="steam-account-help">
        {normalizedPreferredStoreId === 'xbox'
          ? 'Ao clicar em Jogar, o app prioriza a versao do Xbox quando houver copia duplicada.'
          : 'Ao clicar em Jogar, o app prioriza a versao da Steam quando houver copia duplicada.'}
      </span>
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
          disabled={!canSyncSteamAccount || isSteamSyncing || isSteamAccountSyncing}
          onClick={onSyncSteamAccountGames}
        >
          {isSteamAccountSyncing ? 'Sincronizando conta' : 'Sincronizar conta'}
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
  isSteamAccountSyncing,
  isSteamPanelOpen,
  isSteamSyncing,
  isXboxSyncing,
  onSyncXboxGames,
  onToggleSteamPanel,
  panelContent,
  provider,
  steamPanelContentId,
  steamToggleRef,
}) {
  const Icon = provider.icon
  const isSteam = provider.id === 'steam'
  const isXbox = provider.id === 'xbox'
  const isPlanned = provider.tone === 'planned'

  return (
    <article
      className="account-row"
      aria-busy={isSteam ? isSteamSyncing || isSteamAccountSyncing : isXbox ? isXboxSyncing : undefined}
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
        <button className="secondary-button" type="button" disabled={isXboxSyncing} onClick={onSyncXboxGames}>
          {isXboxSyncing ? 'Sincronizando Xbox' : 'Sincronizar local'}
        </button>
      ) : (
        <button className="secondary-button" type="button" disabled={isPlanned}>
          Em breve
        </button>
      )}

      {panelContent}
    </article>
  )
}

export default AccountsSettingsPage
