# Pi-hole {#pi-hole}

O bloqueador de anúncios DNS: se está a bloquear, quantas consultas responde
e bloqueia, quão antigas são as suas listas de bloqueio, se os seus
resolvedores a montante falham, se uma atualização está em espera, e as
mensagens da sua própria página de diagnóstico.

O DumbMonit lê a API REST do Pi-hole v6 (`/api`). O Pi-hole 5 e anteriores
não têm essa API e não são suportados.

O que corre mal num Pi-hole raramente se vê: o DNS continua a responder. O
bloqueio é pausado «durante cinco minutos» e nunca volta, a atualização
semanal da lista de bloqueio (gravity) falha há um mês, um resolvedor a
montante responde SERVFAIL, e a página de diagnóstico que ninguém abre tem
estado a listar o motivo o tempo todo.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_pihole_`. O Pi-hole conta
consultas nas últimas 24 horas, uma janela deslizante que sobe e desce: esses
valores são medidores, com o sufixo `_24h`, não contadores.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `blocking_enabled` | 1 quando o bloqueio está ativo; 0 quando está desligado, pausado ou falhou | |
| `blocking_timer_seconds` | tempo restante antes de um bloqueio pausado voltar a ligar-se | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | consultas respondidas e bloqueadas nas últimas 24 horas | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | consultas enviadas a montante, respondidas da cache, domínios distintos | |
| `queries_per_second` | média no último minuto | |
| `replies_servfail_24h` | respostas SERVFAIL nas últimas 24 horas: um resolvedor a montante que falha aparece aqui primeiro | |
| `clients_active`, `clients_total` | clientes vistos | |
| `gravity_domains` | domínios nas listas de bloqueio | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | última reconstrução da lista de bloqueio, e há quanto tempo | |
| `version_info` | valor 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 1 quando é publicada uma versão mais recente de um componente, 0 caso contrário; ausente até a verificação diária do Pi-hole ter corrido | `component` (`core`, `web`, `ftl`, `docker`) |
| `updates_available` | atualizações a instalar: a imagem no Docker (core, web e FTL vêm com ela), caso contrário cada componente desatualizado | |
| `messages`, `messages_by_type` | mensagens na página de diagnóstico | `type` (`LOAD`, `RATE_LIMIT`, `LIST`…) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | o daemon FTL | |
| `privacy_level` | nível de privacidade do Pi-hole, 0 a 3 | |
| `scrape_errors` | chamadas de API secundárias que falharam durante a última verificação | |

Num ramo de desenvolvimento, o Pi-hole compara commits em vez de números de
versão, e o DumbMonit faz o mesmo.

As [regras incorporadas](../alerting/rules.md#pi-hole) que se aplicam:
Pi-hole blocking disabled, Pi-hole blocklists stale, Pi-hole update
available, Pi-hole diagnosis messages, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca liga ao Pi-hole. Mostra se o bloqueio está ativo (e quando termina uma
pausa), as consultas e a fração bloqueada nas últimas 24 horas, o tamanho e a
idade da lista de bloqueio, os clientes, as versões com alguma atualização em
espera, e o número de mensagens de diagnóstico.

## Criar uma senha de aplicação no Pi-hole {#create-an-app-password-in-pi-hole}

1. É necessário o Pi-hole v6 ou posterior: a versão 5 não tinha API REST. O Pi-hole não tem conta só de leitura; o mais próximo é uma senha de aplicação, que não pode alterar nenhuma definição.

2. Na interface web do Pi-hole, abra Settings → Web interface / API e mude a página de Basic para Expert. Clique em Configure app password, copie a senha mostrada, depois clique em Enable new app password. O Pi-hole só a mostra uma vez, e uma nova substitui a anterior.

3. Uma senha de aplicação mantém-se só de leitura para definições enquanto webserver.api.app_sudo for false, a predefinição. Verifique-o na máquina do Pi-hole (no Docker, prefixe o comando com docker exec e o nome do contentor):

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. No DumbMonit, introduza o endereço do Pi-hole, por exemplo «pi.hole» ou «192.168.1.53», e cole a senha de aplicação. Para um Pi-hole cuja interface web não tem senha, escolha No password em vez disso.

5. O DumbMonit só lê. Nunca pausa o bloqueio e nunca altera uma lista, e mantém uma sessão de API aberta entre verificações em vez de abrir uma por verificação: o Pi-hole permite 16 sessões em simultâneo.

!!! warning
    A senha de aplicação ainda permite pausar o bloqueio e editar listas através da API: reserve-a só para o DumbMonit. O Pi-hole serve HTTP simples na porta 80, e a senha atravessa a rede cada vez que uma sessão abre: numa rede não confiável, use HTTPS na porta 443.

Uma senha de aplicação também inicia sessão sem o segundo fator quando a
autenticação de dois fatores está ativa: é para isso que serve. Para a
revogar, esvazie `webserver.api.app_pwhash` em Settings → All settings, ou
ative uma nova: a anterior deixa de funcionar de imediato.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Senha de aplicação | A senha gerada acima. O DumbMonit troca-a por uma sessão, enviada no cabeçalho `X-FTL-SID`, e reutiliza essa sessão enquanto o Pi-hole a mantiver (30 minutos após a sua última utilização). |
| Sem senha | Só para um Pi-hole cuja interface web não tem senha. |

Endereço: um nome de anfitrião ou IP (`pi.hole`), `anfitrião:porta`, ou um
URL completo (`https://pi.hole`); o caminho da interface web ou da API,
copiado do navegador, é ignorado. Opções: protocolo (HTTP por predefinição),
porta (80), verificação de certificado (para HTTPS na 443, o próprio
certificado do Pi-hole é autoassinado) e tempo limite do pedido (10 s).

Se as verificações falharem com «no free API session», as sessões do Pi-hole
estão todas ocupadas, geralmente por um script que inicia sessão em cada
execução sem a terminar. Expiram após 30 minutos; `webserver.api.max_sessions`
aumenta o limite.
