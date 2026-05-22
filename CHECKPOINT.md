# Checkpoint - Biblioteca de Jogos Unificada

Data: 2026-05-22

## Objetivo do projeto

Criar um aplicativo desktop que funcione como uma biblioteca central de jogos, reunindo contas, bibliotecas, jogos instalados e metadados. As plataformas prioritarias do projeto sao Steam como foco principal, seguida por Xbox/Game Pass e Epic Games. Outras plataformas, como GOG, itch.io, Battle.net, Ubisoft Connect, EA App e jogos locais, ficam como expansoes futuras ou auxiliares.

## Estado atual

Foi feita uma pesquisa inicial de viabilidade. A conclusao principal e que o aplicativo e tecnicamente possivel, mas as integracoes nao terao o mesmo nivel de suporte em todas as plataformas.

Tambem foi iniciada a base do aplicativo dentro da pasta `aplicativo`, usando Tauri 2, React, TypeScript e Vite. O frontend ja possui uma tela de biblioteca com sidebar, resumo, busca, filtros, visualizacao por capas/lista, painel de detalhes e cadastro manual em memoria. A estrutura `src-tauri` foi criada e a execucao desktop nativa ja chegou a compilar localmente apos a instalacao de Rust/Cargo e Visual Studio Build Tools com MSVC/Windows SDK.

Decisao de design adicionada em 2026-05-07: o aplicativo deve usar modo escuro como experiencia padrao. A area de jogos deve abrir por padrao na visualizacao por capas, com cards de capa e nome do jogo abaixo, mantendo a alternativa em lista para leitura rapida de status e tempo jogado.

Atualizacao em 2026-05-07: os modelos centrais foram separados em `aplicativo/src/domain`, incluindo `Game`, `PlatformAccount`, `Provider`, `LaunchAction`, `LibraryEntry` e `SyncHistory`. Os dados mockados da biblioteca foram movidos para `aplicativo/src/data/mockLibrary.ts`, e a UI passou a consumir `LibraryEntry` sem alterar o comportamento visual.

Atualizacao em 2026-05-07: foram adicionados agentes senior de programacao em `cloud - biblioteca de jogos/agents`: backend, frontend e integracao/qualidade. Tambem foram criadas as skills `senior-backend-implementation`, `senior-frontend-implementation` e `senior-integration-quality` para orientar implementacao, revisao e verificacao tecnica.

Atualizacao em 2026-05-07: o cadastro manual de jogos foi implementado no frontend em memoria. O botao `Adicionar jogo` abre um modal com titulo obrigatorio, genero, status instalado/nao instalado e acao de lancamento opcional. Ao salvar, a UI adiciona um `LibraryEntry` manual no topo da biblioteca, com `LaunchAction`, tempo de jogo zerado e cor deterministica.

Atualizacao em 2026-05-07: Rustup foi instalado com sucesso e `rustc 1.95.0`/`cargo 1.95.0` estao disponiveis pelo caminho `%USERPROFILE%\.cargo\bin`. A primeira tentativa de instalar Visual Studio Build Tools 2022 com workload C++ falhou por espaco insuficiente, mas apos liberar espaco no C: a instalacao foi concluida. `npm run tauri:dev` ja compila e executa o app nativo Tauri localmente.

Atualizacao em 2026-05-07: durante a primeira compilacao Tauri, o OneDrive bloqueou a leitura de `aplicativo/src-tauri/build.rs` com erro de operacao de nuvem. O arquivo foi recriado localmente com o conteudo padrao do Tauri para destravar a compilacao.

Atualizacao em 2026-05-08: a tela de biblioteca passou a ter selecao real de jogo ao clicar em cards/lista, busca por nome/plataforma/genero/status, filtros rapidos funcionais (`Todos`, `Instalados`, `Steam`, `Locais`) e feedback para acoes de lancamento/instalacao. O botao `Jogar` tenta abrir URI quando a acao e do tipo `uri`; execucao de executaveis locais ainda depende de comando Tauri especifico. O ESLint passou a ignorar `aplicativo/src-tauri/target`, que contem arquivos gerados pelo build Rust.

Atualizacao em 2026-05-10: a documentacao foi alinhada com a versao atual do aplicativo. O estado confirmado e: frontend funcional com dados mockados e cadastro manual somente em memoria; modelos de dominio separados em `aplicativo/src/domain`; dados iniciais em `aplicativo/src/data/mockLibrary.ts`; backend Tauri ainda no scaffold inicial, sem comandos de dominio, persistencia local ou providers reais. Foi criado o guia `RETOMADA_NOVO_COMPUTADOR.md` para continuar o trabalho em outra maquina.

Atualizacao em 2026-05-10: a Fase 1 de validacao geral foi executada com apoio do agente de integracao/QA. Passaram: verificacao de ambiente (`node`, `npm`, `rustc`, `cargo`), `npm run lint`, `npm run build`, `cargo check`, `npm audit --audit-level=moderate`, subida nativa por `npm run tauri:dev` com janela `Biblioteca de Jogos`, e testes funcionais automatizados da UI web cobrindo estado inicial, metricas, filtros, busca, modo lista/capas, selecao, cadastro manual, validacao de titulo obrigatorio, feedback de jogar/instalar e viewport minimo 960x640. O protocolo `steam://` esta registrado no Windows e o Steam estava em execucao, mas nao foi disparado um jogo real para evitar efeito colateral no ambiente. Uma primeira tentativa de `npm run tauri:build` compilou o `app.exe` release, mas falhou ao empacotar MSI por falta de espaco em disco ao extrair o WiX (`os error 112`). Apos liberar espaco, `npm run tauri:build` passou e gerou os instaladores em `aplicativo/src-tauri/target/release/bundle`: MSI (`Biblioteca de Jogos Unificada_0.1.0_x64_en-US.msi`) e NSIS (`Biblioteca de Jogos Unificada_0.1.0_x64-setup.exe`). O C: ficou com cerca de 4,45 GB livres apos a geracao dos pacotes.

Atualizacao em 2026-05-10: foi criado o agente `cloud - biblioteca de jogos/agents/11-senior-database-agent.md` para orientar a Fase 2. O agente fica responsavel por schema SQLite, migracoes, repositories, constraints, indices, seed/fallback de dados mockados e criterios de aceite para persistencia local.

