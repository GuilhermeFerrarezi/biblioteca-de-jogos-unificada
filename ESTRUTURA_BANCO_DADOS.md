# Estrutura do Banco de Dados

Este documento descreve o SQLite local usado pelo backend Tauri da Biblioteca de Jogos Unificada.

## Local do arquivo

No aplicativo desktop, o banco e criado fora do codigo-fonte em:

```text
%APPDATA%\com.bibliotecajogos.unificada\library.sqlite3
```

O caminho e resolvido pelo Tauri com `app.path().app_data_dir()` e o arquivo e aberto no boot por `storage::open_database`.

## Inicializacao

Ao abrir o banco, o backend executa:

1. `PRAGMA foreign_keys = ON`.
2. `migrate()`, que cria as tabelas e indices base se ainda nao existirem.
3. `ensure_archived_column()`, compatibilidade para bancos antigos sem `library_entries.is_archived`.
4. `ensure_active_entries_index()`, indice parcial para entradas ativas.
5. `ensure_local_cleanup_indexes()`, indices de limpeza de jogos locais.
6. `archive_rejected_local_entries()`, limpeza leve de falsos positivos locais antigos.

O seed dos 4 mocks nao roda no caminho critico do boot. Ele e executado em background por `bootstrap_library`, com `seed_mock_library`, e emite o evento `library-bootstrap-complete` para o frontend recarregar a lista.

## Visao geral das tabelas

```text
schema_migrations

games
  1 -- 1 library_entries
  1 -- N game_sources
  1 -- N launch_actions
  1 -- N game_genres
```

O contrato entregue ao frontend e montado como `LibraryEntryDto`: uma entrada de biblioteca (`library_entries`) com os dados principais do jogo (`games`), fontes (`game_sources`), acoes de lancamento (`launch_actions`) e generos (`game_genres`).

## Tabelas

### `schema_migrations`

Controla a versao aplicada do schema.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `version` | `INTEGER` | `PRIMARY KEY` | Versao da migracao aplicada. Hoje o schema registra `1`. |
| `applied_at` | `TEXT` | `NOT NULL` | Timestamp ISO da aplicacao da migracao. |

### `games`

Guarda o cadastro canonico de cada jogo.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `id` | `TEXT` | `PRIMARY KEY` | ID interno estavel do jogo. |
| `title` | `TEXT` | `NOT NULL` | Nome exibido. |
| `sort_title` | `TEXT` | `NOT NULL` | Nome usado para ordenacao. |
| `installed` | `INTEGER` | `NOT NULL DEFAULT 0` | Booleano SQLite para instalado. |
| `playtime_total_minutes` | `INTEGER` | `NOT NULL DEFAULT 0` | Tempo total em minutos. |
| `accent_color` | `TEXT` | Opcional | Cor usada como fallback visual da capa. |
| `created_at` | `TEXT` | `NOT NULL` | Timestamp ISO de criacao. |
| `updated_at` | `TEXT` | `NOT NULL` | Timestamp ISO de atualizacao. |

### `library_entries`

Representa a presenca de um jogo na biblioteca unificada. Hoje ha uma entrada por jogo, garantida por `game_id UNIQUE`.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `id` | `TEXT` | `PRIMARY KEY` | ID da entrada de biblioteca. |
| `game_id` | `TEXT` | `NOT NULL UNIQUE`, FK `games(id)` `ON DELETE CASCADE` | Relaciona a entrada ao jogo. |
| `primary_platform_id` | `TEXT` | `NOT NULL` | Plataforma principal: `steam`, `local`, `manual`, etc. |
| `install_status` | `TEXT` | `NOT NULL` | Status como `installed` ou `not_installed`. |
| `last_played_label` | `TEXT` | `NOT NULL` | Texto de ultima execucao exibido na UI. |
| `is_archived` | `INTEGER` | `NOT NULL DEFAULT 0` | Booleano SQLite para arquivado. Entradas arquivadas nao aparecem na listagem principal. |
| `added_at` | `TEXT` | `NOT NULL` | Timestamp ISO de entrada na biblioteca. |
| `updated_at` | `TEXT` | `NOT NULL` | Timestamp ISO de atualizacao. |

### `game_sources`

Mapeia a origem externa de um jogo. Essa tabela evita duplicar importacoes da mesma plataforma e ID externo.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `id` | `TEXT` | `PRIMARY KEY` | ID da fonte. |
| `game_id` | `TEXT` | `NOT NULL`, FK `games(id)` `ON DELETE CASCADE` | Jogo relacionado. |
| `platform_id` | `TEXT` | `NOT NULL` | Plataforma da fonte. |
| `external_id` | `TEXT` | `NOT NULL` | ID externo da plataforma, por exemplo app id Steam ou caminho local normalizado. |
| `account_id` | `TEXT` | Opcional | Conta associada, quando providers com conta forem implementados. |

Restricao:

```sql
UNIQUE (platform_id, external_id)
```

### `launch_actions`

