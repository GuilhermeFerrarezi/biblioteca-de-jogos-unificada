---
name: senior-integration-quality
description: Use when a larger cut needs final integration review and release readiness.
---

# Senior Integration Quality

## Use when

- The change touches more than one layer.
- The final release decision depends on end-to-end behavior.
- Stronger QA is needed before shipping.

## Checklist

- Confirm build, lint and relevant tests.
- Check launch safety, persistence and secrets.
- Review accessibility and empty/error states.
- Decide whether the cut is releasable.

## Output

```text
Findings:
Validation:
Risks:
Acceptance:
Release decision:
```
