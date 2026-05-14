import {
  ArrowLeft,
  CheckCircle2,
  CircleDot,
  Clock3,
  Cloud,
  Gamepad2,
  KeyRound,
  LogOut,
  Store,
} from 'lucide-react'
import { useEffect, useId, useState } from 'react'
import {
  deleteSteamApiKey,
  disconnectSteamAccountSettings,
  getSteamAccountSettings,
  getSteamApiKeyStatus,
  saveSteamApiKey,
  saveSteamAccountSettings,
} from '../services/libraryService'

const STEAM_ID64_PATTERN = /^\d{17}$/
const STEAM_API_KEY_PATTERN = /^[a-fA-F0-9]{32}$/

const validateSteamId64 = (steamId64) => {
  if (!steamId64.trim()) {
    return 'Informe um SteamID64.'
  }

  if (!STEAM_ID64_PATTERN.test(steamId64.trim())) {
    return 'Use apenas 17 digitos numericos.'
  }

  return ''
}

const validateSteamApiKey = (apiKey) => {
  if (!apiKey.trim()) {
    return 'Informe a chave Web API.'
  }

  if (!STEAM_API_KEY_PATTERN.test(apiKey.trim())) {
    return 'Use uma chave hexadecimal de 32 caracteres.'
  }

  return ''
}

