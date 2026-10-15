# UPS com NUT {#ups-with-nut}

Uma UPS ligada por USB a um NAS, a um Raspberry Pi ou a um servidor é
normalmente vigiada pelo Network UPS Tools (NUT): o seu daemon, `upsd`,
publica o que a UPS diz na porta TCP 3493. O DumbMonit lê esse servidor, como
qualquer cliente NUT: se a UPS funciona pela rede elétrica ou pela bateria, se
a bateria está fraca ou gasta, a carga, a autonomia que resta, a carga ligada,
a tensão de entrada e a idade da bateria. Um servidor publica muitas vezes
várias UPS; todas são lidas numa só ligação, e cada série traz o nome da sua UPS.

Para uma UPS com placa de rede própria, o tipo de [dispositivo SNMP](snmp.md)
lê antes a UPS-MIB padrão.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nut_` e trazem a etiqueta `ups`, o
nome da UPS em `ups.conf` (`ups` num NAS Synology, `qnapups` num QNAP). Uma
variável que a UPS não publica não produz série.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | Os indicadores de `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 quando ativo, 0 caso contrário, todos em cada leitura | `ups` |
| `ups_status_info` | valor 1, o `ups.status` em bruto | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | carga, e a carga abaixo da qual a UPS reporta bateria fraca | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | autonomia restante na bateria com a carga atual, e a autonomia de bateria fraca | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | tempo desde `battery.date` (instalada ou substituída), ou então `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | lado da rede elétrica; 0 V durante um corte | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | lado da carga | `ups` |
| `ups_load_percent` | carga, em percentagem da capacidade da UPS | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | potência consumida e a potência nominal, quando a UPS as reporta | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 quando o último autoteste (`ups.test.result`) reporta um erro ou um aviso, 0 quando passou; ausente quando nenhum teste correu | `ups`, `result` |
| `ups_info` | valor 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 quando o `upsd` responde mas não tem dados recentes da UPS (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 quando o driver NUT dessa UPS não está a correr | `ups` |
| `server_ups` | número de UPS que o servidor publica | |
| `server_info` | valor 1 | `version` |

Os números de série nunca são postos numa etiqueta. São lidas no máximo 32 UPS
por servidor.

As [regras incorporadas](../alerting/rules.md#ups-behind-nut) que se aplicam:
UPS on battery, UPS battery low, UPS battery needs replacing, UPS load high,
UPS runtime short, UPS data stale, mais Monitor unreachable quando o próprio
servidor NUT deixa de responder.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca se
liga ao servidor NUT. Um cartão por UPS: o seu estado por palavras (na rede
elétrica, na bateria, bateria fraca, substituir bateria, bypass, sobrecarga…),
a carga, a autonomia restante, a carga ligada, a tensão de entrada e a idade da
bateria.

## Deixar o DumbMonit ler o servidor NUT {#let-dumbmonit-read-the-nut-server}

1. Encontre o servidor NUT: a máquina à qual a UPS está ligada (um NAS, um Raspberry Pi, um servidor) corre o upsd na porta TCP 3493. A partir do anfitrião do DumbMonit, verifique que responde e anote os nomes das suas UPS.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Com as ferramentas de cliente NUT instaladas, `upsc -l nas.lan` dá a mesma lista.

2. Numa instalação NUT simples, faça o upsd escutar num endereço que o anfitrião do DumbMonit alcance: adicione esta linha ao upsd.conf, abra a porta TCP 3493 na firewall para o anfitrião do DumbMonit, e reinicie o upsd. Ler não precisa de conta: o upsd deixa qualquer anfitrião que aceite ler as variáveis, e não altera nada sem um utilizador com direito para isso.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Num NAS, o mesmo interruptor tem outro nome. Synology DSM: Painel de Controlo → Hardware e Energia → UPS, marque Ativar servidor UPS de rede, e adicione o anfitrião do DumbMonit em Monitores DiskStation Permitidos; a UPS chama-se ups. QNAP: a página UPS do Painel de Controlo, Enable network UPS master, com o anfitrião do DumbMonit entre os endereços permitidos; a UPS chama-se qnapups. TrueNAS: o serviço UPS com Remote Monitor marcado.

4. Opcional: o DumbMonit só envia nome de utilizador e palavra-passe se introduzir um, e o upsd não pede nenhum para ler. Se a sua política quer todos os clientes identificados, adicione ao upsd.users um utilizador sem actions nem instcmds, com uma palavra-passe longa e aleatória, e reinicie o upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. No DumbMonit, introduza o endereço do servidor NUT, por exemplo «nas.lan». Todas as UPS que publica são vigiadas; para vigiar só algumas, liste os seus nomes na opção UPS.

6. O DumbMonit só lê: envia LIST UPS e LIST VAR, nunca LOGIN, SET, INSTCMD ou FSD, pelo que o servidor nunca o conta como secundário a esperar antes de se desligar.

!!! warning
    O DumbMonit vigia a UPS; não desliga nada. As máquinas que a UPS alimenta continuam a precisar do seu próprio cliente NUT (upsmon) para se desligarem de forma limpa quando a bateria está a acabar.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem autenticação | A predefinição: o `upsd` deixa qualquer anfitrião que aceite ler as variáveis da UPS. |
| Nome de utilizador / palavra-passe do upsd | Enviados com `USERNAME` e `PASSWORD` antes de ler. O `upsd` só os verifica para comandos, que o DumbMonit nunca envia. |

Endereço: um nome de anfitrião ou IP (`nas.lan`), `host:port`, ou a forma
`ups@host` do `upsc`, da qual só o anfitrião é mantido. Opções: porta (3493), a
UPS a vigiar (todas por predefinição) e o tempo limite por troca (10 s).

Uma ligação recusada ou a ausência de resposta conta como o monitor estar
inacessível. Quando a máquina que corre o servidor NUT se desliga no corte que
reporta (é muitas vezes o seu próprio upsmon primário), os alertas da UPS ficam
ocultos como suprimidos enquanto Monitor unreachable dispara para esse mesmo
monitor: leia o alerta Monitor unreachable como o sinal de que a falha
de energia venceu. Corra o servidor NUT numa máquina que sobreviva às outras, ou
vigie a UPS a partir de um segundo servidor NUT. `ACCESS-DENIED` é mostrado como
erro de credencial, e um nome de UPS que o servidor não publica como erro de
configuração; nenhum dos dois acorda ninguém de noite. A ligação é TCP simples:
`STARTTLS` não é usado, e nada do que é lido é secreto.
