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
4. `ensure_favorite_column()`, compatibilidade para bancos antigos sem `library_entries.is_favorite`.
5. `ensure_active_entries_index()`, indice parcial para entradas ativas.
6. `ensure_favorite_entries_index()`, indice parcial para favoritos ativos.
7. `ensure_local_cleanup_indexes()`, indices de limpeza de jogos locais.
8. `archive_rejected_local_entries()`, limpeza leve de falsos positivos locais antigos.
9. `ensure_provider_account_configs_table()`, compatibilidade para contas/provedor, metadados de Steam, configuracoes Xbox e padroes da biblioteca.

O seed dos 4 mocks nao roda no caminho critico do boot. Ele e executado em background por `bootstrap_library`, com `seed_mock_library`, e emite o evento `library-bootstrap-complete` para o frontend recarregar a lista.

## Visao geral das tabelas

```text
schema_migrations

games
  1 -- 1 library_entries
  1 -- N game_sources
  1 -- N launch_actions
  1 -- N game_genres
  1 -- 0/1 game_user_reviews

provider_account_configs
```

O contrato entregue ao frontend e montado como `LibraryEntryDto`: uma entrada de biblioteca (`library_entries`) com os dados principais do jogo (`games`), fontes (`game_sources`), acoes de lancamento (`launch_actions`), generos (`game_genres`) e avaliacao pessoal (`game_user_reviews`).

`schema_migrations` registra a migracao formal atualmente aplicada no banco, mas nao representa sozinho toda a historia de compatibilidade. O backend hoje corrige bancos legados em runtime com `ALTER TABLE` e indices `IF NOT EXISTS`, entao a presenca de uma tabela/coluna no banco nao deve ser inferida apenas pela versao persistida.

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
| `is_favorite` | `INTEGER` | `NOT NULL DEFAULT 0` | Booleano SQLite para favorito. Usado pelo filtro Favoritos e pela ordenacao de favoritos primeiro. |
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

### `game_user_reviews`

Guarda a avaliacao pessoal do usuario para cada jogo canonico. Esta tabela e separada das fontes/providers para que jogos agrupados Steam/Xbox/Epic compartilhem a mesma nota e resenha.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `game_id` | `TEXT` | `PRIMARY KEY`, FK `games(id)` `ON DELETE CASCADE` | Jogo canonico avaliado. |
| `personal_rating` | `REAL` | Opcional | Nota pessoal em incrementos de meia estrela, de `0.5` a `5`; `NULL` representa sem avaliacao. |
| `personal_review` | `TEXT` | Opcional | Resenha pessoal em texto livre, limitada a 4000 caracteres; texto vazio/whitespace e normalizado para `NULL`. |
| `created_at` | `TEXT` | `NOT NULL` | Timestamp ISO de criacao da avaliacao. |
| `updated_at` | `TEXT` | `NOT NULL` | Timestamp ISO da ultima atualizacao. |

O DTO da listagem expoe esses dados como `game.personalRating` e `game.personalReview`. Quando uma entrada visual esta agrupada no frontend, a mesma avaliacao/resenha e exibida ao alternar provider ou launcher.

### `provider_account_configs`

Guarda configuracoes nao secretas de conta por provider. Segredos como Steam Web API key continuam fora desta tabela e devem ficar no AuthVault do backend, usando o keyring/cofre do sistema operacional como primario e um arquivo DPAPI cifrado pelo usuario Windows como fallback quando o Credential Manager nao consegue validar leitura apos gravacao.

| Coluna | Tipo | Regra | Uso |
| --- | --- | --- | --- |
| `provider_id` | `TEXT` | `PRIMARY KEY` | Provider/configuracao, como `steam`, `xbox` ou `library`. |
| `account_id` | `TEXT` | Opcional | ID publico/nao secreto da conta; para Steam, o SteamID64. |
| `steam_id64` | `TEXT` | Opcional | SteamID64 verificado via OpenID ou salvo manualmente. |
| `steam_web_api_key_configured` | `INTEGER` | `NOT NULL DEFAULT 0` | Marcador nao secreto atualizado apos salvar/remover a chave no AuthVault. Nao prova sozinho que o segredo esta legivel. |
| `config_json` | `TEXT` | Opcional | JSON auxiliar nao secreto, usado para metadados de sync Steam, raizes Xbox e padroes da biblioteca. |
| `updated_at` | `TEXT` | `NOT NULL` | Timestamp ISO da ultima atualizacao. |

