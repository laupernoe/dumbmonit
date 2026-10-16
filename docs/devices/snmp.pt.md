# Dispositivo SNMP {#snmp-device}

O tipo mais universal: quase todo o hardware de rede fala SNMP. Comutadores,
routers, NAS, UPS e impressoras qualificam-se todos. SNMP v1, v2c e v3 são
suportados.

## O que monitoriza {#what-it-watches}

O perfil de coleta é detetado automaticamente a partir do `sysObjectID` do
dispositivo (e, para UPS, do seu `sysDescr`). Sete perfis acompanham o
produto; um dispositivo pode corresponder a vários, e os perfis incluem-se
uns aos outros.

| Perfil | Aplica-se a | Métricas (prefixo `dumbmonit_`) |
|---|---|---|
| System (SNMPv2-MIB) | Todos os dispositivos | `system_uptime_seconds`, `system_info` (nome, descrição, localização, contacto como etiquetas) |
| Network interfaces (IF-MIB) | Todos os dispositivos, como alternativa | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out`, `if_discards_in/out`, `if_admin_status`, `if_oper_status`, `if_speed_bps`, por interface |
| Host resources (HOST-RESOURCES-MIB) | Net-SNMP (Linux, BSD, macOS), o agente SNMP do Windows, Synology, QNAP, Solaris | `host_uptime_seconds`, `host_processes`, `host_users`, `cpu_load_percent`, `storage_bytes_total/used` por sistema de ficheiros, `memory_bytes_total/used`, `load_average` (1, 5 e 15 minutos, só Net-SNMP) |
| UPS (UPS-MIB) | APC, Eaton, MGE, CyberPower, Riello, Socomec, Tripp Lite, Vertiv, Salicru, e qualquer agente que anuncie UPS-MIB | `ups_battery_status`, `ups_battery_seconds_on`, `ups_battery_minutes_remaining`, `ups_battery_charge_percent`, volts/amperes/temperatura da bateria, volts/hertz/falhas de linha de entrada, `ups_output_source`, volts/hertz/watts de saída, `ups_output_load_percent`, `ups_alarms_present` |
| Printer (PRINTER-MIB) | HP, Kyocera, Epson, Canon, Brother, Lexmark, Ricoh, Xerox, Samsung, OKI, Konica Minolta | `printer_supply_level/max` por consumível, `printer_input_level/max` por bandeja, `printer_pages_printed`, `printer_status` |
| Ethernet switch (EtherLike-MIB, LLDP) | Escolhido manualmente para qualquer comutador; incluído pelos perfis de comutador por fornecedor | Tudo do IF-MIB, mais por porta `ether_fcs_errors`, `ether_alignment_errors`, `ether_symbol_errors`, `ether_late_collisions`, `ether_excessive_collisions`, `ether_mac_transmit_errors`, `ether_mac_receive_errors`, `ether_duplex_status` (1 desconhecido, 2 meio, 3 total), `if_broadcast_packets_in`, `if_multicast_packets_in` (64 bits), e `lldp_neighbor_info` (quem está ligado onde: `remote_sysname`, `remote_port`, `remote_port_desc`; o número da porta local é a segunda parte da etiqueta `index`) |
| Zyxel switch | Todo `sysObjectID` Zyxel (`1.3.6.1.4.1.890`) | Tudo do Ethernet switch, mais `cpu_load_percent` (média de um minuto), `zyxel_cpu_5min_percent`, `memory_usage_percent`, `zyxel_firmware_info` (firmware e modelo como etiquetas), e nos comutadores que publicam o monitor de hardware, `zyxel_temperature_celsius` e `zyxel_temperature_high_threshold_celsius` por sensor, `zyxel_fan_rpm` e `zyxel_fan_low_threshold_rpm` por ventilador |

Os contadores (octetos de interface, páginas impressas) são guardados em
bruto: use `rate()` numa consulta para obter um caudal. Um reinício do
dispositivo mostra-se como uma reposição do contador, nunca como um pico
falso.

As [regras incorporadas](../alerting/rules.md) que se aplicam a dispositivos
SNMP: Monitor unreachable, High CPU, Disk almost full, Filesystem almost full
(forecast), UPS on battery, UPS battery low, Port accumulating errors, Port
flapping, Unusual CPU (baseline).

### Comutadores Zyxel {#zyxel-switches}

A parte padrão do perfil Zyxel (interfaces, erros Ethernet, LLDP) é comum a
todos os comutadores Zyxel. Os valores de CPU, memória e firmware vêm do
ramo privado que os comutadores geridos atuais partilham sob
`1.3.6.1.4.1.890.1.15` (o XS1930 e o GS1900 entre eles); modelos mais antigos
e firewalls Zyxel organizam os seus de forma diferente, e essas séries
simplesmente ficam vazias aí. O perfil foi verificado contra um XS1930-10
real; as suas séries de temperatura e ventilador seguem a MIB do monitor de
hardware publicada pela Zyxel mas ainda não foram vistas em hardware real.

`dot3StatsFrameTooLongs` não é lida deliberadamente: no XS1930 conta bilhões
de jumbo frames perfeitamente saudáveis, e uma série que sobe sempre numa
rede saudável só ensina as pessoas a ignorá-la.

### Controladores de gestão de servidor (BMC) {#server-management-controllers-bmc}

O próprio agente SNMP de um BMC descreve o BMC, não o servidor: o seu
pequeno Linux ARM, a sua memória, as suas ligações de rede. Um BMC Supermicro,
por exemplo, responde como Net-SNMP e recebe o perfil Host resources, que lê
tudo o que esse agente publica sobre si próprio. As ventoinhas, temperaturas,
fontes de alimentação e discos do servidor são lidos via
[Redfish](redfish.md) em vez disso.

## O que preparar no dispositivo {#what-to-prepare-on-the-device}

1. Abra a interface de administração do dispositivo.
2. Procure a secção SNMP, geralmente em «Network», «Services» ou
   «Administration».
3. Ative o SNMP v2c e anote a comunidade só de leitura («public» por
   predefinição em muitos dispositivos).
4. Se o dispositivo filtrar por endereço, permita o endereço do servidor
   DumbMonit.
5. Volte ao DumbMonit, introduza o endereço e a comunidade: o resto é
   detetado automaticamente.

!!! warning
    Uma comunidade não é encriptada na rede. Numa rede partilhada, prefira o
    SNMP v3, que autentica e encripta.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| SNMP v1 / v2c (comunidade) | A comunidade só de leitura. |
| SNMP v3 | Nome de utilizador; autenticação opcional (MD5, SHA-1, SHA-224, SHA-256, SHA-384 ou SHA-512 e uma frase-passe); privacidade opcional (DES, AES-128, AES-192 ou AES-256 e uma frase-passe); nome de contexto opcional. |

Endereço: um IP ou nome de anfitrião, por exemplo `192.168.1.10`. A porta
predefinida é 161; escreva `anfitrião:porta` para usar outra.

## Opções {#options}

Este tipo não lê opções. As etiquetas livres continuam disponíveis e são
copiadas em cada série como `tag_<key>`.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| *Unreachable* logo após adicionar | Comunidade errada, SNMP não ativado, ou o dispositivo só aceita pedidos de endereços permitidos. O dispositivo nunca responde a uma comunidade errada: parece exatamente um dispositivo que está desligado. |
| Só séries `system_*` e `if_*` | O `sysObjectID` não correspondeu a nenhum perfil específico. IF-MIB e System aplicam-se a tudo; HOST-RESOURCES, UPS, PRINTER e Zyxel precisam de um OID de fornecedor conhecido. Num comutador de outra marca, escolha o perfil Ethernet switch manualmente. |
| Sem tráfego de interface | O dispositivo não expõe os contadores de 64 bits `ifXTable`. |
| Análise vazia | A análise tenta uma comunidade em cada endereço; dispositivos com outra comunidade ou só SNMP v3 não respondem. |

Use **Probe now** na página do monitor: reporta o número de amostras e os
nomes de série produzidos por uma verificação.
