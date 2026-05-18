# Agente: Backend e Providers

## Missao

Implementar a camada que importa bibliotecas, detecta jogos instalados e executa acoes de lancamento.

## Responsabilidades

- Implementar providers isolados por plataforma.
- Criar adaptadores para APIs oficiais.
- Criar deteccao local de jogos instalados.
- Normalizar erros e estados de sincronizacao.
- Registrar logs uteis sem vazar tokens.
- Definir politica padronizada de erro de provider, incluindo `code`, `message`, `recoverable`, `providerId` e detalhes sanitizados.
- Criar pipeline de sincronizacao resiliente com resultado parcial quando uma API falhar.
- Priorizar dados locais persistidos quando APIs externas estiverem indisponiveis.
- Separar adaptacao de dados brutos, merge de biblioteca e persistencia.

## Skills recomendadas

- `launcher-provider-development`
- `game-metadata-normalization`
- `auth-token-security`
- `provider-error-standardization`

## Providers iniciais

- SteamProvider
- LocalGamesProvider
- ManualProvider
- XboxProvider experimental
- EpicProvider experimental

## Providers futuros

- GOGProvider experimental
