import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { getCurrentWindow } from '@tauri-apps/api/window'
import './index.css'
import App from './App.jsx'
import { flushEarlyBootMarks, markBootStep } from './services/bootInstrumentation.js'

const markMainModuleResourceTiming = () => {
  const scriptResources = performance
    .getEntriesByType('resource')
    .filter((entry) => entry.initiatorType === 'script')
  const mainModuleResource = [...scriptResources]
    .reverse()
    .find((entry) => entry.name.includes('/src/main.jsx') || /\/assets\/index-[^/]+\.js$/.test(entry.name))

  if (!mainModuleResource) {
    return
  }

  markBootStep('renderer.main_module.resource', {
    startMs: mainModuleResource.startTime,
    responseEndMs: mainModuleResource.responseEnd,
    durationMs: mainModuleResource.duration,
  })
}

flushEarlyBootMarks()
markMainModuleResourceTiming()
markBootStep('renderer.main_module.executed')

const rootElement = document.getElementById('root')

const showMainWindowAfterFirstFrame = () => {
  if (!window.__TAURI_INTERNALS__) {
    return
  }

  markBootStep('renderer.main_window.show.requested')
  void getCurrentWindow()
    .show()
    .then(() => markBootStep('renderer.main_window.show.complete'))
    .catch(() => markBootStep('renderer.main_window.show.failed'))
}

markBootStep('react.root.create.start')
const root = createRoot(rootElement)
markBootStep('react.root.render.start')

root.render(
  <StrictMode>
    <App />
  </StrictMode>,
)

window.requestAnimationFrame(() => {
  flushEarlyBootMarks()
  showMainWindowAfterFirstFrame()
  markBootStep('react.first_frame.painted')
})
