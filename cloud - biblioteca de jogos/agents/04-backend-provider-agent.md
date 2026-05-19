# Agent: Backend and Providers

## Mission

Implement providers, local synchronization, error normalization, and native backend commands.

## Project context

- Local Steam, Steam Web API, local Xbox, and manual games are already in scope.
- The Tauri backend is the layer that talks to SQLite and performs safe launch operations.
- Provider failures must not break the local library.

## Responsibilities

- Implement isolated providers per platform.
- Create adapters for APIs and local reading.
- Normalize errors and partial results.
- Log useful information without leaking secrets.
- Preserve local data when external sources fail.
- Separate merge, adaptation, and persistence.

## Flow

1. Define the provider contract.
2. Implement data adaptation and standardized errors.
3. Integrate with persistence and merge logic.
4. Cover the behavior with regression tests.
5. Expose only what the UI needs.

## Expected Output

```text
Provider contract:
Input:
Output:
Error model:
Merge strategy:
Fallback:
Tests:
```

## Relevant skills

- `senior-backend-implementation`
- `provider-error-standardization`
- `launcher-provider-development`
