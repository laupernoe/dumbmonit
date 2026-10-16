# Unraid

Ein Unraid-Server über seine GraphQL-API: ein nicht gestartetes Array,
deaktivierte, fehlende oder Lesefehler meldende Platten,
Paritätsprüfungen, die Fehler fanden oder seit Wochen nicht liefen,
sich füllende Cache-Pools, Container, die laufen sollten und nicht
laufen, und der Zustand der VMs.

DumbMonit sendet einen API-Key mit der Viewer-Rolle im Header
`x-api-key` an `/graphql`. Die Viewer-Rolle liest alles und kann nichts
ändern. Die API ist ab 7.2 in Unraid eingebaut; auf 6.12 bis 7.1 kommt
sie mit dem Unraid-Connect-Plugin.

Jede Minute sendet DumbMonit eine erforderliche Abfrage (Version,
Array, Parität, Daten- und Cache-Platten) und vier optionale, jede für
sich, sodass ein Feld, das eine ältere API-Version nicht hat, oder
ausgeschaltete VMs, nur diesen Teil weglässt: die Paritätsprüfung, die
Docker-Container, die VMs und die ungelesenen Benachrichtigungen.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus Unraids API-Dokumentation und dem
    veröffentlichten GraphQL-Schema der Unraid-API gebaut (Feldnamen,
    Enumerationen, Größen in Kilobyte), nicht gegen einen laufenden
    Server. Jede Abfrage ist ein Lesevorgang. Sagen Sie uns, was kaputt
    ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_unraid_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `version_info` | `version` | Wert 1: die Unraid-Version. |
| `array_started` | | 1, wenn das Array gestartet ist. |
| `array_state_info` | `state` | Wert 1: `started`, `stopped`, `new_array`, `recon_disk`, `disable_disk`… |
| `array_size_bytes`, `array_used_bytes`, `array_free_bytes`, `array_used_percent` | | Array-Kapazität. |
| `disk_ok` | `disk`, `role` (`parity`, `data`, `cache`) | 1, wenn der Status der Platte `DISK_OK` ist. Leere Steckplätze werden ausgelassen. |
| `disk_status_info` | `disk`, `role`, `status` | Wert 1: Unraids Status (`disk_ok`, `disk_dsbl`, `disk_np_missing`, `disk_invalid`…). |
| `disk_errors` | `disk`, `role` | Lesefehler seit Array-Start (die Errors-Spalte). |
| `disk_temperature_celsius` | `disk`, `role` | Nur während die Platte dreht: Unraid meldet keine für eine schlafende Platte. |
| `disk_spinning` | `disk`, `role` | 1, wenn die Platte dreht. |
| `disk_size_bytes`, `disk_fs_size_bytes`, `disk_used_percent` | `disk`, `role` | Größe, Dateisystemgröße und Füllstand, für Platten mit einem Dateisystem. |
| `disks`, `disks_problem`, `disks_with_errors` | | Zählungen. |
| `parity_check_status_info` | `status` | Wert 1: `completed`, `running`, `paused`, `cancelled`, `failed`, `never_run`. |
| `parity_check_running`, `parity_check_progress_percent` | | Eine laufende Prüfung. |
| `parity_check_ok` | | Nach einer abgeschlossenen Prüfung: 1 ohne Sync-Fehler, 0 mit Fehlern oder wenn sie fehlschlug. Eine abgebrochene Prüfung beweist nichts und setzt nichts. |
| `parity_check_errors`, `parity_check_age_seconds`, `parity_check_duration_seconds` | | Die letzte Prüfung. |
| `containers`, `containers_running`, `containers_autostart_stopped` | | Docker-Container. |
| `container_running`, `container_autostart_stopped` | `container` | Pro Container; das zweite ist 1, wenn ein zum Start mit dem Array vorgesehener Container nicht läuft. |
| `vms`, `vms_running` | | VMs. |
| `vm_state_info`, `vm_crashed` | `vm` (und `state`) | Pro VM. |
| `notifications_unread` | `importance`: `alert`, `warning`, `info` | Ungelesene Benachrichtigungen. |

