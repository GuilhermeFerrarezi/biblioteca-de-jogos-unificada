# Biblioteca de Jogos Unificada - Aplicativo

Aplicativo desktop para centralizar biblioteca de jogos, contas, instalacoes, metadados e acoes de lancamento. Esta pasta contem a base executavel do projeto.

## Stack

- Tauri 2
- React 18
- TypeScript
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

No Windows, os scripts `tauri:dev` e `tauri:build` usam `CARGO_TARGET_DIR` em `%LOCALAPPDATA%\\BibliotecaJogosUnificada\\cargo-target`. Isso evita erros do Rust/Cargo com artefatos gerados dentro do OneDrive.

Se o app abrir com tela branca ou o navegador ficar carregando em `localhost:5173`, rode:

```powershell
npm run onedrive:keep-local
```

Esse comando marca todos os arquivos de `aplicativo` como disponiveis localmente no OneDrive.

## Estado atual

- Frontend React funcional com shell de biblioteca em modo escuro.
- Visualizacao padrao por capas, com alternativa em lista.
- Sidebar, resumo da biblioteca, busca, filtros rapidos e painel de detalhes.
- Selecao real de jogo por clique ou teclado.
- Cadastro manual de jogos implementado em memoria.
- Acoes de lancamento com feedback visual: URIs como `steam://` sao abertas pelo navegador/sistema; executaveis locais de jogos manuais persistidos sao iniciados por comando Tauri seguro.
- Modelos centrais separados em `src/domain`.
- Dados mockados concentrados em `src/data/mockLibrary.ts`.
- Backend Tauri em `src-tauri` ja possui persistencia SQLite inicial para jogos manuais, com comandos `list_manual_games` e `add_manual_game`.
- O comando `launch_library_entry` abre executaveis locais apenas para jogos manuais persistidos, validando caminho absoluto, arquivo existente, extensao `.exe` e sem usar shell.
- O banco local e criado em `%APPDATA%\\com.bibliotecajogos.unificada\\library.sqlite3`.
- Dados mockados ainda seguem como base/fallback de desenvolvimento; a listagem unificada completa pelo backend fica para o proximo corte.

## Ambiente nativo

Na maquina atual, Rust/Cargo e Visual Studio Build Tools com MSVC/Windows SDK ja foram instalados anteriormente e `npm run tauri:dev` chegou a compilar e executar o app nativo.

Em um novo computador, siga o guia da raiz do projeto:

```text
../RETOMADA_NOVO_COMPUTADOR.md
```

## Proximas implementacoes

- Persistencia local para cadastro manual e biblioteca unificada.
- Seed dos mocks no SQLite ou listagem unificada completa pelo backend.
- Edicao e arquivamento de jogos manuais.
- Suporte futuro a execucao de executaveis de jogos importados por providers locais.
- `SteamProvider` como primeira integracao real.
- `LocalGamesProvider` para importar jogos instalados/localizados no Windows.
