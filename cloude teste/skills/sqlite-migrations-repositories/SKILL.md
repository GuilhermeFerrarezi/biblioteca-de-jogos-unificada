---
name: sqlite-migrations-repositories
description: Use ao implementar migracoes SQLite, repositories, queries e comandos Tauri de persistencia local para biblioteca, jogos manuais e dados de providers.
---

# SQLite Migrations and Repositories

## Objetivo

Implementar a camada SQLite de forma previsivel, testavel e preparada para evoluir junto dos providers.

## Migracoes

- Criar migrations versionadas e aplicadas em ordem.
- Registrar a versao aplicada em tabela propria, como `schema_migrations`.
- Migrations devem ser idempotentes quando possivel.
- Evitar migracoes destrutivas sem plano de copia.
- Cada migracao deve ter um objetivo pequeno e nome claro.
- Falhas de migracao devem impedir inicializacao parcial silenciosa.

## Repositories

Crie repositories pequenos e orientados a caso de uso:

```text
LibraryRepository
ManualGameRepository
LaunchActionRepository
ProviderSourceRepository
SyncHistoryRepository
```

Para a Fase 2, o minimo aceitavel e:

```text
list_library_entries
add_manual_game
update_manual_game
delete_manual_game ou archive_manual_game
```

## Queries

- Preferir queries parametrizadas.
- Nunca montar SQL com entrada do usuario por interpolacao de string.
- Buscar a biblioteca em uma query previsivel ou em poucas queries simples.
- Garantir que filtros comuns usem indices.
- Converter linhas do banco para DTOs estaveis antes de expor ao frontend.

## Transacoes

Use transacao para operacoes que envolvem mais de uma tabela:

- criar jogo manual;
- adicionar fontes;
- adicionar acoes de lancamento;
- atualizar jogo e entrada de biblioteca juntos;
- importar lote de provider.

Se qualquer etapa falhar, a operacao inteira deve ser revertida.

## Erros

Normalize erros antes de retornar para comandos Tauri:

```text
code
message
recoverable
details sanitizados
```

Nao retornar caminhos sensiveis, SQL bruto, tokens, cookies ou payloads completos de provider.

## Testes minimos

Quando houver runner backend:

- migracao cria schema em banco vazio;
- migracao nao quebra se rodada novamente;
- cadastro manual persiste;
- listagem retorna jogo manual com launch action;
- constraint impede fonte externa duplicada;
- transacao reverte cadastro incompleto;
- busca/filtros continuam coerentes apos persistencia.

## Criterios de aceite da Fase 2

- App abre com banco vazio.
- Seed/fallback inicial aparece quando aplicavel.
- Cadastro manual sobrevive a fechar e reabrir o app.
- Busca, filtros, selecao e painel de detalhes continuam funcionando.
- `npm run lint`, `npm run build`, `cargo check` e `npm run tauri:dev` passam.
- Nenhum erro ou log expoe segredo ou SQL sensivel.
