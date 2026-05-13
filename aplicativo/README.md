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
- O frontend tem acoes separadas para sincronizar Steam e jogos locais. A sincronizacao Steam atual cobre instalacoes locais e cria acoes `steam://rungameid/<appid>`; a integracao Web API para biblioteca completa/playtime/metadados fica para o proximo corte.
- O comando `launch_library_entry` abre executaveis locais para jogos manuais e locais persistidos, validando caminho absoluto, arquivo existente, extensao `.exe` e sem usar shell.
- O banco local e criado em `%APPDATA%\\com.bibliotecajogos.unificada\\library.sqlite3`.
- No Tauri, o frontend carrega a biblioteca pelo comando `list_library_entries`; no navegador comum, usa os mocks como fallback de desenvolvimento.

## Sincronizacao Steam local

No Tauri, o botao de sincronizar Steam procura instalacoes padrao em `%PROGRAMFILES(X86)%\Steam`, `%PROGRAMFILES%\Steam` e `%LOCALAPPDATA%\Steam`. Para testar com uma pasta controlada:

```powershell
$env:BIBLIOTECA_JOGOS_STEAM_ROOTS = "C:\Temp\SteamTeste"
npm run tauri:dev
```

A pasta deve conter `steamapps\appmanifest_<appid>.acf`. Se houver `steamapps\libraryfolders.vdf`, bibliotecas extras tambem sao lidas.

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
- Evoluir `SteamProvider` com Web API/configuracao de conta para biblioteca completa, playtime e metadados.
- Melhorias no `LocalGamesProvider` inicial para configurar raizes pela UI.
- Consulta filtrada/paginacao no backend para bibliotecas maiores.

