---
name: sqlite-schema-versioning
description: Use when a schema change needs versioned migration and legacy compatibility.
---

# SQLite Schema Versioning

## Use when

- The local schema changes in a backward-sensitive way.
- Legacy databases need to keep working.
- Migration history must stay explicit.

## Checklist

- Record a version for the change.
- Keep migrations incremental and idempotent.
- Test upgrade from legacy data.
- Preserve user records and settings.

## Output

```text
Schema version:
Tables affected:
Migration path:
Legacy support:
Tests:
Risks:
```
