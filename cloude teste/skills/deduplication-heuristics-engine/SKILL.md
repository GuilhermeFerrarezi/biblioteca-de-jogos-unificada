---
name: deduplication-heuristics-engine
description: Use ao criar heuristicas para identificar duplicatas entre Steam, locais, manuais e outras plataformas.
---

# Deduplication Heuristics Engine

## Sinais fortes

- Mesmo ID externo na mesma plataforma.
- Mapeamento conhecido entre lojas.
- Mesmo executavel/caminho canonicalizado.

## Sinais medios

- Titulo normalizado igual.
- Titulo muito similar.
- Mesmo ano, genero e arte/metadados proximos.

## Regras

- Separar duplicata de jogo de multiplas instalacoes.
- Nunca mesclar automaticamente quando a confianca for baixa.
- Guardar fontes antigas ao mesclar.
- Permitir desfazer/editar quando houver interface para isso.

## Saida esperada

```text
Candidatos:
Sinais:
Confianca:
Acao:
Dados preservados:
Risco:
```
