# Ultimos Desenvolvimentos do Aplicativo

Data de referencia: 2026-05-25

Este arquivo resume os desenvolvimentos mais recentes criados no aplicativo. Para historico completo, use `CHECKPOINT.md`.

## Governanca de desenvolvimento

- As diretrizes agora exigem identificar o agente principal, agentes auxiliares e skills aplicaveis antes de qualquer novo desenvolvimento.
- Subagentes tecnicos podem ser usados para pesquisa, implementacao ou revisao, mas devem ser instruidos a seguir os agentes e skills definidos em `cloud - biblioteca de jogos/agents` e `cloud - biblioteca de jogos/skills`.
- O fluxo obrigatorio esta documentado em `DIRETRIZES_DESENVOLVIMENTO.md` e `cloud - biblioteca de jogos/README_PROJETO.md`.

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
- As pastas adicionais do Xbox agora sao configuraveis na tela de `Contas e integracoes`, com picker e persistencia em SQLite, e a descoberta combina roots salvas, a variavel `BIBLIOTECA_JOGOS_XBOX_ROOTS` e o fallback de drives locais.
- A busca local generica (`sync_local_games`) tambem passou a agregar todos os drives existentes, alem de roots comuns do usuario e instaladores, para nao ficar presa ao armazenamento principal.
- O menu de `Padroes da biblioteca` agora permite escolher o modo do scan local (`automatic`, `selected_only` ou `automatic_plus_extra`), definir pastas extras e marcar pastas excluidas. As configuracoes ficam persistidas em `provider_account_configs` no namespace `library`.
- O card de `Padroes da biblioteca` foi reorganizado para reduzir densidade visual, adicionando resumo de estado, contadores de pastas e blocos mais claros para raizes e exclusoes do scan local.
- A biblioteca do frontend agora unifica Steam e Xbox quando o mesmo titulo aparece nas duas lojas, exibindo um unico registro com as plataformas agregadas e um chooser de launch para perguntar por onde iniciar.
- A classificacao do Xbox local foi endurecida para excluir apps comuns do Windows/Store como Skype, Filmes e TV, Noticias e IntelliGo Neptune, enquanto jogos de desktop como `osu!` passam a entrar como `local`.
- A capability Tauri `default` foi atualizada para liberar `get_xbox_account_config`, `save_xbox_account_config`, `get_xbox_library_roots` e `save_xbox_library_roots`, corrigindo o erro `Command not found / not allowed` que aparecia ao expandir as configuracoes do Xbox.
- Quando o mesmo jogo existe em mais de uma origem, o merge visual prioriza `local` como plataforma principal quando ela estiver presente no grupo.
- Epic Games fica como integracao planejada.
- O fluxo `Entrar com Steam` usa Steam OpenID no navegador externo, com callback local em `127.0.0.1`, valida a resposta com a Steam e persiste apenas o SteamID64 no SQLite.
- A tela tambem permite salvar SteamID64 manualmente no banco local para diagnostico/fallback, sem depender de `localStorage` como fonte de verdade.
- A sincronizacao por conta usa SteamID64 persistido no backend e Web API key salva no AuthVault/keyring do sistema operacional.
- O status da Steam Web API agora e baseado no segredo realmente legivel no AuthVault/keyring; marcador SQLite nao libera sincronizacao sozinho.
- O AuthVault agora tenta validar o round-trip do keyring e, quando o cofre do Windows nao retorna a credencial apos gravacao, usa fallback local cifrado por DPAPI em `%APPDATA%\com.bibliotecajogos.unificada\auth-vault\steam-web-api-key.dpapi`.
- Teste manual confirmado em 2026-05-15: o salvamento da Steam Web API key passou a funcionar apos o fallback DPAPI e a sincronizacao por conta pode usar a credencial lida pelo backend.
- A linha Steam na tela de contas passou a usar disclosure inline com seta. Ao expandir, aparecem login, sincronizacao local, sincronizacao por conta, SteamID64 e Steam Web API key no mesmo painel, com foco previsivel ao abrir e fechar.
- A tela de contas do Xbox voltou a usar disclosure inline para separar os controles rapidos do painel de pastas adicionais, evitando que a configuracao ficasse sempre exposta e quebrasse a hierarquia visual da pagina.
- O Xbox Live agora tem login oficial em janela webview do proprio aplicativo, usando o redirect desktop da Microsoft (`https://login.live.com/oauth20_desktop.srf`), PKCE e persistencia segura do refresh token em cofre separado do Steam. O `client_id` do login deixou de ser um app first-party da Microsoft e passou a ser configuravel nas padroes da biblioteca, porque o usuario precisa informar o `Application (client) ID` do proprio app registration Microsoft para conseguir consentimento. A importacao de title history passa a usar essa sessao autenticada.
- O Xbox Live recebeu tambem o campo de `client secret` no painel de contas. O segredo agora e salvo de forma segura no AuthVault, separado do `client_id` que continua nas configuracoes da biblioteca, e o login/refresh passou a exigir ambos os valores antes de concluir o token exchange. A interface nao reexibe o segredo depois do salvamento.
- O fluxo Xbox Live foi alinhado com os endpoints oficiais `consumers/oauth2/v2.0` da Microsoft. A troca de token agora devolve a mensagem real do provedor quando a autorizacao falha, o que ajuda a diagnosticar app registration/configuracao sem ficar preso no erro generico.
- O modulo de auth Xbox Live tambem passou a devolver status HTTP e corpo bruto das falhas nas chamadas de token, XSTS e identidade. Isso melhora muito o diagnostico quando a resposta real do provedor e o que importa para corrigir a integracao.
- O redirect do login Xbox Live foi trocado para `https://login.microsoftonline.com/common/oauth2/nativeclient`, que e o URI desktop recomendado pela Microsoft. O app registration precisa apontar para esse redirect para que o callback seja capturado corretamente.
- Foi definida como prioridade a proxima revisao arquitetural do Xbox Live para remover a dependencia de `client secret` local e viabilizar distribuicao do aplicativo para terceiros sem exigir configuracao sensivel no dispositivo do usuario final.
- O app nao captura nem armazena senha Steam, Steam Guard, cookies, sessao de navegador ou URL completa de callback OpenID.
- O backend do corte seguinte da Steam Web API passou a classificar falhas internas com `ProviderErrorDto`, tolerar itens invalidos no payload remoto e registrar metadados de sincronizacao nao sensiveis em `provider_account_configs.config_json`, sem alterar o contrato consumido pelo frontend.
- A mensagem de erro da Steam agora fica curta no padrao e expande detalhes tecnicos sob demanda. O backend emite `steam-sync-failed` com payload sanitizado e o frontend mostra `code`, `phase` e detalhes resumidos apenas quando o usuario abre o disclosure.
- O corte atual de Steam enrichment/achievements fica delimitado como best-effort em background: arte, metadados, playtime e sinais publicos de achievements podem enriquecer a biblioteca Steam sem bloquear boot, listagem, login, sincronizacao principal ou lancamento. O fluxo nao deve mais ser lido como um limite unico de 50 jogos; ele roda em lotes continuos, com lote interno conservador, pausa/backoff entre chamadas e parada quando houver rate limit da Steam Web API.
- Xbox achievements cross-title/title history ficam em espera por confirmacao oficial de compliance, escopos, limites e regras de armazenamento antes de serem usados como enriquecimento ou catalogo. Ate la, Xbox permanece conservador: descoberta local, launcher e associacao experimental, sem tratar achievements como ownership.

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

