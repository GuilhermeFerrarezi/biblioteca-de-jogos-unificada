# Retomada em Novo Computador

Este guia serve para continuar o projeto Biblioteca de Jogos Unificada em outra maquina sem depender de memoria da sessao anterior.

## O que precisa estar disponivel antes de comecar

- Windows 10 ou 11.
- Git instalado.
- Node.js LTS com `npm`.
- Rust instalado via `rustup`.
- Visual Studio Build Tools 2022 com workload C++.
- MSVC toolchain.
- Windows SDK.
- Microsoft Edge WebView2 Runtime.
- Espaco livre suficiente em disco para compilar Rust e empacotar o Tauri.
- Se o projeto ficar dentro do OneDrive, a pasta precisa estar disponivel offline.

Se for abrir o projeto em outra maquina a partir do Git, clone o repositorio e preserve a estrutura raiz com `CHECKPOINT.md`, `DIRETRIZES_DESENVOLVIMENTO.md`, `ESTRUTURA_BANCO_DADOS.md`, `RETOMADA_NOVO_COMPUTADOR.md`, `ULTIMOS_DESENVOLVIMENTOS_APLICATIVO.md`, `aplicativo/` e `cloud - biblioteca de jogos/`.

Se o checkout vier de uma copia manual, garanta que a pasta `.git` acompanhe o projeto. Sem isso, o historico e os comandos Git nao vao existir.

## Leitura inicial

1. Leia `CHECKPOINT.md` na raiz do projeto.
2. Leia `DIRETRIZES_DESENVOLVIMENTO.md`.
3. Leia `ESTRUTURA_BANCO_DADOS.md`.
4. Leia `cloud - biblioteca de jogos/README_PROJETO.md` se for trabalhar com planejamento, providers, seguranca, UX, metadados, banco ou QA.
5. Leia `aplicativo/README.md`.
6. Leia este arquivo ate o fim antes de instalar ou alterar qualquer coisa.
7. Confira o commit mais recente para entender exatamente o ultimo corte salvo.

Referencia atual de retorno:

```powershell
git rev-parse --short HEAD
```

No estado atual deste guia, o ultimo corte salvo e `78b9374` (`docs: checkpoint current xbox and merge state`).

## Estado atual do aplicativo

- O app fica em `aplicativo`.
- Stack: Tauri 2, React 18, JavaScript/JSX, Vite, ESLint e lucide-react.
- A UI da biblioteca ja existe e roda com listagem unificada vinda do backend Tauri; no navegador comum, usa mocks como fallback.
- O modo escuro e a experiencia padrao.
- A visualizacao inicial da biblioteca e por capas, com opcao de lista.
- Busca, filtros rapidos, selecao de jogo e painel de detalhes ja funcionam.
- Cadastro manual de jogo ja existe. No Tauri, novos jogos manuais sao salvos em SQLite; no navegador comum, o app usa fallback em memoria.
- Edicao de jogos manuais tambem ja existe, reutilizando o mesmo modal de cadastro.
- O botao `Jogar` abre a URI quando a acao e do tipo `uri`, como `steam://rungameid/...`.
- Executaveis locais de jogos manuais e locais persistidos ja sao iniciados por comando Tauri seguro, sem shell, com validacao de caminho absoluto local, arquivo existente e extensao `.exe`.
- O backend Tauri ja possui persistencia SQLite, migration e compatibilidade de schema no boot, seed idempotente dos 4 mocks em background, comando `list_library_entries` para a listagem unificada, `set_library_entry_archived` para arquivamento, `update_manual_game` para edicao de jogos manuais e `launch_library_entry` para lancamento local seguro.
- O `LocalGamesProvider` inicial ja existe como comando `sync_local_games`, com importacao incremental e acionamento manual pela interface.
- A sincronizacao local nao roda mais no boot para preservar tempo de abertura.
- O scanner local evita bibliotecas Steam por padrao, ignora instaladores/runtimes/servicos como EpicOnlineServices, encontra executaveis em subpastas comuns como `Binaries\Win64` e arquiva falsos positivos locais antigos no boot ou na sincronizacao.
- O scanner local tambem rejeita componentes do launcher Battle.net/Blizzard, como `Battle.net.exe`, `Agent`, `BlizzardBrowser`, `BlizzardError` e `BlizzardUpdateAgent`, arquivando falsos positivos antigos desse tipo sem bloquear subpastas de jogos legitimos na raiz Battle.net.
- O primeiro corte do `SteamProvider` local ja existe como comando `sync_steam_games`, lendo `libraryfolders.vdf` e `appmanifest_*.acf` para importar jogos Steam instalados sem credenciais.
- O provider Xbox local experimental ja existe no Windows. Ele usa o inventario local para trazer jogos instalados, abre o jogo via `explorer.exe` + `shell:AppsFolder` quando instalado e abre a Microsoft Store quando nao esta instalado.
- A heuristica do Xbox foi endurecida para excluir apps comuns do Windows/Store como Skype, Filmes e TV, Noticias, IntelliGo Neptune, Calculadora e outros utilitarios, enquanto jogos desktop populares como `osu!` entram como `local`, nao como `xbox`.
- Ultima revisao confirmada em 2026-05-18: `npm run lint`, `npm run build` e `cargo test` passaram. A suite Rust esta com 76 testes.
- A area de contas ja possui Steam OpenID, SteamID64 persistido no SQLite, Steam Web API key salva pelo AuthVault e sincronizacao por conta via Web API. A credencial usa keyring/cofre do sistema operacional como primario e fallback DPAPI cifrado em `%APPDATA%\com.bibliotecajogos.unificada\auth-vault\steam-web-api-key.dpapi` quando o Credential Manager nao valida leitura apos gravacao.
- A tela de contas da Steam usa disclosure inline: a linha principal mostra resumo e seta, e o painel expandido concentra login, sincronizacao local, sincronizacao por conta, SteamID64 e Steam Web API key. O foco volta para a seta ao fechar.
- O proximo corte da Steam Web API ja foi consolidado no backend com `ProviderErrorDto`, parse tolerante de payload e persistencia de metadados de sync nao sensiveis em `provider_account_configs.config_json`.
- A UX de erro da Steam mostra mensagem curta por padrao e detalhes tecnicos expansiveis quando a operacao falha.

