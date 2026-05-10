---
name: launcher-provider-development
description: Use ao projetar ou implementar providers de biblioteca e lancamento para Steam, Xbox/Game Pass, Epic, jogos locais e outros launchers.
---

# Launcher Provider Development

## Objetivo

Criar providers pequenos, testaveis e isolados para cada plataforma.

## Prioridade atual do projeto

1. SteamProvider
2. XboxProvider
3. EpicProvider
4. LocalGamesProvider e ManualProvider como suporte ao MVP
5. Outros providers somente depois dessas prioridades

## Contrato minimo de provider

Um provider deve responder:

```text
id
displayName
authStatus
syncLibrary()
detectInstalledGames()
getLaunchActions(game)
launch(game, action)
refreshMetadata(game)
disconnect()
```

## Estados comuns

- `connected`
- `disconnected`
- `expired`
- `syncing`
- `rate_limited`
- `unsupported`
- `needs_user_action`

## Regras

- O core nao deve conhecer detalhes internos da plataforma.
- Falhas de um provider nao podem quebrar a biblioteca inteira.
- Logs nunca devem conter tokens, cookies, codigos OAuth ou caminhos sensiveis desnecessarios.
- Provider experimental deve ser marcado como experimental no modelo e na UI.
- Lancar um jogo deve preferir mecanismo oficial/local ja instalado antes de tentar alternativa fragil.

## Testes minimos

- Conta desconectada.
- Token expirado.
- Biblioteca vazia.
- Jogo instalado.
- Jogo nao instalado.
- Erro de rede.
- Plataforma indisponivel.
