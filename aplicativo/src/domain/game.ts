import type { LaunchAction } from './launchAction'
import type { PlatformId } from './platformAccount'

export type GameSource = {
  platformId: PlatformId
  externalId: string
  accountId?: string
}

export type Playtime = {
  totalMinutes: number
  recentMinutes?: number
}

export type GameArtwork = {
  coverUrl?: string
  heroUrl?: string
  iconUrl?: string
  accentColor?: string
}

export type AchievementsSummary = {
  unlocked: number
  total: number
}

export type GameUserOverrides = {
  title?: string
  coverUrl?: string
  tags?: string[]
  hidden?: boolean
}

export type Game = {
  internalId: string
  title: string
  sortTitle: string
  platforms: PlatformId[]
  sources: GameSource[]
  installed: boolean
  installLocations: string[]
  launchActions: LaunchAction[]
  playtime: Playtime
  achievementsSummary?: AchievementsSummary
  artwork: GameArtwork
  genres: string[]
  tags: string[]
  userOverrides: GameUserOverrides
}
