# VMware vSphere {#vmware-vsphere}

Ein vCenter Server und die von ihm verwalteten Hosts oder ein eigenständiger ESXi-Host: Hosts,
die nicht mehr antworten, im Wartungsmodus belassen wurden oder einen roten Zustand melden,
virtuelle Maschinen und ihre VMware Tools, sich füllende oder unzugängliche Datastores und die
von vSphere selbst ausgelösten Alarme.

DumbMonit liest vSphere mit einem dedizierten Benutzer mit der eingebauten Rolle
**Nur Lesen**. Es schaltet nie eine VM ein oder aus, wechselt nie in den Wartungsmodus und
bestätigt nie einen Alarm.

Der Fehler, auf den es ankommt: **ein Datastore, der sich füllt**. Erreicht ein Thin-provisioned
Datastore 100 %, werden alle VMs, die darauf schreiben müssen, auf einmal angehalten. Die von
vCenter ausgelösten Alarme sagen es auch, aber nur dem, der den Client geöffnet hat.

## Warum die Web-Services-API {#why-the-web-services-api}

DumbMonit spricht die vSphere-Web-Services-API (SOAP, `/sdk`), nicht die REST-API
(`/api/vcenter/*`): Ein eigenständiger ESXi-Host hat keine REST-API, und bei einem vCenter
verrät die REST-API weder den Wartungsmodus, den Gesamtzustand (`overallStatus`), den Zustand der
VMware Tools noch die ausgelösten Alarme. Eine Sicht auf das gesamte Inventar wird in einem
Durchgang, Seite für Seite, gelesen und am Ende freigegeben.

