# Ultimos Desenvolvimentos do Aplicativo

Data de referencia: 2026-05-14

Este arquivo resume os desenvolvimentos mais recentes criados no aplicativo. Para historico completo, use `CHECKPOINT.md`.

## Governanca de desenvolvimento

- As diretrizes agora exigem identificar o agente principal, agentes auxiliares e skills aplicaveis antes de qualquer novo desenvolvimento.
- Subagentes tecnicos podem ser usados para pesquisa, implementacao ou revisao, mas devem ser instruidos a seguir os agentes e skills definidos em `cloude teste/agents` e `cloude teste/skills`.
- O fluxo obrigatorio esta documentado em `DIRETRIZES_DESENVOLVIMENTO.md` e `cloude teste/README_PROJETO.md`.

## Frontend reorganizado

- `App.jsx` passou a renderizar a pagina principal em `pages/LibraryPage.jsx`.
- A UI foi separada em componentes como `Sidebar`, `Topbar`, `StatsGrid`, `LibraryBrowser`, `GameDetailsPanel` e `ManualGameModal`.
- A logica principal da tela foi movida para `hooks/useLibraryPageState.js`.
- A filtragem da biblioteca ficou em `hooks/useLibraryFiltering.js`.
- O service principal do frontend passou a ser `src/services/libraryService.js`.
- Constantes compartilhadas ficaram em `src/constants/libraryConstants.js`.
- Estilos da biblioteca foram concentrados em `src/styles/library.css`.

## Melhorias de UI e acessibilidade

- O app segue em modo escuro como experiencia padrao.
- A biblioteca abre com visualizacao por capas e alternativa em lista.
- Controles selecionaveis usam estados acessiveis como `aria-pressed`.
- Processos assincronos usam `aria-busy` e feedback com `role="status"`/`aria-live`.
- O modal manual fecha com `Escape` e marca campos invalidos.
- Foi adicionado `ErrorBoundary` para reduzir queda total da interface em erros de renderizacao.
- O botao de filtros executa uma acao real de limpar filtros.

## Persistencia SQLite

- O backend Tauri usa SQLite local em `%APPDATA%\com.bibliotecajogos.unificada\library.sqlite3`.
- O schema atual inclui `games`, `library_entries`, `game_sources`, `launch_actions`, `game_genres` e `schema_migrations`.
- O seed dos 4 mocks roda em background, sem bloquear a abertura do app.
- A listagem principal vem do comando Tauri `list_library_entries`.
- Entradas arquivadas sao preservadas no banco e ocultadas da listagem principal.
- A estrutura esta documentada em `ESTRUTURA_BANCO_DADOS.md`.

## Jogos manuais

- Cadastro manual de jogos persiste no SQLite quando o app roda no Tauri.
- Edicao de jogos manuais reutiliza o mesmo modal de cadastro.
- Arquivamento e reativacao de entradas sao feitos via `set_library_entry_archived`.
- A inferencia de acao de lancamento distingue `manual`, `uri` e `executable`.

## Lancamento seguro

- O comando `launch_library_entry` abre executaveis locais para entradas `manual` e `local`.
- A execucao usa `std::process::Command`, sem shell.
- O backend valida caminho absoluto, arquivo existente, arquivo local, extensao `.exe` e diretorio de trabalho.
- URIs como `steam://rungameid/<appid>` continuam abertas pelo fluxo de URI do sistema.

## LocalGamesProvider

- A sincronizacao local e manual pelo comando `sync_local_games`.
- O scanner local evita bibliotecas Steam por padrao.
- O provider ignora instaladores, runtimes, redistribuiveis, `_CommonRedist`, DirectX e EpicOnlineServices.
- Executaveis reais podem ser encontrados em subpastas como `Binaries\Win64`.
- Falsos positivos locais antigos sao arquivados no boot ou na sincronizacao.
- Foram adicionados indices SQLite para reduzir o custo dessa limpeza.

## SteamProvider local

- Foi criado o primeiro corte do SteamProvider local pelo comando `sync_steam_games`.
- A sincronizacao detecta raizes padrao da Steam e tambem aceita `BIBLIOTECA_JOGOS_STEAM_ROOTS`.
- O backend le `steamapps/libraryfolders.vdf` para encontrar bibliotecas extras.
- Jogos instalados sao importados a partir de `appmanifest_*.acf`.
- Cada jogo Steam usa `game_sources.external_id` como AppID.
- A acao primaria usa `steam://rungameid/<appid>`.
- O sync e idempotente: nao altera timestamps nem conta update quando nada mudou.
- Quando um manifest some, a entrada e preservada e marcada como `not_installed`, sem arquivar automaticamente.
- A integracao Steam via Web API ainda nao foi implementada.

## Validacoes recentes

Ultima validacao confirmada:

```powershell
npm run lint
npm run build
```

```powershell
cd aplicativo\src-tauri
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\BibliotecaJogosUnificada\cargo-target"
cargo test
```

Resultado registrado: frontend validado e suite Rust com 29 testes passando.

## Commits recentes relevantes

- `c61e6c7 docs(agents): require agent and skill workflow`
- `d131d88 fix(steam): reconcile local sync state`
- `da7d11b feat(steam): sync installed Steam games`
- `906260f docs(agents): update agent and skill governance`
- `8e9e526 feat(frontend): apply technical recommendations`
- `0ce7561 Document database structure`
- `33c2933 Reorganize frontend structure`
- `ab011e1 Add development guidelines`

## Proximos cortes recomendados

1. Validar manualmente no Tauri: cadastro, edicao, arquivamento, reabertura, sincronizacao local e sincronizacao Steam.
2. Evoluir Steam com Web API/configuracao de conta para biblioteca completa, playtime e metadados.
3. Adicionar consulta filtrada/paginacao no backend para bibliotecas maiores.
4. Criar tela de contas/configuracoes para conexoes futuras.
