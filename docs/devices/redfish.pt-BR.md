# Hardware de servidor (Redfish)

O próprio hardware de um servidor, lido de seu controlador de gerenciamento
(BMC): ventoinhas, temperaturas contra seus próprios limites, fontes de
alimentação e sua redundância, discos, memória, processadores e o log de
eventos.

Redfish é a interface HTTPS que a DMTF padronizou para controladores de
gerenciamento, e todo controlador atual a fala: Supermicro, Dell iDRAC, HPE
iLO, Lenovo XClarity Controller, ASRock Rack, OpenBMC. É a forma certa de
observar o hardware do servidor. O agente SNMP de um BMC, em contraste,
geralmente descreve o próprio BMC — seu pequeno Linux ARM, sua memória,
seus links de rede — e não o servidor que ele gerencia (veja
[SNMP](snmp.md#server-management-controllers-bmc)).

!!! warning "Testado apenas contra os mockups de referência da DMTF"

    Esta integração foi construída e verificada contra os mockups Redfish
    oficiais da DMTF (servidor rack, servidor tower com armazenamento
    local, servidor com refrigeração líquida com os esquemas térmicos e de
    energia mais novos), servidos pelo servidor de mockup da DMTF. Toda
    chamada é uma leitura e as formas de recurso são as padrão, mas ainda
    não foi executada contra um Supermicro, iDRAC ou iLO real por
    semanas. Avise-nos o que não funcionar.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_redfish_`. Séries de saúde são
**0 (OK), 1 (Aviso) ou 2 (Crítico)**, direto de `Status.Health`.

| Família | Métricas | Rótulos |
|---|---|---|
| Serviço | `info` (valor 1) | `vendor`, `product`, `redfish_version` |
| Sistema | `system_health` (a do próprio sistema), `system_health_rollup` (tudo que contém), `system_power_on`, `memory_health`, `processor_health` (os resumos do controlador, um valor para todos os DIMMs e todas as CPUs), `memory_total_bytes` | `system` |
| Chassi | `chassis_health`, `chassis_health_rollup` | `chassis` |
| Temperaturas | `temperature_celsius`, `temperature_upper_critical_celsius` e `temperature_upper_caution_celsius` (os próprios limites críticos e de atenção do sensor, quando declarados), `temperature_health` | `chassis`, `sensor` |
| Ventoinhas | `fan_speed_rpm` (ou `fan_speed_percent` quando o controlador reporta uma porcentagem), `fan_lower_critical_rpm` (ou `_percent`: o mínimo próprio declarado da ventoinha, apenas esquema mais antigo), `fan_health`, `fan_redundancy_health` | `chassis`, `fan` / `group` |
| Energia | `psu_health`, `psu_output_watts`, `psu_capacity_watts`, `power_redundancy_health`, `power_consumed_watts` | `chassis`, `psu` / `group` |
| Tensões | `voltage_volts`, `voltage_health` | `chassis`, `sensor` |
| Armazenamento | `storage_health`, `drive_health`, `drive_failure_predicted`, `drive_life_left_percent` (SSDs), `drive_capacity_bytes` | `system`, `storage`, `drive`, `media` |
| Controlador | `manager_health`, `manager_info` (valor 1) | `manager`, `firmware`, `model` |
| Logs | `log_entries`, `log_critical_entries`, `log_warning_entries` | `owner` (sistema ou controlador), `log` |
| Coleta | `scrape_errors`, `scrape_duration_seconds` | |

**Um slot vazio não é uma falha.** Um componente cujo `Status.State` é
`Absent` (um compartimento de fonte de alimentação vazio, um slot DIMM
livre, uma segunda CPU faltando) ou `Disabled` (um grupo de redundância
não configurado) não produz série alguma. Um servidor entregue com uma
fonte de alimentação de duas é saudável, e é mostrado como tal.

**Duas gerações do esquema, um único conjunto de séries.** A maioria dos
controladores em serviço ainda publica os recursos mais antigos
`Chassis/{id}/Thermal` e `Chassis/{id}/Power`, uma requisição cada para
todo sensor; os mais novos só publicam `ThermalSubsystem`,
`PowerSubsystem`, `EnvironmentMetrics` e `Sensors`, um recurso por
ventoinha, fonte de alimentação e sensor. O DumbMonit lê a forma mais
antiga quando existe e a mais nova caso contrário; ambas produzem
exatamente as mesmas séries e rótulos, então uma regra escrita uma vez
funciona em ambas.

**Os logs são contados, nunca lidos.** Entradas de log podem nomear hosts,
endereços e usuários: o DumbMonit mantém apenas quantas entradas existem,
e quantas da primeira página que o controlador serve são Críticas ou de
Aviso.

As [regras integradas](../alerting/rules.md#server-hardware-redfish) que
se aplicam: Server fan failed, Server temperature above critical, Power
supply redundancy lost, Power supply failed, Drive failure predicted,
Server health critical, além de Device unreachable.

## A página do dispositivo

Um dispositivo Redfish recebe seu próprio painel em sua página, acima dos
gráficos, lido do que a última sondagem armazenou: abrir a página nunca
consulta o controlador. Responde, nesta ordem:

1. **Se o servidor está saudável.** Uma palavra — Saudável, Degradado ou
   Crítico — a partir da saúde e resumos do próprio controlador, e abaixo
   dela todo componente que o arrasta para baixo, em uma frase: *CPU1 Temp
   está em 47 °C, no ou acima de seu limite crítico de 45 °C*, *A
   redundância da fonte de alimentação está perdida*, *O Disco 3 prevê
   sua própria falha*. Quando o controlador classifica o servidor como
   degradado sem nomear nada que o DumbMonit leia, o painel avisa isso e
   aponta para seu log de eventos. Abaixo: estado de energia, consumo de
   energia, memória instalada e o firmware do controlador.
2. **Se ventoinhas e temperaturas estão dentro de seus limites.** Cada
   sensor é comparado com os limites que o controlador declara **para
   aquele sensor** — suas temperaturas de atenção e crítica, a velocidade
   mínima de uma ventoinha — nunca com um número do DumbMonit. Um sensor
   que não declara limite mostra sua leitura e nenhum veredito, a menos
   que o próprio controlador o reporte como degradado. Problemas são
   listados primeiro.
3. **Se a energia é redundante.** Cada grupo de redundância (Redundante,
   Redundância degradada, Redundância perdida), depois cada fonte de
   alimentação presente com sua saída contra sua capacidade e seu estado.
   Sem grupo de redundância, o painel diz que o controlador não declara
   nenhum: uma única fonte, ou redundância não configurada.
4. **Se os discos estão saudáveis.** Cada controlador de armazenamento,
   depois cada disco com sua capacidade, a vida restante de um SSD e seu
   estado; *Falha prevista* vem antes de qualquer outra coisa.
5. **Memória, processadores e controlador**: os resumos de uma palavra do
   controlador para todos os DIMMs e todas as CPUs, a própria saúde,
   modelo e firmware do controlador de gerenciamento, e as tensões.
6. **Logs de eventos**: entradas contadas por severidade. Entradas
   permanecem até alguém limpar o log, então uma contagem crítica pode ser
   notícia antiga: é mostrada, mas não altera a palavra de saúde do
   servidor.

Compartimentos vazios e slots livres nunca aparecem: não produzem série
alguma. Todo estado é uma palavra assim como uma cor, e um valor
desconhecido lê "—", nunca 0.

## Criar uma conta somente leitura no controlador de gerenciamento

1. Abra a interface web do controlador de gerenciamento (BMC), não o
   sistema operacional do servidor. Crie um novo usuário local nomeado
   como abaixo, com uma senha longa usada em nenhum outro lugar.

    ```
    dumbmonit
    ```

2. Dê a ele o papel mais baixo que pode ler, e nada mais. Supermicro:
   Configuration → Users → Add User, privilégio User, e marque Redfish no
   tipo de conta. Dell iDRAC: iDRAC Settings → Users → Local Users → Add,
   papel Read Only. HPE iLO: Administration → User Administration → New,
   desmarque todo privilégio exceto Login. Lenovo XClarity Controller:
   BMC Configuration → User/LDAP → Create, papel Read-only. ASRock Rack:
   Settings → User Management, privilégio User.

3. Verifique de qualquer máquina na rede de gerenciamento que a conta pode
   ler o Redfish. O comando pede a senha e imprime o nome e saúde do
   servidor.

    ```
    curl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems
    ```

4. No DumbMonit, informe o endereço do controlador, por exemplo "bmc.lan"
   ou "10.0.0.50", depois o nome de usuário e senha dessa conta.

5. O DumbMonit só lê: ventoinhas, temperaturas, tensões, fontes de
   alimentação, discos, resumos de memória e processador, e quantas
   entradas de log existem por severidade. Ele nunca liga ou desliga o
   servidor, nunca altera uma configuração e nunca lê o texto dos logs.

!!! warning
    Não reutilize a conta de fábrica do controlador: ela pode desligar o
    servidor, montar mídia e regravar o firmware. Controladores de
    gerenciamento também vêm com um certificado autoassinado: se a
    conexão for recusada por esse motivo, marque "Aceitar um certificado
    não verificável" nas opções. Um controlador responde lentamente e uma
    leitura completa leva várias requisições: se o dispositivo reporta
    tempos esgotados, aumente DUMBMONIT_PROBE_TIMEOUT_SECS no servidor.

Os nomes de menu acima são os do firmware atual; gerações mais antigas os
movem mas mantêm os mesmos papéis. No Supermicro, o tipo de conta mostrado
ao criar um usuário decide quais interfaces ele pode usar: uma conta sem
acesso Redfish é recusada com um 401 mesmo com a senha certa.

## Credenciais

| Credencial | Campos |
|---|---|
| Conta do controlador de gerenciamento | Nome de usuário e senha da conta somente leitura. |

Endereço: o nome de host ou IP do controlador, por exemplo `bmc.lan` ou
`10.0.0.50`. A porta padrão é 443; escreva `host:porta` para outra, ou uma
URL completa (`https://bmc.lan:8443`).

## Opções

| Opção | Padrão | Efeito |
|---|---|---|
| `port` | `443` | Porta HTTPS, se o endereço não der uma. |
| `insecure_tls` | `false` | Aceitar um certificado autoassinado. |
| `request_timeout_seconds` | `8` | Tempo permitido para cada chamada Redfish, de 1 a 120. |
| `auth` | `basic` | `basic` envia o nome de usuário e senha com cada requisição e não deixa nada aberto no controlador. `session` entra uma vez e reutiliza o token de sessão (`X-Auth-Token`), reabrindo-o apenas quando o controlador o recusa e fechando o que ele substitui. Use apenas se o controlador recusar autenticação básica: controladores só têm algumas vagas de sessão. |
| `storage` | `true` | Ler os controladores de armazenamento e todo disco detrás deles. Uma requisição por disco. |
| `logs` | `true` | Contar as entradas do log de eventos do sistema e do próprio log do controlador, por severidade. |

Uma leitura completa é algumas dezenas de requisições: a raiz do serviço,
depois cada sistema, chassi e controlador, seus recursos térmicos e de
energia, os controladores de armazenamento e discos, os serviços de log.
O DumbMonit envia no máximo quatro por vez. Um controlador lento pode
precisar de um tempo limite de sondagem mais longo
(`DUMBMONIT_PROBE_TIMEOUT_SECS`, veja
[Configuração](../reference/configuration.md)) e um intervalo de
sondagem de dois minutos ou mais.

## Erros comuns

| Sintoma | Causa provável |
|---|---|
| *Authentication refused* | Senha errada, ou a conta não tem permissão para usar o Redfish (tipo de conta do Supermicro, privilégio "Login" do iLO faltando). |
| *Insufficient privileges* | O papel é muito baixo para um recurso; o resto da leitura ainda passa. Alguns controladores escondem o log de eventos de seu papel mais baixo. |
| *TLS certificate rejected* | O certificado autoassinado do controlador: marque "Aceitar um certificado não verificável", ou instale um certificado confiável no controlador. |
| *Timed out* | O controlador é lento para responder: aumente `DUMBMONIT_PROBE_TIMEOUT_SECS`, desligue `storage` ou `logs`, ou aumente o intervalo. |
| Nenhum limite de temperatura | O controlador não publica `UpperThresholdCritical` para aquele sensor: a temperatura ainda é registrada, mas a regra integrada não pode comparar com nada. |
