# Jellyfin {#jellyfin}

Se o Jellyfin mantém suas bibliotecas atualizadas e seus espectadores
atendidos: tarefas agendadas que falharam na última execução (a varredura de
biblioteca em primeiro lugar), plugins que não carregam mais, um reinício
pendente, e o que está sendo reproduzido agora, transcodificado ou não.

O DumbMonit lê a API do Jellyfin com uma **chave de API** criada no painel. Ele
nunca inicia uma tarefa, nunca para uma reprodução e nunca altera uma
configuração.

A falha que vale a pena observar: **a varredura de biblioteca que falha toda
noite**. Um compartilhamento de rede desmontado, um disco cheio, e novos
filmes e episódios deixam de aparecer; o próprio Jellyfin continua reproduzindo
o que já conhece, e o único vestígio é uma linha em Tarefas Agendadas.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_jellyfin_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `pending_restart` | 1 quando o Jellyfin espera um reinício (após instalar ou atualizar um plugin) | |
| `shutting_down` | 1 enquanto o Jellyfin encerra | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | tarefas cuja última execução falhou ou foi abortada; tarefas em execução agora | |
| `scheduled_task_failed` | valor 1 por tarefa com falha (no máximo 32) | `task`, `error` (primeira linha da mensagem) |
| `library_scan_last_ok`, `library_scan_age_seconds` | última execução de "Scan Media Library": se teve sucesso ou não, e há quanto tempo terminou | |
| `plugins`, `plugins_broken` | plugins instalados; os que falharam ao carregar ou não suportam esta versão | |
| `plugin_broken` | valor 1 por plugin com falha | `plugin`, `status` |
| `sessions`, `streams`, `streams_paused`, `transcodes` | sessões de clientes; as reproduzindo algo, pausadas, e transcodificando | |
| `scrape_errors` | chamadas que falharam na última sondagem | |

O Jellyfin não verifica suas próprias atualizações (o pacote ou a imagem o
fazem), então não há métrica de atualização.

As [regras integradas](../alerting/rules.md#self-hosted-applications) que se
aplicam: Jellyfin scheduled task failed, Jellyfin plugin broken, Jellyfin
restart pending, além de Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca chama o Jellyfin. Ele diz primeiro o que está errado (tarefas com falha,
plugins quebrados, um reinício pendente), depois streams, transcodificações,
sessões e a idade da última varredura de biblioteca, depois as tarefas com
falha e os plugins quebrados pelo nome com o erro do próprio Jellyfin.

## Criar uma chave de API no Jellyfin {#create-an-api-key-in-jellyfin}

1. No Jellyfin: Dashboard → API Keys (em Advanced) → +, e nomeie a chave da
   seguinte forma. Copie-a.

    ```
    DumbMonit
    ```

2. O Jellyfin não consegue limitar o que uma chave pode fazer: toda chave tem
   direitos totais no servidor. Ainda assim é a única forma de ler tarefas
   agendadas e a reprodução de todos; um usuário comum não vê nenhuma das duas.

3. No DumbMonit, informe o endereço do Jellyfin, por exemplo "jellyfin.lan"
   (porta 8096) ou "https://media.example.com" detrás de um proxy reverso, e
   cole a chave.

4. O DumbMonit só lê /System/Info, /ScheduledTasks, /Sessions e /Plugins. Ele
   nunca inicia uma tarefa, nunca para uma reprodução e nunca altera uma
   configuração.

!!! warning
    Como a chave carrega todos os direitos, mantenha-a em uma rede de
    confiança e coloque o Jellyfin detrás de HTTPS pela internet. Se ela
    algum dia for exposta, revogue-a na mesma página: nada mais a utiliza.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API | Enviada no cabeçalho `Authorization: MediaBrowser … Token="…"`, a forma que o Jellyfin 10.11 e posteriores aceitam por padrão. |

Endereço: um nome de host (`jellyfin.lan`), `host:porta`, ou uma URL completa
com um prefixo de caminho (`https://example.com/jellyfin`); um `/web` final é
aceito. Opções: protocolo (HTTP por padrão), porta (8096), verificação de
certificado e tempo limite da requisição (15 s).

Validado contra o Jellyfin 12.1.0 (imagem oficial).
