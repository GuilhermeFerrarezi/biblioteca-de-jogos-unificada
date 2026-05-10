---
name: senior-backend-implementation
description: Use ao implementar backend, dominio, persistencia local, providers, comandos Tauri, services, repositories, testes backend e integracoes de launcher no aplicativo de biblioteca unificada de jogos.
---

# Senior Backend Implementation

## Prioridade tecnica

1. Manter o core independente de detalhes de plataforma.
2. Preservar dados originais por provider.
3. Isolar providers em adapters pequenos.
4. Tratar erro parcial sem quebrar a biblioteca inteira.
5. Nunca expor segredo ao frontend sem necessidade.

## Camadas recomendadas

```text
src-tauri/src
├─ domain/      contratos Rust equivalentes aos modelos TypeScript
├─ storage/     SQLite, migrations e repositories
├─ providers/   Steam, Local, Manual, futuros Xbox/Epic
├─ services/    merge, sync, launch, metadata
├─ commands/    comandos Tauri chamados pelo frontend
└─ security/    vault, sanitizacao de logs, redacao de segredos
```

Enquanto o backend Rust ainda nao estiver pronto, manter contratos TypeScript em `src/domain` alinhados com essa divisao.

## Regras de implementacao

- Preferir tipos explicitos a objetos soltos.
- Separar `Game` canonico de `LibraryEntry` e de dados brutos de provider.
- Guardar IDs externos em `sources[]`, nunca substituir o ID interno por ID de loja.
- Expor comandos de alto nivel para a UI: `list_library`, `sync_provider`, `add_manual_game`, `launch_game`.
- Normalizar erros em formato estavel com `code`, `message` e `recoverable`.
- Fazer providers retornarem resultado parcial quando possivel.
- Redigir tokens, cookies, chaves API e caminhos sensiveis em logs.

## Persistencia local

- Usar SQLite para biblioteca, contas, fontes externas, acoes de lancamento e historico de sync.
- Criar migrations versionadas quando a camada nativa estiver ativa.
- Manter segredos fora das tabelas comuns; usar vault do sistema operacional quando possivel.
- Separar dados importados de edicoes manuais do usuario.

## Providers iniciais

- SteamProvider: prioridade principal, preferir Web API oficial e respeitar privacidade do usuario.
- LocalGamesProvider: detectar executaveis e caminhos locais sem varrer diretorios sensiveis desnecessariamente.
- ManualProvider: permitir cadastro manual sem depender de conta externa.
- Xbox/Epic: marcar experimental ate haver decisao de API/compliance.

## Verificacao minima

- Rodar build/lint do frontend quando contratos TypeScript forem alterados.
- Rodar testes backend assim que houver runner configurado.
- Testar casos: biblioteca vazia, erro de provider, jogo instalado, jogo nao instalado, conta desconectada, token expirado.
