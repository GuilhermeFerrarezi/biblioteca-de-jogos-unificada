# Biblioteca de Jogos Unificada - Aplicativo

Aplicativo desktop para centralizar biblioteca de jogos, contas, instalacoes, metadados e acoes de lancamento. Esta pasta contem a base executavel do projeto.

## Stack

- Tauri 2
- React 18
- JavaScript / JSX
- Vite
- ESLint
- lucide-react

## Comandos

```powershell
npm install
npm run dev
npm run build
npm run lint
npm run onedrive:keep-local
npm run tauri:dev
```

No Windows, os scripts `tauri:dev` e `tauri:build` usam `CARGO_TARGET_DIR` em `%LOCALAPPDATA%\\BibliotecaJogosUnificada\\cargo-target` e preenchem o `PATH` com `%USERPROFILE%\\.cargo\\bin`. Isso evita erros do Rust/Cargo com artefatos gerados dentro do OneDrive e garante que o `cargo` seja encontrado quando o Tauri chama `cargo metadata`.

Se o app abrir com tela branca ou o navegador ficar carregando em `localhost:5173`, rode:

```powershell
npm run onedrive:keep-local
```

Esse comando marca todos os arquivos de `aplicativo` como disponiveis localmente no OneDrive.

## Estado atual

- Frontend React funcional com shell de biblioteca em modo escuro.
- Frontend reorganizado em `pages`, `components`, `hooks`, `adapters`, `services`, `constants`, `data`, `assets` e `styles`.
- A ponte com Tauri fica em `src/services/libraryService.js`, com fallback web carregado dinamicamente apenas em desenvolvimento.
- Visualizacao padrao por capas, com alternativa em lista.
- Sidebar, resumo da biblioteca, busca, filtros rapidos e painel de detalhes.
- Area de `Contas e integracoes` acessivel pela sidebar, com Steam, Xbox/Game Pass e Epic preparados para evolucao.
- Fluxo `Entrar com Steam` implementado com Steam OpenID no navegador externo; o backend valida o retorno e salva apenas o SteamID64 no SQLite.
- Selecao real de jogo por clique ou teclado.
- Cadastro manual de jogos implementado com persistencia SQLite no Tauri e fallback em memoria no navegador.
- Edicao de jogos manuais reutiliza o mesmo modal de cadastro e persiste via comando Tauri.
- O seed dos 4 mocks roda em background apos a abertura da janela; o frontend escuta o evento de bootstrap e refaz a listagem quando ele termina. A sincronizacao de jogos locais ficou manual para nao alongar o boot.
- Acoes de lancamento com feedback visual: URIs como `steam://` sao abertas pelo navegador/sistema; executaveis locais de jogos manuais e locais persistidos sao iniciados por comando Tauri seguro.
- Frontend migrado para JavaScript/JSX; os modelos agora sao contratos de dados mantidos pelos objetos e comandos Tauri.
- Dados mockados concentrados em `src/data/mockLibrary.js` apenas como fallback web e referencia de seed.
- Constantes compartilhadas de UI/dominio ficam em `src/constants/libraryConstants.js`.
- Backend Tauri em `src-tauri` ja possui persistencia SQLite para a biblioteca, com seed idempotente dos 4 mocks e comandos `list_library_entries`, `add_manual_game`, `update_manual_game`, `set_library_entry_archived`, `sync_local_games` e `launch_library_entry`.
- A biblioteca principal exclui entradas arquivadas via `is_archived`; o backend tambem expoe o comando `set_library_entry_archived`. O frontend tem um botao de sincronizacao manual para importar jogos locais a partir de pastas conhecidas ou configuradas via ambiente. O scanner local ignora instaladores, componentes de runtime, servicos como EpicOnlineServices e diretorios de suporte, arquiva falsos positivos antigos desse tipo ao abrir o banco ou sincronizar, e encontra executaveis em subpastas comuns como `Binaries\Win64`.
- A limpeza de falsos positivos locais no boot usa indices especificos em `library_entries` e `launch_actions`, e e ignorada rapidamente quando nao ha entradas locais ativas.
- O `LocalGamesProvider` nao varre bibliotecas Steam por padrao. A importacao Steam fica no comando `sync_steam_games`, que le `libraryfolders.vdf` e `appmanifest_*.acf` para importar jogos instalados sem exigir credenciais.
- O frontend tem acoes separadas para sincronizar Steam local, conectar conta Steam e sincronizar biblioteca da conta. A sincronizacao Steam local cobre instalacoes locais e cria acoes `steam://rungameid/<appid>`.
- A integracao Web API por conta usa SteamID64 salvo no SQLite e Steam Web API key legivel no AuthVault/keyring do sistema operacional. Marcadores SQLite nao liberam sincronizacao sem o segredo no cofre. Senha, Steam Guard, cookies e sessoes de navegador nao sao capturados nem persistidos.
- O comando `launch_library_entry` abre executaveis locais para jogos manuais e locais persistidos, validando caminho absoluto, arquivo existente, extensao `.exe` e sem usar shell.
- O scanner local rejeita componentes auxiliares de launchers e runtimes, incluindo Battle.net/Blizzard (`Battle.net.exe`, `Agent`, `BlizzardBrowser`, `BlizzardError`, `BlizzardUpdateAgent`), e arquiva falsos positivos antigos quando a limpeza local roda.
- O banco local e criado em `%APPDATA%\\com.bibliotecajogos.unificada\\library.sqlite3`.
- No Tauri, o frontend carrega a biblioteca pelo comando `list_library_entries`; no navegador comum, usa os mocks como fallback de desenvolvimento.

