import { X } from 'lucide-react'

function ManualGameModal({
  form,
  isEditing,
  error,
  onChange,
  onClearError,
  onClose,
  onSubmit,
}) {
  return (
    <div className="modal-backdrop" role="presentation">
      <section className="modal-panel" role="dialog" aria-modal="true" aria-labelledby="manual-game-title">
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
              onChange={(event) => {
                onChange((currentForm) => ({ ...currentForm, title: event.target.value }))
                onClearError('')
              }}
              autoFocus
            />
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
                  value="installed"
                  checked={form.installStatus === 'installed'}
                  onChange={() => onChange((currentForm) => ({ ...currentForm, installStatus: 'installed' }))}
                />
                Instalado
              </label>
              <label>
                <input
                  type="radio"
                  name="installStatus"
                  value="not_installed"
                  checked={form.installStatus === 'not_installed'}
                  onChange={() => onChange((currentForm) => ({ ...currentForm, installStatus: 'not_installed' }))}
                />
                Nao instalado
              </label>
            </div>
          </fieldset>

          <label>
            <span>Acao de lancamento</span>
            <input
              type="text"
              value={form.launchTarget}
              onChange={(event) => onChange((currentForm) => ({ ...currentForm, launchTarget: event.target.value }))}
            />
          </label>

          {error ? <p className="form-error">{error}</p> : null}

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