Die geltenden [eingebauten Regeln](../alerting/rules.md#unraid):

- **Unraid array stopped**: das Array ist seit zehn Minuten nicht
  gestartet (Warning).
- **Unraid disk disabled or missing**: der Status einer Platte ist seit
  fünf Minuten nicht `DISK_OK` (Warning). Das Array läuft degradiert.
- **Unraid disk read errors**: eine Platte meldet Lesefehler
  (Advisory).
- **Unraid disk too hot**: eine Daten- oder Paritätsplatte seit
  fünfzehn Minuten über 55 °C (Advisory). Cache-Platten, oft NVMe,
  werden ausgelassen.
- **Unraid parity check found errors** (Advisory).
- **Unraid parity check overdue**: die letzte Prüfung ist mehr als 40
  Tage alt (Advisory).
- **Unraid cache nearly full**: eine Cache-Platte seit dreißig Minuten
  mehr als 90 % voll (Advisory).
- **Unraid container stopped**: ein zum Start mit dem Array
  vorgesehener Container läuft seit zehn Minuten nicht (Advisory).

## Die Monitorseite

Das Array-Panel sagt in einem Satz, was nicht stimmt, zeigt den
Array-Füllstand, die letzte Paritätsprüfung, die laufenden Container
und VMs, listet dann jede Platte — Parität zuerst, dann Daten, dann
Cache — mit ihrem Status, Temperatur und Füllstand, und die Container,
die laufen sollten.

## Einen nur-lesenden API-Key für DumbMonit erstellen

1. Auf Unraid 7.2 oder später ist die API eingebaut. Auf Unraid 6.12
   bis 7.1 installieren Sie zuerst das Unraid-Connect-Plugin aus dem
   Apps-Tab: Es bringt dieselbe API, und für lokale Nutzung ist kein
   Unraid-Connect-Login nötig.

2. Erstellen Sie einen API-Key mit der Viewer-Rolle, die alles liest
   und nichts ändern kann: unter Settings > Management Access > API
   Keys, oder vom Unraid-Terminal.

    ```
    unraid-api apikey --create --name dumbmonit --roles VIEWER --description "DumbMonit monitoring" --json
    ```

3. Kopieren Sie den Key. Geben Sie in DumbMonit die Adresse des
   Unraid-Servers ein, zum Beispiel "tower.lan", und fügen Sie den Key
   als API-Key ein. Wird die Web-Oberfläche über HTTPS bedient, wählen
   Sie https als Protokoll.

!!! warning
    Unraid meldet keine Temperatur für eine Platte, die
    herunterdreht, und DumbMonit weckt nie eine: Eine schlafende Platte
    hat einfach keine Temperatur, bis sie wieder dreht.

## Credentials

| Credential | Felder |
|---|---|
| API-Key | Ein Key mit der Viewer-Rolle, als `x-api-key` gesendet. |

Adresse: der Hostname oder die IP des Unraid-Servers, `host:port`, oder
eine URL. Optionen:

- **Protocol** (`http`) und **Port** (80): die der Web-Oberfläche. Mit
  SSL auf Strict unter Settings > Management Access, nutzen Sie
  `https`, und die `myunraid.net`-Adresse, wenn das Zertifikat diese
  ist.
- **Accept an unverifiable certificate**: für ein selbstsigniertes
  Zertifikat.
- **Timeout pro Request** (15 s).

## Fehlerbehebung

"Unraid refused the API key (401)": der Key ist falsch oder wurde
gelöscht.

"Unraid accepted the API key but refused to answer (403)": der Key hat
keine Rolle, oder eine engere als Viewer.

Keine Container-, VM- oder Paritätszahl: diese optionale Abfrage ist
fehlgeschlagen (VMs ausgeschaltet, eine ältere API-Version ohne das
Feld); der Rest wird trotzdem gelesen. Der Grund steht im Server-Log
auf Debug-Ebene.
