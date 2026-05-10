import type { PlatformId } from './platformAccount'

export type LaunchActionKind = 'uri' | 'executable' | 'manual'

export type LaunchAction = {
  id: string
  platformId: PlatformId
  kind: LaunchActionKind
  label: string
  target: string
  arguments?: string[]
  workingDirectory?: string
  isPrimary: boolean
}
