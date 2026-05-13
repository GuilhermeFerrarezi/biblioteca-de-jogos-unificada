---
name: metadata-fallback-logic
description: Use ao definir prioridade entre metadados de loja, provider externo, arquivos locais e edicoes manuais.
---

# Metadata Fallback Logic

## Hierarquia padrao

1. Edicao manual do usuario.
2. Provider da plataforma principal.
3. Fonte de metadados confiavel configurada.
4. Dados locais inferidos.
5. Fallback visual/textual seguro.

## Regras

- Nunca sobrescrever override manual sem confirmacao.
- Registrar fonte de cada campo relevante.
- Preservar IDs externos por plataforma.
- Campos ausentes devem ter fallback para nao quebrar UI.

## Saida esperada

```text
Campo:
Fonte primaria:
Fonte secundaria:
Regra de conflito:
Fallback:
Editavel pelo usuario:
```
