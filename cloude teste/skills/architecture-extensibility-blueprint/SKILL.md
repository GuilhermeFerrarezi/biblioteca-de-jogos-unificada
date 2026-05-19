---
name: architecture-extensibility-blueprint
description: Use when defining modules, contracts, versioning and future extensibility.
---

# Architecture Extensibility Blueprint

## Use when

- A change affects core contracts or boundaries.
- The app needs to grow without a rewrite.
- Multiple providers or storage layers must stay decoupled.

## Checklist

- Define the main modules and contracts.
- Separate core, UI, services, providers and storage.
- Document versioning and compatibility decisions.
- Note tradeoffs and future extension points.

## Output

```text
Modules:
Main contracts:
Boundaries:
Versioning plan:
Extension points:
Tradeoffs:
```
