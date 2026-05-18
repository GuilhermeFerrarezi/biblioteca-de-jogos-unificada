---
name: game-metadata-normalization
description: Use ao unificar jogos, capas, IDs, generos, tempo de jogo, duplicatas e dados vindos de multiplas plataformas.
---

# Game Metadata Normalization

## Modelo canonico sugerido

```text
Game
+-- internalId
+-- title
+-- sortTitle
+-- platforms[]
+-- sources[]
+-- installed
+-- installLocations[]
+-- launchActions[]
+-- playtime
+-- achievementsSummary
+-- artwork
+-- genres[]
+-- tags[]
+-- userOverrides
```

## Regras de limpeza

- Normalizar titulo para comparacao removendo sufixos comuns como edicao, ano, launcher e marcadores de plataforma apenas na chave auxiliar.
- Preservar o titulo exibido ao usuario quando ele vier de fonte confiavel ou edicao manual.
- Gerar `sortTitle` estavel, sem artigos iniciais quando isso ajudar a ordenacao.
- Separar campos editaveis pelo usuario de metadados remotos para evitar sobrescrita acidental.

## Regras de merge

- Preservar IDs originais de cada loja.
- Nao apagar dados especificos de uma plataforma ao unificar duplicatas.
- Preferir edicoes manuais do usuario sobre metadados remotos.
- Separar "mesmo jogo" de "mesma instalacao".
- Permitir multiplas acoes de lancamento para o mesmo jogo.
- Nunca deduplicar automaticamente quando houver baixa confianca; marcar como candidato.

## Heuristicas de deduplicacao

- Alta confianca: mesmo ID externo em fonte confiavel ou mapeamento explicito entre plataformas.
- Media confianca: titulo normalizado muito proximo, mesmo ano aproximado e metadados coerentes.
- Baixa confianca: apenas titulo parecido, sem fonte cruzada ou com edicoes diferentes.
- Bloqueio: jogos diferentes com mesmo nome, remaster/remake/colecao, demos, betas e edicoes standalone.

## Hierarquia de fontes

1. Edicao manual do usuario.
2. Dados oficiais do provider.
3. Fonte externa especializada em metadados.
4. Seed/fallback de desenvolvimento.
5. Inferencia local por pasta/executavel.

## Fontes possiveis

- Dados da propria loja.
- IGDB.
- RAWG.
- SteamGridDB.
- Metadados manuais.
- Arquivos locais.

## Saida esperada

Ao trabalhar com metadados, declarar:

- fonte primaria;
- fonte secundaria;
- regra de conflito;
- campos editaveis pelo usuario;
- campos preservados por plataforma;
- nivel de confianca da deduplicacao;
- candidatos que exigem confirmacao manual.
