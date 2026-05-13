# Diretrizes de Desenvolvimento

Origem: consolidado a partir do documento `Projeto Biblioteca de Jogos Unificada.docx`, elaborado em 13 de maio de 2026, e ajustado ao estado atual do projeto.

## Objetivo

Estas diretrizes orientam a evolucao tecnica da Biblioteca de Jogos Unificada para que o aplicativo continue escalando sem misturar interface, regras de negocio, persistencia e integracoes externas. O frontend deve permanecer em JavaScript/JSX, conforme decisao atual do projeto. O backend Tauri permanece em Rust, pois e a camada nativa responsavel por SQLite, comandos seguros e acesso local ao Windows.

## Principios

- Separar responsabilidades: componentes React cuidam de interface; services coordenam chamadas; adapters normalizam dados externos; o backend cuida de persistencia e operacoes nativas.
- Manter um modelo interno unico de biblioteca, independentemente da origem do jogo.
- Integrar plataformas por camadas, evitando chamadas diretas de API dentro de componentes visuais.
- Preferir evolucao incremental, mas manter a organizacao atual de `components`, `pages`, `services`, `adapters`, `hooks`, `constants` e `styles` como base para novos desenvolvimentos.
- Preservar verificacoes antes de cada marco: `npm run lint`, `npm run build` e `cargo test` com `CARGO_TARGET_DIR` local.
- Antes de iniciar integracoes, migrations, seguranca, UX complexa ou metadados, consultar os agentes e skills em `cloude teste`.
- Em todo desenvolvimento, identificar explicitamente o agente local e as skills aplicaveis antes de implementar, revisar ou delegar trabalho.

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
- `libraryService.js`: permanece como fronteira do frontend com comandos Tauri e fallback web. Mocks devem ser carregados somente em `import.meta.env.DEV`.

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
- Constantes compartilhadas, como status de instalacao, tipos de acao e labels de plataforma, devem ficar centralizadas em `src/constants`.
- Objetos-base reutilizaveis devem ser imutaveis quando possivel, por exemplo com `Object.freeze`.
- Formularios devem validar campos obrigatorios e formatos esperados antes de chamar o backend.
- Priorizar imutabilidade no frontend com `map`, `filter`, `reduce` e criacao de novos objetos.
- Concentrar tratamento de erro em services e comandos backend, retornando mensagens compreensiveis para a UI.
- Evitar duplicacao de regras entre frontend e backend; validacoes de seguranca ficam no backend.
- Usar Conventional Commits nos commits novos, com verbos e escopo claros.
- Manter um `CHANGELOG.md` quando o projeto passar a ter versoes publicas.
- Considerar Prettier e validacao de schema com Zod quando a superficie de adapters/providers crescer.
- Usar JSDoc em funcoes complexas quando o contrato nao for evidente pelo nome e parametros.

## Frontend e Acessibilidade

- Componentes grandes devem ser quebrados em subcomponentes menores quando ultrapassarem responsabilidades claras.
- Hooks complexos devem ser divididos por responsabilidade, como filtragem, CRUD manual e sincronizacao.
- Controles selecionaveis devem usar `aria-pressed` quando representarem estado ativo.
- Processos em andamento devem expor estado acessivel, como `aria-busy`.
- Modais devem aceitar fechamento por `Escape` e sinalizar campos invalidos com `aria-invalid`.
- Telas de pagina devem ser protegidas por `ErrorBoundary` para evitar queda total da interface em falhas de renderizacao.
- Estilos compartilhados devem usar CSS Custom Properties para cores, bordas e raios principais.

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
- Toda conexao deve definir ciclo de vida de token: criacao, armazenamento seguro, renovacao, expiracao, revogacao e exclusao ao desconectar.
- Tokens e sessoes devem ficar separados dos dados de biblioteca e nunca aparecer em logs, erros ou payloads enviados ao frontend sem necessidade.

## Governanca de Agentes e Skills

Os arquivos em `cloude teste/agents` e `cloude teste/skills` fazem parte das diretrizes do projeto. Eles devem ser usados como checklist operacional, especialmente em tarefas com risco maior.

Regras:

