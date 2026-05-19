---
name: sqlite-migrations-repositories
description: Use when building SQLite repositories, queries and versioned migrations.
---

# SQLite Migrations and Repositories

## Use when

- Database access must be organized around repository boundaries.
- A schema change needs a migration path.
- Legacy compatibility or atomic writes are involved.

## Checklist

- Keep migrations versioned and incremental.
- Build repositories around stable queries.
- Preserve user data during upgrade.
- Add tests for legacy and new schema paths.

## Output

```text
Repositories:
Migration version:
Schema impact:
Atomic writes:
Legacy support:
Tests:
```