## Steam enrichment best-effort

- O enrichment roda em background e complementa registros Steam existentes, sem virar pre-condicao para listagem, login, sync principal ou launch.
- O fluxo deixou de ser limite unico de 50 jogos: a fila processa lotes continuos em background, mantendo um lote interno conservador apenas para controlar pressao na Steam Web API.
- Entre chamadas, o job aplica pausa/backoff; se a Steam Web API sinalizar rate limit, o enrichment para a rodada atual, emite falha sanitizada e preserva o cache/progresso para retomada posterior.
- Artwork e metadados devem usar fallback quando a fonte remota falhar ou estiver incompleta.
- Playtime/achievements devem ser tratados como sinais complementares e sujeitos a privacidade/disponibilidade da Steam Web API, com erro sanitizado e sem segredo exposto ao renderer.
- O backend registra cache SQLite de schema/progresso de achievements Steam com `schema_migrations` versao `2`, mantendo o progresso isolado por `steam_id64 + app_id` e sem substituir dados editados pelo usuario.

## Xbox heuristics hardening

- A heuristica do Xbox local foi reforcada para rejeitar apps de sistema/loja como `Filmes e TV`.
- O provider passou a cortar falsos positivos de instalacao local de desktop, mantendo jogos como `osu!` no fluxo `local` em vez de `xbox`.
- Foram adicionados testes Rust cobrindo a variante `Filmes e TV`, um falso positivo desktop local e o caso legitimo do `Minecraft Launcher`.

