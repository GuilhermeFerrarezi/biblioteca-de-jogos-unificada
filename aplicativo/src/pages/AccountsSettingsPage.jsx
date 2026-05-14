import {
  ArrowLeft,
  CheckCircle2,
  CircleDot,
  Clock3,
  Cloud,
  Gamepad2,
  KeyRound,
  Store,
} from 'lucide-react'

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

function AccountsSettingsPage({ feedbackMessage, isSteamSyncing, onBackToLibrary, onSyncSteamGames }) {
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
            <strong>Nenhum segredo solicitado</strong>
            <p>Esta tela ainda nao pede API key, token ou senha.</p>
          </div>
          <div className="summary-row">
            <KeyRound size={18} aria-hidden="true" />
            <span>Credenciais: nao configuradas</span>
          </div>
          <div className="summary-row">
            <Cloud size={18} aria-hidden="true" />
            <span>Web API Steam: proxima etapa</span>
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
