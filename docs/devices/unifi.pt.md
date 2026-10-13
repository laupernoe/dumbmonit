# UniFi Network {#unifi-network}

A rede UniFi e o que ela gere: gateways, comutadores e pontos de acesso que
estão offline, isolados ou à espera de adoção, atualizações de firmware em
espera, as ligações WAN e a ligação à Internet tal como o gateway as vê, o
número de clientes, e os alarmes que o UniFi levantou. Funciona com a aplicação
UniFi Network de uma consola UniFi OS (Dream Machine, Cloud Gateway, Cloud Key)
e com um UniFi Network Server auto-alojado.

O DumbMonit lê o UniFi com uma chave de API ou com uma conta local que tem o
papel **View Only**. Nunca reinicia, adota, atualiza nem provisiona um
dispositivo.

A falha que vale a pena vigiar: **um ponto de acesso ou um comutador que cai em
silêncio**. Os clientes passam para o ponto de acesso seguinte, a rede continua
a funcionar, e o buraco de cobertura ou o uplink perdido só é notado dias
depois. O mesmo vale para uma segunda ligação WAN que caiu: nada avaria até a
primeira falhar também.

## Duas formas de entrar {#two-ways-in}

| Credencial | API usada | O que lê |
|---|---|---|
| Chave de API (consola UniFi OS) | A Integration API oficial, em `/proxy/network/integration/v1` numa consola e `/integration/v1` num servidor auto-alojado | Versão, dispositivos e o seu estado, atualizações de firmware, carga, memória e tempo de atividade de cada dispositivo online, número de clientes |
| Conta View Only | A API clássica (`/api/s/<site>/stat/*`), em `/proxy/network/api/…` numa consola | Tudo o acima, mais clientes por dispositivo e por tipo, as ligações WAN, o estado da WAN e da Internet, e alarmes |

O estado da WAN, da Internet e dos alarmes só existe na API clássica. Com uma
chave, o DumbMonit pede também à API clássica o estado da WAN e da Internet com
a mesma chave; se o controlador a recusar, essas séries ficam simplesmente
ausentes, sem erro.

Os alarmes são as entradas do registo de sistema «crítico» do UniFi nas últimas
24 horas (UniFi Network 8 e posteriores), ou os alarmes não arquivados nas
versões mais antigas. É o único `POST` que o DumbMonit envia, e só lê.

Uma sessão aberta com a conta View Only é mantida de uma sonda para a seguinte
e reaberta quando expira: o UniFi não regista um novo início de sessão a cada
minuto.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_unifi_`. As séries de dispositivos
trazem `device` (o seu nome, ou o endereço MAC quando não tem nome), `mac`,
`model` e `type` (`gateway`, `switch`, `access_point`, `other`).

| Métrica | O quê | Etiquetas |
|---|---|---|
| `info` | valor 1 | `version`, `api` (`integration` ou `classic`) |
| `devices` | dispositivos por estado: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | dispositivos adotados com atualização de firmware disponível | |
| `device_state` | 0 online, 1 offline, 2 adoção pendente, 3 a atualizar ou a provisionar, 4 a adotar, 5 adoção falhada, 6 isolado, 7 outro | etiquetas do dispositivo |
| `device_up` | 1 online ou a atualizar, 0 offline, isolado ou com adoção falhada; ausente para um dispositivo ainda não adotado | etiquetas do dispositivo |
| `device_upgradable` | 1 quando há uma atualização de firmware disponível | etiquetas do dispositivo, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | apenas dispositivos online | etiquetas do dispositivo |
| `device_clients` | clientes ligados ao dispositivo (conta View Only) | etiquetas do dispositivo |
| `wan_link_up` | 1 quando uma porta WAN do gateway está ativa; uma porta desativada não tem série | etiquetas do dispositivo, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | o estado da WAN e da Internet do gateway; ausente sem um gateway UniFi | |
| `internet_latency_seconds` | latência medida pelo gateway | |
| `wan_availability_percent` | disponibilidade de cada WAN nas últimas 24 horas | `wan` |
| `subsystem_status` | 0 ok, 1 aviso, 2 erro, 3 desconhecido, para `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | clientes no site | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (conta View Only) | `type` |
| `alarms` | entradas do registo de sistema crítico das últimas 24 horas, ou alarmes não arquivados | |
| `scrape_errors` | chamadas que falharam na última sonda | |

