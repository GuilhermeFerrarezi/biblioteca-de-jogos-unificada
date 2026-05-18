---
name: provider-error-standardization
description: Use para padronizar erros de providers, sync parcial, mensagens recuperaveis e fallback de dados locais.
---

# Provider Error Standardization

## Modelo de erro

```text
code
message
recoverable
providerId
phase
details_sanitized
```

## Codigos sugeridos

- `auth_required`
- `auth_expired`
- `rate_limited`
- `network_unavailable`
- `platform_unavailable`
- `unsupported_operation`
- `parse_failed`
- `local_scan_failed`

## Regras

- Nunca retornar payload bruto externo para a UI.
- Falha de um provider nao invalida dados persistidos.
- Sync parcial deve informar itens inseridos, atualizados, ignorados e falhos.
- Mensagens ao usuario devem ser acionaveis e curtas.