Atualizacao em 2026-05-10: foram criadas as skills `sqlite-local-persistence-design` e `sqlite-migrations-repositories` em `cloud - biblioteca de jogos/skills` para apoiar o agente senior de banco de dados na definicao do schema SQLite, migracoes, repositories, queries, transacoes e criterios de aceite da persistencia local.

Atualizacao em 2026-05-10: a Fase 2 foi iniciada com apoio do agente senior de banco de dados. Foi adicionada persistencia SQLite no backend Tauri usando `rusqlite` com SQLite bundled e `chrono` para timestamps. O banco local e criado em `%APPDATA%\\com.bibliotecajogos.unificada\\library.sqlite3`, fora do codigo-fonte. A migracao inicial cria `schema_migrations`, `games`, `library_entries`, `game_sources`, `launch_actions` e `game_genres`, com indices para titulo, status, plataforma e acao primaria. Foram expostos os comandos Tauri `list_manual_games` e `add_manual_game`. O frontend passou a carregar jogos manuais persistidos ao abrir no Tauri e a salvar novos jogos manuais no banco; no navegador comum, mantem fallback em memoria. Foram adicionados testes Rust de migracao e persistencia manual com SQLite em memoria. Validacoes passaram: `cargo fmt`, `cargo check`, `cargo test`, `npm run lint`, `npm run build` e `npm run tauri:dev`. Ficaram para o proximo corte da Fase 2: seed dos 4 mocks no banco, listagem unificada 100% pelo backend, edicao/arquivamento de jogos manuais e testes manuais de fechar/reabrir apos cadastrar pela UI.

Atualizacao em 2026-05-10: apos erro do OneDrive/Cargo em `src-tauri/target`, os scripts `tauri:dev` e `tauri:build` foram ajustados para definir `CARGO_TARGET_DIR=%LOCALAPPDATA%\\BibliotecaJogosUnificada\\cargo-target`. Com isso, `npm run tauri:dev` voltou a abrir a janela `Biblioteca de Jogos` usando o target Rust fora do OneDrive.

Atualizacao em 2026-05-10: foi diagnosticada tela branca no Tauri e carregamento infinito em `localhost:5173` causados por arquivos do frontend/cache do Vite indisponiveis no OneDrive. A pasta `aplicativo` foi marcada como disponivel localmente com `attrib +P -U .\\* /S /D`, o cache do Vite foi limpo e o frontend voltou a renderizar. Foi adicionado o script `npm run onedrive:keep-local` para repetir esse ajuste quando necessario.

Atualizacao em 2026-05-10: a inferencia de acao de lancamento em jogos manuais foi corrigida. Targets vazios continuam como `manual`; targets contendo `://`, como `steam://rungameid/1030300`, agora sao salvos como `uri`; os demais continuam como `executable`. Isso permite que jogos manuais persistidos com URI de launcher usem o mesmo fluxo de abertura dos jogos mockados. Foram adicionados testes Rust para URI manual, e passaram `cargo test`, `npm run lint` e `npm run build`.

Atualizacao em 2026-05-10: foi criada a skill `safe-local-executable-launch` para orientar a implementacao de lancamento seguro de executaveis locais via Tauri, sem shell, com validacao de caminho absoluto, arquivo existente, extensao `.exe`, argumentos estruturados e mensagens de erro seguras.

Atualizacao em 2026-05-10: foi implementado o primeiro corte de lancamento seguro de executaveis locais. O frontend envia apenas o `entryId` para o comando Tauri `launch_library_entry`; o backend busca a acao persistida no SQLite, exige jogo manual, acao primaria `executable`, caminho absoluto, arquivo existente, arquivo local nao remoto e extensao `.exe`, canonicaliza o caminho e executa via `std::process::Command` sem shell. Jogos mockados/importados com acao `executable` ainda exibem mensagem de provider futuro, pois nao estao persistidos no SQLite. Foram adicionados testes Rust de validacao de caminho vazio, relativo, inexistente, diretorio, remoto, extensao invalida, `.exe` existente e diretorio de trabalho padrao. Validacoes passaram: `cargo test` com `CARGO_TARGET_DIR` local, `cargo check`, `npm run lint`, `npm run build` e `npm run tauri:dev`.

Atualizacao em 2026-05-10: a navegacao lateral da biblioteca foi corrigida. Os itens `Biblioteca`, `Steam` e `Locais` deixaram de ser links para `/` e passaram a atuar como botoes conectados ao mesmo estado dos filtros rapidos, limpando busca/mensagens ao trocar de contexto. O item `Contas` permanece como acao de feedback para fase futura de integracoes. Validacoes passaram: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-11: foi criada a versao de trabalho `Biblioteca de Jogos Unificada - v2` e o frontend foi migrado de TypeScript/TSX para JavaScript/JSX. Foram removidos `tsconfig*`, arquivos de tipos em `src/domain`, dependencias TypeScript do `package.json`, e o build passou a usar apenas `vite build`. O backend Tauri permanece em Rust, pois e a camada nativa exigida pela arquitetura Tauri atual.

Atualizacao em 2026-05-11: apos teste manual confirmado de persistencia, foi implementado o proximo corte da Fase 2. O backend Tauri ganhou o comando `list_library_entries`, que retorna a biblioteca unificada sem filtrar apenas jogos manuais. A abertura do banco agora executa seed idempotente dos 4 mocks no SQLite (Steam, local e manual), preservando jogos criados pelo usuario. O frontend passou a carregar a biblioteca por `listLibraryEntries()`; no Tauri esse servico chama `list_library_entries`, e no navegador comum usa `mockLibrary.js` como fallback de desenvolvimento. Foram adicionados estados de carregamento/vazio para evitar quebra quando a listagem ainda nao retornou. Validacoes passaram: `cargo fmt`, `cargo check`, `cargo test` (13 testes), `npm run lint` e `npm run build`.

Atualizacao em 2026-05-11: foi adicionado o arquivamento da biblioteca com o campo booleano `is_archived` em `library_entries`. O backend agora marca entradas arquivadas, exclui arquivados de `list_library_entries`, protege `launch_library_entry` contra jogos arquivados e manteve compatibilidade com bancos existentes via coluna incremental. O frontend ganhou acao de arquivar/reativar na lateral de detalhes e exibe o estado atual do item. Foram adicionados testes Rust para toggles de arquivamento e a validacao completa passou com `cargo fmt`, `cargo test`, `npm run lint` e `npm run build`.

