# Indice - cloude teste

Esta pasta guarda a divisao inicial de trabalho para criar o aplicativo de biblioteca unificada de jogos.

## Pastas

- `agents/`: papeis especializados que podem ser usados para dividir tarefas futuras.
- `skills/`: instrucoes reutilizaveis em formato `SKILL.md` para orientar pesquisas, arquitetura, seguranca, UX e integracoes.

## Como usar em sessoes futuras

1. Comece lendo `../CHECKPOINT.md`.
2. Se estiver em outro computador, siga primeiro `../RETOMADA_NOVO_COMPUTADOR.md`.
3. Escolha o agente mais adequado ao trabalho do momento.
4. Use as skills indicadas no arquivo do agente.
5. Ao concluir uma decisao ou marco importante, atualize `../CHECKPOINT.md`.

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
