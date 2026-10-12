# Hardware de servidor (Redfish) {#server-hardware-redfish}

O próprio hardware de um servidor, lido do seu controlador de gestão
(BMC): ventoinhas, temperaturas contra os seus próprios limiares, fontes
de alimentação e a sua redundância, discos, memória, processadores e o
registo de eventos.

O Redfish é a interface HTTPS que a DMTF padronizou para controladores de
gestão, e todo controlador atual a fala: Supermicro, Dell iDRAC, HPE iLO,
Lenovo XClarity Controller, ASRock Rack, OpenBMC. É a forma correta de
monitorizar hardware de servidor. O agente SNMP de um BMC, por
contraste, geralmente descreve o próprio BMC — o seu pequeno Linux ARM, a
sua memória, as suas ligações de rede — e não o servidor que gere (ver
[SNMP](snmp.md#server-management-controllers-bmc)).

!!! warning "Testado apenas contra os modelos de referência da DMTF"

    Esta integração foi construída e verificada contra os modelos de
    referência oficiais Redfish da DMTF (servidor em rack, servidor torre
    com armazenamento local, servidor arrefecido a líquido com os esquemas
    térmicos e de energia mais recentes), servidos pelo servidor de
    modelos da DMTF. Toda chamada é uma leitura e as formas dos recursos
    são as padrão, mas ainda não foi executada contra um Supermicro, iDRAC
    ou iLO real durante semanas. Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_redfish_`. As séries de saúde
são **0 (OK), 1 (Warning) ou 2 (Critical)**, diretamente de
`Status.Health`.

| Família | Métricas | Etiquetas |
|---|---|---|
| Serviço | `info` (valor 1) | `vendor`, `product`, `redfish_version` |
| Sistema | `system_health` (a sua própria), `system_health_rollup` (tudo o que contém), `system_power_on`, `memory_health`, `processor_health` (os resumos do controlador, um valor para todos os DIMMs e todas as CPUs), `memory_total_bytes` | `system` |
| Chassis | `chassis_health`, `chassis_health_rollup` | `chassis` |
| Temperaturas | `temperature_celsius`, `temperature_upper_critical_celsius` e `temperature_upper_caution_celsius` (os próprios limiares críticos e de aviso do sensor, quando os declara), `temperature_health` | `chassis`, `sensor` |
| Ventoinhas | `fan_speed_rpm` (ou `fan_speed_percent` quando o controlador reporta uma percentagem), `fan_lower_critical_rpm` (ou `_percent`: o mínimo próprio declarado da ventoinha, só esquema mais antigo), `fan_health`, `fan_redundancy_health` | `chassis`, `fan` / `group` |
| Energia | `psu_health`, `psu_output_watts`, `psu_capacity_watts`, `power_redundancy_health`, `power_consumed_watts` | `chassis`, `psu` / `group` |
| Tensões | `voltage_volts`, `voltage_health` | `chassis`, `sensor` |
| Armazenamento | `storage_health`, `drive_health`, `drive_failure_predicted`, `drive_life_left_percent` (SSDs), `drive_capacity_bytes` | `system`, `storage`, `drive`, `media` |
| Controlador | `manager_health`, `manager_info` (valor 1) | `manager`, `firmware`, `model` |
| Registos | `log_entries`, `log_critical_entries`, `log_warning_entries` | `owner` (sistema ou controlador), `log` |
| Coleta | `scrape_errors`, `scrape_duration_seconds` | |

**Uma ranhura vazia não é uma falha.** Um componente cujo `Status.State` é
`Absent` (um compartimento de fonte de alimentação vazio, uma ranhura de
DIMM livre, uma segunda CPU em falta) ou `Disabled` (um grupo de
redundância não configurado) não produz nenhuma série. Um servidor
entregue com uma fonte de alimentação de duas é saudável, e lê-se como
tal.

**Duas gerações do esquema, um conjunto de séries.** A maioria dos
controladores em serviço ainda publica os recursos mais antigos
`Chassis/{id}/Thermal` e `Chassis/{id}/Power`, um pedido por cada sensor;
os mais recentes só publicam `ThermalSubsystem`, `PowerSubsystem`,
`EnvironmentMetrics` e `Sensors`, um recurso por ventoinha, fonte de
alimentação e sensor. O DumbMonit lê a forma mais antiga quando existe e
a mais recente caso contrário; ambas produzem exatamente as mesmas séries
e etiquetas, por isso uma regra escrita uma vez funciona em ambas.

**Os registos são contados, nunca lidos.** As entradas de registo podem
nomear anfitriões, endereços e utilizadores: o DumbMonit mantém só quantas
entradas há, e quantas da primeira página que o controlador serve são
Critical ou Warning.

As [regras incorporadas](../alerting/rules.md#server-hardware-redfish) que
se aplicam: Server fan failed, Server temperature above critical, Power
supply redundancy lost, Power supply failed, Drive failure predicted,
Server health critical, mais Device unreachable.

## A página do dispositivo {#the-device-page}

Um dispositivo Redfish recebe o seu próprio painel na sua página, acima
dos gráficos, lido do que a última verificação guardou: abrir a página
nunca consulta o controlador. Responde, por esta ordem:

1. **Se o servidor está saudável.** Uma palavra — Healthy, Degraded ou
   Critical — da própria saúde e resumos do controlador, e sob ela cada
   componente que a arrasta para baixo, numa frase: *CPU1 Temp is at
   47 °C, at or above its critical threshold of 45 °C*, *Power supply
   redundancy is lost*, *Drive 3 predicts its own failure*. Quando o
   controlador resume o servidor como degradado sem nomear nada que o
   DumbMonit leia, o painel diz-o e aponta para o seu registo de eventos.
   Abaixo: estado de alimentação, consumo de energia, memória instalada e
   o firmware do controlador.
2. **Se as ventoinhas e temperaturas estão dentro dos seus limites.** Cada
   sensor é comparado com os limiares que o controlador declara **para
   esse sensor** — as suas temperaturas de aviso e crítica, a velocidade
   mínima de uma ventoinha — nunca com um número do DumbMonit. Um sensor
   que não declara nenhum limiar mostra a sua leitura e nenhum veredicto,
   a menos que o próprio controlador o reporte como degradado. Os
   problemas são listados primeiro.
3. **Se a energia é redundante.** Cada grupo de redundância (Redundant,
   Redundancy degraded, Redundancy lost), depois cada fonte de
   alimentação presente com a sua saída contra a sua capacidade e o seu
   estado. Sem nenhum grupo de redundância, o painel diz que o
   controlador não declara nenhum: uma única fonte, ou redundância não
   configurada.
4. **Se os discos estão saudáveis.** Cada controlador de armazenamento,
   depois cada disco com a sua capacidade, a vida restante de um SSD e o
   seu estado; *Failure predicted* vem antes de tudo o resto.
5. **Memória, processadores e controlador**: os resumos de uma palavra do
   controlador para todos os DIMMs e todas as CPUs, a própria saúde,
   modelo e firmware do controlador de gestão, e as tensões.
6. **Registos de eventos**: entradas contadas por severidade. As
   entradas permanecem até alguém limpar o registo, por isso uma
   contagem crítica pode ser notícia antiga: é mostrada, mas não altera a
   palavra de saúde do servidor.

Compartimentos vazios e ranhuras livres nunca aparecem: não produzem
nenhuma série. Todo estado é uma palavra além de uma cor, e um valor
desconhecido lê «—», nunca 0.

## Criar uma conta só de leitura no controlador de gestão {#create-a-read-only-account-on-the-management-controller}

1. Abra a interface web do controlador de gestão (BMC), não o sistema operativo do servidor. Crie um novo utilizador local com o seguinte nome, com uma senha longa não usada em nenhum outro lugar.

    ```
    dumbmonit
    ```

2. Dê-lhe a função mais baixa que pode ler, e nada mais. Supermicro: Configuration → Users → Add User, privilégio User, e marque Redfish no tipo de conta. Dell iDRAC: iDRAC Settings → Users → Local Users → Add, função Read Only. HPE iLO: Administration → User Administration → New, desmarque todos os privilégios exceto Login. Lenovo XClarity Controller: BMC Configuration → User/LDAP → Create, função Read-only. ASRock Rack: Settings → User Management, privilégio User.

3. Verifique a partir de qualquer máquina na rede de gestão que a conta consegue ler o Redfish. O comando pede a senha e imprime o nome e a saúde do servidor.

    ```
    curl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems
    ```

4. No DumbMonit, introduza o endereço do controlador, por exemplo «bmc.lan» ou «10.0.0.50», depois o nome de utilizador e senha dessa conta.

5. O DumbMonit só lê: ventoinhas, temperaturas, tensões, fontes de alimentação, discos, resumos de memória e processador, e quantas entradas de registo há por severidade. Nunca liga ou desliga o servidor, nunca altera uma definição e nunca lê o texto dos registos.

!!! warning
    Não reutilize a conta de fábrica do controlador: pode desligar o servidor, montar media e reprogramar o firmware. Os controladores de gestão também vêm com um certificado autoassinado: se a ligação for recusada por esse motivo, marque «Accept an unverifiable certificate» nas opções. Um controlador responde lentamente e uma leitura completa demora vários pedidos: se o dispositivo reportar tempos limite, aumente DUMBMONIT_PROBE_TIMEOUT_SECS no servidor.

Os nomes de menu acima são os do firmware atual; gerações mais antigas
movem-nos mas mantêm as mesmas funções. No Supermicro, o tipo de conta
mostrado ao criar um utilizador decide quais interfaces pode usar: uma
conta sem acesso Redfish é recusada com um 401 mesmo com a senha certa.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Conta do controlador de gestão | Nome de utilizador e senha da conta só de leitura. |

Endereço: o nome de anfitrião ou IP do controlador, por exemplo `bmc.lan`
ou `10.0.0.50`. A porta predefinida é 443; escreva `anfitrião:porta` para
outra, ou um URL completo (`https://bmc.lan:8443`).

## Opções {#options}

| Opção | Predefinição | Efeito |
|---|---|---|
| `port` | `443` | Porta HTTPS, se o endereço não der uma. |
| `insecure_tls` | `false` | Aceitar um certificado autoassinado. |
| `request_timeout_seconds` | `8` | Tempo permitido para cada chamada Redfish, de 1 a 120. |
| `auth` | `basic` | `basic` envia o nome de utilizador e senha em cada pedido e não deixa nada aberto no controlador. `session` inicia sessão uma vez e reutiliza o token de sessão (`X-Auth-Token`), reabrindo-o só quando o controlador o recusa e fechando o que substitui. Use-o só se o controlador recusar a autenticação básica: os controladores só têm um punhado de lugares de sessão. |
| `storage` | `true` | Ler os controladores de armazenamento e cada disco atrás deles. Um pedido por disco. |
| `logs` | `true` | Contar as entradas do registo de eventos do sistema e do próprio registo do controlador, por severidade. |

Uma leitura completa são algumas dezenas de pedidos: a raiz do serviço,
depois cada sistema, chassis e controlador, os seus recursos térmicos e
de energia, os controladores de armazenamento e discos, os serviços de
registo. O DumbMonit envia no máximo quatro ao mesmo tempo. Um
controlador lento pode precisar de um tempo limite de verificação maior
(`DUMBMONIT_PROBE_TIMEOUT_SECS`, ver
[Configuração](../reference/configuration.md)) e um intervalo de
verificação de dois minutos ou mais.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| *Authentication refused* | Senha errada, ou a conta não é autorizada a usar o Redfish (tipo de conta no Supermicro, privilégio «Login» em falta no iLO). |
| *Insufficient privileges* | A função é demasiado baixa para um recurso; o resto da leitura continua a passar. Alguns controladores escondem o registo de eventos da sua função mais baixa. |
| *TLS certificate rejected* | O certificado autoassinado do controlador: marque «Accept an unverifiable certificate», ou instale um certificado de confiança no controlador. |
| *Timed out* | O controlador é lento a responder: aumente `DUMBMONIT_PROBE_TIMEOUT_SECS`, desligue `storage` ou `logs`, ou alargue o intervalo. |
| Nenhum limiar de temperatura | O controlador não publica `UpperThresholdCritical` para esse sensor: a temperatura continua a ser registada, mas a regra incorporada não a pode comparar com nada. |
