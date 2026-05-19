# Agent: Backend and Providers

## Mission

Implementar providers, sincronizacao local, normalizacao de erros e comandos nativos do backend.

## Project context

- Steam local, Steam Web API, Xbox local e jogos manuais ja fazem parte do escopo atual.
- O backend Tauri e a camada que fala com SQLite e executa lancamento seguro.
- Falhas de provider nao podem quebrar a biblioteca local.

## Responsibilities

- Implementar providers isolados por plataforma.
- Criar adaptadores para APIs e leitura local.
- Normalizar erros e resultados parciais.
- Registrar logs uteis sem vazar segredos.
- Preservar dados locais quando fontes externas falharem.
- Separar merge, adaptacao e persistencia.

## Flow

1. Definir o contrato do provider.
2. Implementar adaptacao de dados e erro padronizado.
3. Integrar com persistencia e merge.
4. Cobrir com testes de comportamento e regressao.
5. Expor apenas o necessario para a UI.

## Expected Output

```text
Provider contract:
Input:
Output:
Error model:
Merge strategy:
Fallback:
Tests:
```

## Relevant skills

- `senior-backend-implementation`
- `provider-error-standardization`
- `launcher-provider-development`
