import type { PlatformId } from './platformAccount'

export type SyncStatus = 'success' | 'partial' | 'failed'

export type SyncHistory = {
  id: string
  providerId: PlatformId
  startedAt: string
  finishedAt?: string
  status: SyncStatus
  importedCount: number
  updatedCount: number
  errorCode?: string
}
