---
name: senior-integration-quality
description: Use ao revisar qualidade, testes, integracao frontend-backend, Tauri, builds, regressao, acessibilidade basica, seguranca operacional e criterios de aceite do MVP.
---

# Senior Integration Quality

## Objetivo

Validar que cada mudanca deixa o aplicativo mais executavel, testavel e seguro, sem bloquear o MVP com processo pesado.

## Checklist por mudanca

- O build passa?
- O lint passa?
- A mudanca tem teste quando cobre regra critica?
- A UI ainda tem estados de carregamento, erro e vazio?
- O backend retorna erro normalizado?
- Algum log pode vazar token, cookie, chave API, SteamID privado ou caminho sensivel?
- A integracao usa API oficial ou risco experimental claramente marcado?
- O checkpoint precisa ser atualizado?

## Validacoes recomendadas

- Frontend: `npm run build` e `npm run lint`.
- Tauri: `npm run tauri:dev` somente quando Rust/Cargo e MSVC/Windows SDK estiverem disponiveis.
- Backend: `cargo fmt`, `cargo check` e `cargo test` quando Rust ou SQLite forem alterados.
- Banco: testar migration em banco vazio e upgrade de schema legado preservando dados do usuario.
- Providers: testar conta desconectada, biblioteca vazia, erro de rede, rate limit, jogo instalado e nao instalado.
- UI: testar largura desktop e estreita, textos longos, lista vazia e provider com erro.
- Producao: confirmar que o build nao depende de mocks como caminho principal.
- Resiliencia: confirmar que falha de provider nao impede listagem local ja persistida.

## Criterios de aceite do MVP

- O app abre na biblioteca.
- O usuario consegue ver jogos mockados ou persistidos.
- O usuario consegue adicionar jogo manual.
- O usuario consegue sincronizar Steam quando credenciais/configuracao estiverem presentes.
- O usuario consegue detectar ou cadastrar jogo local.
- O usuario consegue iniciar jogo por `steam://` ou executavel local.
- Erros de provider nao quebram a biblioteca inteira.

## Regras de revisao

- Priorizar bugs e riscos antes de resumo.
- Citar arquivo e linha quando possivel.
- Diferenciar bloqueio real de melhoria futura.
- Registrar risco de compliance quando a integracao depender de endpoint nao oficial.
