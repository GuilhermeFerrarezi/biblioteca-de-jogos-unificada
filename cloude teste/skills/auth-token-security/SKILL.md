---
name: auth-token-security
description: Use ao desenhar login, armazenamento de tokens, cookies, sessoes, revogacao de contas e protecao de dados locais no app.
---

# Auth Token Security

## Principios

- Nunca pedir ou armazenar senha da plataforma.
- Preferir OAuth, device flow ou login oficial em navegador/webview confiavel.
- Guardar apenas tokens, cookies ou sessoes necessarios.
- Criptografar segredos no sistema operacional quando possivel.
- Permitir desconectar conta e apagar dados importados.
- Separar tokens/sessoes dos dados de biblioteca e metadados.
- Solicitar o menor escopo possivel para a feature atual.

## Checklist

- Qual dado sensivel sera salvo?
- Onde sera salvo?
- Quem consegue descriptografar?
- Como expira?
- Como renova?
- Como revoga?
- O que aparece em logs?
- O que aparece em crash reports?

## Regras para implementacao

- Separar `AuthVault` do restante do app.
- Guardar segredos no keychain/cofre do sistema operacional sempre que viavel.
- Nunca expor segredos ao frontend sem necessidade.
- Sanitizar logs por padrao.
- Registrar apenas hashes ou IDs internos quando precisar diagnosticar.
- Tratar cookies de webview como credenciais.
- Ao desconectar conta, remover tokens, sessoes e dados derivados que o usuario optar por apagar.
- Registrar eventos de expiracao, renovacao e revogacao sem incluir o valor do token.

## Ciclo de vida obrigatorio

```text
created -> stored_securely -> used_by_backend -> refreshed_or_expired -> revoked_or_deleted
```

Qualquer implementacao deve documentar:

- onde o segredo fica armazenado;
- como ele e renovado;
- como e invalidado;
- como o usuario desconecta a conta;
- quais dados permanecem depois da desconexao.

## Sinais de bloqueio

- Plataforma exige senha diretamente no app.
- Integracao precisa burlar 2FA, captcha ou protecao anti-automacao.
- Termos proibem o acesso planejado.
