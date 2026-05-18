---
name: react-performance-optimization
description: Use ao otimizar busca, filtros, listas grandes, renderizacao de componentes e estado React.
---

# React Performance Optimization

## Tecnicas

- `useDeferredValue` para busca digitada.
- `useMemo` para listas filtradas e metricas derivadas.
- `useCallback` para callbacks passados a componentes profundos quando houver rerender relevante.
- Subcomponentes pequenos para reduzir escopo de rerender.
- Virtualizacao ou paginacao quando a lista crescer.

## Sinais de alerta

- Componente com multiplas responsabilidades.
- Filtro recalculado em cada render sem necessidade.
- Prop drilling profundo.
- Estado global usado para estado local.

## Saida esperada

```text
Gargalo:
Evidencia:
Mudanca proposta:
Risco:
Validacao:
```