O fluxo `Entrar com Steam` usa OpenID no navegador externo, valida a resposta com a Steam e persiste apenas o SteamID64. O app nao grava senha, Steam Guard, cookies, sessao de navegador, OpenID assertion ou URL completa de callback.

Bancos locais criados durante testes anteriores podem conter uma coluna legada `steam_web_api_key_plaintext_dev`. O codigo atual nao cria nem usa essa coluna no caminho normal; a fonte de verdade da Steam Web API key e somente o AuthVault. O SQLite guarda apenas `steam_web_api_key_configured`, que e um marcador nao secreto e nunca substitui a validacao de leitura do segredo pelo backend.

Compatibilidade atual:

- `ensure_provider_account_configs_table()` cria a tabela se ela ainda nao existir.
- Se o banco legado nao tiver `steam_id64`, `config_json` ou `steam_web_api_key_configured`, o backend faz `ALTER TABLE` para completar o schema.
- O registro Steam e tratado como upsert em `provider_id = 'steam'`; isso preserva o `account_id`/`steam_id64` e os metadados de sync mesmo quando a configuracao da chave muda.
- `config_json` e o ponto de persistencia dos metadados de sync da Steam, incluindo `steamId64`, `linkedBy`, `lastOwnedGamesSyncAt`, `lastOwnedGamesCount` e `lastOwnedGamesSummary`.
- Configuracoes Xbox e padroes de scan local tambem podem usar `provider_account_configs.config_json`, desde que continuem sem segredos e preservem chaves existentes no merge.
- Futuras migracoes nao devem reintroduzir segredo em SQLite nem sobrescrever `config_json` sem preservar os campos existentes.

## AuthVault e segredos locais

A Steam Web API key nao faz parte do schema SQLite. O backend Tauri usa o `AuthVault` para gravar e ler o segredo somente no processo nativo.

Armazenamento atual:

1. Primario: keyring/cofre do sistema operacional via `keyring`.
2. Fallback Windows: arquivo `%APPDATA%\com.bibliotecajogos.unificada\auth-vault\steam-web-api-key.dpapi`, contendo apenas bytes cifrados por DPAPI com entropia especifica do app.

O fallback existe para ambientes em que o Credential Manager aceita a gravacao, mas nao retorna a credencial no read-back imediato. Mesmo nesse caso, o valor nao e salvo em texto puro no banco, em JSON, em `localStorage` ou em payloads para o frontend.

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
| `idx_library_entries_active_favorites` | `library_entries(added_at DESC) WHERE is_archived = 0 AND is_favorite = 1` | Consulta e evolucao de filtros sobre favoritos ativos. |
| `idx_library_entries_local_active_game` | `library_entries(primary_platform_id, is_archived, game_id)` | Atalho para detectar/limpar entradas locais ativas. |
| `idx_launch_actions_platform_kind_game` | `launch_actions(platform_id, kind, game_id)` | `EXISTS` usado na limpeza de falsos positivos locais. |

## Operacoes principais

### Listagem unificada

`list_library_entries` consulta `library_entries` com `games`, filtra `is_archived = 0`, ordena por `added_at DESC, sort_title`, e depois carrega fontes, acoes, generos e avaliacao pessoal por `game_id`.

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

### Favoritos

`set_library_entry_favorite` altera `library_entries.is_favorite` e `updated_at`. A listagem principal continua retornando favoritos e nao favoritos; o frontend usa `isFavorite` para marcar jogos, filtrar favoritos e ordenar favoritos primeiro.

### Avaliacao pessoal

`set_library_entries_personal_review` grava rating e review em `game_user_reviews` para uma ou mais entradas de biblioteca. O rating aceita `NULL` ou valores de meia estrela entre `0.5` e `5`. A review e normalizada com trim; string vazia ou apenas whitespace vira `NULL`, e o limite e de 4000 caracteres.