## Estrutura importante

```text
CHECKPOINT.md
DIRETRIZES_DESENVOLVIMENTO.md
ESTRUTURA_BANCO_DADOS.md
RETOMADA_NOVO_COMPUTADOR.md
aplicativo/
  package.json
  src/
    App.jsx
    adapters/libraryEntryAdapter.js
    components/
    data/mockLibrary.js
    hooks/useLibraryPageState.js
    hooks/useLibraryFiltering.js
    pages/LibraryPage.jsx
    services/libraryService.js
    constants/libraryConstants.js
    styles/library.css
  src-tauri/
    Cargo.toml
    src/lib.rs
    target/              # gerado pelo Rust, nao editar
cloud - biblioteca de jogos/
  README_PROJETO.md
  agents/
  skills/
```

Os agentes e skills em `cloud - biblioteca de jogos` sao parte da governanca atual do projeto. Use-os como checklist antes de alterar integracoes de plataforma, contratos de provider, seguranca Tauri/Auth, metadados, UX de biblioteca, performance ou schema SQLite.

Antes de qualquer desenvolvimento, identifique explicitamente:

- agente principal em `cloud - biblioteca de jogos/agents`;
- agentes auxiliares quando o corte tocar mais de uma area;
- skills obrigatorias em `cloud - biblioteca de jogos/skills`;
- criterios de validacao;
- quais subagentes, se houver, receberao delegacao seguindo esses agentes/skills.

Subagentes tecnicos podem ser usados para paralelizar pesquisa, implementacao ou revisao, mas devem ser instruidos a atuar conforme os agentes e skills locais. Nao usar um subagente generico como substituto da governanca definida nesta pasta.

## Requisitos do ambiente

Instale ou confirme:

- Git.
- Node.js LTS.
- npm.
- Rust via rustup.
- Visual Studio Build Tools 2022 com workload C++.
- MSVC toolchain.
- Windows SDK.
- Microsoft Edge WebView2 Runtime.

Comandos de verificacao:

```powershell
git --version
node --version
npm --version
rustc --version
cargo --version
```

Se `rustc` ou `cargo` nao forem encontrados apos instalar o rustup, feche e abra o terminal. O caminho esperado no Windows e normalmente:

```text
%USERPROFILE%\.cargo\bin
```

## Preparacao do projeto

Se for abrir o projeto a partir de outro checkout, entre na raiz e confirme o estado Git:

```powershell
git status --short
git log --oneline -n 3
```

Depois entre no frontend e instale as dependencias:

