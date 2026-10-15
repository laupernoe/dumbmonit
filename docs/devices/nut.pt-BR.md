# UPS com NUT {#ups-with-nut}

Um UPS conectado a um NAS, um Raspberry Pi ou um servidor via USB geralmente
é observado pelo Network UPS Tools (NUT): seu daemon, `upsd`, publica o que o
UPS informa na porta TCP 3493. O DumbMonit lê esse servidor, como qualquer
cliente NUT: se o UPS roda na rede elétrica ou na bateria, se a bateria está
baixa ou desgastada, a carga, o tempo de execução restante, a carga de
trabalho, a tensão de entrada e a idade da bateria. Um servidor
frequentemente publica vários UPS; todos são lidos em uma conexão, e cada
série carrega o nome de seu UPS.

Para um UPS com sua própria placa de rede, o tipo [dispositivo SNMP](snmp.md)
lê a UPS-MIB padrão em vez disso.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nut_` e carregam o rótulo `ups`, o
nome do UPS em `ups.conf` (`ups` em um NAS Synology, `qnapups` em um QNAP).
Uma variável que o UPS não publica não produz série.

| Métrica | O que é | Rótulos |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | As flags de `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 quando definida, 0 caso contrário, todas presentes a cada leitura | `ups` |
| `ups_status_info` | valor 1, o `ups.status` bruto | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | carga, e a carga sob a qual o UPS reporta bateria baixa | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | tempo restante na bateria na carga de trabalho atual, e o tempo de bateria baixa | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | tempo desde `battery.date` (instalada ou substituída), ou senão `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | lado da rede elétrica; 0 V durante uma queda | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | lado da carga | `ups` |
| `ups_load_percent` | carga de trabalho, em porcentagem da capacidade do UPS | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | energia consumida e a nominal, quando o UPS as reporta | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 quando o último autoteste (`ups.test.result`) reporta um erro ou aviso, 0 quando passou; ausente quando nenhum teste rodou | `ups`, `result` |
| `ups_info` | valor 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 quando `upsd` responde mas não tem dados recentes do UPS (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 quando o driver NUT daquele UPS não está em execução | `ups` |
| `server_ups` | número de UPS que o servidor publica | |
| `server_info` | valor 1 | `version` |

Números de série nunca são colocados em um rótulo. No máximo 32 UPS são
lidos por servidor.

As [regras integradas](../alerting/rules.md#ups-behind-nut) que se aplicam:
UPS on battery, UPS battery low, UPS battery needs replacing, UPS load high,
UPS runtime short, UPS data stale, além de Monitor unreachable quando o
próprio servidor NUT para de responder.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca se conecta ao servidor NUT. Um cartão por UPS: seu estado em palavras
(na rede elétrica, na bateria, bateria baixa, substituir bateria, bypass,
sobrecarga…), a carga, o tempo restante, a carga de trabalho, a tensão de
entrada e a idade da bateria.

## Deixar o DumbMonit ler o servidor NUT {#let-dumbmonit-read-the-nut-server}

1. Encontre o servidor NUT: a máquina em que o UPS está conectado (um NAS, um
   Raspberry Pi, um servidor) executa o upsd na porta TCP 3493. A partir do
   host DumbMonit, verifique que ele responde e anote os nomes de seus UPS.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Com as ferramentas de cliente NUT instaladas, `upsc -l nas.lan` dá a
    mesma lista.

2. Em uma instalação NUT simples, faça o upsd escutar em um endereço que o
   host DumbMonit possa alcançar: adicione esta linha ao upsd.conf, abra a
   porta TCP 3493 no firewall para o host DumbMonit, e reinicie o upsd. A
   leitura não precisa de conta: o upsd permite que qualquer host que ele
   aceite leia as variáveis, e nada é alterado sem um usuário que tenha
   direito a isso.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Em um NAS, o mesmo botão tem outro nome. Synology DSM: Control Panel →
   Hardware & Power → UPS, marque Enable network UPS server, e adicione o
   host DumbMonit em Permitted DiskStation Devices; o UPS se chama ups. QNAP:
   a página UPS do Control Panel, Enable network UPS master, com o host
   DumbMonit entre os endereços permitidos; o UPS se chama qnapups. TrueNAS:
   o serviço UPS com Remote Monitor marcado.

4. Opcional: o DumbMonit envia um nome de usuário e senha só se você informar
   um, e o upsd não pede nenhum para leitura. Se sua política exige que todo
   cliente seja nomeado, adicione um usuário sem ações ou instcmds ao
   upsd.users, com uma senha longa e aleatória, e reinicie o upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. No DumbMonit, informe o endereço do servidor NUT, por exemplo "nas.lan".
   Todo UPS que ele publica é observado; para observar apenas alguns, liste
   seus nomes na opção UPS.

6. O DumbMonit só lê: envia LIST UPS e LIST VAR, nunca LOGIN, SET, INSTCMD ou
   FSD, então o servidor nunca o conta como um secundário a esperar antes de
   desligar.

!!! warning
    O DumbMonit observa o UPS; ele não desliga nada. As máquinas que o UPS
    alimenta ainda precisam de seu próprio cliente NUT (upsmon) para desligar
    corretamente quando a bateria ficar baixa.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem autenticação | O padrão: `upsd` permite que qualquer host que aceite leia as variáveis do UPS. |
| Nome de usuário / senha do upsd | Enviados com `USERNAME` e `PASSWORD` antes da leitura. `upsd` só os verifica para comandos, que o DumbMonit nunca envia. |

Endereço: um nome de host ou IP (`nas.lan`), `host:porta`, ou a forma
`ups@host` do `upsc`, da qual só o host é mantido. Opções: porta (3493), o
UPS a observar (todos por padrão) e o tempo limite por troca (10 s).

Uma conexão recusada ou sem resposta conta como o monitor estando
inacessível. `ACCESS-DENIED` é mostrado como um erro de credencial, e um nome
de UPS que o servidor não publica como um erro de configuração; nenhum dos
dois acorda ninguém à noite. A conexão é TCP simples: `STARTTLS` não é usado,
e nada lido é secreto.
