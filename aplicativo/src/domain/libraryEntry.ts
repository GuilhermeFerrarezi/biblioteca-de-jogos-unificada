import type { Game } from './game'
import type { PlatformId } from './platformAccount'

export type InstallStatus = 'installed' | 'not_installed'

export type LibraryEntry = {
  id: string
  game: Game
  primaryPlatformId: PlatformId
  installStatus: InstallStatus
  lastPlayedLabel: string
  addedAt: string
  updatedAt: string
}
