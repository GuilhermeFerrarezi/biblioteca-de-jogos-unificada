import { listen } from '@tauri-apps/api/event'
import { useEffect, useRef, useState } from 'react'
import { normalizeSteamEnrichmentStatus } from '../../domain/steamEnrichmentStatus.js'
import { markBootStep } from '../../services/bootInstrumentation.js'
import { hasTauriRuntime } from '../../services/tauriRuntime.js'

const STEAM_ENRICHMENT_STARTED_EVENT = 'steam-enrichment-started'
const STEAM_ENRICHMENT_PROGRESS_EVENT = 'steam-enrichment-progress'
const STEAM_ENRICHMENT_COMPLETED_EVENT = 'steam-enrichment-completed'
const STEAM_ENRICHMENT_FAILED_EVENT = 'steam-enrichment-failed'
const STEAM_ENRICHMENT_STATUS_TIMEOUT_MS = 6000

export function useSteamEnrichmentStatus() {
  const [steamEnrichmentStatus, setSteamEnrichmentStatus] = useState(null)
  const statusTimeoutRef = useRef(null)

  useEffect(() => {
    let isMounted = true
    const unlisteners = []

    const clearStatusTimeout = () => {
      if (statusTimeoutRef.current !== null) {
        clearTimeout(statusTimeoutRef.current)
        statusTimeoutRef.current = null
      }
    }

    const showStatus = (phase, payload) => {
      if (!isMounted) {
        return
      }

      markBootStep(`frontend.steam_enrichment.${phase}`, { critical: false })
      clearStatusTimeout()
      setSteamEnrichmentStatus(normalizeSteamEnrichmentStatus(phase, payload))

      if (phase === 'completed' || phase === 'failed') {
        statusTimeoutRef.current = window.setTimeout(() => {
          if (isMounted) {
            setSteamEnrichmentStatus(null)
          }
        }, STEAM_ENRICHMENT_STATUS_TIMEOUT_MS)
      }
    }

    const registerSteamEnrichmentListeners = async () => {
      if (!hasTauriRuntime()) {
        return
      }

      try {
        unlisteners.push(await listen(STEAM_ENRICHMENT_STARTED_EVENT, (event) => showStatus('running', event?.payload)))
        unlisteners.push(await listen(STEAM_ENRICHMENT_PROGRESS_EVENT, (event) => showStatus('running', event?.payload)))
        unlisteners.push(await listen(STEAM_ENRICHMENT_COMPLETED_EVENT, (event) => showStatus('completed', event?.payload)))
        unlisteners.push(await listen(STEAM_ENRICHMENT_FAILED_EVENT, (event) => showStatus('failed', event?.payload)))
      } catch {
        if (isMounted) {
          setSteamEnrichmentStatus(null)
        }
      }
    }

    void registerSteamEnrichmentListeners()

    return () => {
      isMounted = false
      clearStatusTimeout()
      for (const unlisten of unlisteners) {
        void unlisten()
      }
    }
  }, [])

  return steamEnrichmentStatus
}
