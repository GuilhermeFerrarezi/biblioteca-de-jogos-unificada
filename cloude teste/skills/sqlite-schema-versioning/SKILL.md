---
name: sqlite-schema-versioning
description: Use ao evoluir schema SQLite, criar migracoes idempotentes, testar upgrades e documentar versoes.
---

# SQLite Schema Versioning

## Regras

- Toda alteracao de schema precisa de versao ou rotina de compatibilidade.
- Migracoes devem ser idempotentes quando possivel.
- Evitar operacoes destrutivas sem copia/transicao.
- Testar banco vazio e banco legado.
- Atualizar `ESTRUTURA_BANCO_DADOS.md`.

## Checklist

- Nova tabela, coluna, indice ou constraint documentada.
- Query afetada revisada.
- Teste Rust cobrindo upgrade.
- Seed continua idempotente.
- Dados do usuario preservados.

## Saida esperada

```text
Versao/compat:
Mudanca:
Motivo:
Migracao:
Rollback logico:
Testes:
Documentacao:
```
