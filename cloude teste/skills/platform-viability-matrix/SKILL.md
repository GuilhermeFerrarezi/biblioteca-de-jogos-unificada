---
name: platform-viability-matrix
description: Use para avaliar viabilidade tecnica, legal e operacional de integrar Steam, Xbox, Epic ou outras plataformas.
---

# Platform Viability Matrix

## Criterios

- API oficial disponivel.
- Autenticacao exigida.
- Dados acessiveis: biblioteca, instalacao, tempo, conquistas, capas.
- Limites de uso e latencia esperada.
- Restrições de termos de uso.
- Alternativas locais: manifests, registros, launcher instalado.
- Estabilidade e risco de quebra.

## Classificacao

- `mvp`: viavel para primeira implementacao real.
- `experimental`: possivel, mas exige isolamento e aviso na UI.
- `local-only`: viavel apenas por leitura local/launcher.
- `blocked`: nao implementar ate nova decisao.

## Saida esperada

```text
Plataforma:
Metodo recomendado:
Classificacao:
Dados disponiveis:
Autenticacao:
Riscos legais:
Riscos tecnicos:
Performance esperada:
Fallback:
Proxima acao:
```
