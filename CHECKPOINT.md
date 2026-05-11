# Checkpoint - Biblioteca de Jogos Unificada

Data: 2026-05-11

## Objetivo do projeto

Criar um aplicativo desktop que funcione como uma biblioteca central de jogos, reunindo contas, bibliotecas, jogos instalados e metadados. As plataformas prioritarias do projeto sao Steam como foco principal, seguida por Xbox/Game Pass e Epic Games. Outras plataformas, como GOG, itch.io, Battle.net, Ubisoft Connect, EA App e jogos locais, ficam como expansoes futuras ou auxiliares.

## Estado atual

Foi feita uma pesquisa inicial de viabilidade. A conclusao principal e que o aplicativo e tecnicamente possivel, mas as integracoes nao terao o mesmo nivel de suporte em todas as plataformas.

Tambem foi iniciada a base do aplicativo dentro da pasta `aplicativo`, usando Tauri 2, React, TypeScript e Vite. O frontend ja possui uma tela de biblioteca com sidebar, resumo, busca, filtros, visualizacao por capas/lista, painel de detalhes e cadastro manual em memoria. A estrutura `src-tauri` foi criada e a execucao desktop nativa ja chegou a compilar localmente apos a instalacao de Rust/Cargo e Visual Studio Build Tools com MSVC/Windows SDK.

Decisao de design adicionada em 2026-05-07: o aplicativo deve usar modo escuro como experiencia padrao. A area de jogos deve abrir por padrao na visualizacao por capas, com cards de capa e nome do jogo abaixo, mantendo a alternativa em lista para leitura rapida de status e tempo jogado.

Atualizacao em 2026-05-07: os modelos centrais foram separados em `aplicativo/src/domain`, incluindo `Game`, `PlatformAccount`, `Provider`, `LaunchAction`, `LibraryEntry` e `SyncHistory`. Os dados mockados da biblioteca foram movidos para `aplicativo/src/data/mockLibrary.ts`, e a UI passou a consumir `LibraryEntry` sem alterar o comportamento visual.

Atualizacao em 2026-05-07: foram adicionados agentes senior de programacao em `cloude teste/agents`: backend, frontend e integracao/qualidade. Tambem foram criadas as skills `senior-backend-implementation`, `senior-frontend-implementation` e `senior-integration-quality` para orientar implementacao, revisao e verificacao tecnica.

Atualizacao em 2026-05-07: o cadastro manual de jogos foi implementado no frontend em memoria. O botao `Adicionar jogo` abre um modal com titulo obrigatorio, genero, status instalado/nao instalado e acao de lancamento opcional. Ao salvar, a UI adiciona um `LibraryEntry` manual no topo da biblioteca, com `LaunchAction`, tempo de jogo zerado e cor deterministica.

Atualizacao em 2026-05-07: Rustup foi instalado com sucesso e `rustc 1.95.0`/`cargo 1.95.0` estao disponiveis pelo caminho `%USERPROFILE%\.cargo\bin`. A primeira tentativa de instalar Visual Studio Build Tools 2022 com workload C++ falhou por espaco insuficiente, mas apos liberar espaco no C: a instalacao foi concluida. `npm run tauri:dev` ja compila e executa o app nativo Tauri localmente.

Atualizacao em 2026-05-07: durante a primeira compilacao Tauri, o OneDrive bloqueou a leitura de `aplicativo/src-tauri/build.rs` com erro de operacao de nuvem. O arquivo foi recriado localmente com o conteudo padrao do Tauri para destravar a compilacao.

Atualizacao em 2026-05-08: a tela de biblioteca passou a ter selecao real de jogo ao clicar em cards/lista, busca por nome/plataforma/genero/status, filtros rapidos funcionais (`Todos`, `Instalados`, `Steam`, `Locais`) e feedback para acoes de lancamento/instalacao. O botao `Jogar` tenta abrir URI quando a acao e do tipo `uri`; execucao de executaveis locais ainda depende de comando Tauri especifico. O ESLint passou a ignorar `aplicativo/src-tauri/target`, que contem arquivos gerados pelo build Rust.

Atualizacao em 2026-05-10: a documentacao foi alinhada com a versao atual do aplicativo. O estado confirmado e: frontend funcional com dados mockados e cadastro manual somente em memoria; modelos de dominio separados em `aplicativo/src/domain`; dados iniciais em `aplicativo/src/data/mockLibrary.ts`; backend Tauri ainda no scaffold inicial, sem comandos de dominio, persistencia local ou providers reais. Foi criado o guia `RETOMADA_NOVO_COMPUTADOR.md` para continuar o trabalho em outra maquina.

Atualizacao em 2026-05-10: a Fase 1 de validacao geral foi executada com apoio do agente de integracao/QA. Passaram: verificacao de ambiente (`node`, `npm`, `rustc`, `cargo`), `npm run lint`, `npm run build`, `cargo check`, `npm audit --audit-level=moderate`, subida nativa por `npm run tauri:dev` com janela `Biblioteca de Jogos`, e testes funcionais automatizados da UI web cobrindo estado inicial, metricas, filtros, busca, modo lista/capas, selecao, cadastro manual, validacao de titulo obrigatorio, feedback de jogar/instalar e viewport minimo 960x640. O protocolo `steam://` esta registrado no Windows e o Steam estava em execucao, mas nao foi disparado um jogo real para evitar efeito colateral no ambiente. Uma primeira tentativa de `npm run tauri:build` compilou o `app.exe` release, mas falhou ao empacotar MSI por falta de espaco em disco ao extrair o WiX (`os error 112`). Apos liberar espaco, `npm run tauri:build` passou e gerou os instaladores em `aplicativo/src-tauri/target/release/bundle`: MSI (`Biblioteca de Jogos Unificada_0.1.0_x64_en-US.msi`) e NSIS (`Biblioteca de Jogos Unificada_0.1.0_x64-setup.exe`). O C: ficou com cerca de 4,45 GB livres apos a geracao dos pacotes.

Atualizacao em 2026-05-10: foi criado o agente `cloude teste/agents/11-senior-database-agent.md` para orientar a Fase 2. O agente fica responsavel por schema SQLite, migracoes, repositories, constraints, indices, seed/fallback de dados mockados e criterios de aceite para persistencia local.

Atualizacao em 2026-05-10: foram criadas as skills `sqlite-local-persistence-design` e `sqlite-migrations-repositories` em `cloude teste/skills` para apoiar o agente senior de banco de dados na definicao do schema SQLite, migracoes, repositories, queries, transacoes e criterios de aceite da persistencia local.

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
5. Testar manualmente no Tauri o fluxo completo: abertura rapida, bootstrap dos 4 mocks, adicionar jogo manual, editar, arquivar, reabrir e confirmar persistencia.
6. Validar manualmente a sincronizacao local com uma pasta controlada via `BIBLIOTECA_JOGOS_LOCAL_ROOTS`, conferindo insercao incremental e evitando falsos positivos.
7. Corrigir textos com acentuacao quebrada no frontend, como `Salvar alteraÃ§Ãµes`, antes de ampliar a interface.
8. Iniciar `SteamProvider` como primeira integracao real.
9. Depois, adicionar consulta filtrada/paginacao no backend para bibliotecas maiores e pesquisar/prototipar `XboxProvider` e `EpicProvider`.
