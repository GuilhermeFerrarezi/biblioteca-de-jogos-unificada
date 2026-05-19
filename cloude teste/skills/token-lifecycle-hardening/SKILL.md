---
name: token-lifecycle-hardening
description: Use when defining the lifecycle of account tokens or API keys.
---

# Token Lifecycle Hardening

## Use when

- A token must be created, stored, refreshed or revoked.
- Secret handling needs a full lifecycle policy.
- The app must clean up credentials safely.

## Checklist

- Define creation and storage rules.
- Specify refresh and expiration behavior.
- Define revoke and delete behavior.
- Keep tokens out of logs and UI payloads.

## Output

```text
Token type:
Creation:
Storage:
Refresh:
Revocation:
Deletion:
```
