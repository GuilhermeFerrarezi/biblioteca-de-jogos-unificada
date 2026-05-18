import { X } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { INSTALL_STATUS } from '../constants/libraryConstants'

function ManualGameModal({
  form,
  isEditing,
  errors,
  onChange,
  onClearErrors,
  onClose,
  onSubmit,
}) {
  const panelRef = useRef(null)
  const titleInputRef = useRef(null)
  const previouslyFocusedRef = useRef(null)

  useEffect(() => {
    previouslyFocusedRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null

    const focusFirstElement = () => {
      const focusableElements = panelRef.current?.querySelectorAll(
        'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
      )
      const firstFocusable = focusableElements?.[0] ?? titleInputRef.current

      if (firstFocusable instanceof HTMLElement) {
        firstFocusable.focus()
      }
    }

    const handleKeyDown = (event) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onClose()
        return
      }

      if (event.key !== 'Tab') {
        return
      }

      const focusableElements = Array.from(
        panelRef.current?.querySelectorAll(
          'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
        ) ?? [],
      ).filter((element) => element instanceof HTMLElement)

      if (focusableElements.length === 0) {
        return
      }

      const firstFocusable = focusableElements[0]
      const lastFocusable = focusableElements[focusableElements.length - 1]

      if (event.shiftKey && document.activeElement === firstFocusable) {
        event.preventDefault()
        lastFocusable.focus()
        return
      }

      if (!event.shiftKey && document.activeElement === lastFocusable) {
        event.preventDefault()
        firstFocusable.focus()
      }
    }

    const focusTimeoutId = window.setTimeout(focusFirstElement, 0)
    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.clearTimeout(focusTimeoutId)
      window.removeEventListener('keydown', handleKeyDown)
      previouslyFocusedRef.current?.focus?.()
    }
  }, [onClose])

  return (
    <div className="modal-backdrop" role="presentation">
      <section ref={panelRef} className="modal-panel" role="dialog" aria-modal="true" aria-labelledby="manual-game-title">
        <header className="modal-header">
          <div>
            <span>Cadastro manual</span>
            <h2 id="manual-game-title">{isEditing ? 'Editar jogo' : 'Adicionar jogo'}</h2>
          </div>
          <button className="icon-button" type="button" aria-label="Fechar cadastro" title="Fechar" onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <form className="manual-game-form" onSubmit={onSubmit}>
          <label>
            <span>Titulo</span>
            <input
              type="text"
              value={form.title}
              ref={titleInputRef}
              aria-invalid={Boolean(errors.title)}
              aria-describedby={errors.title ? 'manual-game-title-error' : undefined}
              onChange={(event) => {
                onChange((currentForm) => ({ ...currentForm, title: event.target.value }))
                onClearErrors({})
              }}
            />
            {errors.title ? <span className="field-error" id="manual-game-title-error">{errors.title}</span> : null}
          </label>

          <label>
            <span>Genero</span>
            <input
              type="text"
              value={form.genre}
              onChange={(event) => onChange((currentForm) => ({ ...currentForm, genre: event.target.value }))}
            />
          </label>

          <fieldset>
            <legend>Status</legend>
            <div className="segmented-control">
              <label>
                <input
                  type="radio"
                  name="installStatus"
                  value={INSTALL_STATUS.INSTALLED}
                  checked={form.installStatus === INSTALL_STATUS.INSTALLED}
                  onChange={() => {
                    onChange((currentForm) => ({ ...currentForm, installStatus: INSTALL_STATUS.INSTALLED }))
                    onClearErrors({})
                  }}
                />
                Instalado
              </label>
              <label>
                <input
                  type="radio"
                  name="installStatus"
                  value={INSTALL_STATUS.NOT_INSTALLED}
                  checked={form.installStatus === INSTALL_STATUS.NOT_INSTALLED}
                  onChange={() => {
                    onChange((currentForm) => ({ ...currentForm, installStatus: INSTALL_STATUS.NOT_INSTALLED }))
                    onClearErrors({})
                  }}
                />
                Nao instalado
              </label>
            </div>
            {errors.installStatus ? <span className="field-error">{errors.installStatus}</span> : null}
          </fieldset>

          <label>
            <span>Acao de lancamento</span>
            <input
              type="text"
              value={form.launchTarget}
              aria-invalid={Boolean(errors.launchTarget)}
              aria-describedby={errors.launchTarget ? 'manual-game-launch-error' : undefined}
              onChange={(event) => {
                onChange((currentForm) => ({ ...currentForm, launchTarget: event.target.value }))
                onClearErrors({})
              }}
            />
            {errors.launchTarget ? <span className="field-error" id="manual-game-launch-error">{errors.launchTarget}</span> : null}
          </label>

          {Object.keys(errors).length > 0 ? <p className="form-error">Revise os campos destacados.</p> : null}

          <div className="modal-actions">
            <button className="secondary-button" type="button" onClick={onClose}>
              Cancelar
            </button>
            <button className="primary-button" type="submit">
              {isEditing ? 'Salvar alterações' : 'Salvar'}
            </button>
          </div>
        </form>
      </section>
    </div>
  )
}

export default ManualGameModal
