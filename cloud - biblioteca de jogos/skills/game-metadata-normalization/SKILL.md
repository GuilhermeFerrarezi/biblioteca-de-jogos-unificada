---
name: game-metadata-normalization
description: Use when normalizing game titles, genres, sources and launch metadata.
---

# Game Metadata Normalization

## Use when

- Multiple sources describe the same game differently.
- A manual entry must remain the source of truth.
- Metadata needs stable sort and merge behavior.

## Checklist

- Normalize titles and sort keys.
- Preserve source-specific IDs and user overrides.
- Separate same game from same installation.
- Mark low-confidence matches for manual review.

## Output

```text
Primary source:
Secondary source:
Conflict rule:
Editable fields:
Preserved platform fields:
Dedup confidence:
Manual review candidates:
```
