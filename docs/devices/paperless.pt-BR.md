# Paperless-ngx {#paperless-ngx}

Se o Paperless-ngx ainda importa o que recebe: a saúde de seu banco de dados,
Redis, workers Celery, índice de busca, classificador e verificação de
integridade, tarefas com falha, atualizações, espaço em disco e o número de
documentos.

O DumbMonit lê a API REST do Paperless-ngx com um **usuário dedicado que não
pode ler nenhum documento**: três permissões de visualização dão a ele a
lista de tarefas, o status do sistema e as contagens globais, nada mais.

A falha que vale a pena observar: **Redis ou Celery parados**. A interface web
abre e documentos podem ser lidos, mas nada do que foi colocado na pasta de
consumo ou obtido por e-mail é importado até alguém notar o acúmulo. O status
do sistema avisa isso; o DumbMonit alerta sobre ele.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_paperless_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de versão do Paperless-ngx conhece uma versão mais nova (ausente quando a verificação está desligada) | `latest_version` |
| `component_status` | 0 OK, 1 aviso, 2 erro, como a página de status do sistema reporta | `component`: `database`, `redis`, `celery`, `index`, `classifier`, `sanity_check`, `llm_index` (só quando ativado) |
| `unapplied_migrations` | migrações de banco de dados ainda não aplicadas | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | o disco que contém a mídia | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | todas as tarefas dos últimos 30 dias: com falha, total, e ainda pendentes | |
| `tasks_failed_recent` | tarefas com falha que este usuário pode ver, mais recentes que a janela de tarefas com falha e não descartadas | |
| `task_failed` | valor 1 por tal tarefa (no máximo 32) | `task`, `file` |
| `documents`, `documents_inbox` | documentos no Paperless, e na caixa de entrada | |
| `status_readable` | 0 quando a conta não tem a permissão de monitoramento do sistema | |
| `scrape_errors` | chamadas que falharam na última sondagem | |

O Paperless mantém as tarefas de cada proprietário privadas: o usuário
dedicado vê tarefas agendadas (verificação de integridade, índice,
classificador, correio) pelo nome, enquanto importações de outros usuários
são contadas em `tasks_failed_window`, que cobre todas as tarefas.

As [regras integradas](../alerting/rules.md#self-hosted-applications) que se
aplicam: Paperless task queue down, Paperless index or sanity check error,
Paperless migrations pending, Paperless task failed, Paperless storage almost
full, Paperless update available, além de Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca chama o Paperless. Ele diz primeiro o que está errado (um componente em
erro, tarefas com falha, migrações pendentes), depois atualizações, depois
documentos, caixa de entrada, tarefas pendentes e com falha e espaço em
disco, depois cada componente e as tarefas com falha pelo nome.

## Criar um usuário somente leitura no Paperless-ngx {#create-a-read-only-user-in-paperless-ngx}

1. Crie um usuário nomeado como abaixo, com uma senha longa e aleatória, sem
   status de superusuário ou staff, e apenas três permissões de visualização:
   tarefas, monitoramento do sistema e estatísticas globais. Com a API e sua
   própria conta, os dois comandos abaixo o criam e imprimem seu token.

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. Na interface web, em vez disso: Users & Groups → Add user, depois em suas
   permissões marque View apenas nas linhas PaperlessTask, SystemMonitoring e
   GlobalStatistics. Entre como esse usuário uma vez, abra My Profile e gere
   seu token de API.

3. Esse usuário não lê nenhum documento: as estatísticas globais dão as
   contagens, o monitoramento do sistema dá a saúde do banco de dados, Redis,
   Celery e o índice, e as tarefas mostram as com falha. No Paperless-ngx 2.x,
   onde as duas últimas permissões não existem, a saúde e as contagens são
   ignoradas e o resto é lido.

4. No DumbMonit, informe o endereço, por exemplo "paperless.lan" (porta 8000)
   ou "https://paperless.example.com" detrás de um proxy reverso, e cole o
   token.

!!! warning
    Quando Redis ou Celery para, o Paperless continua servindo suas páginas e
    documentos, mas nada novo é importado, da pasta de consumo ou do e-mail,
    até alguém olhar. O DumbMonit alerta sobre ambos.

Substitua `CHANGE-ME` por uma senha longa e aleatória antes de executar os
comandos, e `YOUR_USER` por uma conta com permissão para gerenciar usuários (o
curl pede sua senha). Pedir o status do sistema pode levar vários segundos
enquanto o Redis está fora do ar: o Paperless espera por ele antes de
responder.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de API (recomendado) | O token do usuário dedicado, enviado como `Authorization: Token …`. |
| Nome de usuário / senha | O próprio usuário dedicado, enviado como autenticação básica HTTP. |

Endereço: um nome de host (`paperless.lan`), `host:porta`, ou uma URL
completa com um prefixo de caminho; um `/api` final é aceito. Opções:
protocolo (HTTP por padrão), porta (8000), verificação de certificado, tempo
limite da requisição (15 s) e a janela de tarefas com falha (24 horas, de 1 a
720).

Validado contra o Paperless-ngx 3.2.1 (imagem oficial, SQLite, Valkey).
