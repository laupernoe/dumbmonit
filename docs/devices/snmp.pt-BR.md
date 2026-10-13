# Dispositivo SNMP {#snmp-device}

O tipo mais universal: quase todo equipamento de rede fala SNMP. Switches,
roteadores, NAS, nobreaks e impressoras todos qualificam. SNMP v1, v2c e v3
são suportados.

## O que ele observa {#what-it-watches}

O perfil de coleta é detectado automaticamente a partir do `sysObjectID` do
dispositivo (e, para nobreaks, de seu `sysDescr`). Sete perfis acompanham o
produto; um dispositivo pode corresponder a vários, e os perfis se incluem
uns aos outros.

| Perfil | Aplica-se a | Métricas (prefixo `dumbmonit_`) |
|---|---|---|
| Sistema (SNMPv2-MIB) | Todo dispositivo | `system_uptime_seconds`, `system_info` (nome, descrição, localização, contato como rótulos) |
| Interfaces de rede (IF-MIB) | Todo dispositivo, como fallback | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out`, `if_discards_in/out`, `if_admin_status`, `if_oper_status`, `if_speed_bps`, por interface |
| Recursos de host (HOST-RESOURCES-MIB) | Net-SNMP (Linux, BSD, macOS), o agente SNMP do Windows, Synology, QNAP, Solaris | `host_uptime_seconds`, `host_processes`, `host_users`, `cpu_load_percent`, `storage_bytes_total/used` por sistema de arquivos, `memory_bytes_total/used`, `load_average` (1, 5 e 15 minutos, somente Net-SNMP) |
| Nobreak (UPS-MIB) | APC, Eaton, MGE, CyberPower, Riello, Socomec, Tripp Lite, Vertiv, Salicru, e qualquer agente que anuncie UPS-MIB | `ups_battery_status`, `ups_battery_seconds_on`, `ups_battery_minutes_remaining`, `ups_battery_charge_percent`, volts/amperes/temperatura da bateria, volts/hertz de entrada/falhas de linha, `ups_output_source`, volts/hertz/watts de saída, `ups_output_load_percent`, `ups_alarms_present` |
| Impressora (PRINTER-MIB) | HP, Kyocera, Epson, Canon, Brother, Lexmark, Ricoh, Xerox, Samsung, OKI, Konica Minolta | `printer_supply_level/max` por suprimento, `printer_input_level/max` por bandeja, `printer_pages_printed`, `printer_status` |
| Switch Ethernet (EtherLike-MIB, LLDP) | Escolhido manualmente para qualquer switch; incluído pelos perfis de switch de fabricantes | Tudo do IF-MIB, mais por porta `ether_fcs_errors`, `ether_alignment_errors`, `ether_symbol_errors`, `ether_late_collisions`, `ether_excessive_collisions`, `ether_mac_transmit_errors`, `ether_mac_receive_errors`, `ether_duplex_status` (1 desconhecido, 2 meio, 3 total), `if_broadcast_packets_in`, `if_multicast_packets_in` (64 bits), e `lldp_neighbor_info` (quem está conectado onde: `remote_sysname`, `remote_port`, `remote_port_desc`; o número da porta local é a segunda parte do rótulo `index`) |
| Switch Zyxel | Todo `sysObjectID` Zyxel (`1.3.6.1.4.1.890`) | Tudo do switch Ethernet, mais `cpu_load_percent` (média de um minuto), `zyxel_cpu_5min_percent`, `memory_usage_percent`, `zyxel_firmware_info` (firmware e modelo como rótulos), e em switches que publicam o monitor de hardware, `zyxel_temperature_celsius` e `zyxel_temperature_high_threshold_celsius` por sensor, `zyxel_fan_rpm` e `zyxel_fan_low_threshold_rpm` por ventoinha |

Contadores (octetos de interface, páginas impressas) são armazenados em
bruto: use `rate()` em uma consulta para obter uma vazão. Uma reinicialização
do dispositivo aparece como um reset de contador, nunca como um pico falso.

As [regras integradas](../alerting/rules.md) que se aplicam a dispositivos
SNMP: Device unreachable, High CPU, Disk almost full, Filesystem almost full
(forecast), UPS on battery, UPS battery low, Port accumulating errors, Port
flapping, Unusual CPU (baseline).

### Switches Zyxel {#zyxel-switches}

A parte padrão do perfil Zyxel (interfaces, erros Ethernet, LLDP) é comum a
todo switch Zyxel. Os números de CPU, memória e firmware vêm do ramo privado
que os switches gerenciados atuais compartilham sob `1.3.6.1.4.1.890.1.15`
(o XS1930 e o GS1900 entre eles); modelos mais antigos e firewalls Zyxel
organizam os seus de forma diferente, e essas séries simplesmente ficam
vazias ali. O perfil foi verificado contra um XS1930-10 real; suas séries de
temperatura e ventoinha seguem a MIB de monitor de hardware publicada pela
Zyxel mas ainda não foram vistas em hardware real.

`dot3StatsFrameTooLongs` deliberadamente não é lido: no XS1930 ele conta
bilhões de quadros jumbo perfeitamente saudáveis, e uma série que sempre sobe
em uma rede saudável só ensina as pessoas a ignorá-la.

### Controladores de gerenciamento de servidor (BMC) {#server-management-controllers-bmc}

O próprio agente SNMP de um BMC descreve o BMC, não o servidor: seu pequeno
Linux ARM, sua memória, seus links de rede. Um BMC Supermicro, por exemplo,
responde como Net-SNMP e recebe o perfil Recursos de host, que lê tudo que
esse agente publica sobre si mesmo. As ventoinhas, temperaturas, fontes de
alimentação e discos do servidor são lidos via [Redfish](redfish.md) em vez
disso.

## O que preparar no dispositivo {#what-to-prepare-on-the-device}

1. Abra a interface de administração do dispositivo.
2. Procure a seção SNMP, geralmente sob "Rede", "Serviços" ou
   "Administração".
3. Ative o SNMP v2c e anote a comunidade somente leitura ("public" por
   padrão em muitos dispositivos).
4. Se o dispositivo filtra por endereço, permita o endereço do servidor
   DumbMonit.
5. Volte ao DumbMonit, informe o endereço e a comunidade: o resto é
   detectado automaticamente.

!!! warning
    Uma comunidade não é criptografada na rede. Em uma rede compartilhada,
    prefira SNMP v3, que autentica e criptografa.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| SNMP v1 / v2c (comunidade) | A comunidade somente leitura. |
| SNMP v3 | Nome de usuário; autenticação opcional (MD5, SHA-1, SHA-224, SHA-256, SHA-384 ou SHA-512 e uma frase secreta); privacidade opcional (DES, AES-128, AES-192 ou AES-256 e uma frase secreta); nome de contexto opcional. |

Endereço: um IP ou nome de host, por exemplo `192.168.1.10`. A porta padrão
é 161; escreva `host:porta` para usar outra.

## Opções {#options}

Este tipo não lê opções. Tags livres ainda estão disponíveis e são copiadas
em toda série como `tag_<key>`.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| *Inacessível* logo após adicionar | Comunidade errada, SNMP não ativado, ou o dispositivo só aceita requisições de endereços permitidos. O dispositivo nunca responde a uma comunidade errada: parece exatamente um dispositivo desligado. |
| Apenas séries `system_*` e `if_*` | O `sysObjectID` não correspondeu a nenhum perfil específico. IF-MIB e System se aplicam a tudo; HOST-RESOURCES, UPS, PRINTER e Zyxel precisam de um OID de fabricante conhecido. Em um switch de outra marca, escolha o perfil Switch Ethernet manualmente. |
| Nenhum tráfego de interface | O dispositivo não expõe os contadores de 64 bits `ifXTable`. |
| Varredura vazia | A varredura tenta uma comunidade em cada endereço; dispositivos com outra comunidade ou apenas SNMP v3 não respondem. |

Use **Sondar agora** na página do dispositivo: ela reporta o número de
amostras e os nomes de série produzidos por uma sondagem.
