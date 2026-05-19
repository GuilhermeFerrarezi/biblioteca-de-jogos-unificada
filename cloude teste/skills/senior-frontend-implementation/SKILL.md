---
name: senior-frontend-implementation
description: Use when a larger frontend slice needs reusable React structure and careful state management.
---

# Senior Frontend Implementation

## Use when

- A screen has several components, hooks or states.
- The cut needs cleaner boundaries and better performance.
- The UI must stay accessible and stable.

## Checklist

- Split components by responsibility.
- Keep services at the boundary.
- Preserve loading, empty and error states.
- Avoid unnecessary rerenders and prop drilling.
- Add tests when a rule is critical.

## Output

```text
Screen:
Components:
State:
Services:
Accessibility:
Performance:
Tests:
```
