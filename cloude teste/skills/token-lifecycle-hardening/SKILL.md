---
name: token-lifecycle-hardening
description: Use para desenhar ciclo de vida de tokens, renovacao, expiracao, revogacao, armazenamento seguro e limpeza de sessao.
---

# Token Lifecycle Hardening

## Ciclo minimo

- obter token por fluxo oficial;
- armazenar no cofre do SO quando possivel;
- renovar antes de expirar;
- detectar expiracao e pedir reconexao;
- revogar/desconectar;
- apagar dados sensiveis locais conforme escolha do usuario.

## Regras

- Token nunca deve aparecer em log, erro, screenshot ou frontend sem necessidade.
- Separar token de dados de biblioteca.
- Preferir escopos minimos.
- Tratar cookies de webview como segredo.

## Saida esperada

```text
Plataforma:
Fluxo:
Escopos:
Armazenamento:
Renovacao:
Revogacao:
Logs:
Riscos:
```
