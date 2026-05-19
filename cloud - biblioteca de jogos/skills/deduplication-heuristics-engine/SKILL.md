---
name: deduplication-heuristics-engine
description: Use when deciding whether two game records should merge or stay separate.
---

# Deduplication Heuristics Engine

## Use when

- The same game can appear from multiple platforms.
- Low-confidence matches must not merge automatically.
- Manual review may be needed for duplicates.

## Checklist

- Prefer exact IDs and official mappings.
- Use title normalization only as a secondary signal.
- Block obvious false matches.
- Mark uncertain matches for manual confirmation.

## Output

```text
Match level:
Primary signals:
Secondary signals:
Blocked cases:
Manual review:
Decision:
```
