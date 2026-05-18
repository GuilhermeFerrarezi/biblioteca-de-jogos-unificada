---
name: senior-frontend-implementation
description: Use ao implementar frontend React/JavaScript, componentes, estado de UI, chamadas Tauri, telas de biblioteca, filtros, detalhes, cadastro manual, responsividade e estados visuais do aplicativo desktop.
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
+-- adapters/     normalizacao entre UI, backend e providers
+-- components/   layout, biblioteca, filtros, cards, detalhes
+-- constants/    constantes de dominio e UI
+-- data/         mocks controlados para fallback de desenvolvimento
+-- hooks/        estado e fluxos de tela
+-- pages/        telas principais
+-- services/     chamadas Tauri e fallback web
+-- styles/       CSS compartilhado e tokens visuais
```

## Regras de implementacao

- Manter componentes pequenos e nomeados pelo papel na UI.
- Evitar chamadas Tauri diretas dentro de componentes; usar `services/libraryService.js`.
- Centralizar constantes em `src/constants` quando usadas por mais de um modulo.
- Controlar busca, filtros e visualizacao por estado simples antes de adicionar bibliotecas externas.
- Usar `lucide-react` para icones de botoes e ferramentas.
- Garantir que textos longos quebrem ou sejam truncados sem estourar cards, botoes ou painels.
- Manter cards com raio de borda de ate 8px, salvo padrao local diferente.
- Nao criar landing page.

## Acessibilidade e performance

- Usar `aria-pressed`, `aria-busy`, labels e foco previsivel em controles interativos.
- Modais devem fechar com Escape e restaurar estado sem corromper formularios.
- Listas grandes devem considerar paginacao, virtualizacao ou filtro backend antes de renderizar tudo.
- Evitar recriar listas derivadas caras sem necessidade; memoizar apenas quando houver ganho claro.
- Mocks devem ser carregados apenas como fallback de desenvolvimento, preferencialmente via import dinamico.

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
- Adicionar e editar jogo manualmente.
- Arquivar e reativar entrada.
- Conectar/desconectar conta.
- Sincronizar provider.

## Verificacao minima

- Rodar `npm run build`.
- Rodar `npm run lint`.
- Conferir responsividade em largura desktop e janela estreita.
- Confirmar que estados vazios/erro nao quebram layout.
- Confirmar que build de producao nao carrega mocks como caminho principal.