Atualizacao em 2026-05-11: foi implementada a edicao de jogos manuais. O backend ganhou o comando `update_manual_game`, que atualiza `games`, `library_entries`, `launch_actions` e `game_genres` em transacao, preservando `is_archived` e os ids existentes. O frontend passou a reutilizar o modal manual em modo adicionar/editar, com pre-preenchimento dos campos para entradas manuais selecionadas. Foram adicionados testes Rust para update normal e update de entrada arquivada sem reativacao. Validacoes passaram: `cargo fmt`, `cargo check`, `cargo test` (16 testes), `npm run lint` e `npm run build`.

Atualizacao em 2026-05-11: o boot do aplicativo foi desacoplado do seed da biblioteca. O backend agora aplica migration e compatibilidade de schema no caminho critico e executa apenas o seed idempotente dos 4 mocks em background, emitindo um evento quando termina. A sincronizacao de jogos locais passou a ser manual, via comando `sync_local_games` e acao dedicada na interface, para nao impactar o tempo de abertura. O aplicativo ganhou o `LocalGamesProvider` inicial com importacao incremental de executaveis locais. Foram adicionados testes para banco vazio antes do bootstrap, seed manual explicito, upgrade de schema legado e importacao local. Validacoes passaram: `cargo fmt`, `cargo check`, `cargo test` (20 testes), `npm run lint` e `npm run build`.

Atualizacao em 2026-05-11: foi feita uma revisao de estado para retomada em outro computador. O app atual esta em Tauri 2 + React 18 + JavaScript/JSX, com backend Rust e SQLite local. O fluxo implementado inclui listagem unificada via `list_library_entries`, seed idempotente dos 4 mocks em background, cadastro/edicao de jogos manuais, arquivamento, lancamento seguro de executaveis locais para entradas `manual` e `local`, abertura de URIs como `steam://`, e sincronizacao manual inicial de jogos locais via `sync_local_games`. A sincronizacao local usa `BIBLIOTECA_JOGOS_LOCAL_ROOTS` quando definida, ou raizes comuns como `%USERPROFILE%\Games`, `%USERPROFILE%\Documents\Games`, `%PROGRAMFILES%\GOG Games`, `%PROGRAMFILES%\Epic Games`, `%PROGRAMFILES%\EA Games`, `%PROGRAMFILES%\Ubisoft`, `%PROGRAMFILES%\Battle.net` e equivalentes em `%PROGRAMFILES(X86)%`/`%PUBLIC%`. Validacoes executadas nesta revisao: `npm run lint`, `npm run build` e `cargo test` com `CARGO_TARGET_DIR` local, todas aprovadas; a suite Rust esta com 21 testes passando.

Atualizacao em 2026-05-12: o `LocalGamesProvider` foi ajustado apos validacao manual apontar falsos positivos como setup do DirectX e EpicOnlineServices. A sincronizacao local escaneia subpastas de jogo com profundidade limitada para encontrar executaveis em estruturas como `Binaries\Win64`, evita tratar a raiz inteira da biblioteca como um unico jogo quando ela contem subpastas, rejeita instaladores/runtimes/servicos em vez de usa-los como fallback e arquiva entradas locais antigas que apontem para esses alvos auxiliares. A limpeza agora tambem roda ao abrir o banco, entao falsos positivos antigos somem ao reabrir o app. Foi decidido que o `LocalGamesProvider` nao deve varrer bibliotecas Steam por padrao; a importacao Steam deve ficar no futuro `SteamProvider` para evitar duplicidade e preservar metadados corretos. Foram adicionados testes para nao importar apenas DirectX/EpicOnlineServices, arquivar falso positivo ja persistido no sync/boot e encontrar executavel real aninhado.

Atualizacao em 2026-05-12: a limpeza de falsos positivos locais no boot foi otimizada para reduzir impacto na abertura do aplicativo. Foram adicionados indices SQLite especificos para entradas locais ativas e acoes de lancamento (`idx_library_entries_local_active_game` e `idx_launch_actions_platform_kind_game`), alem de um atalho que evita a query de limpeza quando nao ha entradas locais ativas. A query de arquivamento agora parte de `library_entries` locais ativas e usa `EXISTS` contra `launch_actions`, preservando a limpeza no boot sem varrer toda a tabela de acoes como caminho principal. Validacoes passaram: `cargo fmt`, `cargo test` (26 testes), `npm run lint` e `npm run build`.

Atualizacao em 2026-05-13: a documentacao foi comparada com o estado atual do codigo. Confirmado que o backend expoe `list_library_entries`, `add_manual_game`, `update_manual_game`, `set_library_entry_archived`, `sync_local_games` e `launch_library_entry`; o frontend consome esses comandos via `libraryApi.js` e mantem fallback web em `mockLibrary.js`. A pendencia de texto quebrado no botao de edicao nao foi encontrada no frontend atual (`Salvar alterações` esta correto). Validacoes passaram novamente: `npm run lint`, `npm run build` e `cargo test` (26 testes).

Atualizacao em 2026-05-13: o documento externo `Projeto Biblioteca de Jogos Unificada.docx` foi incorporado ao projeto como `DIRETRIZES_DESENVOLVIMENTO.md`. As diretrizes foram adaptadas ao estado real da aplicacao: frontend em JavaScript/JSX, backend Tauri em Rust, SQLite como persistencia principal, uso gradual de `components`, `pages`, `services`, `adapters`, `hooks`, `assets` e `styles`, padrao Service-Adapter para providers, contrato interno baseado em `LibraryEntry` e criterios de aceite para novas features.

Atualizacao em 2026-05-13: a reorganizacao inicial de frontend foi executada antes do `SteamProvider`. `App.jsx` passou a apenas renderizar `pages/LibraryPage.jsx`; a tela foi separada em `components/Sidebar.jsx`, `Topbar.jsx`, `StatsGrid.jsx`, `LibraryBrowser.jsx`, `GameDetailsPanel.jsx` e `ManualGameModal.jsx`; a logica de estado e fluxos foi movida para `hooks/useLibraryPageState.js`; a normalizacao/criacao de entradas manuais foi movida para `adapters/libraryEntryAdapter.js`; e os estilos foram movidos para `styles/library.css`. O comportamento visual e funcional foi preservado, e o texto do botao de edicao foi corrigido para `Salvar alterações`. Validacoes passaram: `npm run lint`, `npm run build` e `cargo test` (26 testes).

Atualizacao em 2026-05-13: foi criado `ESTRUTURA_BANCO_DADOS.md`, documentando o SQLite local, localizacao do arquivo, fluxo de inicializacao, tabelas, relacionamentos, indices, comandos principais e regras para evolucao do schema.

