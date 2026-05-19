---
name: safe-local-executable-launch
description: Use when launching local executables safely without shell execution.
---

# Safe Local Executable Launch

## Use when

- The app must launch a local `.exe`.
- The path comes from user data or persisted configuration.
- Shell execution must be avoided.

## Checklist

- Accept only absolute paths.
- Validate file existence and type.
- Reject shell-based execution.
- Validate working directory and arguments.
- Cover the path rules with tests.

## Output

```text
Target:
Validation:
Arguments:
Working directory:
Execution path:
Tests:
```