Guarda formas de iniciar um jogo.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `id` | `TEXT` | `PRIMARY KEY` | ID da acao. |
| `game_id` | `TEXT` | `NOT NULL`, FK `games(id)` `ON DELETE CASCADE` | Jogo relacionado. |
| `platform_id` | `TEXT` | `NOT NULL` | Plataforma dona da acao. |
| `kind` | `TEXT` | `NOT NULL` | Tipo: `manual`, `uri` ou `executable`. |
| `label` | `TEXT` | `NOT NULL` | Texto exibido na UI. |
| `target` | `TEXT` | `NOT NULL` | URI ou caminho do executavel. |
| `arguments_json` | `TEXT` | Opcional | Argumentos serializados em JSON. Hoje geralmente `[]`. |
| `working_directory` | `TEXT` | Opcional | Diretorio de trabalho para executaveis. |
| `is_primary` | `INTEGER` | `NOT NULL DEFAULT 0` | Booleano SQLite para acao principal. |

O comando `launch_library_entry` usa somente a acao primaria `executable` de entradas `manual` ou `local` ativas. Ele valida caminho absoluto, arquivo existente, arquivo local, extensao `.exe` e executa via `std::process::Command`, sem shell.

### `game_genres`

Lista generos de cada jogo em ordem.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `game_id` | `TEXT` | FK `games(id)` `ON DELETE CASCADE` | Jogo relacionado. |
| `genre` | `TEXT` | `NOT NULL` | Genero exibido/filtro. |
| `position` | `INTEGER` | `NOT NULL DEFAULT 0` | Ordem de exibicao. |

Chave primaria composta:

```sql
PRIMARY KEY (game_id, genre)
```

## Indices

Indices base criados por `migrate()`:

| Indice | Tabela/colunas | Objetivo |
| --- | --- | --- |
| `idx_games_sort_title` | `games(sort_title)` | Ordenacao por titulo. |
| `idx_library_entries_install_status` | `library_entries(install_status)` | Filtros por status de instalacao. |
| `idx_library_entries_platform` | `library_entries(primary_platform_id)` | Filtros por plataforma. |
| `idx_launch_actions_game_primary` | `launch_actions(game_id, is_primary)` | Busca da acao principal de um jogo. |

Indices adicionais de compatibilidade/otimizacao:

| Indice | Tabela/colunas | Objetivo |
| --- | --- | --- |
| `idx_library_entries_active_added_at` | `library_entries(added_at DESC) WHERE is_archived = 0` | Listagem principal de entradas ativas. |
| `idx_library_entries_local_active_game` | `library_entries(primary_platform_id, is_archived, game_id)` | Atalho para detectar/limpar entradas locais ativas. |
| `idx_launch_actions_platform_kind_game` | `launch_actions(platform_id, kind, game_id)` | `EXISTS` usado na limpeza de falsos positivos locais. |

## Operacoes principais

### Listagem unificada

`list_library_entries` consulta `library_entries` com `games`, filtra `is_archived = 0`, ordena por `added_at DESC, sort_title`, e depois carrega fontes, acoes e generos por `game_id`.

### Cadastro manual

`add_manual_game` cria, em transacao:

1. Registro em `games`.
2. Registro em `library_entries` com `primary_platform_id = 'manual'`.
3. Registro em `game_sources`.
4. Acao em `launch_actions`, inferindo `manual`, `uri` ou `executable`.
5. Genero em `game_genres`.

### Edicao manual

`update_manual_game` atualiza, em transacao:

1. `games`.
2. `library_entries.install_status`.
3. `launch_actions` primaria.
4. `game_genres`, recriando o genero principal.

O estado `is_archived` e preservado.

### Arquivamento

`set_library_entry_archived` altera `library_entries.is_archived` e `updated_at`. A listagem principal oculta arquivados.

### Sincronizacao local

`sync_local_games` descobre executaveis locais, cria/atualiza entradas `local` e arquiva falsos positivos antigos como DirectX, `_CommonRedist`, EpicOnlineServices, redistribuiveis e instaladores.

### Sincronizacao Steam local

`sync_steam_games` descobre bibliotecas Steam instaladas, le `steamapps/libraryfolders.vdf` para encontrar bibliotecas extras e importa `appmanifest_*.acf` como entradas `steam`.

Regras atuais:

1. `game_sources.platform_id = 'steam'`.
2. `game_sources.external_id` guarda o AppID Steam.
3. `library_entries.primary_platform_id = 'steam'`.
4. `library_entries.install_status = 'installed'`.
5. `launch_actions.kind = 'uri'`.
6. `launch_actions.target = 'steam://rungameid/<appid>'`.
7. `launch_actions.working_directory` guarda o diretorio instalado quando `installdir` aponta para uma pasta existente.

A sincronizacao e idempotente: se o AppID ja existir em `game_sources`, a entrada e atualizada em vez de duplicada.

## Regras para evoluir o schema

- Toda mudanca de schema deve ser idempotente.
- Manter compatibilidade com bancos ja existentes por meio de `ALTER TABLE`/`CREATE INDEX IF NOT EXISTS` quando necessario.
- Preservar `PRAGMA foreign_keys = ON`.
- Incluir testes Rust para migracao, upgrade de banco legado e operacoes afetadas.
- Atualizar este documento, `CHECKPOINT.md` e `RETOMADA_NOVO_COMPUTADOR.md` quando o modelo mudar.