## Xbox persisted cleanup

- A limpeza de Xbox persistido agora tambem considera o alvo de lancamento armazenado para arquivar entradas antigas claramente ligadas a executaveis desktop locais.
- O provider continua preservando Appx/Xbox reais, inclusive quando a descoberta roda sem encontrar o mesmo registro na forma atual.
- Foram adicionados testes Rust para o falso positivo desktop local e para um alvo Appx real continuar valido.

## Xbox achievements compliance hold

- Achievements/title history do Xbox seguem como area sensivel de compliance e nao devem ser promovidos a fonte cross-title ate haver confirmacao oficial sobre uso permitido para app desktop de terceiros.
- Antes de implementar novo corte, a decisao precisa registrar API/endpoints oficiais, escopos, dados armazenaveis, dados exibiveis, limites, revogacao e riscos.
- O comportamento atual recomendado e manter Xbox como provider local/launcher, com importacao remota atras de bloqueio explicito de compliance.

## Local staging hardening

- O scanner local passou a tratar `staging` como diretório auxiliar.
- Itens sob `_staging` nao entram mais como jogos locais, reduzindo ruido de build/infra.
- Foi adicionado teste Rust dedicado para garantir que um executavel em `_staging` nao seja promovido a `local`.

## Battle.net local hardening

- O scanner local passou a rejeitar componentes do launcher Battle.net/Blizzard que estavam entrando como jogos.
- A regra cobre `Battle.net.exe`, pastas versionadas como `Battle.net.14542`, `Agent`, `BlizzardBrowser`, `BlizzardError` e `BlizzardUpdateAgent`.
- A rejeicao foi limitada aos componentes do launcher para preservar subpastas de jogos legitimos dentro da raiz Battle.net.
- Entradas locais antigas que apontem para esses componentes agora sao arquivadas pela limpeza local.
- Foram adicionados testes Rust para importacao e limpeza regressiva desses falsos positivos.

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

Validacao do corte em 2026-05-20:

```powershell
npm run lint
npm run build
```

```powershell
cd aplicativo\src-tauri
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\BibliotecaJogosUnificada\cargo-target"
cargo test
```

Resultado validado do corte: Xbox local mais conservador contra apps/infrastrutura do Windows Store e boot inicial mais leve com a tela de contas carregada sob demanda.

Validacao do corte em 2026-05-23:

```powershell
npm run lint
npm run test:smoke
npm run build
```

```powershell
cd aplicativo\src-tauri
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\BibliotecaJogosUnificada\cargo-target"
cargo test
```

Resultado validado: lint/build aprovados, smoke frontend com 29 testes passando e suite Rust com 132 testes passando apos o hardening Battle.net do scan local.

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

1. Validar manualmente o Steam enrichment best-effort em background no Tauri, com lotes continuos, fallback de artwork/metadados, pausa/backoff, parada em rate limit e sem bloquear boot, listagem, sync principal ou launch.
2. Manter Xbox achievements cross-title/title history aguardando confirmacao oficial de compliance antes de qualquer uso como enrichment/catalogo.
3. Consolidar e testar o Xbox/Game Pass local como provider experimental do Windows, refinando a heuristica contra apps falsos positivos e validando mais jogos de desktop que devem entrar como `local`.
4. Em seguida, fazer o corte inicial de Epic Games, tratando compliance e limite de API como bloqueios de decisao antes de qualquer fluxo de conta.
5. Depois das novas plataformas, preparar consulta filtrada/paginacao no backend para bibliotecas maiores e ajustar a UI para listas mais longas.
6. Quando houver stack dedicada de browser automation, criar smoke/e2e real para bootstrap, login Steam, syncs e launch.
7. Refinar a UX do bloqueio de importacao Xbox, explicando melhor ao usuario porque a operacao ainda pode falhar quando compliance/autenticacao nao estiverem concluidos.
8. O proximo ajuste do projeto e reduzir o tempo de abertura do aplicativo, porque o Tauri esta demorando demais para mostrar a interface completa.

## Corte Xbox Public Client

