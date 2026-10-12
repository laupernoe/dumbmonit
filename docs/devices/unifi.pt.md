# UniFi Network {#unifi-network}

A rede UniFi e o que ela gere: gateways, comutadores e pontos de acesso
offline, isolados ou à espera de adoção, atualizações de firmware em
espera, as ligações WAN e a ligação à Internet como o gateway as vê, o
número de clientes, e os alarmes que o UniFi levantou. Funciona com a
aplicação UniFi Network de uma consola UniFi OS (Dream Machine, Cloud
Gateway, Cloud Key) e com um UniFi Network Server autoalojado.

O DumbMonit lê o UniFi com uma chave de API ou com uma conta local que
detém a função **View Only**. Nunca reinicia, adota, atualiza ou provisiona
um dispositivo.

A falha a vigiar: **um ponto de acesso ou um comutador cai silenciosamente**.
Os clientes migram para o ponto de acesso seguinte, a rede continua a
funcionar, e o buraco de cobertura ou a ligação ascendente perdida só é
notado dias depois. O mesmo se aplica a uma segunda ligação WAN que caiu:
nada se quebra até a primeira também falhar.

## Duas formas de entrar {#two-ways-in}

| Credencial | API usada | O que lê |
|---|---|---|
| Chave de API (consola UniFi OS) | A API de Integração oficial, em `/proxy/network/integration/v1` numa consola e `/integration/v1` num servidor autoalojado | Versão, dispositivos e o seu estado, atualizações de firmware, carga, memória e tempo de atividade de cada dispositivo online, número de clientes |
| Conta View Only | A API clássica (`/api/s/<site>/stat/*`), em `/proxy/network/api/…` numa consola | Tudo o anterior, mais clientes por dispositivo e por tipo, as ligações WAN, o estado WAN e da Internet, e alarmes |

O estado WAN, da Internet e os alarmes só existem na API clássica. Com uma
chave, o DumbMonit também pergunta à API clássica pelo estado WAN e da
Internet com a mesma chave; se o controlador a recusar, essas séries
simplesmente ficam ausentes, sem erro.

Os alarmes são as entradas do log de sistema «crítico» do UniFi nas
últimas 24 horas (UniFi Network 8 e posterior), ou os alarmes não
arquivados em versões mais antigas. Esse é o único `POST` que o DumbMonit
envia, e só lê.

Uma sessão aberta com a conta View Only é mantida de uma verificação para a
outra e reaberta quando expira: o UniFi não regista um novo início de
sessão a cada minuto.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_unifi_`. As séries de
dispositivo transportam `device` (o seu nome, ou o seu endereço MAC quando
não tem nenhum), `mac`, `model` e `type` (`gateway`, `switch`,
`access_point`, `other`).

| Métrica | O quê | Etiquetas |
|---|---|---|
| `info` | valor 1 | `version`, `api` (`integration` ou `classic`) |
| `devices` | dispositivos por estado: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | dispositivos adotados com uma atualização de firmware disponível | |
| `device_state` | 0 online, 1 offline, 2 pendente de adoção, 3 a atualizar ou provisionar, 4 a adotar, 5 adoção falhada, 6 isolado, 7 outro | etiquetas de dispositivo |
| `device_up` | 1 online ou a atualizar, 0 offline, isolado ou falhou ao adotar; ausente para um dispositivo ainda não adotado | etiquetas de dispositivo |
| `device_upgradable` | 1 quando uma atualização de firmware está disponível | etiquetas de dispositivo, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | só dispositivos online | etiquetas de dispositivo |
| `device_clients` | clientes ligados ao dispositivo (conta View Only) | etiquetas de dispositivo |
| `wan_link_up` | 1 quando uma porta WAN do gateway está ativa; uma porta desativada não tem série | etiquetas de dispositivo, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | o estado WAN e da Internet do gateway; ausente sem um gateway UniFi | |
| `internet_latency_seconds` | latência medida pelo gateway | |
| `wan_availability_percent` | disponibilidade de cada WAN nas últimas 24 horas | `wan` |
| `subsystem_status` | 0 ok, 1 aviso, 2 erro, 3 desconhecido, para `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | clientes no site | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (conta View Only) | `type` |
| `alarms` | entradas do log de sistema crítico das últimas 24 horas, ou alarmes não arquivados | |
| `scrape_errors` | chamadas que falharam na última verificação | |

