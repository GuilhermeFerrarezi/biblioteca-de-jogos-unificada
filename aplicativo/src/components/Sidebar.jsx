import { CircleDot, Gamepad2, HardDrive, Library, Settings } from 'lucide-react'

function Sidebar({ quickFilter, onFilterChange, onAccountsClick }) {
  return (
    <aside className="sidebar" aria-label="Navegacao principal">
      <div className="brand">
        <div className="brand-mark">
          <Gamepad2 size={22} aria-hidden="true" />
        </div>
        <div>
          <strong>Biblioteca</strong>
          <span>Jogos Unificados</span>
        </div>
      </div>

      <nav className="nav-list" aria-label="Filtros da biblioteca">
        <button
          className={quickFilter === 'all' ? 'nav-item active' : 'nav-item'}
          type="button"
          onClick={() => onFilterChange('all')}
        >
          <Library size={18} aria-hidden="true" />
          Biblioteca
        </button>
        <button
          className={quickFilter === 'steam' ? 'nav-item active' : 'nav-item'}
          type="button"
          onClick={() => onFilterChange('steam')}
        >
          <CircleDot size={18} aria-hidden="true" />
          Steam
        </button>
        <button
          className={quickFilter === 'local' ? 'nav-item active' : 'nav-item'}
          type="button"
          onClick={() => onFilterChange('local')}
        >
          <HardDrive size={18} aria-hidden="true" />
          Locais
        </button>
        <button className="nav-item" type="button" onClick={onAccountsClick}>
          <Settings size={18} aria-hidden="true" />
          Contas
        </button>
      </nav>

      <div className="sync-panel">
        <span>Ultima sincronizacao</span>
        <strong>Steam pendente</strong>
      </div>
    </aside>
  )
}

export default Sidebar
