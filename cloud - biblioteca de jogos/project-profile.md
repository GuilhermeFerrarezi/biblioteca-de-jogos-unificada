# Project Profile

Use this file as the source of truth for the current project.

## Required

- Project name: Biblioteca de Jogos Unificada
- Product type: Desktop application
- Domain: Game library management
- Primary users: Windows desktop users who keep a unified game library
- Platforms: Windows desktop
- Delivery model: Installed desktop app with local storage
- Main goal: Unify game libraries, accounts, installed games and launch actions in one desktop app
- Known constraints: Offline-first behavior, safe local launching, provider differences and Windows-specific integration limits
- Known risks: Platform compliance, local data migration, provider changes and unsafe process launch
- Stack: Tauri 2, Rust, React 18, JavaScript/JSX, Vite, SQLite
- Architecture: UI shell, frontend services/adapters/hooks, backend Tauri commands, local SQLite database, provider layer and launcher layer
- Required checks: npm run lint, npm run build, cargo test
- Release criteria: App opens on the library, manual games persist across restart, local launch stays safe, and provider errors do not break the library

## Optional

- Secondary goals: Fast search, clear filters, safe account flows and future platform expansion
- Non-goals: Cloud sync as first release, password capture, shell-based launch
- Success metrics: Stable startup, correct persistence, and low-friction game launch
- Current stage: Active local development
- External dependencies: Steam, Windows file system, WebView2, Rust toolchain and Visual Studio Build Tools
- Compliance requirements: Platform API rules, safe process launch, token handling and local data privacy
- Data storage: SQLite for app data and secure local vault for secrets
- Integrations: Steam OpenID, Steam Web API, local game discovery, Steam launcher protocol, Xbox local discovery
- Deployment: Desktop installer
- Priority 1: Reliable unified library
- Priority 2: Safe local launch and persistence
- Priority 3: Platform expansion and metadata quality
- Canonical project terms: Library entry, launch action, provider, account config, sync summary
- Terms to avoid: Ad hoc naming that mixes UI state with domain state
- Naming conventions: Stable IDs, explicit action names and normalized provider IDs
- QA focus: Persistence, launch safety, provider error handling and UI state coverage
- Open questions: Epic and other third-party providers, query scaling for larger libraries
- Decisions already made: Dark mode by default, library-first UI, JavaScript/JSX frontend, Rust backend, SQLite persistence
- Documentation to keep updated: CHECKPOINT.md, DIRETRIZES_DESENVOLVIMENTO.md, ESTRUTURA_BANCO_DADOS.md, RETOMADA_NOVO_COMPUTADOR.md

## Profile Summary

- One-line project summary: A Windows desktop app for unifying game libraries with safe local launch and local persistence.
- Main delivery risk: Provider differences and unsafe launch behavior.
- Main quality bar: Stable library state across restart.
- Main release blocker: Persistence and launcher safety must be reliable.
