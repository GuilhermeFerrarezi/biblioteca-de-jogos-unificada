---
name: senior-backend-implementation
description: Use when a backend slice needs structured domain logic, persistence and tests.
---

# Senior Backend Implementation

## Use when

- The backend cut needs multiple layers.
- SQLite, Tauri commands or provider merge logic are involved.
- Compatibility and test coverage matter.

## Checklist

- Keep domain, persistence and commands separated.
- Preserve compatibility with existing data.
- Add tests for critical behavior.
- Avoid leaking UI concerns into backend code.

## Output

```text
Domain:
Persistence:
Commands:
Tests:
Validation:
Risks:
```
