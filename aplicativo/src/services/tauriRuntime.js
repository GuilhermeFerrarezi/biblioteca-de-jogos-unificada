export const hasTauriRuntime = () =>
  typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

export const hasLocalStorage = () =>
  typeof window !== 'undefined' && typeof window.localStorage !== 'undefined'
