# UniFi Network {#unifi-network}

A rede UniFi e o que ela gerencia: gateways, switches e pontos de acesso que
estão offline, isolados ou esperando adoção, atualizações de firmware
esperando, os links WAN e a conexão com a Internet como o gateway os vê, o
número de clientes, e os alarmes que o UniFi disparou. Funciona com o
aplicativo UniFi Network de um console UniFi OS (Dream Machine, Cloud
Gateway, Cloud Key) e com um UniFi Network Server autoalojado.

O DumbMonit lê o UniFi com uma chave de API ou com uma conta local que
possui o papel **View Only**. Ele nunca reinicia, adota, atualiza ou
provisiona um dispositivo.

A falha que vale a pena observar: **um ponto de acesso ou switch cai
silenciosamente**. Clientes migram para o próximo ponto de acesso, a rede
continua funcionando, e o buraco de cobertura ou o uplink perdido só é
notado dias depois. O mesmo vale para um segundo link WAN que caiu: nada
quebra até o primeiro também falhar.

## Duas formas de entrada {#two-ways-in}

| Credencial | API usada | O que lê |
|---|---|---|
| Chave de API (console UniFi OS) | A Integration API oficial, em `/proxy/network/integration/v1` em um console e `/integration/v1` em um servidor autoalojado | Versão, dispositivos e seu estado, atualizações de firmware, carga, memória e tempo de atividade de cada dispositivo online, número de clientes |
| Conta View Only | A API clássica (`/api/s/<site>/stat/*`), em `/proxy/network/api/…` em um console | Tudo acima, mais clientes por dispositivo e por tipo, os links WAN, o status WAN e de Internet, e alarmes |

O status WAN, de Internet e alarme só existem na API clássica. Com uma
chave, o DumbMonit também pede à API clássica o status WAN e de Internet
com a mesma chave; se o controlador a recusa, essas séries simplesmente
ficam ausentes, sem erro.

Alarmes são as entradas do log de sistema "critical" do UniFi nas últimas
24 horas (UniFi Network 8 e posterior), ou os alarmes não arquivados em
versões mais antigas. Esse é o único `POST` que o DumbMonit envia, e ele só
lê.

Uma sessão aberta com a conta View Only é mantida de uma sondagem para a
próxima e reaberta quando expira: o UniFi não registra um novo login a cada
minuto.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_unifi_`. Séries de dispositivo
carregam `device` (seu nome, ou seu endereço MAC quando não tem nenhum),
`mac`, `model` e `type` (`gateway`, `switch`, `access_point`, `other`).

| Métrica | O que é | Rótulos |
|---|---|---|
| `info` | valor 1 | `version`, `api` (`integration` ou `classic`) |
| `devices` | dispositivos por estado: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | dispositivos adotados com uma atualização de firmware disponível | |
| `device_state` | 0 online, 1 offline, 2 pendente adoção, 3 atualizando ou provisionando, 4 adotando, 5 adoção falhou, 6 isolado, 7 outro | rótulos de dispositivo |
| `device_up` | 1 online ou atualizando, 0 offline, isolado ou falhou ao adotar; ausente para um dispositivo ainda não adotado | rótulos de dispositivo |
| `device_upgradable` | 1 quando uma atualização de firmware está disponível | rótulos de dispositivo, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | apenas dispositivos online | rótulos de dispositivo |
| `device_clients` | clientes conectados ao dispositivo (conta View Only) | rótulos de dispositivo |
| `wan_link_up` | 1 quando uma porta WAN do gateway está ativa; uma porta desativada não tem série | rótulos de dispositivo, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | o status WAN e de Internet do gateway; ausente sem um gateway UniFi | |
| `internet_latency_seconds` | latência medida pelo gateway | |
| `wan_availability_percent` | disponibilidade de cada WAN nas últimas 24 horas | `wan` |
| `subsystem_status` | 0 ok, 1 aviso, 2 erro, 3 desconhecido, para `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | clientes no site | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (conta View Only) | `type` |
| `alarms` | entradas do log de sistema crítico das últimas 24 horas, ou alarmes não arquivados | |
| `scrape_errors` | chamadas que falharam na última sondagem | |

