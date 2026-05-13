---
name: platform-integration-research
description: Use ao pesquisar integracoes com plataformas de jogos, APIs oficiais, launchers, bibliotecas de usuario, projetos open source e viabilidade tecnica por loja.
---

# Platform Integration Research

## Workflow

1. Comece por documentacao oficial da plataforma.
2. Verifique se existe API publica para biblioteca do usuario.
3. Classifique o metodo de integracao:
   - oficial
   - oficial com limitacoes
   - local via launcher instalado
   - comunitario/open source
   - endpoint nao documentado
   - nao recomendado
4. Identifique autenticacao, permissoes, dados retornados, limites e termos.
5. Procure projetos open source relevantes apenas para entender arquitetura e riscos.
6. Registre a data da pesquisa e links das fontes.

## Prioridade atual

Pesquise primeiro Steam. Depois Xbox/Game Pass. Depois Epic Games. Outras plataformas devem ser tratadas como expansoes futuras, a menos que sejam necessarias para desbloquear uma decisao do MVP.

## Saida esperada

Para cada plataforma, produza:

```text
Plataforma:
Status:
Metodo recomendado:
Classificacao de viabilidade:
Dados disponiveis:
Autenticacao:
Limites/performance:
Restricoes legais:
Riscos:
Fontes:
Proxima acao:
```

## Regras

- Nao assumir que uma API antiga ainda funciona.
- Nao tratar endpoint nao documentado como contrato estavel.
- Separar importacao de biblioteca, instalacao, atualizacao, lancamento, conquistas e tempo de jogo.
- Marcar claramente qualquer inferencia.
- Diferenciar viabilidade tecnica de permissao/compliance.
- Registrar quando a integracao depender de manifesto local, launcher instalado, API publica, scraping ou projeto comunitario.
