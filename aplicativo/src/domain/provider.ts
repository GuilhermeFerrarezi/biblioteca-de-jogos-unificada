import type { Game } from './game'
import type { LaunchAction } from './launchAction'
import type { PlatformAccount, PlatformId, ProviderAuthStatus } from './platformAccount'

export type ProviderSyncResult = {
  imported: Game[]
  updated: Game[]
  failed: ProviderError[]
}

export type ProviderError = {
  code: string
  message: string
  recoverable: boolean
}

export type Provider = {
  id: PlatformId
  displayName: string
  authStatus: ProviderAuthStatus
  account?: PlatformAccount
  syncLibrary: () => Promise<ProviderSyncResult>
  detectInstalledGames: () => Promise<Game[]>
  getLaunchActions: (game: Game) => Promise<LaunchAction[]>
  launch: (game: Game, action: LaunchAction) => Promise<void>
  refreshMetadata: (game: Game) => Promise<Game>
  disconnect: () => Promise<void>
}
