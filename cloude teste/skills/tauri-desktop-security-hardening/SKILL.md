---
name: tauri-desktop-security-hardening
description: Use ao revisar comandos Tauri, permissoes, IPC, filesystem, execucao local, logs e fronteira frontend-backend.
---

# Tauri Desktop Security Hardening

## Checklist

- Comandos Tauri expostos sao estritamente necessarios.
- Entrada do frontend e validada no backend.
- Operacoes de filesystem usam caminhos canonicalizados quando aplicavel.
- Execucao de processo nao usa shell.
- Logs nao contem tokens, cookies, chaves ou caminhos sensiveis desnecessarios.
- `tauri.conf.json` nao concede permissoes amplas sem necessidade.
- Falhas retornam mensagens seguras para a UI.

## Regras

- Nunca confiar em validacao apenas no frontend.
- Bloquear automacoes que burlem protecoes de plataforma.
- Separar comando de leitura, escrita, sync e lancamento.

## Saida esperada

```text
Comando/recurso:
Entradas:
Validacoes:
Permissoes:
Riscos:
Mitigacoes:
Testes:
```
