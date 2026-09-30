# UPS with NUT

A UPS plugged into a NAS, a Raspberry Pi or a server over USB is usually
watched by Network UPS Tools (NUT): its daemon, `upsd`, publishes what the UPS
says on TCP port 3493. DumbMonit reads that server, like any NUT client: whether
the UPS runs on mains or battery, whether the battery is low or worn out, the
charge, the runtime left, the load, the input voltage and the age of the
battery. One server often publishes several UPS; all of them are read in one
connection, and every series carries the name of its UPS.

For a UPS with its own network card, the [SNMP device](snmp.md) type reads the
standard UPS-MIB instead.

## What it watches

All metrics are prefixed `dumbmonit_nut_` and carry the label `ups`, the UPS
name in `ups.conf` (`ups` on a Synology NAS, `qnapups` on a QNAP). A variable
the UPS does not publish produces no series.

| Metric | What | Labels |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | The flags of `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 when set, 0 otherwise, all of them at every read | `ups` |
| `ups_status_info` | value 1, the raw `ups.status` | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | charge, and the charge under which the UPS reports a low battery | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | runtime left on battery at the present load, and the low-battery runtime | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | time since `battery.date` (installed or replaced), or else `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | mains side; 0 V during an outage | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | load side | `ups` |
| `ups_load_percent` | load, in percent of the UPS capacity | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | power drawn and the rating, when the UPS reports them | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 when the last self-test (`ups.test.result`) reports an error or a warning, 0 when it passed; absent when no test ran | `ups`, `result` |
| `ups_info` | value 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 when `upsd` answers but has no fresh data from the UPS (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 when the NUT driver of that UPS is not running | `ups` |
| `server_ups` | number of UPS the server publishes | |
| `server_info` | value 1 | `version` |

Serial numbers are never put in a label. At most 32 UPS are read per server.

The [built-in rules](../alerting/rules.md#ups-behind-nut) that apply: UPS on
battery, UPS battery low, UPS battery needs replacing, UPS load high, UPS
runtime short, UPS data stale, plus Device unreachable when the NUT server
itself stops answering.

## The device page

The panel above the charts reads what the probe stored; opening the page never
connects to the NUT server. One card per UPS: its state in words (on mains, on
battery, low battery, replace battery, bypass, overload…), the charge, the
runtime left, the load, the input voltage and the age of the battery.

## Let DumbMonit read the NUT server

1. Find the NUT server: the machine the UPS is plugged into (a NAS, a Raspberry Pi, a server) runs upsd on TCP port 3493. From the DumbMonit host, check that it answers and note the names of its UPS.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    With the NUT client tools installed, `upsc -l nas.lan` gives the same list.

2. On a plain NUT install, make upsd listen on an address the DumbMonit host can reach: add this line to upsd.conf, open TCP port 3493 in the firewall for the DumbMonit host, and restart upsd. Reading needs no account: upsd lets any host it accepts read the variables, and changes nothing without a user that has the right to.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. On a NAS, the same switch has another name. Synology DSM: Control Panel → Hardware & Power → UPS, tick Enable network UPS server, and add the DumbMonit host under Permitted DiskStation Devices; the UPS is named ups. QNAP: the UPS page of the Control Panel, Enable network UPS master, with the DumbMonit host among the allowed addresses; the UPS is named qnapups. TrueNAS: the UPS service with Remote Monitor ticked.

4. Optional: DumbMonit sends a user name and password only if you enter one, and upsd does not ask for any to read. If your policy wants every client named, add a user without actions or instcmds to upsd.users, with a long random password, and restart upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. In DumbMonit, enter the address of the NUT server, for example "nas.lan". Every UPS it publishes is watched; to watch only some, list their names in the UPS option.

6. DumbMonit only reads: it sends LIST UPS and LIST VAR, never LOGIN, SET, INSTCMD or FSD, so the server never counts it as a secondary to wait for before shutting down.

!!! warning
    DumbMonit watches the UPS; it shuts nothing down. The machines the UPS powers still need their own NUT client (upsmon) to shut down cleanly when the battery runs low.

## Credentials

| Credential | Fields |
|---|---|
| No authentication | The default: `upsd` lets any host it accepts read the UPS variables. |
| upsd user name / password | Sent with `USERNAME` and `PASSWORD` before reading. `upsd` only checks them for commands, which DumbMonit never sends. |

Address: a host name or IP (`nas.lan`), `host:port`, or the `ups@host` form of
`upsc`, of which only the host is kept. Options: port (3493), the UPS to watch
(all by default) and the timeout per exchange (10 s).

A refused connection or no answer counts as the device being unreachable.
`ACCESS-DENIED` is shown as a credential error, and a UPS name the server does
not publish as a configuration error; neither wakes anyone at night. The
connection is plain TCP: `STARTTLS` is not used, and nothing read is secret.
