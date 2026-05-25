import { CircleDot, Gamepad2, HardDrive, Heart, Library, Settings } from 'lucide-react'
import { QUICK_FILTER_IDS } from '../constants/libraryConstants'
import SteamIcon from './icons/SteamIcon'
import XboxIcon from './icons/XboxIcon'

const navItems = Object.freeze([
  { id: QUICK_FILTER_IDS.ALL, label: 'Todos', icon: Library },
  { id: QUICK_FILTER_IDS.FAVORITES, label: 'Favoritos', icon: Heart },
  { id: QUICK_FILTER_IDS.INSTALLED, label: 'Instalados', icon: CircleDot },
  { id: QUICK_FILTER_IDS.NOT_INSTALLED, label: 'Nao instalados', icon: HardDrive },
  { id: QUICK_FILTER_IDS.STEAM, label: 'Steam', icon: SteamIcon },
  { id: QUICK_FILTER_IDS.XBOX, label: 'Xbox', icon: XboxIcon },
  { id: QUICK_FILTER_IDS.LOCAL, label: 'Locais', icon: HardDrive },
])

function Sidebar({ activeSection, quickFilters, onFilterChange, onAccountsClick }) {
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
          const isActive = isFilterActive(quickFilters, item.id)

          return (
            <button
              className={activeSection === 'library' && isActive ? 'nav-item active' : 'nav-item'}
              type="button"
              key={item.id}
              aria-pressed={activeSection === 'library' && isActive}
              onClick={() => onFilterChange(item.id)}
            >
              <Icon size={18} aria-hidden="true" />
              {item.label}
            </button>
          )
        })}
      </nav>

      <nav className="nav-list nav-list-bottom" aria-label="Configuracoes" role="navigation">
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
    </aside>
  )
}

function isFilterActive(quickFilters, filterId) {
  return filterId === QUICK_FILTER_IDS.ALL ? quickFilters.length === 0 : quickFilters.includes(filterId)
}

export default Sidebar
