import {
  ArrowLeft,
  AlertCircle,
  CheckCircle2,
  CircleDot,
  Clock3,
  Cloud,
  LogIn,
  Gamepad2,
  KeyRound,
  LockKeyhole,
  Save,
  Trash2,
  Store,
  UserRound,
} from 'lucide-react'
import { listen } from '@tauri-apps/api/event'
import { useEffect, useMemo, useState } from 'react'
import {
  deleteSteamApiKey,
  getSteamAccountConfig,
  getSteamApiKeyStatus,
  saveSteamApiKey,
  saveSteamAccountConfig,
  startSteamLogin,
} from '../services/libraryService'

const emptySteamApiKeyForm = Object.freeze({
  apiKey: '',
})

const emptySteamAccountForm = Object.freeze({
  steamId64: '',
})

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

const accountProviders = Object.freeze([
  {
    id: 'steam',
    name: 'Steam',
    icon: CircleDot,
    state: 'Sync local e por conta',
    tone: 'ready',
    detail: 'Manifestos instalados continuam disponiveis sem credencial.',
    nextStep: 'A conta usa SteamID64 e AuthVault configurado.',
  },
  {
    id: 'xbox',
    name: 'Xbox / Game Pass',
    icon: Gamepad2,
    state: 'Planejado',
    tone: 'planned',
    detail: 'Area preparada para conta e catalogo.',
    nextStep: 'Sem credenciais nesta versao.',
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

function AccountsSettingsPage({
  feedbackMessage,
  isSteamAccountSyncing,
  isSteamSyncing,
  onBackToLibrary,
  onSyncSteamAccountGames,
  onSyncSteamGames,
}) {
  const [steamApiKeyForm, setSteamApiKeyForm] = useState(emptySteamApiKeyForm)
  const [steamAccountForm, setSteamAccountForm] = useState(emptySteamAccountForm)
  const [steamApiKeyConfigured, setSteamApiKeyConfigured] = useState(false)
  const [steamApiKeyStatusMessage, setSteamApiKeyStatusMessage] = useState('')
  const [steamApiKeyError, setSteamApiKeyError] = useState('')
  const [steamAccountStatusMessage, setSteamAccountStatusMessage] = useState('')
  const [steamAccountError, setSteamAccountError] = useState('')
  const [isSteamAccountConnected, setIsSteamAccountConnected] = useState(false)
  const [isSteamLoginStarting, setIsSteamLoginStarting] = useState(false)
  const [isSteamApiKeyLoading, setIsSteamApiKeyLoading] = useState(true)
  const [isSteamApiKeySaving, setIsSteamApiKeySaving] = useState(false)
  const [isSteamApiKeyDeleting, setIsSteamApiKeyDeleting] = useState(false)

  const steamApiKeyStatusLabel = useMemo(
    () => {
      if (!steamApiKeyConfigured) {
        return 'Cofre nao configurado'
      }

      return 'Cofre configurado'
    },
    [steamApiKeyConfigured],
  )
  const isSteamApiKeyBusy = isSteamApiKeyLoading || isSteamApiKeySaving || isSteamApiKeyDeleting
  const steamId64Error = validateSteamId64Input(steamAccountForm.steamId64)
  const canSyncSteamAccount = steamApiKeyConfigured && isSteamAccountConnected && !steamId64Error

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
      } catch {
        if (isMounted) {
          setSteamAccountError('Nao foi possivel consultar a conta Steam conectada.')
        }
      }
    }

    void loadSteamAccountConfig()

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
            setSteamAccountError('')
            setSteamAccountStatusMessage(payload.message || 'Conta Steam conectada neste dispositivo.')
            return
          }

          setIsSteamAccountConnected(false)
          setSteamAccountError(payload.message || 'Nao foi possivel concluir o login Steam.')
        })
      } catch {
        if (isMounted) {
          setSteamAccountError('Nao foi possivel acompanhar o retorno do login Steam.')
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
      setSteamApiKeyError('')

      try {
        const status = await getSteamApiKeyStatus()

        if (!isMounted) {
          return
        }

        const isConfigured = normalizeSteamApiKeyStatus(status)

        setSteamApiKeyConfigured(isConfigured)
        setSteamApiKeyStatusMessage(
          isConfigured ? 'Cofre configurado.' : 'Cofre nao configurado.',
        )
      } catch {
        if (isMounted) {
          setSteamApiKeyError('Nao foi possivel consultar o status do cofre Steam Web API.')
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

  const handleSteamApiKeyChange = (event) => {
    setSteamApiKeyForm({ apiKey: event.target.value })
    setSteamApiKeyError('')
  }

  const handleSteamId64Change = (event) => {
    const steamId64 = event.target.value.replace(/\D/g, '').slice(0, 17)

    setSteamAccountForm({ steamId64 })
    setSteamAccountError('')
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
      setSteamAccountError('')
      setSteamAccountStatusMessage('SteamID64 salvo no banco local.')
    } catch {
      setIsSteamAccountConnected(false)
      setSteamAccountError('Nao foi possivel salvar o SteamID64 no banco local.')
    }
  }

  const handleSteamLoginStart = async () => {
    setIsSteamLoginStarting(true)
    setSteamAccountError('')
    setSteamAccountStatusMessage('Abrindo login oficial da Steam no navegador.')

    try {
      await startSteamLogin()
      setSteamAccountStatusMessage('Conclua o login no navegador para conectar a conta.')
    } catch {
      setIsSteamLoginStarting(false)
      setSteamAccountError('Nao foi possivel iniciar o login oficial da Steam.')
    }
  }

  const handleSteamAccountSync = async () => {
    const validationError = validateSteamId64Input(steamAccountForm.steamId64)

    if (validationError) {
      setSteamAccountError(validationError)
      return
    }

    if (!steamApiKeyConfigured) {
      setSteamAccountError('Configure o AuthVault antes de sincronizar a conta.')
      return
    }

    setSteamAccountError('')
    setSteamAccountStatusMessage('Sincronizacao por conta em andamento.')
    if (!isSteamAccountConnected) {
      setSteamAccountError('Conecte a conta Steam antes de sincronizar pela Web API.')
      return
    }

    try {
      await onSyncSteamAccountGames()
      setSteamAccountStatusMessage('Sincronizacao por conta finalizada.')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSteamAccountError(`Nao foi possivel sincronizar a conta Steam: ${message}`)
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
    setSteamApiKeyError('')
    setSteamApiKeyStatusMessage('Salvando credencial Steam Web API no AuthVault.')

    try {
      const status = await saveSteamApiKey(steamApiKeyForm.apiKey.trim())
      const isConfigured = normalizeSteamApiKeyStatus(status)
      setSteamApiKeyForm(emptySteamApiKeyForm)
      setSteamApiKeyConfigured(isConfigured)
      setSteamApiKeyStatusMessage(isConfigured ? 'AuthVault configurado.' : 'Cofre nao configurado.')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSteamApiKeyError(`Nao foi possivel configurar o AuthVault: ${message}`)
    } finally {
      setIsSteamApiKeySaving(false)
    }
  }

  const handleSteamApiKeyDelete = async () => {
    setIsSteamApiKeyDeleting(true)
    setSteamApiKeyError('')
    setSteamApiKeyStatusMessage('Removendo credencial Steam Web API do AuthVault.')

    try {
      await deleteSteamApiKey()
      setSteamApiKeyForm(emptySteamApiKeyForm)
      setSteamApiKeyConfigured(false)
      setSteamApiKeyStatusMessage('AuthVault nao configurado.')
    } catch {
      setSteamApiKeyError('Nao foi possivel limpar o AuthVault.')
    } finally {
      setIsSteamApiKeyDeleting(false)
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
              isSteamAccountSyncing={isSteamAccountSyncing}
              isSteamSyncing={isSteamSyncing}
              key={provider.id}
              provider={provider}
              steamAccountDisabledReason={
                !steamApiKeyConfigured
                  ? 'Configure o AuthVault para sincronizar a conta.'
                  : !isSteamAccountConnected
                    ? 'Conecte a conta Steam para sincronizar.'
                  : steamId64Error || ''
              }
              isSteamLoginStarting={isSteamLoginStarting}
              isSteamAccountConnected={isSteamAccountConnected}
              onStartSteamLogin={handleSteamLoginStart}
              onSyncSteamAccountGames={handleSteamAccountSync}
              onSyncSteamGames={onSyncSteamGames}
            />
          ))}

          <SteamAccountPanel
            errorMessage={steamAccountError}
            form={steamAccountForm}
            isBusy={isSteamAccountSyncing}
            isConfigured={!steamId64Error}
            statusMessage={steamAccountStatusMessage}
            onChange={handleSteamId64Change}
            onSave={handleSteamAccountSave}
          />

          <SteamWebApiVaultPanel
            form={steamApiKeyForm}
            isBusy={isSteamApiKeyBusy}
            isDeleting={isSteamApiKeyDeleting}
            isLoading={isSteamApiKeyLoading}
            isSaving={isSteamApiKeySaving}
            isConfigured={steamApiKeyConfigured}
            statusLabel={steamApiKeyStatusLabel}
            statusMessage={steamApiKeyStatusMessage}
            errorMessage={steamApiKeyError}
            onChange={handleSteamApiKeyChange}
            onDelete={handleSteamApiKeyDelete}
            onSubmit={handleSteamApiKeySubmit}
          />
        </section>

        <aside className="accounts-summary" aria-label="Estado das integracoes">
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
            <div className="account-feedback" role="status" aria-live="polite">
              {feedbackMessage}
            </div>
          ) : null}
        </aside>
      </div>
    </section>
  )
}

