---
name: tauri-desktop-security-hardening
description: Use when hardening Tauri IPC, file access and local execution boundaries.
---

# Tauri Desktop Security Hardening

## Use when

- A change touches Tauri commands or IPC.
- File paths, local execution or window behavior are involved.
- The app needs a tighter desktop security posture.

## Checklist

- Keep allowlists narrow.
- Avoid shell execution.
- Validate paths, arguments and work directories.
- Protect secrets and sensitive payloads.

## Output

```text
Risk surface:
Allowlist changes:
Path validation:
Execution rules:
Logging rules:
Open risks:
```