- Nenhuma tarefa de desenvolvimento relevante deve comecar sem declarar qual agente de `cloude teste/agents` conduz o corte e quais skills de `cloude teste/skills` serao usadas.
- Se houver delegacao para subagente, a instrucao deve citar o agente local escolhido e as skills relacionadas, pedindo que o subagente use esses arquivos como checklist. Subagentes genericos, como exploradores, so podem ser usados como mecanismo de execucao/revisao, nao como substitutos dos agentes definidos no projeto.
- Quando a tarefa envolver mais de uma area, usar um agente principal e agentes auxiliares. Exemplo: provider Steam usa `04-backend-provider-agent.md` como principal, com apoio de `03-security-auth-agent.md`, `11-senior-database-agent.md`, `09-senior-frontend-development-agent.md` e `10-senior-integration-qa-agent.md` conforme o escopo.
- Antes de editar codigo, registrar na conversa ou no plano de trabalho: agente principal, agentes auxiliares, skills aplicadas, arquivos provaveis e criterios de validacao.
- Ao finalizar, atualizar a documentacao relevante quando uma decisao de agente, skill, arquitetura, banco, seguranca ou QA mudar.
- Pesquisa de plataforma deve produzir matriz de viabilidade, compliance, limites, autenticacao, dados disponiveis e alternativa tecnica.
- Nova integracao deve seguir contrato de provider, erro padronizado, estrategia de fallback local/cache e modelo de `LaunchAction`.
- Mudancas em Tauri, tokens, IPC ou lancamento local devem passar pelas skills de hardening de seguranca.
- Mudancas em UI devem seguir design system, acessibilidade basica, estados obrigatorios e criterios de performance para bibliotecas grandes.
- Mudancas em metadados devem declarar hierarquia de fontes, regras de conflito, heuristicas de deduplicacao e campos preservados por plataforma.
- Mudancas em SQLite devem ter versionamento, migration testavel, indices coerentes e preservacao de dados do usuario.

## Fluxo Obrigatorio Antes de Implementar

Para cada novo corte:

1. Ler ou reler o agente mais adequado em `cloude teste/agents`.
2. Listar as skills obrigatorias e transversais em `cloude teste/skills`.
3. Definir o agente principal, agentes auxiliares e criterios de aceite.
4. Delegar desenvolvimento, revisao ou pesquisa apenas para subagentes instruidos com esse agente/skills quando a tarefa puder ser paralelizada com seguranca.
5. Implementar seguindo os checklists desses arquivos.
6. Validar com os comandos adequados e registrar resultados.

Exemplos:

- Provider/backend Steam: `04-backend-provider-agent.md`, `08-senior-backend-development-agent.md`, `platform-viability-matrix`, `platform-integration-research`, `api-compliance-review`, `launcher-provider-development`, `provider-error-standardization`, `senior-backend-implementation`, `sqlite-migrations-repositories`.
- Seguranca/Auth/Tauri: `03-security-auth-agent.md`, `tauri-desktop-security-hardening`, `auth-token-security`, `token-lifecycle-hardening`, `safe-local-executable-launch`.
- Frontend/UX: `05-frontend-ux-agent.md`, `09-senior-frontend-development-agent.md`, `desktop-app-product-design`, `ui-component-standardization`, `react-performance-optimization`, `senior-frontend-implementation`, `usability-heuristics-evaluation`.
- QA/revisao: `07-qa-compliance-agent.md`, `10-senior-integration-qa-agent.md`, `senior-integration-quality`, `api-compliance-review`, `sqlite-schema-versioning`.

## Roadmap Orientado por Diretrizes

1. Integracao Steam: criar adapter/provider, normalizar dados para `LibraryEntry` e manter fallback seguro.
2. Unificacao e escala: adicionar filtros/paginacao no backend para bibliotecas maiores.
3. Funcionalidades: evoluir busca, filtros, configuracao de raizes locais e area de contas.
4. Refinamento: melhorar UX, tratamento global de erros, performance e telemetria local nao sensivel.

## Criterios de Aceite Para Novas Features

- A feature preserva o contrato unificado de biblioteca.
- A UI continua funcionando com dados vazios, carregando, erro e sucesso.
- O modo navegador mantem fallback de desenvolvimento quando aplicavel.
- Mocks de desenvolvimento nao devem ser importados estaticamente por services usados em producao.
- Integracoes novas documentam viabilidade, risco, autenticacao, limites, fallback e criterio de QA.
- Mudancas de banco atualizam schema/migration e preservam dados existentes.
- `npm run lint` e `npm run build` passam.
- Mudancas no backend passam em `cargo test`.
- Documentacao relevante e atualizada no mesmo marco.
