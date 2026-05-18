function StatsGrid({ entriesCount, installedCount, totalHours }) {
  const gameLabel = entriesCount === 1 ? 'jogo' : 'jogos'
  const installedLabel = installedCount === 1 ? 'instalado' : 'instalados'

  return (
    <section className="stats-grid" aria-label="Resumo da biblioteca">
      <MetricItem label={`Total de ${gameLabel}`} value={entriesCount} />
      <MetricItem label={installedLabel[0].toUpperCase() + installedLabel.slice(1)} value={installedCount} />
      <MetricItem label="Horas jogadas" value={`${totalHours}h`} />
    </section>
  )
}

function MetricItem({ label, value }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  )
}

export default StatsGrid
