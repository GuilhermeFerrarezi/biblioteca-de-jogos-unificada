---
name: game-metadata-normalization
description: Use ao unificar jogos, capas, IDs, generos, tempo de jogo, duplicatas e dados vindos de multiplas plataformas.
---

# Game Metadata Normalization

## Modelo canonico sugerido

```text
Game
├─ internalId
├─ title
├─ sortTitle
├─ platforms[]
├─ sources[]
├─ installed
├─ installLocations[]
├─ launchActions[]
├─ playtime
├─ achievementsSummary
├─ artwork
├─ genres[]
├─ tags[]
└─ userOverrides
```

## Regras de merge

- Preservar IDs originais de cada loja.
- Nao apagar dados especificos de uma plataforma ao unificar duplicatas.
- Preferir edicoes manuais do usuario sobre metadados remotos.
- Separar "mesmo jogo" de "mesma instalacao".
- Permitir multiplas acoes de lancamento para o mesmo jogo.

## Fontes possiveis

- Dados da propria loja.
- IGDB.
- RAWG.
- SteamGridDB.
- Metadados manuais.
- Arquivos locais.

## Saida esperada

Ao trabalhar com metadados, declarar:

- fonte primaria
- fonte secundaria
- regra de conflito
- campos editaveis pelo usuario
- campos preservados por plataforma

