---
name: safe-local-executable-launch
description: Use ao implementar lancamento de executaveis locais pelo Tauri, incluindo validacao de caminho, argumentos, diretorio de trabalho, erros seguros e testes.
---

# Safe Local Executable Launch

## Objetivo

Permitir que o app abra executaveis locais cadastrados pelo usuario sem usar shell e sem expor detalhes sensiveis desnecessarios.

## Regras de seguranca

- Nunca executar por `cmd.exe`, PowerShell ou shell generico.
- Usar API de processo direta, como `std::process::Command`.
- Aceitar apenas caminho absoluto.
- Validar que o caminho existe e aponta para arquivo.
- No Windows, aceitar inicialmente apenas extensao `.exe`.
- Rejeitar caminho vazio, relativo, diretorio, extensao nao suportada e arquivo inexistente.
- Nao tentar elevar privilegios.
- Nao registrar caminho completo em logs de erro, salvo quando o usuario ja estiver vendo esse caminho na UI como configuracao propria.
- Argumentos devem ser lista estruturada, nunca string unica interpolada.
- Diretorio de trabalho deve existir e ser diretorio; se ausente, usar o diretorio pai do executavel quando possivel.

## UX esperada

- Sucesso: mostrar que a inicializacao foi solicitada.
- Falha validavel: explicar em linguagem simples, como arquivo nao encontrado ou tipo nao suportado.
- Falha inesperada: mensagem generica e recuperavel.
- O app nao deve travar se o processo do jogo encerrar rapidamente.

## Contrato sugerido

```text
launch_local_executable(target, arguments?, working_directory?) -> LaunchResult
```

```text
LaunchResult
- started: boolean
- message: string
```

## Testes minimos

- Rejeita caminho vazio.
- Rejeita caminho relativo.
- Rejeita diretorio.
- Rejeita arquivo inexistente.
- Rejeita extensao nao `.exe`.
- Aceita caminho absoluto `.exe` existente.
- Resolve diretorio de trabalho padrao pelo pai do executavel.
- Nao usa shell.

## Decisoes adiadas

- Suporte a `.lnk`, `.bat`, `.cmd` ou launchers com argumentos complexos.
- Controle de processo em execucao.
- Captura de stdout/stderr.
- Permissoes por pasta ou lista de confianca.