Atualizacao em 2026-05-13: foram aplicadas recomendacoes dos documentos externos `GuilhermeFerrarezi Projeto Biblioteca de Jogos Unificada.docx` e `Biblioteca de Jogos Unificada.docx`. O service de frontend foi renomeado de `libraryApi.js` para `libraryService.js`; mocks passaram a ser carregados dinamicamente apenas em `import.meta.env.DEV`; constantes compartilhadas foram centralizadas em `src/constants/libraryConstants.js`; o adapter manual ganhou `Object.freeze` e validacao de entrada; `LibraryBrowser` foi quebrado em subcomponentes internos; `Sidebar`, filtros e alternancia de visualizacao receberam `aria-pressed`; `Topbar` recebeu `aria-busy` e acao para filtro; `ManualGameModal` passou a fechar com Escape e marcar campos invalidos; `StatsGrid` ganhou subcomponente de metrica; `ErrorBoundary` foi adicionado; e `library.css` passou a usar CSS Custom Properties para tokens principais. Validacoes passaram: `npm run lint`, `npm run build` e `cargo test` (26 testes).

Atualizacao em 2026-05-13: o documento externo `Biblioteca de Jogos Unificada - agentes.skills.docx` foi incorporado as diretrizes de trabalho. Todos os agentes em `cloud - biblioteca de jogos/agents` foram reforcados com responsabilidades mais claras de coordenacao, pesquisa de plataforma, arquitetura extensivel, seguranca Tauri/Auth, providers, UX, metadados, QA, backend, frontend, integracao e banco. Tambem foram adicionadas as skills `project-scoping-and-coordination`, `platform-viability-matrix`, `architecture-extensibility-blueprint`, `tauri-desktop-security-hardening`, `provider-error-standardization`, `ui-component-standardization`, `react-performance-optimization`, `metadata-fallback-logic`, `deduplication-heuristics-engine`, `sqlite-schema-versioning`, `token-lifecycle-hardening` e `usability-heuristics-evaluation`. As skills existentes foram atualizadas com matriz de risco por API, ciclo de vida de tokens, design system/acessibilidade/performance, modelo minimo de `LaunchAction`, classificacao de viabilidade, hardening de executaveis locais, QA de migrations, indices parciais e versionamento SQLite. O indice `cloud - biblioteca de jogos/README_PROJETO.md` agora documenta quando usar cada skill transversal.

Atualizacao em 2026-05-13: foi implementado o primeiro corte do `SteamProvider` local, sem credenciais e sem Web API. O backend Tauri ganhou o comando `sync_steam_games`, que detecta instalacoes Steam por raizes padrao ou pela variavel `BIBLIOTECA_JOGOS_STEAM_ROOTS`, le `steamapps/libraryfolders.vdf`, percorre bibliotecas adicionais e importa `appmanifest_*.acf` como entradas `steam` no SQLite. Cada jogo Steam importado usa `game_sources.external_id` como AppID, `install_status = installed` e `launch_actions.kind = uri` com alvo `steam://rungameid/<appid>`. Apos revisao com agentes, o sync passou a nao atualizar timestamps quando nada mudou, a marcar manifests removidos como `not_installed`, a fazer upsert da acao Steam primaria com filtro por plataforma e a expor feedback acessivel no frontend. O frontend ganhou acao de sincronizar Steam na topbar e mensagens de resumo. Foram adicionados testes Rust para importacao idempotente de manifests, leitura de bibliotecas extras e manifest removido; validacoes passaram: `npm run lint`, `npm run build` e `cargo test` (29 testes). A integracao Steam via Web API continua como etapa seguinte para biblioteca completa, playtime e metadados de conta, pois exige chave Web API e depende da visibilidade da biblioteca do usuario.

Atualizacao em 2026-05-13: as diretrizes de governanca foram reforcadas para exigir que todo novo desenvolvimento identifique antes da implementacao o agente principal em `cloud - biblioteca de jogos/agents`, agentes auxiliares, skills aplicaveis em `cloud - biblioteca de jogos/skills`, criterios de validacao e delegacoes planejadas. Subagentes tecnicos podem ser usados para execucao ou revisao, mas devem ser instruidos a seguir os agentes e skills locais, sem substituir a governanca do projeto.

Atualizacao em 2026-05-14: foi criado `ULTIMOS_DESENVOLVIMENTOS_APLICATIVO.md`, um resumo de leitura rapida dos marcos recentes do aplicativo. O arquivo consolida governanca de agentes/skills, reorganizacao do frontend, melhorias de UI/acessibilidade, persistencia SQLite, jogos manuais, lancamento seguro, `LocalGamesProvider`, primeiro corte do `SteamProvider` local, validacoes recentes, commits relevantes e proximos cortes recomendados.

Atualizacao em 2026-05-14: apos teste manual da sincronizacao Steam, foi corrigido um falso positivo do `SteamProvider` local. O AppID `228980` (`Steamworks Common Redistributables`) agora e rejeitado na descoberta por manifest e qualquer entrada Steam desse tipo ja importada e arquivada na proxima sincronizacao, removendo-a da listagem principal sem apagar dados. Foi adicionado teste Rust para garantir que redistribuiveis comuns da Steam sejam ignorados/arquivados.

Atualizacao em 2026-05-14: apos revisao delegada aos agentes locais `04-backend-provider-agent.md`, `11-senior-database-agent.md` e `10-senior-integration-qa-agent.md`, a correcao do `Steamworks Common Redistributables` foi refinada. O arquivamento de entradas Steam tecnicas passou a ocorrer dentro da mesma transacao da sincronizacao, a heuristica automatica ficou baseada no AppID `228980`, e o feedback da UI passou a informar tambem a quantidade de entradas Steam arquivadas.

