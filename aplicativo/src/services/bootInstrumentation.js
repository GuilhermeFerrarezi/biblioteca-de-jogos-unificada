import { invoke } from '@tauri-apps/api/core'

const BOOT_LOG_PREFIX = '[library-boot]'
const bootStartMs =
  typeof window !== 'undefined' && typeof window.__BJU_BOOT_STARTED_MS === 'number'
    ? window.__BJU_BOOT_STARTED_MS
    : typeof performance !== 'undefined'
      ? performance.now()
      : Date.now()
const flushedEarlyBootMarks = new Set()

const isBootInstrumentationEnabled = () =>
  Boolean(import.meta.env.DEV && typeof console !== 'undefined' && typeof console.info === 'function')

const sanitizeBootDetailValue = (value) => {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? Math.round(value) : null
  }

  if (typeof value === 'boolean') {
    return value
  }

  if (typeof value === 'string') {
    return value.replace(/[^a-zA-Z0-9_.:-]/g, '').slice(0, 48)
  }

  return null
}

export const markBootStep = (step, details = {}) => {
  if (!isBootInstrumentationEnabled()) {
    return
  }

  const elapsedMs =
    typeof details.elapsedMs === 'number'
      ? details.elapsedMs
      : Math.round((typeof performance !== 'undefined' ? performance.now() : Date.now()) - bootStartMs)
  const sanitizedDetails = Object.fromEntries(
    Object.entries(details)
      .filter(([key]) => key !== 'elapsedMs')
      .map(([key, value]) => [key, sanitizeBootDetailValue(value)])
      .filter(([, value]) => value !== null),
  )

  console.info(BOOT_LOG_PREFIX, {
    step,
    elapsedMs,
    ...sanitizedDetails,
  })

  if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
    void invoke('record_boot_marker', {
      input: {
        step,
        elapsedMs,
        details: sanitizedDetails,
      },
    }).catch(() => {})
  }
}

export const flushEarlyBootMarks = () => {
  if (typeof window === 'undefined' || !Array.isArray(window.__BJU_BOOT_MARKS)) {
    return
  }

  window.__BJU_BOOT_MARKS.forEach((mark, index) => {
    const key = `${index}:${mark?.step ?? ''}:${mark?.elapsedMs ?? ''}`

    if (flushedEarlyBootMarks.has(key)) {
      return
    }

    flushedEarlyBootMarks.add(key)
    markBootStep(mark?.step ?? 'document.early_mark', { elapsedMs: Number(mark?.elapsedMs ?? 0) })
  })
}
