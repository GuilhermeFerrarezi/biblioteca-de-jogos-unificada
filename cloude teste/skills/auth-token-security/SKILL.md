---
name: auth-token-security
description: Use when handling auth tokens, account configs or secure credential storage.
---

# Auth Token Security

## Use when

- A token, key or session must be stored or refreshed.
- The app cannot expose secrets to UI or logs.
- Account configuration is part of the cut.

## Checklist

- Keep secrets out of public storage.
- Separate secrets from non-secret metadata.
- Define revoke and delete behavior.
- Avoid logging raw credentials or callbacks.

## Output

```text
Secret type:
Storage location:
Non-secret metadata:
Revocation:
Logging rules:
Open risks:
```