Atualizacao em 2026-05-14: foi implementado o primeiro corte da area `Contas e integracoes`, com execucao delegada ao agente local de frontend/UX e revisao de escopo pelo agente de seguranca/QA. O botao `Contas` da sidebar agora abre uma tela real para Steam, Xbox/Game Pass e Epic. A Steam exibe o estado de sincronizacao local ativa e permite disparar a sincronizacao Steam ja existente; Xbox e Epic aparecem como integracoes planejadas. A tela nao pede nem salva API key, token, senha, cookie ou Steam Guard, preservando a decisao de implementar `AuthVault`/cofre seguro antes da Web API. Validacoes passaram: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-14: foi implementado o primeiro fluxo seguro de `Entrar com Steam` usando Steam OpenID. A implementacao seguiu os agentes locais `01-platform-research-agent`, `03-security-auth-agent`, `04-backend-provider-agent`, `09-senior-frontend-development-agent` e `10-senior-integration-qa-agent`, com skills de pesquisa de plataforma, compliance, seguranca de tokens, backend, frontend e qualidade. O app abre o login oficial da Steam no navegador externo, recebe o callback em `127.0.0.1`, valida a resposta com a Steam por `check_authentication` e persiste somente o SteamID64 em `provider_account_configs`. O frontend deixou de usar `localStorage` como fonte de verdade para SteamID64 e passou a carregar/salvar a conta pelo backend. A sincronizacao por conta agora usa o SteamID64 salvo no SQLite e a Web API key do AuthVault/keyring. O fluxo nao captura senha, Steam Guard, cookies, sessao de navegador, HTML bruto ou URL completa de callback. Validacoes passaram: `npm run lint`, `npm run build`, `cargo check` e `cargo test` com 41 testes.

Atualizacao em 2026-05-14: apos teste manual apontar divergencia entre `Cofre configurado` e falha de sincronizacao, o estado da Steam Web API foi reconciliado seguindo revisao dos agentes locais de seguranca, backend/provider, banco e frontend/QA. O status de Web API agora considera configurado apenas quando o AuthVault/keyring retorna uma chave legivel; o sync por conta deixou de usar fallback de segredo em SQLite. O SQLite mantem apenas metadados nao secretos, como `steam_web_api_key_configured`, e colunas legadas de teste com segredo passam a ser ignoradas. O frontend tambem deixou de marcar a sincronizacao por conta como finalizada quando o hook global captura erro, propagando a falha para o painel da conta. O sync remoto agora preenche `game_sources.account_id` para jogos Steam que ja tinham sido importados localmente. Validacoes passaram: `npm run lint`, `npm run build`, `cargo fmt -- --check` e `cargo test` com 42 testes.

Atualizacao em 2026-05-15: apos teste manual indicar falha do cofre do Windows ao validar a leitura da Steam Web API key, o AuthVault foi endurecido com fallback DPAPI local. O keyring/cofre do sistema operacional segue como armazenamento primario; quando ele aceita gravacao mas nao retorna a credencial no read-back, o backend grava um arquivo cifrado por DPAPI em `%APPDATA%\\com.bibliotecajogos.unificada\\auth-vault\\steam-web-api-key.dpapi`, vinculado ao usuario Windows e sem usar SQLite, JSON, `localStorage`, frontend ou logs para segredo. O fluxo foi revisado com os agentes locais de seguranca/auth e integracao/QA, e ajustado para evitar perda de credencial antiga quando o fallback falha e para limpar arquivos temporarios DPAPI na desconexao. O teste manual do usuario confirmou que o salvamento da Web API key passou a funcionar. Validacoes passaram: `npm run lint`, `npm run build` e `cargo test` com 42 testes.

Atualizacao em 2026-05-15: a biblioteca principal ganhou selecao multipla de filtros e o filtro `Nao instalados`. Os filtros rapidos agora funcionam como toggles acumulativos, com `Todos` limpando a selecao e a combinacao adotada por interseccao. A mudanca foi aplicada na sidebar e nos chips da area principal sem alterar busca, alternancia de visualizacao, selecao de jogo ou feedback. Validacoes passaram: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-15: a semantica dos filtros foi refinada para combinar por categoria. Status (`Instalados` / `Nao instalados`) passam a combinar por OR entre si, plataformas (`Steam` / `Locais`) tambem combinam por OR entre si, e as categorias continuam combinando entre si por AND. A busca segue como filtro adicional sobre o resultado final. Validacoes passaram novamente: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-15: o estado vazio da biblioteca ficou contextual. Agora a UI diferencia biblioteca vazia de nenhum resultado por busca/filtros, com mensagens especificas para busca, combinacao de filtros e ausencia total de itens. Validacoes passaram novamente: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-15: a mensagem de estado vazio ficou mais especifica para filtros. Quando a busca nao e o fator principal, a UI passa a indicar se a ausencia veio de status, plataforma ou da combinacao entre ambos, em vez de usar apenas um texto generico. Validacoes passaram novamente: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-15: a tela `Contas e integracoes` da Steam foi refinada com um disclosure inline. A linha principal agora mostra apenas o resumo e uma seta; ao expandir, aparecem os controles de login, sincronizacao local, sincronizacao por conta, SteamID64 e Steam Web API key no mesmo painel. O toggle recebeu `aria-expanded`, `aria-controls`, foco previsivel ao abrir/fechar e o layout foi ajustado para manter o conteudo acessivel em desktop. Validacoes passaram novamente: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-15: o corte seguinte da Steam Web API foi consolidado no backend. O comando `sync_steam_account_games` ganhou classificacao interna de erro com `ProviderErrorDto`, parse mais tolerante no cliente da Web API, e persistencia de metadados de sincronizacao nao sensiveis em `provider_account_configs.config_json`. A UI do frontend permaneceu inalterada porque o contrato atual ja cobre o retorno de sucesso/erro necessario. Validacao reportada pelo backend: `cargo test --lib` com 50 testes passando, sem necessidade de ajuste adicional em frontend.

Atualizacao em 2026-05-15: a camada de feedback da Steam passou a exibir mensagem curta por padrao e detalhes tecnicos expansiveis quando a operacao falha. O backend agora emite `steam-sync-failed` com o erro sanitizado e o frontend usa `ProviderFeedback`/`StatusDisclosure` para mostrar apenas o resumo inicialmente, mantendo `code`, `phase` e `details` em area secundaria. Validacoes passaram novamente: `npm run lint` e `npm run build`.

Atualizacao em 2026-05-18: a documentacao foi reconciliada com o estado atual do aplicativo. O frontend ja esta em JavaScript/JSX com a tela de contas visualmente refinada, a Steam ja entrega playtime e metadados via Web API, o allowlist do Tauri foi fechado por `build.rs`/capability, e a base de smoke automatica cobre os contratos mais sensiveis de biblioteca, selecao e lancamento. O proximo foco de produto nao e mais a Steam em si, mas a expansao para Xbox/Game Pass e Epic Games, seguida por consultas filtradas/paginacao para bibliotecas maiores e um reforco de smoke/e2e quando houver stack dedicada.

