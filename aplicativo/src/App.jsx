import ErrorBoundary from './components/ErrorBoundary'
import LibraryPage from './pages/LibraryPage'

function App() {
  return (
    <ErrorBoundary>
      <LibraryPage />
    </ErrorBoundary>
  )
}

export default App
