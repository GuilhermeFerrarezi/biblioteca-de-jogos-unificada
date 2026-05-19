---
name: metadata-fallback-logic
description: Use when deciding fallback sources and confidence rules for metadata.
---

# Metadata Fallback Logic

## Use when

- A primary provider has incomplete data.
- The app needs a safe fallback hierarchy.
- Manual data should not be overwritten by weak inference.

## Checklist

- Define source priority.
- Preserve manual and official data first.
- Mark weak inference explicitly.
- Keep fallback results stable and explainable.

## Output

```text
Primary source:
Fallback source:
Priority rule:
Editable fields:
Confidence:
Manual review:
```