No frontend, jogos agrupados por multiplos providers usam uma avaliacao por jogo agrupado. O comando recebe os ids das entradas membros e grava o mesmo rating/review para os jogos canonicos correspondentes, preservando a experiencia de uma unica avaliacao ao alternar Steam/Xbox/Epic. A acao `Limpar nota` envia `personalRating = NULL` preservando `personalReview`; apagar a review continua sendo feito manualmente limpando o texto e salvando.

A navegacao da biblioteca usa esses campos apenas no frontend: badges discretos em cards/lista, ordenacoes `Melhor avaliados` e `Pior avaliados` com jogos sem nota no fim, e filtros `Avaliados`/`Não avaliados`. Nao ha filtro por faixa de estrelas neste corte.

### Sincronizacao local

`sync_local_games` descobre executaveis locais, cria/atualiza entradas `local` e arquiva falsos positivos antigos como DirectX, `_CommonRedist`, EpicOnlineServices, redistribuiveis, instaladores e componentes auxiliares do Battle.net/Blizzard.

Heuristica de limpeza local:

- agir apenas sobre entradas `local` ativas;
- arquivar, nao deletar, quando a acao primaria apontar para um executavel e o alvo/rotulo baterem com redistribuiveis, instaladores conhecidos ou componentes de launcher como `Battle.net.exe`, `Agent`, `BlizzardBrowser`, `BlizzardError` e `BlizzardUpdateAgent`;
- manter conservadorismo: a regra existe para reduzir ruido, nao para remover instalacoes validas;
- usar os indices `idx_library_entries_local_active_game` e `idx_launch_actions_platform_kind_game` para evitar varreduras completas.

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
Quando um AppID Steam persistido deixa de aparecer nos manifests locais, a entrada e preservada, mas `games.installed` passa para `0` e `library_entries.install_status` passa para `not_installed`. Arquivamento continua sendo decisao explicita do usuario.
Entradas tecnicas da Steam que nao representam jogos, como AppID `228980` (`Steamworks Common Redistributables`), sao rejeitadas na descoberta e arquivadas caso ja tenham sido importadas antes.

Os metadados de sync da Steam devem ser preservados como complementos do registro, nao como substituicao do cadastro. `record_steam_account_sync_metadata()` faz merge em `config_json` e nao deve apagar chaves anteriores nem trocar o registro da conta.

### Conta Steam e Web API

`save_steam_account_config` grava SteamID64 em `provider_account_configs` para que o backend seja a fonte de verdade da sincronizacao por conta. `sync_steam_account_games` le esse valor do SQLite e usa a chave Steam Web API somente a partir do AuthVault, sem expor o segredo ao frontend e sem fallback de segredo no SQLite.

O comando de login `start_steam_openid_login` apenas vincula a identidade Steam. Ele nao retorna token OAuth nem permissao para biblioteca privada; a consulta de jogos continua sujeita a chave Web API valida e visibilidade da biblioteca Steam.

### Steam enrichment e achievements

O enrichment/achievements da Steam roda best-effort em background apos `sync_steam_account_games`: metadados, artwork e sinais de achievements podem complementar registros ja sincronizados, mas nao devem bloquear boot, listagem, sincronizacao principal ou lancamento.

O processamento deixou de ser um limite unico de 50 jogos. A fila pode continuar em lotes sucessivos em background, usando um lote interno conservador para controlar pressao no provedor. Entre chamadas, o job deve respeitar pausa/backoff; se a Steam Web API responder com rate limit, a rodada para, o erro emitido ao frontend fica sanitizado e o progresso ja salvo no cache permanece disponivel para retomada posterior.

O schema registra a versao `3` para cache Steam e marcadores de tentativa de enrichment:

```text
steam_achievement_schema_cache
  app_id TEXT PRIMARY KEY
  schema_json TEXT NOT NULL
  achievement_count INTEGER NOT NULL DEFAULT 0
  fetched_at TEXT NOT NULL
  expires_at TEXT NOT NULL
  updated_at TEXT NOT NULL

steam_player_achievement_cache
  steam_id64 TEXT NOT NULL
  app_id TEXT NOT NULL
  achievements_json TEXT NOT NULL
  unlocked_count INTEGER NOT NULL DEFAULT 0
  total_count INTEGER NOT NULL DEFAULT 0
  fetched_at TEXT NOT NULL
  expires_at TEXT NOT NULL
  updated_at TEXT NOT NULL
  PRIMARY KEY (steam_id64, app_id)

steam_enrichment_attempt_cache
  steam_id64 TEXT NOT NULL DEFAULT ''
  app_id TEXT NOT NULL
  phase TEXT NOT NULL
  outcome TEXT NOT NULL
  attempted_at TEXT NOT NULL
  expires_at TEXT NOT NULL
  updated_at TEXT NOT NULL
  PRIMARY KEY (steam_id64, app_id, phase)
```

