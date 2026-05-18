# Ultimos Desenvolvimentos do Aplicativo

Data de referencia: 2026-05-18

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
- Os filtros rapidos agora combinam por categoria: status com status e plataforma com plataforma usam OR, enquanto categorias diferentes continuam combinando por AND.
- O estado vazio da biblioteca agora diferencia biblioteca vazia de ausencia de resultados por busca ou filtros.
- Quando os filtros eliminam todos os resultados, a mensagem aponta melhor se o bloqueio veio de status, plataforma ou da combinacao entre ambos.

## Contas e configuracoes

- O botao `Contas` da sidebar agora abre uma area real de `Contas e integracoes`.
- A tela mostra Steam, Xbox/Game Pass e Epic Games em linhas de integracao.
- Steam indica que a sincronizacao local por manifests ja esta ativa e permite disparar essa sincronizacao.
- Xbox/Game Pass fica como descoberta local primeiro: jogos instalados podem entrar na biblioteca, e sinais de achievements servem apenas como indicio auxiliar de associacao com a conta, nao como prova de posse.
- Quando um jogo Xbox/Game Pass nao estiver instalado, a acao esperada e abrir a Microsoft Store para o usuario iniciar o download.
- A heuristica de descoberta local do Xbox foi refinada para aceitar melhor pacotes reais do Windows/Store e ainda excluir helpers como Gaming Services, Xbox TCUI e componentes de infraestrutura.
- A biblioteca do frontend agora unifica Steam e Xbox quando o mesmo titulo aparece nas duas lojas, exibindo um unico registro com as plataformas agregadas e um chooser de launch para perguntar por onde iniciar.
- A classificacao do Xbox local foi endurecida para excluir apps comuns do Windows/Store como Skype, Filmes e TV, Noticias e IntelliGo Neptune, enquanto jogos de desktop como `osu!` passam a entrar como `local`.
- Quando o mesmo jogo existe em mais de uma origem, o merge visual prioriza `local` como plataforma principal quando ela estiver presente no grupo.
- Epic Games fica como integracao planejada.
- O fluxo `Entrar com Steam` usa Steam OpenID no navegador externo, com callback local em `127.0.0.1`, valida a resposta com a Steam e persiste apenas o SteamID64 no SQLite.
- A tela tambem permite salvar SteamID64 manualmente no banco local para diagnostico/fallback, sem depender de `localStorage` como fonte de verdade.
- A sincronizacao por conta usa SteamID64 persistido no backend e Web API key salva no AuthVault/keyring do sistema operacional.
- O status da Steam Web API agora e baseado no segredo realmente legivel no AuthVault/keyring; marcador SQLite nao libera sincronizacao sozinho.
- O AuthVault agora tenta validar o round-trip do keyring e, quando o cofre do Windows nao retorna a credencial apos gravacao, usa fallback local cifrado por DPAPI em `%APPDATA%\com.bibliotecajogos.unificada\auth-vault\steam-web-api-key.dpapi`.
- Teste manual confirmado em 2026-05-15: o salvamento da Steam Web API key passou a funcionar apos o fallback DPAPI e a sincronizacao por conta pode usar a credencial lida pelo backend.
- A linha Steam na tela de contas passou a usar disclosure inline com seta. Ao expandir, aparecem login, sincronizacao local, sincronizacao por conta, SteamID64 e Steam Web API key no mesmo painel, com foco previsivel ao abrir e fechar.
- O app nao captura nem armazena senha Steam, Steam Guard, cookies, sessao de navegador ou URL completa de callback OpenID.
- O backend do corte seguinte da Steam Web API passou a classificar falhas internas com `ProviderErrorDto`, tolerar itens invalidos no payload remoto e registrar metadados de sincronizacao nao sensiveis em `provider_account_configs.config_json`, sem alterar o contrato consumido pelo frontend.
- A mensagem de erro da Steam agora fica curta no padrao e expande detalhes tecnicos sob demanda. O backend emite `steam-sync-failed` com payload sanitizado e o frontend mostra `code`, `phase` e detalhes resumidos apenas quando o usuario abre o disclosure.

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
- O AppID `228980` (`Steamworks Common Redistributables`) e filtrado por nao representar um jogo; se ja tiver sido importado, a proxima sincronizacao o arquiva.
- A integracao Steam via Web API ja possui primeiro corte para biblioteca de conta: usa `IPlayerService/GetOwnedGames`, SteamID64 persistido e chave Web API no AuthVault.
- Quando um jogo Steam local tambem aparece na biblioteca remota, o sync por conta preserva o estado instalado local e preenche `game_sources.account_id`.
- O OpenID nao concede acesso automatico a biblioteca privada; ele apenas verifica a identidade e retorna SteamID64. A Web API continua dependendo de chave valida e visibilidade da biblioteca.

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

Validacao adicional em 2026-05-15:

```powershell
npm run lint
npm run build
```

```powershell
cd aplicativo\src-tauri
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\BibliotecaJogosUnificada\cargo-target"
cargo test
```

Resultado registrado: lint/build aprovados e suite Rust com 42 testes passando apos o ajuste do AuthVault DPAPI.

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

1. Consolidar e testar o Xbox/Game Pass local como provider experimental do Windows, refinando a heuristica contra apps falsos positivos e validando mais jogos de desktop que devem entrar como `local`.
2. Em seguida, fazer o corte inicial de Epic Games, tratando compliance e limite de API como bloqueios de decisao antes de qualquer fluxo de conta.
3. Depois das novas plataformas, preparar consulta filtrada/paginacao no backend para bibliotecas maiores e ajustar a UI para listas mais longas.
4. Quando houver stack dedicada de browser automation, criar smoke/e2e real para bootstrap, login Steam, syncs e launch.
