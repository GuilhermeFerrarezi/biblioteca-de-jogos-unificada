---
name: architecture-extensibility-blueprint
description: Use ao definir contratos de providers, plugins futuros, DTOs e boundaries para adicionar plataformas sem refatorar o core.
---

# Architecture Extensibility Blueprint

## Contratos obrigatorios

- Provider isolado por plataforma.
- Adapter de normalizacao para o contrato interno.
- Service para orquestrar sync/merge.
- Repository para persistencia quando houver SQLite.
- DTO estavel para comandos Tauri.

## Regras

- O core nao conhece payload bruto de plataforma.
- Provider falho retorna erro recuperavel e nao quebra a biblioteca.
- Dados originais ficam preservados em `game_sources` ou estrutura equivalente.
- Integracoes experimentais devem ser marcadas como tal.

## Saida esperada

```text
Modulo:
Contrato publico:
Entradas:
Saidas:
Erros:
Persistencia:
Dependencias:
Compatibilidade futura:
```
