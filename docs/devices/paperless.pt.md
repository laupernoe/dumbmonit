# Paperless-ngx {#paperless-ngx}

Se o Paperless-ngx continua a importar o que lhe é enviado: a saúde da sua
base de dados, Redis, trabalhadores Celery, índice de pesquisa, classificador
e verificação de integridade, tarefas falhadas, atualizações, espaço em disco
e o número de documentos.

O DumbMonit lê a API REST do Paperless-ngx com um **utilizador dedicado que
não pode ler nenhum documento**: três permissões de visualização dão-lhe a
lista de tarefas, o estado do sistema e as contagens globais, nada mais.

A falha a vigiar: **Redis ou Celery parados**. A interface web abre e os
documentos podem ser lidos, mas nada largado na pasta de consumo ou obtido do
email é importado até alguém reparar na pilha. O estado do sistema diz-o; o
DumbMonit alerta sobre isso.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_paperless_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de versão do Paperless-ngx conhece uma versão mais recente (ausente quando a verificação está desligada) | `latest_version` |
| `component_status` | 0 OK, 1 aviso, 2 erro, como a página de estado do sistema o reporta | `component`: `database`, `redis`, `celery`, `index`, `classifier`, `sanity_check`, `llm_index` (apenas quando ativado) |
| `unapplied_migrations` | migrações da base de dados ainda não aplicadas | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | o disco que guarda os media | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | todas as tarefas dos últimos 30 dias: falhadas, total, e ainda pendentes | |
| `tasks_failed_recent` | tarefas falhadas que este utilizador pode ver, mais recentes do que a janela de tarefas falhadas e não dispensadas | |
| `task_failed` | valor 1 por tal tarefa (no máximo 32) | `task`, `file` |
| `documents`, `documents_inbox` | documentos no Paperless, e na caixa de entrada | |
| `status_readable` | 0 quando a conta não tem a permissão de monitorização do sistema | |
| `scrape_errors` | chamadas que falharam na última verificação | |

O Paperless mantém as tarefas de cada proprietário privadas: o utilizador
dedicado vê as tarefas agendadas (verificação de integridade, índice,
classificador, email) pelo nome, enquanto as importações de outros
utilizadores são contadas em `tasks_failed_window`, que cobre todas as
tarefas.

As [regras incorporadas](../alerting/rules.md#self-hosted-applications) que se
aplicam: Paperless task queue down, Paperless index or sanity check error,
Paperless migrations pending, Paperless task failed, Paperless storage almost
full, Paperless update available, mais Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca chama o Paperless. Diz primeiro o que está mal (um componente em erro,
tarefas falhadas, migrações pendentes), depois atualizações, depois
documentos, caixa de entrada, tarefas pendentes e falhadas e espaço em disco,
depois cada componente e as tarefas falhadas pelo nome.

## Criar um utilizador só de leitura no Paperless-ngx {#create-a-read-only-user-in-paperless-ngx}

1. Crie um utilizador com o seguinte nome, com uma senha longa e aleatória, sem estado de superutilizador ou staff, e apenas três permissões de visualização: tarefas, monitorização do sistema e estatísticas globais. Com a API e a sua própria conta, os dois comandos abaixo criam-no e imprimem o seu token.

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. Na interface web em vez disso: Users & Groups → Add user, depois nas suas permissões marque View apenas nas linhas PaperlessTask, SystemMonitoring e GlobalStatistics. Inicie sessão como esse utilizador uma vez, abra My Profile e gere o seu token de API.

3. Este utilizador não lê nenhum documento: as estatísticas globais dão as contagens, a monitorização do sistema dá a saúde da base de dados, Redis, Celery e o índice, e as tarefas mostram as falhadas. No Paperless-ngx 2.x, onde as duas últimas permissões não existem, a saúde e as contagens são ignoradas e o resto é lido.

4. No DumbMonit, introduza o endereço, por exemplo «paperless.lan» (porta 8000) ou «https://paperless.example.com» atrás de um proxy inverso, e cole o token.

!!! warning
    Quando o Redis ou o Celery para, o Paperless continua a servir as suas páginas e documentos, mas nada de novo é importado, da pasta de consumo ou do email, até alguém reparar. O DumbMonit alerta sobre ambos.

Substitua `CHANGE-ME` por uma senha longa e aleatória antes de executar os
comandos, e `YOUR_USER` por uma conta autorizada a gerir utilizadores (o curl
pede a sua senha). Pedir o estado do sistema pode demorar vários segundos
enquanto o Redis está em baixo: o Paperless espera por ele antes de
responder.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de API (recomendado) | O token do utilizador dedicado, enviado como `Authorization: Token …`. |
| Nome de utilizador / senha | O próprio utilizador dedicado, enviado como autenticação HTTP básica. |

Endereço: um nome de anfitrião (`paperless.lan`), `anfitrião:porta`, ou um URL
completo com um prefixo de caminho; um `/api` final é aceite. Opções:
protocolo (HTTP por predefinição), porta (8000), verificação de certificado,
tempo limite do pedido (15 s) e a janela de tarefas falhadas (24 horas, de 1 a
720).

Validado com o Paperless-ngx 3.2.1 (imagem oficial, SQLite, Valkey).