```powershell
cd .\aplicativo
npm install
npm run onedrive:keep-local
```

Valide o frontend:

```powershell
npm run lint
npm run build
```

Rode em modo web, se quiser testar so a UI:

```powershell
npm run dev
```

Rode como app desktop Tauri:

```powershell
npm run tauri:dev
```

Se quiser validar o backend isoladamente antes de subir a interface:

```powershell
cd .\src-tauri
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\BibliotecaJogosUnificada\cargo-target"
cargo test
```

## Configuracao contra erro do OneDrive no Cargo

O projeto pode ficar dentro do OneDrive, mas os artefatos gerados pelo Rust/Cargo nao devem ficar dentro do OneDrive. Para evitar travamentos e erros como `A operacao de nuvem solicitada foi malsucedida`, os scripts do `package.json` ja configuram:

```text
CARGO_TARGET_DIR=%LOCALAPPDATA%\BibliotecaJogosUnificada\cargo-target
```

Os scripts `tauri:dev` e `tauri:build` tambem preenchem o `PATH` com `%USERPROFILE%\.cargo\bin`, para o Tauri encontrar `cargo` ao executar `cargo metadata`.

Isso faz `npm run tauri:dev` e `npm run tauri:build` compilarem fora de `aplicativo/src-tauri/target`.

Em um computador novo, confirme se `aplicativo/package.json` contem:

```json
"tauri:dev": "set PATH=%USERPROFILE%\\.cargo\\bin;%PATH%&& set CARGO_TARGET_DIR=%LOCALAPPDATA%\\BibliotecaJogosUnificada\\cargo-target&& tauri dev",
"tauri:build": "set PATH=%USERPROFILE%\\.cargo\\bin;%PATH%&& set CARGO_TARGET_DIR=%LOCALAPPDATA%\\BibliotecaJogosUnificada\\cargo-target&& tauri build"
```

Opcionalmente, crie a pasta local antes do primeiro build:

```powershell
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\BibliotecaJogosUnificada\cargo-target"
```

Se precisar testar o Cargo isoladamente:

```powershell
cd .\aplicativo\src-tauri
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\BibliotecaJogosUnificada\cargo-target"
cargo check
cargo test
```

Depois volte para `aplicativo` e rode:

```powershell
cd ..
npm run tauri:dev
```

## Problemas conhecidos

- Se o projeto estiver dentro do OneDrive, arquivos podem ficar apenas na nuvem e bloquear o build Tauri. Marque a pasta do projeto como disponivel offline.
- Se o app abrir com tela branca ou `localhost:5173` ficar carregando, rode dentro de `aplicativo`:
  ```powershell
  npm run onedrive:keep-local
  ```
  Depois feche processos antigos de `npm`, `node` ou `cargo` ligados ao projeto e rode `npm run tauri:dev` novamente.
- Se o Tauri parecer travado em `npm run tauri:dev`, confirme que nao ha processos `cargo.exe` antigos:
  ```powershell
  Get-Process cargo -ErrorAction SilentlyContinue
  ```
  Se houver processo antigo travado do proprio projeto, feche o terminal que iniciou o comando ou encerre o processo antes de tentar novamente.
- A primeira compilacao usando o target local pode demorar mais porque recompila dependencias. Depois disso, `npm run tauri:dev` deve abrir mais rapido.
- Se o build nativo falhar por MSVC ou Windows SDK, revise a instalacao do Visual Studio Build Tools.
- `src-tauri/target` e gerado pelo Rust e nao deve ser tratado como codigo fonte.
- No navegador comum (`npm run dev`), os dados adicionados pelo modal somem ao recarregar porque o fallback e em memoria. No Tauri (`npm run tauri:dev`), novos jogos manuais devem persistir em SQLite.
- Para testar lancamento local com seguranca, cadastre um jogo manual no Tauri com acao como `C:\Windows\System32\notepad.exe`. O backend aceita apenas caminho absoluto local, arquivo existente e extensao `.exe`.
- Para testar a sincronizacao local sem varrer pastas reais do computador, defina uma raiz controlada antes de abrir o app:
  ```powershell
  $env:BIBLIOTECA_JOGOS_LOCAL_ROOTS = "C:\Temp\BibliotecaJogosTeste"
  npm run tauri:dev
  ```
  Dentro dessa raiz, crie subpastas com um `.exe` de teste para validar o comando `sync_local_games`.
