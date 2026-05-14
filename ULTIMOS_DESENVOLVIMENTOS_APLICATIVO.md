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

## Contas e configuracoes

- O botao `Contas` da sidebar agora abre uma area real de `Contas e integracoes`.
- A tela mostra Steam, Xbox/Game Pass e Epic Games em linhas de integracao.
- Steam indica que a sincronizacao local por manifests ja esta ativa e permite disparar essa sincronizacao.
- Steam agora permite salvar e remover a configuracao local por SteamID64.
- Steam tambem permite salvar/remover a chave Web API no AuthVault local, sem devolver o segredo ao frontend.
- Xbox/Game Pass e Epic Games ficam como integracoes planejadas.
- A tela nao pede senha, token, cookie ou Steam Guard.
- A tela tem acoes separadas para sincronizar Steam local por manifests e sincronizar a conta Steam pela Web API.

## Persistencia SQLite

- O backend Tauri usa SQLite local em `%APPDATA%\com.bibliotecajogos.unificada\library.sqlite3`.
- O schema atual inclui `games`, `library_entries`, `game_sources`, `launch_actions`, `game_genres`, `provider_account_configs` e `schema_migrations`.
- `provider_account_configs` guarda somente estado local de integracao e SteamID64 publico, sem segredos.
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
- O AppID `228980` (`Steamworks Common Redistributables`) e filtrado por nao representar um jogo; se ja tiver sido importado, a proxima sincronizacao o arquiva.
- A configuracao local por SteamID64 usa os comandos `list_steam_account_config`, `save_steam_account_config` e `disconnect_steam_account_config`.
- O AuthVault inicial usa o cofre do sistema operacional via backend Tauri para a chave Steam Web API.
- Os comandos `get_steam_api_key_status`, `save_steam_api_key` e `delete_steam_api_key` nunca retornam o segredo para o frontend.
- O comando `sync_steam_account_games` consulta `IPlayerService/GetOwnedGames/v1` usando SteamID64 e a chave do AuthVault somente no backend.
- Jogos retornados pela Web API entram como Steam com `install_status = not_installed` quando nao existem manifests locais.
- O playtime remoto de `playtime_forever` e salvo em `games.playtime_total_minutes`.
- Jogos ja instalados por manifest continuam como `installed`, preservando diretorio de trabalho e acao `steam://rungameid/<appid>`.

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

Resultado registrado: frontend validado e suite Rust com 42 testes passando.

## Commits recentes relevantes

- `a99f0cf fix(steam): apply provider agent review`
- `87af4c2 fix(steam): ignore common redistributables`
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
2. Validar manualmente no Tauri o fluxo Steam por conta com SteamID64, chave no AuthVault e biblioteca publica/visivel.
3. Adicionar consulta filtrada/paginacao no backend para bibliotecas maiores.
4. Criar tela de contas/configuracoes para conexoes futuras.