Atualizacao em 2026-05-18: a biblioteca do frontend passou a unificar visualmente o mesmo jogo vindo de Steam e Xbox em um unico registro quando o titulo coincide. O card/lista mostra as duas plataformas como origem do mesmo item, os filtros passam a considerar Xbox como plataforma rapida, e o painel de detalhes exibe um chooser de launch para perguntar se o usuario quer iniciar pela Steam ou pelo Xbox antes de disparar a acao correspondente.

Atualizacao em 2026-05-18: a descoberta local do Xbox foi refinada para filtrar apps de sistema e produtividade que estavam entrando indevidamente como jogos, como Skype, Filmes e TV, Noticias e IntelliGo Neptune. Jogos de desktop/launcher como `osu!` deixaram de ser classificados como Xbox e passaram a entrar pela descoberta local, mantendo a loja Xbox apenas para pacotes reais do ecossistema Microsoft. A regra de merge prioriza `local` quando houver duplicidade futura com Steam ou Xbox.

Atualizacao em 2026-05-19: a heuristica do Xbox local foi endurecida novamente para bloquear falsos positivos de sistema/loja como `Filmes e TV` e cortar registros de instalacao desktop local que apareciam como Xbox. O corte manteve o caso legitimo do `Minecraft Launcher` e reforcou que jogos de desktop como `osu!` continuam no fluxo `local`. Foram adicionados testes Rust cobrindo a variante `Filmes e TV`, um falso positivo desktop local e a preservacao do caso Minecraft. Validacoes passaram: `cargo test`, `npm run lint` e `npm run build`.

Atualizacao em 2026-05-19: o scanner local foi ajustado para tratar `staging` como diretório auxiliar, impedindo que itens como `_staging` entrem na biblioteca como jogos locais. A mudanca ficou concentrada em `is_helper_directory()` e ganhou teste dedicado sem quebrar os casos legitimos de jogo local com executavel e subpastas validas. Validacao passada: `cargo test`.

Atualizacao em 2026-05-19: a limpeza do Xbox passou a considerar o alvo de lancamento persistido para arquivar falsos positivos antigos que ainda tinham acao executavel de desktop local. O provider continua aceitando Appx/Xbox reais, mas agora consegue remover entradas persistidas claramente ligadas a binarios locais quando a descoberta volta a rodar. Foram adicionados testes Rust para arquivamento de falso positivo desktop local e preservacao de um alvo Appx real. Validacao passada: `cargo test` com 92 testes.

Atualizacao em 2026-05-20: o corte de refinamento do Xbox local foi endurecido para bloquear infraestruturas do Windows/Store como `App Installer`, `Windows App Runtime`, `Windows App SDK` e pacotes correlatos antes que entrem como jogos. A heuristica de rejeicao agora cruza `title`, `package_name`, `package_family_name` e `app_id` de forma mais ampla, mantendo o caso legitimo de jogos de desktop como `osu!` no fluxo `local`. Em paralelo, o boot percebido do app foi reduzido ao mover a tela `Contas e integracoes` para carregamento sob demanda com `React.lazy`/`Suspense` e ao adiar a consulta de identidade Xbox para quando a tela de contas e aberta, tirando essa leitura do caminho inicial da biblioteca.

Atualizacao em 2026-05-20: o Xbox/Game Pass local passou a aceitar pastas adicionais configuraveis, seguindo o mesmo padrao de persistencia ja usado pela Steam. O backend Tauri ganhou `get_xbox_library_roots` e `save_xbox_library_roots`, com armazenamento em `provider_account_configs.config_json` sob `provider_id = 'xbox'` e chave `additionalGameRoots`. A descoberta Xbox agora combina roots salvas, a variavel `BIBLIOTECA_JOGOS_XBOX_ROOTS` e o fallback de drives locais, varrendo `XboxGames` nessas origens. A tela de `Contas e integracoes` ganhou painel proprio para selecionar e salvar pastas Xbox adicionais. Validacoes executadas neste corte: `npm run lint`, `npm run build` e `cargo test` (99 testes).

Atualizacao em 2026-05-20: a busca local generica tambem foi ampliada para varrer todos os armazenamentos disponiveis em vez de ficar limitada ao conjunto padrao de pastas conhecidas. O coletor local agora agrega `BIBLIOTECA_JOGOS_LOCAL_ROOTS`, pastas usuais do usuario e de instaladores, e todas as letras de drive existentes, mantendo a deduplicacao e os filtros de rejeicao de runtimes/helper apps. Isso faz o `sync_local_games` enxergar instalacoes fora do disco principal sem depender de bibliotecas Steam ou Xbox.

Atualizacao em 2026-05-20: a busca local ficou configuravel no menu de `Padroes da biblioteca`. O backend agora persiste `localScanMode`, `localScanRoots` e `localScanExcludedRoots` no objeto `library` de `provider_account_configs`, com modos `automatic`, `selected_only` e `automatic_plus_extra`. O modo `automatic` mantém a varredura atual; `selected_only` usa apenas as pastas escolhidas; `automatic_plus_extra` combina o scan automatico com pastas extras; e as exclusoes valem em todos os modos. A interface ganhou select de modo, campos para pastas extras e pastas excluidas, com picker e salvamento proprio.

Atualizacao em 2026-05-20: a UX da area de Xbox em `Contas e integracoes` foi corrigida depois do corte de scan local. O painel de pastas adicionais voltou a ser disclosure inline, separado dos controles rapidos de sincronizacao e importacao, para evitar que a configuracao ficasse sempre exposta e quebrasse a leitura visual da pagina.

Atualizacao em 2026-05-20: a capability `default` do Tauri foi corrigida para incluir os comandos Xbox de leitura e gravacao (`get_xbox_account_config`, `save_xbox_account_config`, `get_xbox_library_roots`, `save_xbox_library_roots`). Essa permissao faltante era a causa do erro `Command not found / not allowed` exibido ao abrir as configuracoes do Xbox.

Atualizacao em 2026-05-20: o card `Padroes da biblioteca` foi refinado para ficar mais leve e legivel. Agora ele mostra o estado atual da loja padrao e do scan em chips resumidos, separa melhor as raizes do scan e as exclusoes, e reduz a densidade visual sem alterar o comportamento funcional.

