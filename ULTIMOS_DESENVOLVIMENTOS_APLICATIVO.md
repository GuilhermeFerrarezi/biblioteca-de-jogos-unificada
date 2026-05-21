# Ultimos Desenvolvimentos do Aplicativo

Data de referencia: 2026-05-20

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

## Xbox heuristics hardening

- A heuristica do Xbox local foi reforcada para rejeitar apps de sistema/loja como `Filmes e TV`.
- O provider passou a cortar falsos positivos de instalacao local de desktop, mantendo jogos como `osu!` no fluxo `local` em vez de `xbox`.
- Foram adicionados testes Rust cobrindo a variante `Filmes e TV`, um falso positivo desktop local e o caso legitimo do `Minecraft Launcher`.

## Xbox persisted cleanup

- A limpeza de Xbox persistido agora tambem considera o alvo de lancamento armazenado para arquivar entradas antigas claramente ligadas a executaveis desktop locais.
- O provider continua preservando Appx/Xbox reais, inclusive quando a descoberta roda sem encontrar o mesmo registro na forma atual.
- Foram adicionados testes Rust para o falso positivo desktop local e para um alvo Appx real continuar valido.

## Local staging hardening

- O scanner local passou a tratar `staging` como diretório auxiliar.
- Itens sob `_staging` nao entram mais como jogos locais, reduzindo ruido de build/infra.
- Foi adicionado teste Rust dedicado para garantir que um executavel em `_staging` nao seja promovido a `local`.

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
5. Concluir a importacao de title history do Xbox usando o fluxo Xbox Live ja autenticado.
6. Refinar a UX do bloqueio de importacao Xbox, explicando melhor ao usuario porque a operacao ainda pode falhar quando a autenticacao nao estiver concluida.
7. O proximo ajuste do projeto e reduzir o tempo de abertura do aplicativo, porque o Tauri esta demorando demais para mostrar a interface completa.

## Corte Xbox Public Client

- O login Xbox Live passou a usar `authorization code + PKCE` como fluxo de `public client` desktop.
- O fluxo deixou de exigir `client secret` do usuario final; o `client_id` passou a ser configuracao interna ou de build da instancia do projeto.
- O refresh token continua salvo no `AuthVault`, sem expor segredo ao renderer nem ao fluxo de contas.
- Para a build final, o `client_id` nao deve permanecer editavel na interface. O valor deve vir de configuracao interna, variavel de ambiente ou build, e a tela deve apenas reportar estado/erro se ele nao estiver definido.
