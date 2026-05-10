---
name: api-compliance-review
description: Use ao revisar termos de uso, privacidade, limites de API, endpoints nao documentados e riscos legais de integracoes com plataformas de jogos.
---

# API Compliance Review

## Workflow

1. Localize termos oficiais da API ou plataforma.
2. Verifique se o uso pretendido e permitido.
3. Separe dados publicos, dados autenticados e dados sensiveis.
4. Identifique restricoes de cache, redistribuicao e exibicao.
5. Verifique regras de marca, nome e logos.
6. Registre riscos e alternativas.

## Classificacao de risco

- Baixo: API publica documentada cobre o caso.
- Medio: API documentada cobre parcialmente ou exige consentimento especifico.
- Alto: depende de endpoint interno, scraping, automacao de launcher ou comportamento nao garantido.
- Bloqueado: termos proibem o uso ou exigem credenciais/senhas do usuario.

## Saida esperada

```text
Plataforma:
Uso pretendido:
Permissao aparente:
Risco:
Condicoes:
Dados que podem ser armazenados:
Dados que nao devem ser armazenados:
Fontes:
Recomendacao:
```

## Regras

- Nao dar parecer juridico definitivo.
- Usar fontes oficiais quando possivel.
- Se houver incerteza material, marcar como risco e propor alternativa tecnica.

