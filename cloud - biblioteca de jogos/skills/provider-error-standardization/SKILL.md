---
name: provider-error-standardization
description: Use when normalizing provider errors, partial sync results and user-facing feedback.
---

# Provider Error Standardization

## Use when

- A provider can fail independently from the rest of the library.
- The UI needs short actionable messages.
- Raw external payloads must not reach the frontend.

## Checklist

- Return a stable error shape.
- Include recoverability and provider identity.
- Sanitize details before exposing them.
- Keep sync partial when possible.

## Output

```text
Code:
Message:
Recoverable:
Provider:
Phase:
Sanitized details:
```
