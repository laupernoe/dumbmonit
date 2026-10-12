# UPS com NUT {#ups-with-nut}

Um UPS ligado a um NAS, um Raspberry Pi ou um servidor via USB é geralmente
monitorizado pelo Network UPS Tools (NUT): o seu daemon, `upsd`, publica o
que o UPS diz na porta TCP 3493. O DumbMonit lê esse servidor, como qualquer
cliente NUT: se o UPS funciona com corrente elétrica ou com bateria, se a
bateria está baixa ou gasta, a carga, o tempo de autonomia restante, a carga
de trabalho, a tensão de entrada e a idade da bateria. Um servidor
frequentemente publica vários UPS; todos são lidos numa única ligação, e
cada série transporta o nome do seu UPS.

Para um UPS com a sua própria placa de rede, o tipo [dispositivo
SNMP](snmp.md) lê a UPS-MIB padrão em vez disso.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nut_` e transportam a etiqueta
`ups`, o nome do UPS em `ups.conf` (`ups` num NAS Synology, `qnapups` num
QNAP). Uma variável que o UPS não publica não produz série.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | As flags de `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 quando ativa, 0 caso contrário, todas de cada vez em cada leitura | `ups` |
| `ups_status_info` | valor 1, o `ups.status` em bruto | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | carga, e a carga sob a qual o UPS reporta bateria baixa | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | autonomia restante na bateria à carga de trabalho atual, e a autonomia de bateria baixa | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | tempo desde `battery.date` (instalada ou substituída), ou senão `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | lado da rede elétrica; 0 V durante um corte | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | lado da carga | `ups` |
| `ups_load_percent` | carga de trabalho, em percentagem da capacidade do UPS | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | energia consumida e a capacidade nominal, quando o UPS as reporta | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 quando o último autoteste (`ups.test.result`) reporta um erro ou aviso, 0 quando passou; ausente quando nenhum teste correu | `ups`, `result` |
| `ups_info` | valor 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 quando `upsd` responde mas não tem dados recentes do UPS (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 quando o driver NUT desse UPS não está em execução | `ups` |
| `server_ups` | número de UPS que o servidor publica | |
| `server_info` | valor 1 | `version` |

Números de série nunca são colocados numa etiqueta. No máximo 32 UPS são
lidos por servidor.

As [regras incorporadas](../alerting/rules.md#ups-behind-nut) que se aplicam:
UPS on battery, UPS battery low, UPS battery needs replacing, UPS load high,
UPS runtime short, UPS data stale, mais Device unreachable quando o próprio
servidor NUT deixa de responder.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca liga ao servidor NUT. Um cartão por UPS: o seu estado em palavras (com
corrente elétrica, com bateria, bateria baixa, substituir bateria, bypass,
sobrecarga…), a carga, a autonomia restante, a carga de trabalho, a tensão
de entrada e a idade da bateria.

## Deixar o DumbMonit ler o servidor NUT {#let-dumbmonit-read-the-nut-server}

1. Encontre o servidor NUT: a máquina a que o UPS está ligado (um NAS, um Raspberry Pi, um servidor) executa o upsd na porta TCP 3493. A partir do anfitrião do DumbMonit, verifique que responde e anote os nomes dos seus UPS.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Com as ferramentas de cliente NUT instaladas, `upsc -l nas.lan` dá a mesma lista.

2. Numa instalação NUT simples, faça o upsd escutar num endereço que o anfitrião do DumbMonit possa alcançar: adicione esta linha ao upsd.conf, abra a porta TCP 3493 na firewall para o anfitrião do DumbMonit, e reinicie o upsd. A leitura não precisa de nenhuma conta: o upsd deixa qualquer anfitrião que aceite ler as variáveis, e não altera nada sem um utilizador que tenha o direito de o fazer.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Num NAS, o mesmo interruptor tem outro nome. Synology DSM: Control Panel → Hardware & Power → UPS, marque Enable network UPS server, e adicione o anfitrião do DumbMonit em Permitted DiskStation Devices; o UPS chama-se ups. QNAP: a página UPS do Control Panel, Enable network UPS master, com o anfitrião do DumbMonit entre os endereços permitidos; o UPS chama-se qnapups. TrueNAS: o serviço UPS com Remote Monitor marcado.

4. Opcional: o DumbMonit só envia um nome de utilizador e senha se introduzir um, e o upsd não pede nenhum para ler. Se a sua política exigir que todo cliente seja nomeado, adicione um utilizador sem actions nem instcmds a upsd.users, com uma senha longa e aleatória, e reinicie o upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. No DumbMonit, introduza o endereço do servidor NUT, por exemplo «nas.lan». Todos os UPS que publica são monitorizados; para monitorizar só alguns, liste os seus nomes na opção UPS.

6. O DumbMonit só lê: envia LIST UPS e LIST VAR, nunca LOGIN, SET, INSTCMD ou FSD, pelo que o servidor nunca o conta como um secundário a esperar antes de encerrar.

!!! warning
    O DumbMonit monitoriza o UPS; não encerra nada. As máquinas que o UPS alimenta continuam a precisar do seu próprio cliente NUT (upsmon) para encerrar corretamente quando a bateria esgota.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem autenticação | A predefinição: o `upsd` deixa qualquer anfitrião que aceite ler as variáveis do UPS. |
| Nome de utilizador / senha do upsd | Enviado com `USERNAME` e `PASSWORD` antes de ler. O `upsd` só os verifica para comandos, que o DumbMonit nunca envia. |

Endereço: um nome de anfitrião ou IP (`nas.lan`), `anfitrião:porta`, ou a
forma `ups@host` do `upsc`, da qual só o anfitrião é mantido. Opções: porta
(3493), o UPS a monitorizar (todos por predefinição) e o tempo limite por
troca (10 s).

Uma ligação recusada ou nenhuma resposta conta como o dispositivo estar
inacessível. `ACCESS-DENIED` é mostrado como um erro de credencial, e um
nome de UPS que o servidor não publica como um erro de configuração; nenhum
dos dois acorda ninguém de noite. A ligação é TCP simples: `STARTTLS` não é
usado, e nada que é lido é secreto.