Atualizacao em 2026-05-20: o login Xbox Live foi corrigido para seguir o fluxo desktop suportado pela Microsoft. Em vez de callback local em `127.0.0.1`, o aplicativo agora abre uma janela webview controlada pelo proprio Tauri e usa `https://login.live.com/oauth20_desktop.srf` como redirect oficial, capturando o `code` pela navegacao da janela. O `client_id` passou a ser configuravel nas padroes da biblioteca, porque o client first-party da Microsoft usado antes nao era consentivel para o nosso caso; o usuario precisa informar o `Application (client) ID` do proprio app registration Microsoft. O backend continua modularizado: o modulo de auth cuida apenas do login e da sessao, enquanto o provider segue responsavel pelo merge e pela importacao de title history. Validacoes executadas neste corte: `npm run lint`, `npm run build` e `cargo test` (103 testes).

Atualizacao em 2026-05-20: o Xbox Live ganhou o campo de `client secret` no painel de contas e o backend passou a salvá-lo com seguranca no AuthVault, separado do `client_id` que continua nas configuracoes da biblioteca. O fluxo de login e o refresh de token agora leem tanto o `Application (client) ID` quanto o `client secret` antes de concluir o exchange, mantendo o segredo fora do SQLite e sem expor o valor na interface depois do salvamento. Validacoes executadas neste corte: `npm run lint`, `npm run build` e `cargo test` (104 testes).

Atualizacao em 2026-05-20: o fluxo Xbox Live foi alinhado com os endpoints oficiais da Microsoft para `consumers/oauth2/v2.0`, e a troca de token passou a expor a resposta real do provedor quando a autorizacao falha. Isso ajuda a diagnosticar problemas de app registration sem depender de erro generico no popup. Validacao executada neste corte: `cargo test` (104 testes).

Atualizacao em 2026-05-20: o modulo de auth Xbox Live foi instrumentado para incluir status HTTP e corpo bruto das respostas quando o token exchange ou as chamadas posteriores falham. Isso torna o erro visivel no frontend muito mais util para depurar consentimento, client secret, redirect URI e respostas de XSTS sem precisar adivinhar a causa. Validacao executada neste corte: `cargo test` (104 testes).

Atualizacao em 2026-05-20: o redirect do login Xbox Live foi ajustado para o URI desktop recomendado pela Microsoft, `https://login.microsoftonline.com/common/oauth2/nativeclient`. O app registration precisa usar esse redirect para que o callback seja reconhecido pelo fluxo novo. Validacao executada neste corte: `cargo test` (104 testes).

Atualizacao em 2026-05-20: ficou definido como prioridade do proximo ciclo remover a dependencia de `client secret` local no fluxo Xbox Live para viabilizar distribuicao a terceiros com uma arquitetura mais adequada para desktop/public client. O estado atual serve para validacao local, mas o corte seguinte deve revisar essa arquitetura antes de qualquer novo refinamento de UI ou importacao. O `client secret` nao deve permanecer como requisito de uso final no caminho de entrega do aplicativo.

## Prioridade de plataformas

1. Steam - plataforma principal e primeira integracao real do MVP.
2. Xbox/Game Pass - segunda prioridade, com pesquisa especifica sobre limites de API publica e integracao local no Windows.
3. Epic Games - terceira prioridade, provavelmente com abordagem experimental por causa das limitacoes de API publica de biblioteca.
4. Outras plataformas - somente depois que Steam, Xbox e Epic estiverem bem definidas.

## Conclusoes principais

- Steam tem API oficial para jogos do usuario, com restricoes de privacidade e chave Web API.
- Epic tem login oficial via Epic Account Services, mas nao parece oferecer API publica completa para biblioteca de usuario em apps genericos. Projetos como Heroic usam Legendary.
- GOG tem o ecossistema Galaxy e uma API Python de integracoes para importar jogos, conquistas, tempo de jogo e recursos relacionados, mas nao e prioridade inicial.
- Playnite prova que a ideia e viavel no Windows usando plugins, sessoes locais e launchers existentes.
- GOG Galaxy prova que o modelo de integracoes oficiais e comunitarias tambem e viavel.
- Xbox PC App com biblioteca agregada mostra que a industria esta caminhando nessa direcao, mas isso nao implica API publica para terceiros.
- Battle.net, Ubisoft, EA e Rockstar tendem a exigir integracoes locais, leitura de instalacoes, protocolos de launcher ou plugins comunitarios.

## Arquitetura sugerida

```text
App Desktop
+-- Core Library
|   +-- catalogo unificado de jogos
|   +-- status instalado/nao instalado
|   +-- tempo jogado
|   +-- tags, filtros e colecoes
|   +-- historico de sincronizacao
+-- Provider Layer
|   +-- SteamProvider
|   +-- XboxProvider
|   +-- EpicProvider
|   +-- GOGProvider futuro
|   +-- ItchProvider futuro
|   +-- LocalGamesProvider
|   +-- providers comunitarios futuros
+-- Auth Vault
|   +-- OAuth quando disponivel
|   +-- tokens/sessoes criptografados localmente
|   +-- revogacao e reconexao de contas
+-- Metadata Layer
|   +-- capas
|   +-- descricoes
|   +-- generos
|   +-- screenshots
|   +-- IDs cruzados entre lojas
+-- Launcher Layer
    +-- protocolos como steam://
    +-- executaveis locais
    +-- launchers oficiais
    +-- comandos por provider
```

## Diretrizes de desenvolvimento

As diretrizes consolidadas ficam em `DIRETRIZES_DESENVOLVIMENTO.md`. Em resumo:

- Manter frontend em JavaScript/JSX e backend Tauri em Rust.
- Separar UI, services, adapters, hooks, persistencia e operacoes nativas.
- Usar o padrao Service-Adapter para novas integracoes como Steam, Xbox, Epic e providers locais.
- Normalizar toda origem de dados para o contrato interno `LibraryEntry` antes de chegar na interface.
- Centralizar constantes de dominio no frontend e validar formularios antes de chamar comandos Tauri.
- Manter acessibilidade basica em controles, modais e processos assincronos.
- Usar SQLite como fonte principal de persistencia no desktop; LocalStorage/IndexedDB somente como cache auxiliar de modo web quando necessario.
- Evoluir a estrutura de pastas incrementalmente, sem refatoracao ampla desnecessaria.
- Validar cada marco com `npm run lint`, `npm run build` e `cargo test` quando houver backend.

## Banco de dados