As [regras incorporadas](../alerting/rules.md#unifi-network) que se aplicam:
UniFi device offline, UniFi Internet down, UniFi WAN link down, UniFi gateway
CPU high, UniFi device waiting for adoption, UniFi firmware update available,
UniFi alarms, mais Device unreachable.

## Dispositivos {#devices}

`/rest/user` (a API clássica, lida com qualquer das credenciais) lista todos os
clientes que o controlador alguma vez viu — a maior parte de uma rede
doméstica, na prática: telefones de passagem, Wi-Fi de convidados, dispositivos
inteligentes que nunca voltam. Acompanhar todos para detetar inatividade
enterraria o alerta sob dispositivos que simplesmente nunca regressam, não
dispositivos que ficaram em silêncio. Por predefinição só contam os clientes
que o utilizador marcou de uma forma ou de outra: com nome no UniFi, reservados
num IP fixo, ou anotados. A opção **Clientes a vigiar além dos marcados**
acrescenta endereços MAC ou nomes de cliente específicos, ou `all` para
acompanhar todos os clientes (não recomendado em nada que não seja uma rede
pequena e quase estática).

Os clientes acompanhados são publicados sob a família genérica partilhada por
todas as integrações do DumbMonit que acompanham dispositivos de clientes —
`dumbmonit_client_device_*`, com prefixo sem `unifi`, documentada por inteiro em
[Dispositivos de clientes](../alerting/rules.md#client-devices):

| Métrica | O quê | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última ligação, como o `/rest/user` a reporta | `device`, `type` (`Wired`, `Wireless` ou `Unknown`), `os` (vazio), `user` (vazio), `kind` |
| `stale_seconds` | Positivo quando um cliente acompanhado passou `device_stale_days` (3 por predefinição) sem se ligar; alimenta a regra [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: o UniFi não tem cópia de segurança a reportar) |

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
consulta o UniFi. Diz primeiro o que está errado (dispositivos offline,
Internet ou uma ligação WAN em baixo, alarmes), depois os dispositivos por tipo
com o seu estado, firmware e carga, e os clientes, depois a tabela de clientes
acompanhados com a sua última ligação.

## Como foi validado {#how-it-was-validated}

O início de sessão, o papel View Only, as respostas de erro (palavra-passe
recusada, site desconhecido, chave recusada, sessão expirada) e as respostas de
um controlador sem dispositivos foram verificados contra o UniFi Network
10.6.106, auto-alojado. Nenhum dispositivo pôde ser adotado aí: a lista de
dispositivos, a saúde de um site equipado, os clientes conhecidos
(`/rest/user`) e as cargas úteis da Integration API são validados contra a
documentação oficial.

## Criar um acesso só de leitura no UniFi Network {#create-a-read-only-access-in-unifi-network}

1. Numa consola UniFi OS (Dream Machine, Cloud Gateway, Cloud Key), pode criar uma chave de API: no UniFi Network, Settings → Control Plane → Integrations → Create API Key. Dê-lhe o nome do DumbMonit e copie-a agora: só é mostrada uma vez. A chave lê dispositivos, a sua carga e o número de clientes através da Integration API oficial.

2. Para o estado da WAN, da Internet e dos alarmes, e num UniFi Network Server auto-alojado, use antes uma conta local com o papel View Only: Settings → Admins & Users → Create New. Dê-lhe o nome seguinte, uma palavra-passe longa e aleatória, restrinja-a ao acesso local, e defina o seu papel no UniFi Network como View Only. O View Only lê tudo e não pode alterar nada.

    ```
    dumbmonit
    ```

3. No DumbMonit, introduza o endereço da consola ou do servidor, por exemplo «unifi.lan» (porta 443 numa consola) ou «unifi.lan:8443» para um servidor auto-alojado, e cole a chave ou a conta. Se a rede não for o site predefinido, defina a opção Site para o nome curto mostrado na barra de endereço depois de /manage/ ou /network/.

4. O DumbMonit só lê. Nunca reinicia, adota, atualiza nem provisiona um dispositivo, e nunca reconhece um alarme.

!!! warning
    Não use a conta de proprietário nem uma conta na nuvem UI.com: uma palavra-passe vazada abriria todas as consolas associadas a ela. Mantenha a autenticação de dois fatores desligada apenas para esta conta local View Only, já que um servidor de monitorização não pode escrever um código. As consolas usam por predefinição um certificado autoassinado: ative «Aceitar um certificado não verificável» a menos que tenha instalado o seu. O UniFi conhece todos os clientes que alguma vez se ligaram, a maioria telefones de passagem: por predefinição só os que têm nome, estão reservados num IP fixo ou anotados no UniFi são vigiados quanto a inatividade, para evitar um alerta por cada telefone de visitante. Liste mais na opção Clientes a vigiar, ou introduza all.

## Controlador ainda não configurado {#controller-not-set-up-yet}

Um controlador auto-alojado cujo assistente de primeira configuração não está concluído responde a todos os pedidos com um redirecionamento para `/setup`. O DumbMonit reporta então «The UniFi controller is not set up yet» como erro de configuração (não como palavra-passe recusada): conclua o assistente no navegador, depois sonde de novo.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API (consola UniFi OS) | Uma chave criada em Settings → Control Plane → Integrations, enviada como `X-API-KEY`. |
| Conta View Only | O nome de utilizador e a palavra-passe da conta local View Only. |

Endereço: um nome de anfitrião ou IP (`unifi.lan`), `host:port`
(`unifi.lan:8443` para um servidor auto-alojado), ou um URL completo; um
endereço copiado do navegador, com `/manage/…` ou `/network/…`, é aceite.
Opções: site (`default`), porta (443), verificação do certificado, tempo limite
do pedido (15 s), clientes a vigiar além dos marcados (vazio, ou `all`), e
inatividade de dispositivos (`device_stale_days`, 3 dias por predefinição).
