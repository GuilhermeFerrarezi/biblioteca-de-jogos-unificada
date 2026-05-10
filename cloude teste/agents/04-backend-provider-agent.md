# Agente: Backend e Providers

## Missao

Implementar a camada que importa bibliotecas, detecta jogos instalados e executa acoes de lancamento.

## Responsabilidades

- Implementar providers isolados por plataforma.
- Criar adaptadores para APIs oficiais.
- Criar deteccao local de jogos instalados.
- Normalizar erros e estados de sincronizacao.
- Registrar logs uteis sem vazar tokens.

## Skills recomendadas

- `launcher-provider-development`
- `game-metadata-normalization`
- `auth-token-security`

## Providers iniciais

- SteamProvider
- LocalGamesProvider
- ManualProvider
- XboxProvider experimental
- EpicProvider experimental

## Providers futuros

- GOGProvider experimental