## Sincronizacao Steam local

No Tauri, o botao de sincronizar Steam procura instalacoes padrao em `%PROGRAMFILES(X86)%\Steam`, `%PROGRAMFILES%\Steam` e `%LOCALAPPDATA%\Steam`. Para testar com uma pasta controlada:

```powershell
$env:BIBLIOTECA_JOGOS_STEAM_ROOTS = "C:\Temp\SteamTeste"
npm run tauri:dev
```

A pasta deve conter `steamapps\appmanifest_<appid>.acf`. Se houver `steamapps\libraryfolders.vdf`, bibliotecas extras tambem sao lidas.

## Conta Steam e Web API

O botao `Entrar com Steam` abre o login oficial no navegador externo e recebe o retorno em um callback local temporario (`127.0.0.1`). Depois de validar a resposta com a Steam, o app salva apenas o SteamID64 no banco local.

Para sincronizar a biblioteca da conta, ainda e necessaria uma Steam Web API key valida salva no AuthVault. O OpenID confirma a identidade, mas nao fornece token OAuth nem acesso automatico a bibliotecas privadas.

## Ambiente nativo

Na maquina atual, Rust/Cargo e Visual Studio Build Tools com MSVC/Windows SDK ja foram instalados anteriormente e `npm run tauri:dev` chegou a compilar e executar o app nativo.

Em um novo computador, siga o guia da raiz do projeto:

```text
../RETOMADA_NOVO_COMPUTADOR.md
```

As diretrizes de arquitetura e evolucao ficam em:

```text
../DIRETRIZES_DESENVOLVIMENTO.md
```

A estrutura do SQLite local fica em:

```text
../ESTRUTURA_BANCO_DADOS.md
```

## Proximas implementacoes

- Validacao manual do fluxo completo de adicionar, editar, arquivar, sincronizar Steam e sincronizar locais no Tauri.
- Validar manualmente o login Steam OpenID e os casos de conta privada/chave Web API invalida.
- Melhorias no `LocalGamesProvider` inicial para configurar raizes pela UI.
- Consulta filtrada/paginacao no backend para bibliotecas maiores.

