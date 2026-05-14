import { CircleDot, Gamepad2, HardDrive, Library, Settings } from 'lucide-react'

const navItems = Object.freeze([
  { id: 'all', label: 'Biblioteca', icon: Library },
  { id: 'steam', label: 'Steam', icon: CircleDot },
  { id: 'local', label: 'Locais', icon: HardDrive },
])

function Sidebar({ activeSection, quickFilter, onFilterChange, onAccountsClick }) {
  const isAccountsActive = activeSection === 'accounts'

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

      <nav className="nav-list" aria-label="Filtros da biblioteca" role="navigation">
        {navItems.map((item) => {
          const Icon = item.icon

          return (
            <button
              className={activeSection === 'library' && quickFilter === item.id ? 'nav-item active' : 'nav-item'}
              type="button"
              key={item.id}
              aria-pressed={activeSection === 'library' && quickFilter === item.id}
              onClick={() => onFilterChange(item.id)}
            >
              <Icon size={18} aria-hidden="true" />
              {item.label}
            </button>
          )
        })}
        <button
          className={isAccountsActive ? 'nav-item active' : 'nav-item'}
          type="button"
          aria-pressed={isAccountsActive}
          onClick={onAccountsClick}
        >
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
