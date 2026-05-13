function StatsGrid({ entriesCount, installedCount, totalHours }) {
  return (
    <section className="stats-grid" aria-label="Resumo da biblioteca">
      <div className="metric">
        <span>Total</span>
        <strong>{entriesCount}</strong>
      </div>
      <div className="metric">
        <span>Instalados</span>
        <strong>{installedCount}</strong>
      </div>
      <div className="metric">
        <span>Horas jogadas</span>
        <strong>{totalHours}h</strong>
      </div>
    </section>
  )
}

export default StatsGrid
