# Diretrizes de Desenvolvimento

Origem: consolidado a partir do documento `Projeto Biblioteca de Jogos Unificada.docx`, elaborado em 13 de maio de 2026, e ajustado ao estado atual do projeto.

## Objetivo

Estas diretrizes orientam a evolucao tecnica da Biblioteca de Jogos Unificada para que o aplicativo continue escalando sem misturar interface, regras de negocio, persistencia e integracoes externas. O frontend deve permanecer em JavaScript/JSX, conforme decisao atual do projeto. O backend Tauri permanece em Rust, pois e a camada nativa responsavel por SQLite, comandos seguros e acesso local ao Windows.

## Principios

- Separar responsabilidades: componentes React cuidam de interface; services coordenam chamadas; adapters normalizam dados externos; o backend cuida de persistencia e operacoes nativas.
- Manter um modelo interno unico de biblioteca, independentemente da origem do jogo.
- Integrar plataformas por camadas, evitando chamadas diretas de API dentro de componentes visuais.
- Preferir evolucao incremental: reorganizar pastas quando houver necessidade real ou quando uma area crescer, sem refatorar tudo de uma vez.
- Preservar verificacoes antes de cada marco: `npm run lint`, `npm run build` e `cargo test` com `CARGO_TARGET_DIR` local.

## Organizacao Recomendada

A estrutura inicial de frontend ja foi aberta em camadas. A medida que novas telas e providers forem adicionados, a organizacao recomendada para `aplicativo/src` e:

```text
src/
  components/   # componentes reutilizaveis: Sidebar, GameCard, modal, botoes, paineis
  pages/        # telas principais: Library, Settings, Accounts
  services/     # comunicacao com Tauri/API e orquestracao de casos de uso
  adapters/     # normalizacao de dados por plataforma: steamAdapter.js, localAdapter.js
  hooks/        # hooks React para estado, efeitos e fluxos de tela
  data/         # mocks/fallbacks de desenvolvimento e seeds de referencia
  assets/       # imagens, icones e fontes
  styles/       # estilos globais, temas e tokens visuais
```

Essa divisao deve continuar sendo aplicada de forma pragmatica quando o arquivo ou responsabilidade passar a dificultar manutencao. O proximo corte natural e implementar o `SteamProvider` dentro desse desenho, usando `services` e `adapters` em vez de acoplar integracoes diretamente na UI.

## Padrao Service-Adapter

Cada fonte de dados deve ter um adapter especifico que converte dados brutos para o contrato interno da biblioteca.

- `SteamProvider` ou adapter Steam: consulta a fonte Steam disponivel, interpreta IDs, status e metadados, e retorna objetos normalizados.
- `LocalGamesProvider`: continua responsavel por descoberta local, mas deve expor dados no mesmo formato interno dos demais providers.
- `LibraryService`: consolida entradas, trata erros, decide fallback e entrega uma lista unificada para a UI.
- `libraryApi.js`: permanece como fronteira do frontend com comandos Tauri e fallback web.

Componentes React nao devem chamar adapters externos diretamente. Eles devem consumir services/hooks que ja entregam estado pronto para renderizacao.

## Modelo Interno

O documento original sugeria um `Game` minimo com `id`, `title`, `platform`, `imageUrl` e `playTime`. O projeto ja possui um contrato mais rico via `LibraryEntry`, com `game`, `primaryPlatformId`, `installStatus`, `isArchived`, `sources`, `launchActions`, `playtime`, `artwork`, `genres` e datas.

Diretriz atual:

- Manter `LibraryEntry` como contrato principal de tela.
- Preservar IDs estaveis para jogo, entrada, fonte e acao de lancamento.
- Toda integracao nova deve normalizar dados para esse contrato antes de chegar na UI.
- Campos opcionais vindos de plataformas externas devem ter fallback seguro, sem quebrar filtros, busca ou painel de detalhes.

## Boas Praticas de Codigo

- Componentes de UI nao devem conter filtragens complexas, regras de sincronizacao, persistencia ou chamadas nativas diretas.
- Funcoes devem usar nomes com verbos claros, como `fetchGames`, `syncLocalGames`, `normalizeSteamGame`.
- Dados e colecoes devem usar nomes substantivos, como `gameList`, `libraryEntries`, `selectedEntry`.
- Priorizar imutabilidade no frontend com `map`, `filter`, `reduce` e criacao de novos objetos.
- Concentrar tratamento de erro em services e comandos backend, retornando mensagens compreensiveis para a UI.
- Evitar duplicacao de regras entre frontend e backend; validacoes de seguranca ficam no backend.

## Persistencia e Cache

A persistencia principal do app desktop e SQLite em `%APPDATA%\\com.bibliotecajogos.unificada\\library.sqlite3`. LocalStorage ou IndexedDB podem ser usados apenas como cache auxiliar do modo web/desenvolvimento, nunca como fonte principal de verdade no Tauri.

Regras:

- O banco local deve continuar fora do codigo-fonte.
- Migracoes e compatibilidade de schema devem ser testadas.
- Seeds devem ser idempotentes.
- Cache deve melhorar tempo de abertura, mas nao pode esconder erro de sincronizacao real.
- A estrutura atual do SQLite deve ser mantida em `ESTRUTURA_BANCO_DADOS.md`.

## Credenciais e Contas

Para futuras integracoes com Steam, Xbox, Epic ou outras plataformas:

- Credenciais, tokens e chaves de API nao devem ficar hardcoded.
- A UI de contas/configuracoes deve separar claramente conexao, revogacao e estado de sincronizacao.
- Tokens persistidos devem usar armazenamento local seguro quando disponivel no Tauri.
- Endpoints nao documentados ou automacoes devem passar por revisao de risco antes de entrar no fluxo principal.

## Roadmap Orientado por Diretrizes

1. Integracao Steam: criar adapter/provider, normalizar dados para `LibraryEntry` e manter fallback seguro.
2. Unificacao e escala: adicionar filtros/paginacao no backend para bibliotecas maiores.
3. Funcionalidades: evoluir busca, filtros, configuracao de raizes locais e area de contas.
4. Refinamento: melhorar UX, tratamento global de erros, performance e telemetria local nao sensivel.

## Criterios de Aceite Para Novas Features

- A feature preserva o contrato unificado de biblioteca.
- A UI continua funcionando com dados vazios, carregando, erro e sucesso.
- O modo navegador mantem fallback de desenvolvimento quando aplicavel.
- `npm run lint` e `npm run build` passam.
- Mudancas no backend passam em `cargo test`.
- Documentacao relevante e atualizada no mesmo marco.