- O login Xbox Live passou a usar `authorization code + PKCE` como fluxo de `public client` desktop.
- O fluxo deixou de exigir `client secret` do usuario final; o `client_id` passou a ser configuracao interna ou de build da instancia do projeto.
- O refresh token continua salvo no `AuthVault`, sem expor segredo ao renderer nem ao fluxo de contas.
- Para a build final, o `client_id` nao deve permanecer editavel na interface. O valor deve vir de configuracao interna, variavel de ambiente ou build, e a tela deve apenas reportar estado/erro se ele nao estiver definido.

## Atualizacao 2026-05-25

- Corte de Steam enrichment/achievements iniciado sob apoio de QA/compliance e integracao.
- Escopo registrado: Steam enrichment best-effort em background; Xbox achievements cross-title aguardando confirmacao oficial/compliance.
- O backend passou a preparar cache SQLite para achievements Steam e eventos de enrichment em background; a UI mostra um indicador discreto sem bloquear a biblioteca.
- O enrichment Steam foi atualizado de um limite unico de 50 para processamento continuo em background, respeitando lote interno conservador, backoff/pausa entre requisicoes e parada imediata quando houver rate limit.

## Atualizacao 2026-05-26

- O gate Rust do backend Tauri no Windows foi restaurado depois de `cargo test` compilar, mas o binario unitario abortar antes da suite com `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)`.
- A causa confirmada foi carregamento de superficies nativas Tauri/Wry/Tao no binario unitario `app_lib-*.exe`, que importavam `TaskDialogIndirect` de `comctl32.dll`; sem manifesto Common Controls v6, o Windows resolvia o `comctl32` antigo e encerrava o processo antes do harness de testes.
- Para manter o gate estavel, `run()`, `bootstrap_library`, o modulo de comandos Tauri e os fluxos Steam OpenID/Xbox Live dependentes de `AppHandle`, eventos ou webview ficaram fora do build unitario por `cfg(test)`, sem alterar o runtime normal do aplicativo.
- Em seguida, a cobertura command-adjacent foi recuperada sem reintroduzir Tauri/Wry/Tao no binario unitario: a logica de preflight da sincronizacao Steam por conta foi extraida para `command_logic`, incluindo resolucao de SteamID64/API key, mapeamento de erros do AuthVault e guarda contra sincronizacao concorrente.
- Os wrappers `#[tauri::command]` permanecem finos e continuam fora dos testes unitarios diretos quando isso exigiria carregar o runtime nativo; a logica de negocio por tras deles voltou a ser testada por helpers puros.
- Validacoes do corte: `cargo test` com `CARGO_TARGET_DIR` local passou com 141 testes executados, `cargo fmt -- --check` passou e `cargo check` passou. Tambem foi confirmado por `dumpbin /imports` que o binario unitario filtrado nao voltou a importar `comctl32.dll`/`TaskDialogIndirect`.
- O proximo ciclo de roadmap permanece achievements, nao Epic.

## Atualizacao 2026-05-26 - Steam achievements best-effort

- O backend Steam achievements foi reforcado como enrichment complementar em background, sem virar pre-condicao para boot, listagem, login, sincronizacao principal ou launch.
- A decisao de falhas do enrichment saiu do modulo Tauri para helper testavel: rate limit encerra a rodada atual com evento sanitizado, erros recuperaveis de provider viram marcador de retry/backoff e falhas de rede continuam transientes sem poluir o cache de tentativa.
- Respostas de achievements privados/indisponiveis da Steam Web API agora sao tratadas como cache vazio esperado, sem gravar mensagem bruta de privacidade/API no cache e sem falhar o enrichment.
- O cache de progresso de player achievements permanece isolado por `steam_id64 + app_id`, preservando dados de outras contas para o mesmo jogo.
- Validacao do corte: `cargo test` passou com 146 testes, mantendo o binario unitario sem carregar wrappers Tauri de comandos.
- Xbox achievements continuam em hold por compliance; Epic nao foi iniciado neste ciclo.

## Atualizacao 2026-05-26 - Steam achievements na UI

