---
name: ui-component-standardization
description: Use para padronizar componentes React, tokens de design, acessibilidade, densidade visual e comportamento de controles.
---

# UI Component Standardization

## Tokens minimos

- cores base, painel, borda, texto, texto secundario e destaque;
- raio de borda de controles;
- espacamentos de layout;
- estados de foco, hover, ativo e desabilitado.

## Componentes esperados

- botoes de icone;
- filtros/chips;
- cards de jogo;
- lista de jogo;
- painel de detalhes;
- modal;
- estados vazio/carregando/erro.

## Regras

- Controles selecionaveis usam `aria-pressed`.
- Processos em andamento usam `aria-busy` quando aplicavel.
- Texto longo nao deve estourar containers.
- Componentes nao chamam Tauri diretamente.