function SteamWebApiVaultPanel({
  errorMessage,
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
          <LockKeyhole size={22} />
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
            aria-invalid={errorMessage ? 'true' : 'false'}
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

        <div
          id="steam-api-key-feedback"
          className={errorMessage ? 'steam-api-key-feedback error' : 'steam-api-key-feedback'}
          role="status"
          aria-live="polite"
        >
          {errorMessage || statusMessage || 'O status do cofre sera mostrado aqui.'}
        </div>
      </form>
    </article>
  )
}

function SteamAccountPanel({ errorMessage, form, isBusy, isConfigured, onChange, onSave, statusMessage }) {
  return (
    <article className="steam-account-panel" aria-labelledby="steam-account-title" aria-busy={isBusy}>
      <div className="steam-api-vault-heading">
        <div className="account-provider-icon" aria-hidden="true">
          <UserRound size={22} />
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
          aria-invalid={errorMessage ? 'true' : 'false'}
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
      <div
        id="steam-id64-feedback"
        className={errorMessage ? 'steam-api-key-feedback error' : 'steam-api-key-feedback'}
        role="status"
        aria-live="polite"
      >
        {errorMessage || statusMessage || 'Informe o SteamID64 para liberar a sincronizacao por conta.'}
      </div>
    </article>
  )
}

function ProviderAccountRow({
  canSyncSteamAccount,
  isSteamAccountConnected,
  isSteamAccountSyncing,
  isSteamLoginStarting,
  isSteamSyncing,
  onStartSteamLogin,
  onSyncSteamAccountGames,
  onSyncSteamGames,
  provider,
  steamAccountDisabledReason,
}) {
  const Icon = provider.icon
  const isSteam = provider.id === 'steam'
  const isPlanned = provider.tone === 'planned'

  return (
    <article className="account-row" aria-busy={isSteam ? isSteamSyncing || isSteamAccountSyncing : undefined}>
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
        <div className="account-actions">
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
      ) : (
        <button className="secondary-button" type="button" disabled={isPlanned}>
          Em breve
        </button>
      )}
    </article>
  )
}

export default AccountsSettingsPage
