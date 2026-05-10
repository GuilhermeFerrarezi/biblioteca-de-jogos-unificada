# Retomada em Novo Computador

Este guia serve para continuar o projeto Biblioteca de Jogos Unificada em outra maquina sem depender de memoria da sessao anterior.

## Leitura inicial

1. Leia `CHECKPOINT.md` na raiz do projeto.
2. Leia `aplicativo/README.md`.
3. Leia este arquivo ate o fim antes de instalar ou alterar qualquer coisa.

## Estado atual do aplicativo

- O app fica em `aplicativo`.
- Stack: Tauri 2, React 18, TypeScript, Vite, ESLint e lucide-react.
- A UI da biblioteca ja existe e roda com dados mockados.
- O modo escuro e a experiencia padrao.
- A visualizacao inicial da biblioteca e por capas, com opcao de lista.
- Busca, filtros rapidos, selecao de jogo e painel de detalhes ja funcionam.
- Cadastro manual de jogo ja existe. No Tauri, novos jogos manuais sao salvos em SQLite; no navegador comum, o app usa fallback em memoria.
- O botao `Jogar` abre a URI quando a acao e do tipo `uri`, como `steam://rungameid/...`.
- Executaveis locais de jogos manuais persistidos ja sao iniciados por comando Tauri seguro. Jogos mockados/importados ainda dependem de provider local futuro.
- O backend Tauri ja possui persistencia SQLite inicial para jogos manuais, mas ainda nao possui listagem unificada completa pelo backend nem providers reais.

## Estrutura importante

```text
CHECKPOINT.md
RETOMADA_NOVO_COMPUTADOR.md
aplicativo/
  package.json
  src/
    App.tsx
    App.css
    data/mockLibrary.ts
    domain/
  src-tauri/
    Cargo.toml
    src/lib.rs
    target/              # gerado pelo Rust, nao editar
cloude teste/
  README_PROJETO.md
  agents/
  skills/
```

## Requisitos do ambiente

Instale ou confirme:

- Node.js LTS.
- npm.
- Rust via rustup.
- Visual Studio Build Tools 2022 com workload C++.
- MSVC toolchain.
- Windows SDK.
- Microsoft Edge WebView2 Runtime.

Comandos de verificacao:

```powershell
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

Na raiz do projeto:

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

## Configuracao contra erro do OneDrive no Cargo

O projeto pode ficar dentro do OneDrive, mas os artefatos gerados pelo Rust/Cargo nao devem ficar dentro do OneDrive. Para evitar travamentos e erros como `A operacao de nuvem solicitada foi malsucedida`, os scripts do `package.json` ja configuram:

```text
CARGO_TARGET_DIR=%LOCALAPPDATA%\BibliotecaJogosUnificada\cargo-target
```

Isso faz `npm run tauri:dev` e `npm run tauri:build` compilarem fora de `aplicativo/src-tauri/target`.

Em um computador novo, confirme se `aplicativo/package.json` contem:

```json
"tauri:dev": "set CARGO_TARGET_DIR=%LOCALAPPDATA%\\BibliotecaJogosUnificada\\cargo-target&& tauri dev",
"tauri:build": "set CARGO_TARGET_DIR=%LOCALAPPDATA%\\BibliotecaJogosUnificada\\cargo-target&& tauri build"
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

## Ponto exato para continuar

A persistencia local inicial ja foi iniciada. O proximo corte deve consolidar a Fase 2.

Ordem sugerida:

1. Testar manualmente no Tauri: cadastrar jogo manual, fechar o app, reabrir e confirmar que o jogo continua na biblioteca.
2. Decidir se os 4 mocks serao seedados no SQLite agora ou se o backend retornara uma listagem unificada combinando mocks/fallback e persistidos.
3. Implementar edicao e arquivamento de jogos manuais.
4. Manter `mockLibrary.ts` apenas como seed/fallback de desenvolvimento.
5. Depois disso, implementar `LocalGamesProvider`.
6. Em seguida, iniciar `SteamProvider` como primeira integracao real.

## Criterios minimos antes de seguir para providers

- `npm run lint` passa.
- `npm run build` passa.
- `npm run tauri:dev` abre o app desktop.
- Cadastro manual persiste apos fechar e abrir o app.
- A UI continua funcionando com busca, filtros, selecao e detalhes.

## Documentacao a manter atualizada

Sempre que houver marco importante, atualize:

- `CHECKPOINT.md`: estado geral, decisoes e proxima sessao.
- `aplicativo/README.md`: comandos, stack e estado especifico do app.
- `RETOMADA_NOVO_COMPUTADOR.md`: requisitos ou passos de ambiente que mudarem.
