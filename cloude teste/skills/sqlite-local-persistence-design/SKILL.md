---
name: sqlite-local-persistence-design
description: Use ao desenhar a persistencia local SQLite do aplicativo, incluindo schema, tabelas, constraints, indices, seed de dados e local do arquivo de banco no Tauri.
---

# SQLite Local Persistence Design

## Objetivo

Projetar uma persistencia local simples, duravel e evolutiva para o MVP da biblioteca unificada de jogos.

## Prioridades

1. Persistir jogos manuais sem depender de conta externa.
2. Preservar separacao entre jogo canonico, entrada de biblioteca, fonte externa e acao de lancamento.
3. Evitar duplicidade quando providers reais forem adicionados.
4. Manter queries simples para busca, filtros e painel de detalhes.
5. Nao armazenar segredos em tabelas comuns.

## Schema minimo recomendado

Comece pelo minimo necessario para a Fase 2:

```text
games
library_entries
game_sources
launch_actions
game_genres
game_tags
sync_history
platform_accounts
```

Para a primeira entrega de persistencia manual, `platform_accounts` e `sync_history` podem existir apenas como preparacao ou ficar para migracao seguinte, desde que o schema nao bloqueie essa evolucao.

## Regras de modelagem

- `games` representa o jogo canonico.
- `library_entries` representa a presenca do jogo na biblioteca do usuario.
- `game_sources` guarda IDs externos por plataforma, inclusive `manual`.
- `launch_actions` guarda URI, executavel ou acao manual.
- Generos e tags devem ser normalizados apenas se isso simplificar filtros futuros; para MVP, listas simples por tabela relacional sao suficientes.
- Campos editaveis pelo usuario devem ficar separados ou claramente marcados para nao serem sobrescritos por providers.
- Use timestamps `created_at` e `updated_at` em registros alteraveis.
- Use chaves estaveis de texto quando elas ja existirem no dominio, evitando depender de ordem de insercao para contratos externos.

## Constraints e indices

Defina pelo menos:

- `PRIMARY KEY` em IDs internos.
- `UNIQUE(platform_id, external_id)` em fontes externas.
- indice em `games(sort_title)`.
- indice em `library_entries(install_status)`.
- indice em `library_entries(primary_platform_id)`.
- indice em `launch_actions(game_id, is_primary)`.
- indices parciais para caminhos frequentes, como entradas locais ativas e acoes primarias por plataforma.
- indices de limpeza para rotinas que arquivam falsos positivos sem varrer tabelas inteiras.

## Seed e fallback

- `mockLibrary.js` deve continuar servindo como seed/fallback de desenvolvimento, nao como fonte permanente depois que o banco existir.
- Seed inicial deve ser idempotente.
- Dados criados pelo usuario nunca devem ser apagados por seed.
- Se usar seed dos mocks, marque a origem como `seed` ou `manual/dev` para diferenciar de importacao real.

## Versionamento de schema

- Toda mudanca estrutural deve ter versao registrada em `schema_migrations`.
- Preferir `ALTER TABLE` e `CREATE INDEX IF NOT EXISTS` para upgrades incrementais simples.
- Migrations devem preservar jogos manuais, arquivamentos, launch actions e fontes externas.
- Incluir teste de banco legado quando a migration mexer em tabela existente.

## Local do banco

- Usar diretorio de dados da aplicacao via API/caminho Tauri quando disponivel.
- Evitar salvar banco dentro de `src`, `dist`, `target` ou junto do codigo fonte.
- Documentar o caminho usado e como apagar o banco em ambiente de desenvolvimento.

## Saida esperada

Ao aplicar esta skill, entregue:

- lista de tabelas com campos principais;
- constraints e indices;
- versao/migration proposta;
- estrategia de seed/fallback;
- local do arquivo SQLite;
- riscos e decisoes adiadas.
