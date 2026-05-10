export type PlatformId =
  | 'steam'
  | 'xbox'
  | 'epic'
  | 'gog'
  | 'itch'
  | 'battle-net'
  | 'ubisoft'
  | 'ea'
  | 'local'
  | 'manual'

export type ProviderAuthStatus =
  | 'connected'
  | 'disconnected'
  | 'expired'
  | 'syncing'
  | 'rate_limited'
  | 'unsupported'
  | 'needs_user_action'

export type PlatformAccount = {
  id: string
  platformId: PlatformId
  displayName: string
  authStatus: ProviderAuthStatus
  userHandle?: string
  lastSyncedAt?: string
  isExperimental?: boolean
}