- Para testar a sincronizacao Steam sem depender da instalacao real, defina uma raiz controlada antes de abrir o app:
  ```powershell
  $env:BIBLIOTECA_JOGOS_STEAM_ROOTS = "C:\Temp\SteamTeste"
  npm run tauri:dev
  ```
  Dentro dessa raiz, crie `steamapps\appmanifest_<appid>.acf`. O comando tambem le bibliotecas extras declaradas em `steamapps\libraryfolders.vdf`.
- Para testar a descoberta Xbox local, abra `Contas` e acione `Sincronizar local`. Se algum app comum do Windows/Store entrar como jogo, a heuristica deve ser ajustada antes de seguir adiante.

## Ponto exato para continuar

A persistencia local inicial, a listagem unificada pelo backend, o arquivamento, a edicao de jogos manuais, o bootstrap assincrono da biblioteca, o lancamento local seguro, o `LocalGamesProvider` inicial, o `SteamProvider` local, Steam OpenID, AuthVault para Web API, a selecao multipla de filtros por categoria na biblioteca principal e o merge visual Steam + Xbox ja foram implementados. A sincronizacao local e Steam continuam manuais.

O proximo corte imediato deve continuar refinando o Xbox/Game Pass local para reduzir falsos positivos e manter jogos de desktop como `osu!` em `local`, sem reintroduzir apps comuns da Microsoft Store como `xbox`.

As proximas mudancas devem seguir `DIRETRIZES_DESENVOLVIMENTO.md`: frontend em JavaScript/JSX, backend Tauri em Rust, SQLite como persistencia principal, padrao Service-Adapter para providers e normalizacao para `LibraryEntry`. A reorganizacao inicial do frontend ja foi feita com `components`, `services`, `adapters`, `hooks`, `pages`, `constants` e `styles`.

Ordem sugerida:

1. Testar manualmente no Tauri: conferir se a janela abre rapido e a biblioteca carrega os 4 mocks apos o bootstrap.
2. Validar o fluxo completo de adicionar, editar e arquivar jogos manuais.
3. Manter `mockLibrary.js` apenas como fallback web e referencia de seed.
4. Fechar e reabrir o app para confirmar persistencia SQLite dos jogos manuais e do arquivamento.
5. Validar o fluxo manual de sincronizacao local no Tauri, preferencialmente com `BIBLIOTECA_JOGOS_LOCAL_ROOTS` apontando para uma pasta de teste.
6. Validar o fluxo manual de sincronizacao Steam no Tauri, preferencialmente com `BIBLIOTECA_JOGOS_STEAM_ROOTS` apontando para uma pasta de teste.
7. Continuar o refinamento do provider Xbox local para filtrar melhor apps comuns e manter jogos de desktop em `local`.
8. Melhorar a visualizacao da tela de configuracoes/contas apenas se aparecer nova necessidade de produto.
9. Evoluir a Steam com Web API para metadados/playtime adicionais.
10. Depois, adicionar consulta filtrada/paginacao no backend para suportar bibliotecas maiores.

## Criterios minimos antes de seguir para providers

- `npm run lint` passa.
- `npm run build` passa.
- `cargo test` passa em `aplicativo/src-tauri` com `CARGO_TARGET_DIR` local.
- `npm run tauri:dev` abre o app desktop.
- Cadastro manual persiste apos fechar e abrir o app.
- A UI continua funcionando com busca, filtros, selecao e detalhes.
- O provider Xbox local nao importa apps comuns do Windows/Store como jogos.
- Jogos de desktop como `osu!` aparecem como `local`, nao como `xbox`.

## Documentacao a manter atualizada

Sempre que houver marco importante, atualize:

- `CHECKPOINT.md`: estado geral, decisoes e proxima sessao.
- `DIRETRIZES_DESENVOLVIMENTO.md`: regras arquiteturais e criterios de aceite quando uma decisao de desenvolvimento mudar.
- `ESTRUTURA_BANCO_DADOS.md`: schema SQLite, indices, relacoes ou regras de migracao quando o backend persistente mudar.
- `aplicativo/README.md`: comandos, stack e estado especifico do app.
- `RETOMADA_NOVO_COMPUTADOR.md`: requisitos ou passos de ambiente que mudarem.
- `cloud - biblioteca de jogos/README_PROJETO.md`, `cloud - biblioteca de jogos/agents` e `cloud - biblioteca de jogos/skills`: responsabilidades, checklists e criterios operacionais quando a forma de trabalho mudar.
