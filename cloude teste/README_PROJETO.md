# Indice - cloude teste

Esta pasta guarda a divisao inicial de trabalho para criar o aplicativo de biblioteca unificada de jogos.

## Pastas

- `agents/`: papeis especializados que podem ser usados para dividir tarefas futuras.
- `skills/`: instrucoes reutilizaveis em formato `SKILL.md` para orientar pesquisas, arquitetura, seguranca, UX e integracoes.

## Como usar em sessoes futuras

1. Comece lendo `../CHECKPOINT.md`.
2. Leia `../DIRETRIZES_DESENVOLVIMENTO.md` para seguir as regras arquiteturais atuais.
3. Se estiver em outro computador, siga primeiro `../RETOMADA_NOVO_COMPUTADOR.md`.
4. Consulte `../ESTRUTURA_BANCO_DADOS.md` quando a tarefa tocar persistencia, migrations, providers ou dados locais.
5. Escolha o agente mais adequado ao trabalho do momento.
6. Declare o agente principal, agentes auxiliares, skills aplicadas, arquivos provaveis e criterios de validacao antes de implementar.
7. Use as skills indicadas no arquivo do agente e complemente com as skills transversais abaixo quando houver risco de arquitetura, seguranca, UX, metadados ou banco.
8. Se delegar para subagentes, instrua cada subagente a seguir explicitamente o agente local e as skills escolhidas. O tipo tecnico do subagente nao substitui os agentes desta pasta.
9. Ao concluir uma decisao ou marco importante, atualize `../CHECKPOINT.md`.

## Ordem recomendada

1. `00-project-manager.md`
2. `01-platform-research-agent.md`
3. `02-architecture-agent.md`
4. `03-security-auth-agent.md`
5. `04-backend-provider-agent.md`
6. `05-frontend-ux-agent.md`
7. `06-metadata-agent.md`
8. `11-senior-database-agent.md`
9. `08-senior-backend-development-agent.md`
10. `09-senior-frontend-development-agent.md`
11. `07-qa-compliance-agent.md`
12. `10-senior-integration-qa-agent.md`

## Agentes de programacao

- Use `08-senior-backend-development-agent.md` para implementar dominio, persistencia, providers, services, comandos Tauri e testes backend.
- Use `09-senior-frontend-development-agent.md` para implementar componentes React, estado de UI, camada de API, fluxos de biblioteca e cadastro manual.
- Use `10-senior-integration-qa-agent.md` para revisar integracao, builds, lint, testes, riscos de seguranca e criterios de aceite.
- Use `11-senior-database-agent.md` com as skills `sqlite-local-persistence-design` e `sqlite-migrations-repositories` para definir schema SQLite, migracoes, repositories, constraints, indices e estrategia de persistencia local antes da implementacao da Fase 2.

## Skills transversais adicionadas

Use estas skills como reforco quando a tarefa ultrapassar uma camada isolada:

- Governanca e escopo: `project-scoping-and-coordination`.
- Pesquisa de plataforma: `platform-viability-matrix`, `platform-integration-research`, `api-compliance-review`.
- Arquitetura: `architecture-extensibility-blueprint`, `launcher-provider-development`.
- Seguranca: `tauri-desktop-security-hardening`, `auth-token-security`, `token-lifecycle-hardening`, `safe-local-executable-launch`.
- Backend/providers: `provider-error-standardization`, `senior-backend-implementation`.
- Frontend/UX: `ui-component-standardization`, `react-performance-optimization`, `desktop-app-product-design`, `senior-frontend-implementation`, `usability-heuristics-evaluation`.
- Metadados: `metadata-fallback-logic`, `deduplication-heuristics-engine`, `game-metadata-normalization`.
- Banco: `sqlite-local-persistence-design`, `sqlite-migrations-repositories`, `sqlite-schema-versioning`.
- Qualidade integrada: `senior-integration-quality`.

## Regras de alinhamento

- O frontend atual e React 18 com JavaScript/JSX; nao assumir TypeScript nos documentos, exemplos ou novas tarefas.
- O service principal do frontend e `src/services/libraryService.js`; mocks devem permanecer como fallback controlado de desenvolvimento.
- Toda integracao nova deve ter matriz de viabilidade, risco de compliance, contrato de provider, erro padronizado, estrategia de cache/fallback e criterio de QA.
- Toda alteracao de banco deve atualizar ou validar `ESTRUTURA_BANCO_DADOS.md` e incluir migration versionada quando necessario.
- Toda tarefa de desenvolvimento deve identificar agente e skills antes de editar codigo. Para tarefas com mais de uma area, use um agente principal e agentes auxiliares.
- Delegacoes devem ser feitas como execucao/revisao de um agente local. Exemplo: "atuar como `04-backend-provider-agent.md` usando as skills `platform-viability-matrix`, `launcher-provider-development` e `senior-backend-implementation`".