As [regras incorporadas](../alerting/rules.md#unifi-network) que se
aplicam: UniFi device offline, UniFi Internet down, UniFi WAN link down,
UniFi gateway CPU high, UniFi device waiting for adoption, UniFi firmware
update available, UniFi alarms, mais Device unreachable.

## Dispositivos {#devices}

`/rest/user` (a API clássica, lida com qualquer credencial) lista todo
cliente que o controlador já viu — a maior parte de uma rede doméstica, na
prática: telefones a passar, Wi-Fi de convidados, dispositivos inteligentes
que nunca voltam. Acompanhar todos eles para obsolescência enterraria o
alerta sob dispositivos que simplesmente nunca voltam, não dispositivos que
ficaram silenciosos. Por predefinição só os clientes que o utilizador
marcou de alguma forma contam: nomeados no UniFi, reservados num IP fixo,
ou anotados. A opção **Clients to watch besides the marked ones** adiciona
endereços MAC ou nomes de cliente específicos, ou `all` para acompanhar
todo cliente (não recomendado em nada além de uma rede pequena e
maioritariamente estática).

Os clientes acompanhados são publicados sob a família genérica partilhada
por toda integração do DumbMonit que acompanha dispositivos cliente —
`dumbmonit_client_device_*`, prefixada sem `unifi`, documentada na íntegra
em [Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | O quê | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última ligação, como `/rest/user` a reporta | `device`, `type` (`Wired`, `Wireless` ou `Unknown`), `os` (vazio), `user` (vazio), `kind` |
| `stale_seconds` | Positivo uma vez que um cliente acompanhado passou `device_stale_days` (3 por predefinição) sem se ligar; alimenta a regra [Dispositivo obsoleto](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: o UniFi não tem nenhuma cópia de segurança a reportar) |

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o UniFi. Diz primeiro o que está mal (dispositivos offline,
Internet ou uma ligação WAN em baixo, alarmes), depois os dispositivos por
tipo com o seu estado, firmware e carga, e os clientes, depois a tabela de
clientes acompanhados com a sua última ligação.

## Como foi validado {#how-it-was-validated}

Início de sessão, a função View Only, respostas de erro (senha recusada,
site desconhecido, chave recusada, sessão expirada) e as respostas de um
controlador sem dispositivos foram verificadas contra o UniFi Network
10.6.106, autoalojado. Nenhum dispositivo pôde ser adotado aí: a lista de
dispositivos, a saúde de um site equipado, os clientes conhecidos
(`/rest/user`) e as cargas úteis da API de Integração são validadas contra
a documentação oficial.

## Criar um acesso só de leitura no UniFi Network {#create-a-read-only-access-in-unifi-network}

1. Numa consola UniFi OS (Dream Machine, Cloud Gateway, Cloud Key), pode criar uma chave de API: no UniFi Network, Settings → Control Plane → Integrations → Create API Key. Dê-lhe o nome do DumbMonit e copie-a agora: só é mostrada uma vez. A chave lê dispositivos, a sua carga e o número de clientes através da API de Integração oficial.

2. Para o estado WAN, da Internet e de alarmes, e num UniFi Network Server autoalojado, use em vez disso uma conta local com a função View Only: Settings → Admins & Users → Create New. Dê-lhe o seguinte nome, uma senha longa e aleatória, restrinja-a ao acesso local, e defina a sua função UniFi Network como View Only. View Only lê tudo e não pode alterar nada.

    ```
    dumbmonit
    ```

3. No DumbMonit, introduza o endereço da consola ou servidor, por exemplo «unifi.lan» (porta 443 numa consola) ou «unifi.lan:8443» para um servidor autoalojado, e cole a chave ou a conta. Se a rede não for o site predefinido, defina a opção Site para o nome curto mostrado na barra de endereço depois de /manage/ ou /network/.

4. O DumbMonit só lê. Nunca reinicia, adota, atualiza ou provisiona um dispositivo, e nunca confirma um alarme.

!!! warning
    Não use a conta do proprietário nem uma conta cloud UI.com: uma senha exposta abriria toda consola a ela ligada. Mantenha a autenticação de dois fatores desligada só para esta conta local View Only, já que um servidor de monitorização não pode escrever um código. As consolas usam um certificado autoassinado por predefinição: ative «Accept an unverifiable certificate» a menos que tenha instalado o seu próprio. O UniFi conhece todo cliente que já se ligou, a maioria telefones a passar: só os nomeados, reservados num IP fixo, ou anotados no UniFi são monitorizados para obsolescência por predefinição, para evitar um alerta por cada telefone de visitante. Liste mais na opção Clients to watch, ou introduza all.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API (consola UniFi OS) | Uma chave criada em Settings → Control Plane → Integrations, enviada como `X-API-KEY`. |
| Conta View Only | O nome de utilizador e a senha da conta local View Only. |

Endereço: um nome de anfitrião ou IP (`unifi.lan`), `anfitrião:porta`
(`unifi.lan:8443` para um servidor autoalojado), ou um URL completo; um
endereço copiado do navegador, com `/manage/…` ou `/network/…`, é aceite.
Opções: site (`default`), porta (443), verificação de certificado, tempo
limite do pedido (15 s), clientes a monitorizar além dos marcados (vazio,
ou `all`), e obsolescência de dispositivo (`device_stale_days`, 3 dias por
predefinição).