`steam_achievement_schema_cache` guarda definicoes por AppID com TTL mais longo. `steam_player_achievement_cache` guarda progresso por `steam_id64 + app_id`, mantendo isolamento por conta. `steam_enrichment_attempt_cache` funciona como negative cache temporario por fase (`artwork`, `achievement_schema`, `player_achievements`) para que jogos sem artwork/achievements disponiveis ou com falha nao rate-limit nao voltem para a fila imediatamente a cada nova sincronizacao. O cache nao substitui `games`, `library_entries`, `game_sources` nem dados editados pelo usuario. Como a fila e continua, esses caches tambem funcionam como checkpoint natural entre lotes e depois de uma parada por rate limit.

### Xbox achievements em espera de compliance

Achievements/title history cross-title do Xbox nao devem receber nova tabela, coluna ou persistencia enquanto nao houver confirmacao oficial de uso permitido, escopos, limites, revogacao e regras de armazenamento/exibicao. Qualquer persistencia futura desses dados precisa passar por decisao de compliance e por nova versao formal de schema.

## Regras para evoluir o schema

- Toda mudanca de schema deve ganhar versao formal em `schema_migrations` e, quando necessario, compatibilidade de bootstrap para bancos legados.
- Manter compatibilidade com bancos ja existentes por meio de `ALTER TABLE`/`CREATE INDEX IF NOT EXISTS` quando necessario.
- Preservar `PRAGMA foreign_keys = ON`.
- Incluir testes Rust para migracao, upgrade de banco legado e operacoes afetadas.
- Atualizar este documento, `CHECKPOINT.md` e `RETOMADA_NOVO_COMPUTADOR.md` quando o modelo mudar.
- Nao assumir que `schema_migrations` sozinho descreve o estado real do banco; validar colunas e indices quando a migracao depender disso.

## N+1 E Listagem

`list_library_entries()` faz a consulta base de `library_entries + games`, mas a hidracao atual ainda chama `list_sources()`, `list_launch_actions()` e `list_genres()` para cada jogo. Na pratica, isso vira um padrao N+1 de 1 consulta base + 3 consultas por entrada.

Isso e aceitavel enquanto a biblioteca for pequena e os indices atuais forem mantidos, mas passa a ser um risco real se:

- a lista crescer muito;
- mais colecoes forem adicionadas ao `LibraryEntryDto`;
- a listagem passar a carregar mais campos derivados por entrada.

Quando uma migration ou ajuste de dominio aumentar esse custo, a solucao esperada e prefetch/batch por `game_id`, nao mais consultas por item sem justificativa.

## O que passa a ser obrigatorio em futuras migracoes

1. Registrar a nova versao em `schema_migrations` e manter a rotina de bootstrap compativel com bancos antigos.
2. Entregar teste de upgrade a partir de schema legado quando houver alteracao em tabela existente.
3. Proteger dados do usuario: `games`, `library_entries`, `game_sources`, `launch_actions`, `game_genres` e `config_json` de Steam/Xbox/library nao podem ser apagados por compatibilidade.
4. Se `provider_account_configs` mudar, preservar o separador entre metadados nao secretos e segredos no AuthVault.
5. Se a listagem principal mudar, declarar o impacto de consultas e indices e evitar piorar o N+1 sem batch/prefetch.
6. Se a limpeza local mudar, continuar arquivando em vez de deletar, a menos que exista plano de restauracao/copia explicitamente documentado.
7. Dados pessoais como reviews, colecoes e tags podem exigir no futuro uma identidade canonica persistente de jogo agrupado no backend; hoje o agrupamento cross-platform ainda e sintetico no frontend por titulo normalizado.
