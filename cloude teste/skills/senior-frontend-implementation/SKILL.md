---
name: senior-frontend-implementation
description: Use ao implementar frontend React/TypeScript, componentes, estado de UI, chamadas Tauri, telas de biblioteca, filtros, detalhes, cadastro manual, responsividade e estados visuais do aplicativo desktop.
---

# Senior Frontend Implementation

## Prioridade de UX

1. Abrir diretamente na biblioteca utilizavel.
2. Usar modo escuro como padrao.
3. Mostrar capas por padrao, com alternativa em lista.
4. Priorizar densidade, leitura rapida e controles previsiveis.
5. Evitar textos instrucionais longos dentro da UI.

## Estrutura recomendada

```text
src
├─ domain/       tipos compartilhados do frontend
├─ data/         mocks controlados enquanto backend nao existe
├─ api/          clients para comandos Tauri
├─ components/   layout, biblioteca, filtros, cards, detalhes
├─ features/     fluxos como biblioteca, contas, cadastro manual
└─ styles/       estilos compartilhados se o CSS crescer
```

## Regras de implementacao

- Manter componentes pequenos e nomeados pelo papel na UI.
- Evitar chamadas Tauri diretas dentro de componentes profundos; usar camada `api`.
- Tipar props com modelos de `src/domain`.
- Controlar busca, filtros e visualizacao por estado simples antes de adicionar bibliotecas externas.
- Usar `lucide-react` para icones de botoes e ferramentas.
- Garantir que textos longos quebrem ou sejam truncados sem estourar cards, botoes ou painels.
- Manter cards com raio de borda de ate 8px, salvo padrao local diferente.
- Nao criar landing page.

## Estados obrigatorios

- Carregando.
- Biblioteca vazia.
- Sincronizando.
- Erro de provider.
- Conta expirada.
- Jogo instalado.
- Jogo nao instalado.
- Sem internet.

## Fluxos principais

- Ver biblioteca.
- Buscar jogo.
- Filtrar por plataforma, instalado, genero e tag.
- Alternar capas/lista.
- Abrir detalhes.
- Lancar jogo.
- Adicionar jogo manualmente.
- Conectar/desconectar conta.
- Sincronizar provider.

## Verificacao minima

- Rodar `npm run build`.
- Rodar `npm run lint`.
- Conferir responsividade em largura desktop e janela estreita.
- Confirmar que estados vazios/erro nao quebram layout.