- O contrato `LibraryEntry` passou a expor `game.achievements` quando o cache Steam tem progresso do jogador, com totais, percentual, data de cache e lista de conquistas derivada de schema + player achievements.
- A biblioteca agora mostra resumo de conquistas nos cards/lista e o painel de detalhes ganhou uma secao compacta com progresso e lista de achievements.
- A ordenacao `Conquistas: maior` e `Conquistas: menor` passou a usar progresso real e envia jogos sem dados de achievements para o fim em ambos os modos.
- O seletor de ordenacao foi ajustado para manter contraste legivel no tema escuro do Windows.
- Achievements Steam hidden bloqueadas aparecem como `Conquista secreta`, com conteudo oculto por padrao e revelacao manual por clique/teclado; hidden ja desbloqueadas aparecem normalmente.
- O fluxo permanece best-effort: falta de achievements nao bloqueia boot, listagem, login, sincronizacao principal ou launch.
- Xbox achievements continuam em hold por compliance; Epic nao foi iniciado neste ciclo.

## Atualizacao 2026-05-26 - Preview e modal de Steam achievements

- A secao de conquistas no painel de detalhes passou de lista inline longa para preview compacto, com progresso, barra, grupos de conquistas alcancadas/pendentes, contador `+N` quando ha mais itens e acao `Ver todas`.
- O modal de Steam achievements mostra a lista completa recebida pelo frontend, com busca, contador de resultados, scroll interno e fechamento por botao, clique fora ou `Escape`, mantendo suporte a listas grandes como jogos com centenas de achievements.
- A ordenacao da lista de achievements foi centralizada no frontend: alcancadas primeiro, depois bloqueadas, com hidden bloqueadas apos as nao-hidden e ordenacao alfabetica pelo texto visivel.
- A busca usa apenas o texto visivel: hidden bloqueadas nao reveladas nao vazam nome/descricao reais, mas passam a ser encontradas por esse conteudo depois da revelacao manual.
- Hidden ja desbloqueadas continuam visiveis normalmente e agora exibem indicador discreto `(secreta)`.
- Percentual global de jogadores ficou fora deste ciclo; Xbox achievements continuam em hold por compliance; Epic nao foi iniciado.

## Atualizacao 2026-05-26 - Ajustes de hidden achievements e sync Steam

- A revelacao de hidden achievements bloqueadas passou a ser estado local do modal: a ordem inicial fica congelada durante a abertura, a busca filtra preservando a ordem relativa e o preview do painel continua censurado ao fechar o modal.
- Hidden bloqueadas reveladas no modal agora usam nome, descricao e icone reais quando disponiveis, com fallback especifico para descricao indisponivel, sem trocar conteudo por textos genericos de status.
- A busca por `secreta` encontra conquistas hidden pelo indicador visivel, sem vazar nome, descricao ou identificador tecnico de hidden bloqueada nao revelada; depois da revelacao no modal, o conteudo real tambem passa a ser pesquisavel.
- O fluxo de sincronizacao por conta Steam na tela de contas passou a renderizar o estado de verificacao/sincronizacao antes de iniciar as chamadas de retry summary e sync, reduzindo a sensacao de travamento em ambiente sem cache.
- Observabilidade completa nao foi implementada neste ciclo; Xbox achievements continuam em hold por compliance; Epic nao foi iniciado.

## Atualizacao 2026-05-26 - Evidencia de hidden Steam API

- Foi inspecionado o cache bruto salvo pelo enrichment autenticado da Steam para o caso local `appid 4286550 / ach_secretWorld`, sem imprimir chave, headers ou payload completo: o schema cacheado trouxe `displayName`, `hidden`, `icon` e `icongray`, mas nao trouxe `description`; o player achievements cacheado trouxe `name` e chave `description`, porem vazia. A chamada direta sem chave ao endpoint retornou HTTP 400, entao a evidencia usada foi o cache bruto preservado da chamada autenticada anterior.
- Classificacao do caso testado: comportamento variavel/limitacao do payload retornado pela Steam API para essa hidden bloqueada; o app nao estava perdendo a descricao real nesse caso, porque ela nao veio preenchida no cache bruto. Ainda assim, o backend foi reforcado para preservar `description` vinda do player payload quando o schema estiver sem descricao.
- A UI de hidden bloqueada revelada agora mostra o indicador discreto `(secreta)` e usa o fallback especifico `A Steam não disponibilizou a descrição desta conquista secreta.` quando a descricao real nao estiver disponivel.
- Nao foi identificado endpoint oficial para listar a biblioteca compartilhada completa do Steam Families; o tema permanece como pesquisa futura/experimental, sem implementacao neste ciclo.
- Observabilidade completa nao foi implementada; Steam Families nao foi implementado; Xbox achievements continuam em hold por compliance; Epic nao foi iniciado.