const accountProviders = Object.freeze([
  {
    id: 'steam',
    name: 'Steam',
    icon: CircleDot,
    state: 'Sync local ativo',
    tone: 'ready',
    detail: 'Manifestos instalados ja entram na biblioteca.',
    nextStep: 'Web API e a proxima etapa.',
    actionLabel: 'Sincronizar local',
    actionKind: 'primary',
  },
  {
    id: 'xbox',
    name: 'Xbox / Game Pass',
    icon: Gamepad2,
    state: 'Planejado',
    tone: 'planned',
    detail: 'Area preparada para conta e catalogo.',
    nextStep: 'Sem credenciais nesta versao.',
    actionLabel: 'Em breve',
    actionKind: 'disabled',
  },
  {
    id: 'epic',
    name: 'Epic Games',
    icon: Store,
    state: 'Planejado',
    tone: 'planned',
    detail: 'Provider reservado para integracao futura.',
    nextStep: 'Sem segredo salvo agora.',
    actionLabel: 'Em breve',
    actionKind: 'disabled',
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
  const steamIdInputId = useId()
  const steamIdErrorId = useId()
  const steamApiKeyInputId = useId()
  const steamApiKeyErrorId = useId()
  const [steamId64, setSteamId64] = useState('')
  const [steamIdError, setSteamIdError] = useState('')
  const [steamSettingsMessage, setSteamSettingsMessage] = useState('')
  const [steamApiKey, setSteamApiKey] = useState('')
  const [steamApiKeyError, setSteamApiKeyError] = useState('')
  const [steamApiKeyMessage, setSteamApiKeyMessage] = useState('')
  const [isSteamSettingsLoading, setIsSteamSettingsLoading] = useState(true)
  const [isSteamSettingsSaving, setIsSteamSettingsSaving] = useState(false)
  const [isSteamSettingsDisconnecting, setIsSteamSettingsDisconnecting] = useState(false)
  const [isSteamSettingsBackendAvailable, setIsSteamSettingsBackendAvailable] = useState(false)
  const [isSteamApiKeyLoading, setIsSteamApiKeyLoading] = useState(true)
  const [isSteamApiKeySaving, setIsSteamApiKeySaving] = useState(false)
  const [isSteamApiKeyDeleting, setIsSteamApiKeyDeleting] = useState(false)
  const [isSteamApiKeyBackendAvailable, setIsSteamApiKeyBackendAvailable] = useState(false)
  const [isSteamApiKeyConfigured, setIsSteamApiKeyConfigured] = useState(false)
  const [steamAuthState, setSteamAuthState] = useState('disconnected')
  const trimmedSteamId64 = steamId64.trim()
  const trimmedSteamApiKey = steamApiKey.trim()
  const isSteamConfigured = steamAuthState === 'configured' && Boolean(trimmedSteamId64)

  useEffect(() => {
    let isMounted = true

    const loadSteamSettings = async () => {
      setIsSteamSettingsLoading(true)

      try {
        const settings = await getSteamAccountSettings()

        if (!isMounted) {
          return
        }

        setSteamId64(settings.steamId64)
        setSteamAuthState(settings.authState)
        setIsSteamSettingsBackendAvailable(settings.isBackendAvailable)
        setSteamSettingsMessage(
          settings.isBackendAvailable
            ? 'Configuracao local Steam pronta.'
            : 'Comando de configuracao Steam aguardando backend.',
        )
      } catch {
        if (isMounted) {
          setSteamSettingsMessage('Nao foi possivel carregar a configuracao Steam.')
        }
      } finally {
        if (isMounted) {
          setIsSteamSettingsLoading(false)
        }
      }
    }

    void loadSteamSettings()

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    let isMounted = true

    const loadSteamApiKeyStatus = async () => {
      setIsSteamApiKeyLoading(true)

      try {
        const status = await getSteamApiKeyStatus()

        if (!isMounted) {
          return
        }

        setIsSteamApiKeyBackendAvailable(status.isBackendAvailable)
        setIsSteamApiKeyConfigured(status.isConfigured)
        setSteamApiKeyMessage(
          status.isBackendAvailable && status.isConfigured
            ? 'Chave salva no cofre. Por seguranca, o valor nao e exibido.'
            : status.isBackendAvailable
              ? 'Cofre Steam pronto. Salve a chave Web API para sincronizar a conta.'
            : 'Comando do AuthVault Steam aguardando backend.',
        )
      } catch {
        if (isMounted) {
          setSteamApiKeyMessage('Nao foi possivel consultar o cofre Steam.')
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

  const handleSteamId64Change = (event) => {
    const value = event.target.value.replace(/\D/g, '').slice(0, 17)

    setSteamId64(value)
    if (steamIdError) {
      setSteamIdError(validateSteamId64(value))
    }
  }

  const handleSteamSettingsSubmit = async (event) => {
    event.preventDefault()

    const validationError = validateSteamId64(trimmedSteamId64)

    if (validationError) {
      setSteamIdError(validationError)
      setSteamSettingsMessage('Revise o SteamID64 antes de salvar.')
      return
    }

    setSteamIdError('')
    setIsSteamSettingsSaving(true)
    setSteamSettingsMessage('Salvando configuracao Steam...')

    try {
      const result = await saveSteamAccountSettings({ steamId64: trimmedSteamId64 })

      setIsSteamSettingsBackendAvailable(result.isBackendAvailable)
      setSteamAuthState(result.authState)
      setSteamId64(result.steamId64 || trimmedSteamId64)
      setSteamSettingsMessage(
        result.saved
          ? 'SteamID64 salvo na configuracao local.'
          : 'Backend ainda nao expoe o comando de salvar SteamID64.',
      )
    } catch {
      setSteamSettingsMessage('Nao foi possivel salvar a configuracao Steam.')
    } finally {
      setIsSteamSettingsSaving(false)
    }
  }

  const handleSteamSettingsDisconnect = async () => {
    setIsSteamSettingsDisconnecting(true)
    setSteamSettingsMessage('Removendo configuracao local Steam...')

    try {
      const result = await disconnectSteamAccountSettings()

      setIsSteamSettingsBackendAvailable(result.isBackendAvailable)
      setSteamAuthState(result.authState)
      setSteamId64(result.steamId64)
      setSteamIdError('')
      setSteamSettingsMessage(
        result.disconnected
          ? 'Configuracao local Steam removida. Jogos importados foram preservados.'
          : 'Backend ainda nao expoe o comando de desconectar Steam.',
      )
    } catch {
      setSteamSettingsMessage('Nao foi possivel remover a configuracao Steam.')
    } finally {
      setIsSteamSettingsDisconnecting(false)
    }
  }

  const handleSteamApiKeyChange = (event) => {
    const value = event.target.value.replace(/\s/g, '').slice(0, 32)

    setSteamApiKey(value)
    if (steamApiKeyError) {
      setSteamApiKeyError(validateSteamApiKey(value))
    }
  }

  const handleSteamApiKeySubmit = async (event) => {
    event.preventDefault()

    const validationError = validateSteamApiKey(trimmedSteamApiKey)

    if (validationError) {
      setSteamApiKeyError(validationError)
      setSteamApiKeyMessage('Revise a chave antes de salvar no cofre.')
      return
    }

    setSteamApiKeyError('')
    setIsSteamApiKeySaving(true)
    setSteamApiKeyMessage('Salvando chave Steam no cofre seguro...')

    try {
      const result = await saveSteamApiKey({ apiKey: trimmedSteamApiKey })

      setIsSteamApiKeyBackendAvailable(result.isBackendAvailable)
      setIsSteamApiKeyConfigured(result.isConfigured)
      setSteamApiKey('')
      setSteamApiKeyMessage(
        result.saved
          ? 'Chave Steam salva no cofre seguro. Por seguranca, o campo foi limpo.'
          : 'Backend ainda nao expoe o comando de salvar chave Steam.',
      )
    } catch {
      setSteamApiKeyMessage('Nao foi possivel salvar a chave Steam no cofre.')
    } finally {
      setIsSteamApiKeySaving(false)
    }
  }

  const handleSteamApiKeyDelete = async () => {
    setIsSteamApiKeyDeleting(true)
    setSteamApiKeyMessage('Removendo chave Steam do cofre seguro...')

    try {
      const result = await deleteSteamApiKey()

      setIsSteamApiKeyBackendAvailable(result.isBackendAvailable)
      setIsSteamApiKeyConfigured(result.isConfigured)
      setSteamApiKey('')
      setSteamApiKeyError('')
      setSteamApiKeyMessage(
        result.deleted
          ? 'Chave Steam removida do cofre seguro.'
          : 'Backend ainda nao expoe o comando de remover chave Steam.',
      )
    } catch {
      setSteamApiKeyMessage('Nao foi possivel remover a chave Steam do cofre.')
    } finally {
      setIsSteamApiKeyDeleting(false)
    }
  }

  const isSteamSettingsBusy =
    isSteamSettingsLoading || isSteamSettingsSaving || isSteamSettingsDisconnecting
  const isSteamApiKeyBusy = isSteamApiKeyLoading || isSteamApiKeySaving || isSteamApiKeyDeleting
  const canSyncSteamAccount = isSteamConfigured && isSteamApiKeyConfigured && !isSteamApiKeyBusy

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
              isSteamSyncing={isSteamSyncing}
              key={provider.id}
              provider={provider}
              onSyncSteamGames={onSyncSteamGames}
            />
          ))}
        </section>

        <aside className="accounts-summary" aria-label="Estado das integracoes">
          <div>
            <span className="summary-kicker">Seguranca</span>
            <strong>Segredos ficam no cofre</strong>
            <p>SteamID64 e publico. A chave Web API fica no AuthVault e nao volta para a interface.</p>
          </div>
          <div className="summary-row">
            <KeyRound size={18} aria-hidden="true" />
            <span>
              Credenciais: {isSteamApiKeyConfigured ? 'cofre Steam configurado' : 'nao configuradas'}
            </span>
          </div>
          <div className="summary-row">
            <Cloud size={18} aria-hidden="true" />
            <span>
              Web API: {canSyncSteamAccount ? 'pronta para sincronizar' : 'aguardando SteamID64 e chave'}
            </span>
          </div>
          <div className="summary-row">
            <CheckCircle2 size={18} aria-hidden="true" />
            <span>
              {isSteamSettingsBackendAvailable
                ? `Configuracao Steam: ${isSteamConfigured ? 'local salva' : 'nao configurada'}`
                : 'Configuracao Steam: aguardando comandos'}
            </span>
          </div>
          {feedbackMessage ? (
            <div className="account-feedback" role="status" aria-live="polite">
              {feedbackMessage}
            </div>
          ) : null}
        </aside>
      </div>

      <section className="steam-settings-panel" aria-labelledby="steam-settings-title" aria-busy={isSteamSettingsBusy}>
        <div className="steam-settings-heading">
          <div>
            <span className="summary-kicker">Steam</span>
            <h2 id="steam-settings-title">SteamID64</h2>
            <p>Identifica a conta consultada pela sincronizacao Web API.</p>
          </div>
          <span className="account-status" data-tone={isSteamSettingsBackendAvailable ? 'ready' : 'planned'}>
            {isSteamSettingsBackendAvailable ? <CheckCircle2 size={14} aria-hidden="true" /> : <Clock3 size={14} aria-hidden="true" />}
            {isSteamSettingsBackendAvailable ? (isSteamConfigured ? 'Configuracao local' : 'Nao configurada') : 'Aguardando backend'}
          </span>
        </div>

        <form className="steam-settings-form" onSubmit={handleSteamSettingsSubmit}>
          <label htmlFor={steamIdInputId}>
            <span>SteamID64</span>
            <input
              id={steamIdInputId}
              inputMode="numeric"
              maxLength={17}
              pattern="[0-9]{17}"
              placeholder="76561198000000000"
              type="text"
              value={steamId64}
              aria-describedby={steamIdError ? steamIdErrorId : undefined}
              aria-invalid={steamIdError ? 'true' : 'false'}
              disabled={isSteamSettingsLoading}
              onChange={handleSteamId64Change}
            />
          </label>

          {steamIdError ? (
            <p className="form-error" id={steamIdErrorId}>
              {steamIdError}
            </p>
          ) : null}

          <div className="steam-settings-actions">
            <p role="status" aria-live="polite">
              {steamSettingsMessage}
            </p>
            <div className="steam-settings-buttons">
              <button
                className="secondary-button"
                type="button"
                disabled={isSteamSettingsBusy || !isSteamConfigured}
                onClick={handleSteamSettingsDisconnect}
              >
                <LogOut size={18} aria-hidden="true" />
                {isSteamSettingsDisconnecting ? 'Removendo' : 'Remover configuracao'}
              </button>
              <button className="primary-button" type="submit" disabled={isSteamSettingsBusy}>
                {isSteamSettingsSaving ? 'Salvando' : 'Salvar SteamID64'}
              </button>
            </div>
          </div>
        </form>
      </section>

      <section className="steam-settings-panel" aria-labelledby="steam-api-key-title" aria-busy={isSteamApiKeyBusy}>
        <div className="steam-settings-heading">
          <div>
            <span className="summary-kicker">AuthVault</span>
            <h2 id="steam-api-key-title">Steam Web API</h2>
            <p>Usa o cofre do sistema para sincronizar a biblioteca da conta pela Web API.</p>
          </div>
          <span className="account-status" data-tone={isSteamApiKeyBackendAvailable ? 'ready' : 'planned'}>
            {isSteamApiKeyBackendAvailable ? <CheckCircle2 size={14} aria-hidden="true" /> : <Clock3 size={14} aria-hidden="true" />}
            {isSteamApiKeyBackendAvailable
              ? isSteamApiKeyConfigured
                ? 'Cofre configurado'
                : 'Cofre vazio'
              : 'Aguardando backend'}
          </span>
        </div>

        <form className="steam-settings-form" onSubmit={handleSteamApiKeySubmit}>
          <label htmlFor={steamApiKeyInputId}>
            <span>Chave Web API</span>
            <input
              id={steamApiKeyInputId}
              autoComplete="off"
              inputMode="text"
              maxLength={32}
              placeholder={isSteamApiKeyConfigured ? 'Chave salva no cofre' : '32 caracteres hexadecimais'}
              type="password"
              value={steamApiKey}
              aria-describedby={steamApiKeyError ? steamApiKeyErrorId : undefined}
              aria-invalid={steamApiKeyError ? 'true' : 'false'}
              disabled={isSteamApiKeyLoading}
              onChange={handleSteamApiKeyChange}
            />
            {isSteamApiKeyConfigured ? (
              <small className="field-hint">Chave salva no cofre. O valor nao e carregado na tela.</small>
            ) : null}
          </label>

          {steamApiKeyError ? (
            <p className="form-error" id={steamApiKeyErrorId}>
              {steamApiKeyError}
            </p>
          ) : null}

          <div className="steam-settings-actions">
            <p role="status" aria-live="polite">
              {steamApiKeyMessage}
            </p>
            <div className="steam-settings-buttons">
              <button
                className="secondary-button"
                type="button"
                disabled={isSteamApiKeyBusy || !isSteamApiKeyConfigured}
                onClick={handleSteamApiKeyDelete}
              >
                <LogOut size={18} aria-hidden="true" />
                {isSteamApiKeyDeleting ? 'Removendo' : 'Remover chave'}
              </button>
              <button className="primary-button" type="submit" disabled={isSteamApiKeyBusy}>
                {isSteamApiKeySaving ? 'Salvando' : 'Salvar no cofre'}
              </button>
              <button
                className="primary-button"
                type="button"
                disabled={!canSyncSteamAccount || isSteamAccountSyncing}
                onClick={onSyncSteamAccountGames}
              >
                {isSteamAccountSyncing ? 'Sincronizando' : 'Sincronizar conta'}
              </button>
            </div>
          </div>
        </form>
      </section>
    </section>
  )
}

function ProviderAccountRow({ isSteamSyncing, provider, onSyncSteamGames }) {
  const Icon = provider.icon
  const isSteam = provider.id === 'steam'
  const isDisabled = provider.actionKind === 'disabled' || (isSteam && isSteamSyncing)

  return (
    <article className="account-row">
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

      <button
        className={provider.actionKind === 'primary' ? 'primary-button' : 'secondary-button'}
        type="button"
        aria-label={`${provider.actionLabel}: ${provider.name}`}
        disabled={isDisabled}
        onClick={isSteam ? onSyncSteamGames : undefined}
      >
        {isSteam && isSteamSyncing ? 'Sincronizando' : provider.actionLabel}
      </button>
    </article>
  )
}

export default AccountsSettingsPage
