import { Component } from 'react'

class ErrorBoundary extends Component {
  constructor(props) {
    super(props)
    this.state = { hasError: false }
  }

  static getDerivedStateFromError() {
    return { hasError: true }
  }

  render() {
    if (this.state.hasError) {
      return (
        <main className="app-shell single-panel">
          <section className="error-panel" role="alert">
            <span className="platform-label">Biblioteca</span>
            <h1>Nao foi possivel renderizar a interface.</h1>
            <p>Feche e abra o aplicativo novamente. Se o erro persistir, rode as validacoes do projeto.</p>
          </section>
        </main>
      )
    }

    return this.props.children
  }
}

export default ErrorBoundary