Die Sitzung wird von einer Prüfung zur nächsten behalten und nur bei Ablauf neu geöffnet:
vCenter protokolliert nicht jede Minute eine neue Anmeldung.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_vsphere_`. Zustandswerte folgen dem `overallStatus` von
vSphere: 0 grün, 1 gelb, 2 rot, 3 grau (unbekannt).

| Metrik | Was | Labels |
|---|---|---|
| `info` | Wert 1 | `product` (`vcenter` oder `esxi`), `version`, `build` |
| `hosts` | Hosts nach Verbindungszustand: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 verbunden, 1 von einem Administrator getrennt, 2 antwortet nicht | `esxi` |
| `host_power_state` | 0 eingeschaltet, 1 Standby, 2 ausgeschaltet, 3 unbekannt | `esxi` |
| `host_maintenance` | 1 im Wartungsmodus | `esxi` |
| `host_status` | Gesamtzustand | `esxi` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | nur verbundene Hosts | `esxi` |
| `vms` | VMs nach Betriebszustand: `poweredOn`, `poweredOff`, `suspended` (Vorlagen ausgenommen) | `state` |
| `templates` | VM-Vorlagen | |
| `vm_power_state` | 0 eingeschaltet, 1 ausgeschaltet, 2 angehalten; höchstens 1000 VMs | `vm` |
| `vm_status` | Gesamtzustand | `vm` |
| `vm_tools_status` | nur eingeschaltete VMs: 0 aktuell, 1 alte Version, 2 installiert, aber nicht laufend, 3 nicht installiert | `vm` |
| `datastores` | Datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 zugänglich, 0 unzugänglich | `datastore` |
| `datastore_status` | Gesamtzustand | `datastore` |
| `alarms` | ausgelöste, nicht bestätigte Alarme nach Farbe: `red`, `yellow` | `status` |
| `alarms_acknowledged` | bereits bestätigte ausgelöste Alarme | |
| `alarm` | eine Serie pro unbestätigtem Alarm, Wert = sein Zustandscode, höchstens 64 | `alarm` (sein Name), `entity`, `entity_type`, `status` |
| `scrape_errors` | bei der letzten Prüfung fehlgeschlagene Aufrufe | |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#vmware-vsphere): vSphere-Host
antwortet nicht, vSphere-Host-Zustand rot, vSphere-Host im Wartungsmodus belassen,
vSphere-Datastore fast voll, vSphere-Datastore unzugänglich, vSphere-VM-Tools laufen nicht,
vSphere-Alarm rot, außerdem Gerät nicht erreichbar.

## Die Geräteseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite fragt
vSphere nie ab. Es nennt zuerst, was nicht stimmt (Hosts, die nicht antworten, rote Alarme,
fast volle oder unzugängliche Datastores), dann die Hosts mit ihrer Last, die Datastores mit
ihrem Füllstand und die VMs nach Betriebszustand.

## Wie es validiert wurde {#how-it-was-validated}

Gegen den govmomi-Simulator `vcsim` 0.56.0, im vCenter- und im ESXi-Modus: Anmeldung und
abgelehntes Passwort, Wiederverwendung und Ablauf der Sitzung, Seitenweise Lesen des Inventars,
ein nicht antwortender Host, ein Host im Wartungsmodus, eine ausgeschaltete VM und eine
Vorlage. Der Simulator löst keinen Alarm aus: Das Lesen der ausgelösten Alarme ist gegen die
dokumentierte Struktur `AlarmState` der Web-Services-API validiert.

## Einen Nur-Lese-Benutzer in vSphere erstellen {#create-a-read-only-user-in-vsphere}

1. Auf einem vCenter: Administration → Single Sign On → Benutzer und Gruppen → Benutzer, Domäne vsphere.local → Hinzufügen. Benennen Sie den Benutzer wie folgt und geben Sie ihm ein langes zufälliges Passwort.

    ```
    dumbmonit
    ```

2. Weiterhin auf dem vCenter: Administration → Zugriffskontrolle → Globale Berechtigungen → Hinzufügen. Wählen Sie diesen Benutzer, die Rolle Nur Lesen und haken Sie An untergeordnete Objekte weitergeben an. Nur Lesen sieht jeden Host, jede VM, jeden Datastore und jeden Alarm und kann nichts ändern.

3. Stattdessen auf einem eigenständigen ESXi-Host: im Host-Client Verwalten → Sicherheit & Benutzer → Benutzer → Benutzer hinzufügen, mit demselben Namen. Dann Host → Aktionen → Berechtigungen → Benutzer hinzufügen: Wählen Sie ihn, die Rolle Nur Lesen und haken Sie An alle untergeordneten Objekte weitergeben an.

4. Geben Sie in DumbMonit die Adresse des vCenters oder des Hosts ein, zum Beispiel „vcenter.lan“ oder „esxi1.lan“, mit dem Benutzernamen wie erstellt: dumbmonit@vsphere.local auf einem vCenter, dumbmonit auf einem Host. Verwaltet ein vCenter die Hosts, fügen Sie nur das vCenter hinzu: Es meldet jeden Host, jede VM und jeden Datastore.

5. DumbMonit liest nur, über die vSphere-Web-Services-API (/sdk). Es schaltet nie eine VM ein oder aus, wechselt nie in den Wartungsmodus und bestätigt nie einen Alarm.

!!! warning
    Verwenden Sie weder das Konto der Single-Sign-On-Domäne noch das eingebaute Konto des Hosts: Die obige Rolle Nur Lesen ist alles, was DumbMonit braucht. vCenter und ESXi verwenden standardmäßig ein selbstsigniertes Zertifikat: Aktivieren Sie „Nicht prüfbares Zertifikat akzeptieren“, sofern der DumbMonit-Host ihrer Zertifizierungsstelle nicht vertraut.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Benutzername / Passwort | Der dedizierte Benutzer mit der Rolle Nur Lesen: `dumbmonit@vsphere.local` auf einem vCenter, `dumbmonit` auf einem Host. |

Adresse: ein Hostname oder eine IP (`vcenter.lan`), `host:port` oder eine vollständige URL; ein
abschließendes `/sdk` oder `/ui` wird akzeptiert. Optionen: Port (443), Zertifikatsprüfung,
Anfrage-Timeout (20 s) und ausgelöste Alarme (lesen).