A estrutura SQLite local esta documentada em `ESTRUTURA_BANCO_DADOS.md`. O banco principal fica em `%APPDATA%\com.bibliotecajogos.unificada\library.sqlite3` e e composto por `games`, `library_entries`, `game_sources`, `launch_actions`, `game_genres` e `schema_migrations`, com indices para listagem, filtros, acoes primarias e limpeza de falsos positivos locais.

## MVP recomendado

1. App desktop Windows.
2. Importacao da biblioteca Steam.
3. Importacao de jogos locais instalados.
4. Cadastro manual de jogos.
5. Modelo interno unico de jogo.
6. Tela de biblioteca com busca, filtros e detalhes.
7. Lancamento de jogo pelo launcher original ou executavel local.
8. Pesquisa e prototipo do XboxProvider.
9. Integracao experimental com Epic em etapa posterior.

## Riscos

- APIs e endpoints internos podem mudar sem aviso.
- Algumas plataformas podem proibir scraping, automacao ou uso de endpoints nao documentados.
- Guardar credenciais de usuario de forma errada seria um risco critico.
- Sincronizacao de tempo de jogo e conquistas varia muito por plataforma.
- Instalacao/download de jogos e muito mais complexa que apenas listar e lancar jogos.

## Proxima sessao sugerida

1. Em qualquer maquina nova, seguir `RETOMADA_NOVO_COMPUTADOR.md` para preparar Node.js, Rust, Visual Studio Build Tools, dependencias npm e validacoes iniciais.
2. Rodar `npm run lint`, `npm run build` e, quando o ambiente nativo estiver pronto, `npm run tauri:dev` dentro de `aplicativo`.
3. Rodar `cargo test` em `aplicativo/src-tauri` usando `CARGO_TARGET_DIR=%LOCALAPPDATA%\BibliotecaJogosUnificada\cargo-target`.
4. Manter espaco livre suficiente no C: antes de novos builds Tauri; 4,45 GB livres funcionaram para o empacotamento anterior, mas ainda e uma margem apertada.
5. Validar manualmente a sincronizacao local com uma pasta controlada via `BIBLIOTECA_JOGOS_LOCAL_ROOTS`, conferindo insercao incremental e evitando falsos positivos.
6. Xbox/Game Pass local ja entrou como provider experimental no Windows: o app descobre jogos instalados via inventario local, abre o jogo com `explorer.exe` + `shell:AppsFolder` quando instalado e usa Microsoft Store quando nao instalado. A heuristica foi refinada para cobrir melhor pacotes reais sem reabrir helpers do Windows/Xbox. Achievements/title history continuam apenas como sinal auxiliar, nunca ownership.
7. Validar e refinar a importacao de title history do Xbox agora que o fluxo Xbox Live autenticado ja esta integrado, mantendo o contrato de seguranca e sem expor credenciais.
8. Se a importacao de title history ainda nao for prioridade, melhorar a UX do erro de importacao Xbox para deixar mais claro ao usuario por que a operacao esta indisponivel e o que ele precisa configurar.
   A descoberta local tambem recebe explicitamente jogos de desktop populares como `osu!`, que aparecem como `local` e nao como `xbox`.
7. Em paralelo ou logo depois, fazer o corte inicial de Epic Games com foco em viabilidade e limites de API/compliance antes de qualquer fluxo de conta.
8. Depois das novas plataformas, adicionar consulta filtrada/paginacao no backend para bibliotecas maiores e ajustar a UI para suportar listas mais longas.
9. Quando houver stack dedicada de browser automation, criar smoke/e2e real para bootstrap, login Steam, syncs e launch, usando a base de contratos ja existente.
10. Manter a tela de contas e a Steam em modo de manutenção incremental, priorizando regressao e pequenos refinamentos apenas quando surgirem gaps claros.
11. O proximo ajuste do projeto e reduzir o tempo de abertura do aplicativo, porque o Tauri esta demorando demais para mostrar a interface completa.

Atualizacao em 2026-05-21: o login Xbox Live foi convertido para fluxo de `public client` desktop com `authorization code + PKCE`, sem dependencia de `client_secret` no caminho critico. O usuario final agora apenas autentica com a conta Microsoft/Xbox; o `client_id` ficou como configuracao interna ou de build para a instancia do projeto, e o refresh token continua persistido no AuthVault do backend. Validacoes do corte: `npm run lint`, `npm run build` e `cargo test` passaram.

Atualizacao em 2026-05-21: ficou registrado para a build final que o `client_id` do Xbox Live nao deve permanecer como campo editavel para o usuario final na tela de configuracoes. O valor deve vir de configuracao interna da instancia ou de variavel de build/env, e a UI final deve apenas exibir estado/erro se essa configuracao faltar.

Atualizacao em 2026-05-22: o fluxo Xbox Live publico foi refinado ate concluir login e importacao por title history/achievements com paginacao mais ampla, deduplicacao de aliases curtos e filtros para apps de midia como YouTube/Netflix. A sincronizacao Steam por conta ganhou protecao contra execucoes concorrentes, timeout/retry mais seguro no `GetOwnedGames`, aquecimento de cache de artwork em background sem sobrepor sincronizacoes e limpeza de falsos positivos locais como `Program Files`, `Python312`, drivers, `XboxGames`, Dell/NVIDIA/AMD/Intel. A integracao de capas Steam foi separada em `coverUrl`, `heroUrl` e `fallbackUrl`: cards usam capa vertical com fallback para imagem antiga, o painel lateral usa banner Steam com fallback, e a CSP do Tauri aceita os CDNs Steam necessarios. O painel de detalhes passou a trocar status, caminho, tempo, ultima vez e acao conforme o launcher selecionado em jogos agrupados Steam + Xbox, mantendo a artwork preferencialmente da Steam. Tambem foram refinados o tamanho do banner do painel para a proporcao Steam `1920x620`, o texto/botao de lancamento por plataforma, a posicao do menu `Contas` no rodape da sidebar e a porta fixa `5173` do Vite com `strictPort` para evitar tela branca por servidor dev em porta diferente. Validacoes executadas durante o corte: `cargo test`, `npm run lint`, `npm run test:smoke` e `npm run build`.

Proxima prioridade sugerida em 2026-05-22: testar manualmente uma biblioteca agrupada Steam + Xbox grande, conferindo se `Caminho`, tempo jogado, status e acao mudam corretamente ao alternar launcher; depois iniciar a persistencia/edicao manual de artwork para jogos que ainda dependem de fallback ou imagem incorreta.