As [regras integradas](../alerting/rules.md#unifi-network) que se aplicam:
UniFi device offline, UniFi Internet down, UniFi WAN link down, UniFi
gateway CPU high, UniFi device waiting for adoption, UniFi firmware update
available, UniFi alarms, além de Device unreachable.

## Dispositivos {#devices}

`/rest/user` (a API clássica, lida com qualquer credencial) lista todo
cliente que o controlador já viu — a maior parte de uma rede doméstica, na
prática: celulares passando, Wi-Fi de convidado, dispositivos inteligentes
que nunca voltam. Rastrear todos para desatualização enterraria o alerta
sob dispositivos que simplesmente nunca retornam, não dispositivos que
ficaram silenciosos. Por padrão só clientes que o usuário marcou de alguma
forma contam: nomeados no UniFi, reservados em um IP fixo, ou anotados. A
opção **Clientes a observar além dos marcados** adiciona endereços MAC ou
nomes de cliente específicos, ou `all` para rastrear todo cliente (não
recomendado em nada além de uma rede pequena e majoritariamente estática).

Clientes rastreados são publicados sob a família genérica compartilhada por
toda integração do DumbMonit que rastreia dispositivos clientes —
`dumbmonit_client_device_*`, com prefixo sem `unifi`, documentada
completamente em [Dispositivos clientes](../alerting/rules.md#client-devices):

| Métrica | O que é | Rótulos |
|---|---|---|
| `last_seen_timestamp_seconds` | Última conexão, como `/rest/user` a reporta | `device`, `type` (`Wired`, `Wireless` ou `Unknown`), `os` (vazio), `user` (vazio), `kind` |
| `stale_seconds` | Positivo uma vez que um cliente rastreado passou `device_stale_days` (3 por padrão) sem se conectar; aciona a regra [Dispositivo cliente desatualizado](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: o UniFi não tem backup para reportar) |

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o UniFi. Ele diz primeiro o que está errado (dispositivos
offline, Internet ou um link WAN fora do ar, alarmes), depois os
dispositivos por tipo com seu estado, firmware e carga, e os clientes,
depois a tabela de clientes rastreados com sua última conexão.

## Como foi validado {#how-it-was-validated}

Login, o papel View Only, respostas de erro (senha recusada, site
desconhecido, chave recusada, sessão expirada) e as respostas de um
controlador sem dispositivos foram verificados contra o UniFi Network
10.6.106, autoalojado. Nenhum dispositivo pôde ser adotado lá: a lista de
dispositivos, a saúde de um site equipado, os clientes conhecidos
(`/rest/user`) e os payloads da Integration API são validados contra a
documentação oficial.

## Criar um acesso somente leitura no UniFi Network {#create-a-read-only-access-in-unifi-network}

1. Em um console UniFi OS (Dream Machine, Cloud Gateway, Cloud Key), você
   pode criar uma chave de API: no UniFi Network, Settings → Control Plane
   → Integrations → Create API Key. Nomeie-a após o DumbMonit e copie-a
   agora: ela é mostrada apenas uma vez. A chave lê dispositivos, sua carga
   e o número de clientes através da Integration API oficial.

2. Para o status WAN, de Internet e alarme, e em um UniFi Network Server
   autoalojado, use uma conta local com o papel View Only em vez disso:
   Settings → Admins & Users → Create New. Nomeie-a como abaixo, dê a ela
   uma senha longa e aleatória, restrinja-a ao acesso local, e defina seu
   papel UniFi Network como View Only. O View Only lê tudo e não pode
   alterar nada.

    ```
    dumbmonit
    ```

3. No DumbMonit, informe o endereço do console ou servidor, por exemplo
   "unifi.lan" (porta 443 em um console) ou "unifi.lan:8443" para um
   servidor autoalojado, e cole a chave ou a conta. Se a rede não for o
   site padrão, defina a opção Site para o nome curto mostrado na barra de
   endereço depois de /manage/ ou /network/.

4. O DumbMonit só lê. Ele nunca reinicia, adota, atualiza ou provisiona um
   dispositivo, e nunca reconhece um alarme.

!!! warning
    Não use a conta do proprietário ou uma conta cloud UI.com: uma senha
    exposta abriria todo console vinculado a ela. Mantenha a autenticação
    de dois fatores desligada apenas para essa conta local View Only, já
    que um servidor de monitoramento não pode digitar um código. Consoles
    usam um certificado autoassinado por padrão: ative "Aceitar um
    certificado não verificável" a menos que você tenha instalado o seu
    próprio. O UniFi conhece todo cliente que já se conectou, a maioria
    celulares passando: só os nomeados, reservados em um IP fixo, ou
    anotados no UniFi são observados para desatualização por padrão, para
    evitar um alerta para o celular de todo visitante. Liste mais na opção
    Clientes a observar, ou informe all.

## Credenciais {#controller-not-set-up-yet}

| Credencial | Campos |
|---|---|
| Chave de API (console UniFi OS) | Uma chave criada em Settings → Control Plane → Integrations, enviada como `X-API-KEY`. |
| Conta View Only | O nome de usuário e senha da conta local View Only. |

Endereço: um nome de host ou IP (`unifi.lan`), `host:porta`
(`unifi.lan:8443` para um servidor autoalojado), ou uma URL completa; um
endereço copiado do navegador, com `/manage/…` ou `/network/…`, é aceito.
Opções: site (`default`), porta (443), verificação de certificado, tempo
limite da requisição (15 s), clientes a observar além dos marcados (vazio,
ou `all`), e desatualização de dispositivo (`device_stale_days`, 3 dias por
padrão).
