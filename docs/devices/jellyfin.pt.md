# Jellyfin {#jellyfin}

Se o Jellyfin mantém as suas bibliotecas atualizadas e os seus espetadores
servidos: tarefas agendadas que falharam na última execução (a análise da
biblioteca em primeiro lugar), plugins que já não carregam, um reinício em
espera, e o que está a ser reproduzido agora, transcodificado ou não.

O DumbMonit lê a API do Jellyfin com uma **chave de API** criada no painel.
Nunca inicia uma tarefa, nunca para uma reprodução e nunca altera uma
definição.

A falha a vigiar: **a análise da biblioteca que falha todas as noites**. Uma
partilha de rede desmontada, um disco cheio, e os novos filmes e episódios
deixam de aparecer; o próprio Jellyfin continua a reproduzir o que já conhece,
e o único rasto é uma linha em Scheduled Tasks.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_jellyfin_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `pending_restart` | 1 quando o Jellyfin espera por um reinício (após a instalação ou atualização de um plugin) | |
| `shutting_down` | 1 enquanto o Jellyfin encerra | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | tarefas cuja última execução falhou ou foi interrompida; tarefas a correr agora | |
| `scheduled_task_failed` | valor 1 por tarefa falhada (no máximo 32) | `task`, `error` (primeira linha da mensagem) |
| `library_scan_last_ok`, `library_scan_age_seconds` | última execução de «Scan Media Library»: se teve sucesso ou não, e há quanto tempo terminou | |
| `plugins`, `plugins_broken` | plugins instalados; os que falharam ao carregar ou não suportam esta versão | |
| `plugin_broken` | valor 1 por plugin avariado | `plugin`, `status` |
| `sessions`, `streams`, `streams_paused`, `transcodes` | sessões de cliente; as que estão a reproduzir algo, em pausa, e a transcodificar | |
| `scrape_errors` | chamadas que falharam na última verificação | |

O Jellyfin não verifica as suas próprias atualizações (o pacote ou a imagem
fazem-no), pelo que não há métrica de atualização.

As [regras incorporadas](../alerting/rules.md#self-hosted-applications) que se
aplicam: Jellyfin scheduled task failed, Jellyfin plugin broken, Jellyfin
restart pending, mais Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca chama o Jellyfin. Diz primeiro o que está mal (tarefas falhadas, plugins
avariados, um reinício em espera), depois streams, transcodificações, sessões
e a idade da última análise da biblioteca, depois as tarefas falhadas e os
plugins avariados pelo nome com o próprio erro do Jellyfin.

## Criar uma chave de API no Jellyfin {#create-an-api-key-in-jellyfin}

1. No Jellyfin: Dashboard → API Keys (em Advanced) → +, e dê o seguinte nome à chave. Copie-a.

    ```
    DumbMonit
    ```

2. O Jellyfin não consegue limitar o que uma chave pode fazer: cada chave tem direitos totais sobre o servidor. Ainda assim é a única forma de ler as tarefas agendadas e a reprodução de todos; um utilizador comum não vê nenhuma das duas.

3. No DumbMonit, introduza o endereço do Jellyfin, por exemplo «jellyfin.lan» (porta 8096) ou «https://media.example.com» atrás de um proxy inverso, e cole a chave.

4. O DumbMonit só lê /System/Info, /ScheduledTasks, /Sessions e /Plugins. Nunca inicia uma tarefa, nunca para uma reprodução e nunca altera uma definição.

!!! warning
    Como a chave tem todos os direitos, mantenha-a numa rede em que confia e coloque o Jellyfin atrás de HTTPS na internet. Se alguma vez for exposta, revogue-a na mesma página: nada mais a usa.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API | Enviada no cabeçalho `Authorization: MediaBrowser … Token="…"`, a forma que o Jellyfin 10.11 e posteriores aceitam por predefinição. |

Endereço: um nome de anfitrião (`jellyfin.lan`), `anfitrião:porta`, ou um URL
completo com um prefixo de caminho (`https://example.com/jellyfin`); um `/web`
final é aceite.
Opções: protocolo (HTTP por predefinição), porta (8096), verificação de
certificado e tempo limite do pedido (15 s).

Validado com o Jellyfin 12.1.0 (imagem oficial).
