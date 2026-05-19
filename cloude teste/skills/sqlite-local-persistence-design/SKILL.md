---
name: sqlite-local-persistence-design
description: Use when designing the desktop SQLite database for the library app.
---

# SQLite Local Persistence Design

## Use when

- The desktop app needs a local source of truth.
- Manual games, syncs or settings must survive restart.
- Storage design must stay simple and evolvable.

## Checklist

- Separate game, entry, source, action and metadata tables.
- Keep secrets out of the main schema.
- Add useful indexes for list and filters.
- Make seed and migrations idempotent.

## Output

```text
Tables:
Constraints:
Indexes:
Migration plan:
Seed strategy:
Compatibility notes:
```
