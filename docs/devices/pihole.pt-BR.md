# Pi-hole {#pi-hole}

O bloqueador de anúncios DNS: se está bloqueando, quantas consultas responde e
bloqueia, quão antigas são suas listas de bloqueio, se seus resolvedores
upstream falham, se uma atualização espera, e as mensagens de sua própria
página de diagnóstico.

O DumbMonit lê a API REST do Pi-hole v6 (`/api`). O Pi-hole 5 e anteriores não
têm essa API e não são suportados.

O que dá errado em um Pi-hole raramente aparece: o DNS continua respondendo.
O bloqueio é pausado "por cinco minutos" e nunca volta, a atualização semanal
das listas de bloqueio (gravity) falha há um mês, um resolvedor upstream
responde SERVFAIL, e a página de diagnóstico que ninguém abre tem listado o
motivo o tempo todo.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_pihole_`. O Pi-hole conta consultas
nas últimas 24 horas, uma janela móvel que sobe e desce: esses números são
gauges, com sufixo `_24h`, não contadores.

| Métrica | O que é | Rótulos |
|---|---|---|
| `blocking_enabled` | 1 quando o bloqueio está ativo; 0 quando está desativado, pausado ou falhou | |
| `blocking_timer_seconds` | tempo restante antes de um bloqueio pausado se reativar | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | consultas respondidas e bloqueadas nas últimas 24 horas | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | consultas enviadas ao upstream, respondidas pelo cache, domínios distintos | |
| `queries_per_second` | média do último minuto | |
| `replies_servfail_24h` | respostas SERVFAIL nas últimas 24 horas: um resolvedor upstream que falha aparece aqui primeiro | |
| `clients_active`, `clients_total` | clientes vistos | |
| `gravity_domains` | domínios nas listas de bloqueio | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | última reconstrução da lista de bloqueio, e há quanto tempo | |
| `version_info` | valor 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 1 quando uma versão mais nova de um componente é publicada, 0 caso contrário; ausente até a verificação diária do Pi-hole ter rodado | `component` (`core`, `web`, `ftl`, `docker`) |
| `updates_available` | atualizações a instalar: a imagem no Docker (core, web e FTL vêm com ela), senão cada componente desatualizado | |
| `messages`, `messages_by_type` | mensagens na página de diagnóstico | `type` (`LOAD`, `RATE_LIMIT`, `LIST`…) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | o daemon FTL | |
| `privacy_level` | nível de privacidade do Pi-hole, 0 a 3 | |
| `scrape_errors` | chamadas secundárias da API que falharam durante a última verificação | |

Em um branch de desenvolvimento, o Pi-hole compara commits em vez de números
de versão, e o DumbMonit também.

As [regras integradas](../alerting/rules.md#pi-hole) que se aplicam: Pi-hole
blocking disabled, Pi-hole blocklists stale, Pi-hole update available, Pi-hole
diagnosis messages, além de Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação armazenou; abrir a página
nunca se conecta ao Pi-hole. Ele mostra se o bloqueio está ativo (e quando uma
pausa termina), as consultas e a parcela bloqueada nas últimas 24 horas, o
tamanho e a idade da lista de bloqueio, os clientes, as versões com qualquer
atualização esperando, e o número de mensagens de diagnóstico.

## Criar uma senha de app no Pi-hole {#create-an-app-password-in-pi-hole}

1. O Pi-hole v6 ou posterior é necessário: a versão 5 não tinha API REST. O
   Pi-hole não tem conta somente leitura; o mais próximo é uma senha de app,
   que não pode alterar nenhuma configuração.

2. Na interface web do Pi-hole, abra Settings → Web interface / API e mude a
   página de Basic para Expert. Clique em Configure app password, copie a
   senha mostrada, depois clique em Enable new app password. O Pi-hole a
   mostra apenas uma vez, e uma nova substitui a anterior.

3. Uma senha de app permanece somente leitura para configurações enquanto
   webserver.api.app_sudo for false, o padrão. Verifique na máquina do
   Pi-hole (no Docker, prefixe o comando com docker exec e o nome do
   contêiner):

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. No DumbMonit, informe o endereço do Pi-hole, por exemplo "pi.hole" ou
   "192.168.1.53", e cole a senha de app. Para um Pi-hole cuja interface web
   não tem senha, escolha Sem senha em vez disso.

5. O DumbMonit só lê. Ele nunca pausa o bloqueio e nunca altera uma lista, e
   mantém uma sessão de API aberta entre verificações em vez de abrir uma por
   verificação: o Pi-hole permite 16 sessões por vez.

!!! warning
    A senha de app ainda permite pausar o bloqueio e editar listas através da
    API: mantenha-a só para o DumbMonit. O Pi-hole serve HTTP simples na
    porta 80, e a senha cruza a rede a cada abertura de sessão: em uma rede
    não confiável, use HTTPS na porta 443.

Uma senha de app também faz login sem o segundo fator quando a autenticação
de dois fatores está ativa: é para isso que ela serve. Para revogá-la, limpe
`webserver.api.app_pwhash` em Settings → All settings, ou ative uma nova: a
anterior para de funcionar imediatamente.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Senha de app | A senha gerada acima. O DumbMonit a troca por uma sessão, enviada no cabeçalho `X-FTL-SID`, e reutiliza essa sessão enquanto o Pi-hole a mantém (30 minutos após o último uso). |
| Sem senha | Apenas para um Pi-hole cuja interface web não tem senha. |

Endereço: um nome de host ou IP (`pi.hole`), `host:porta`, ou uma URL
completa (`https://pi.hole`); o caminho da interface web ou da API, copiado
do navegador, é ignorado. Opções: protocolo (HTTP por padrão), porta (80),
verificação de certificado (para HTTPS em 443, o próprio certificado do
Pi-hole é autoassinado) e tempo limite da requisição (10 s).

Se as verificações falharem com "no free API session", todas as sessões do
Pi-hole estão ocupadas, geralmente por um script que faz login a cada
execução sem fazer logout. Elas expiram após 30 minutos; `webserver.api.max_sessions`
aumenta o limite.
