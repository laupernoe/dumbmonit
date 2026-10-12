# VMware vSphere

Ein vCenter Server und die von ihm verwalteten Hosts, oder ein
eigenständiger ESXi-Host: Hosts, die aufhören zu antworten, im
Wartungsmodus verbleiben oder eine rote Gesundheit melden, virtuelle
Maschinen und ihre VMware Tools, sich füllende oder unzugänglich
werdende Datastores, und die von vSphere selbst gehobenen Alarme.

DumbMonit liest vSphere mit einem dedizierten Benutzer, der die
eingebaute Rolle **Read-only** hält. Es schaltet nie eine VM ein oder
aus, geht nie in den Wartungsmodus und bestätigt nie einen Alarm.

Der beobachtenswerte Fehler: **ein Datastore, der sich füllt**. Wenn
ein Thin-Provisioned-Datastore 100 % erreicht, wird jede VM, die darauf
schreiben muss, sofort pausiert. Die von vCenter gehobenen Alarme sagen
es auch, aber nur an den, der den Client offen hat.

## Warum die Web Services API

DumbMonit spricht mit der vSphere-Web-Services-API (SOAP, `/sdk`), nicht
mit der REST-API (`/api/vcenter/*`): Ein eigenständiger ESXi-Host hat
keine REST-API, und auf einem vCenter sagt die REST-API nichts über
Wartungsmodus, die Gesamtgesundheit (`overallStatus`), den Zustand der
VMware Tools oder ausgelöste Alarme. Eine Ansicht des gesamten
Inventars wird in einem Durchgang gelesen, Seite für Seite, und am
Ende freigegeben.

Die Sitzung wird von einer Abfrage zur nächsten behalten und nur neu
geöffnet, wenn sie abläuft: vCenter protokolliert nicht jede Minute
einen neuen Login.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_vsphere_`. Health-Werte
folgen vSpheres `overallStatus`: 0 grün, 1 gelb, 2 rot, 3 grau
(unbekannt).

| Metrik | Was | Labels |
|---|---|---|
| `info` | Wert 1 | `product` (`vcenter` oder `esxi`), `version`, `build` |
| `hosts` | Hosts nach Verbindungszustand: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 connected, 1 von einem Administrator disconnected, 2 not responding | `host` |
| `host_power_state` | 0 eingeschaltet, 1 Standby, 2 ausgeschaltet, 3 unbekannt | `host` |
| `host_maintenance` | 1 im Wartungsmodus | `host` |
| `host_status` | Gesamtgesundheit | `host` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | nur verbundene Hosts | `host` |
| `vms` | VMs nach Power-Zustand: `poweredOn`, `poweredOff`, `suspended` (Templates ausgeschlossen) | `state` |
| `templates` | VM-Templates | |
| `vm_power_state` | 0 eingeschaltet, 1 ausgeschaltet, 2 suspendiert; höchstens 1000 VMs | `vm` |
| `vm_status` | Gesamtgesundheit | `vm` |
| `vm_tools_status` | nur eingeschaltete VMs: 0 aktuell, 1 alte Version, 2 installiert, aber nicht laufend, 3 nicht installiert | `vm` |
| `datastores` | Datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 zugänglich, 0 unzugänglich | `datastore` |
| `datastore_status` | Gesamtgesundheit | `datastore` |
| `alarms` | ausgelöste, nicht bestätigte Alarme, nach Farbe: `red`, `yellow` | `status` |
| `alarms_acknowledged` | bereits bestätigte ausgelöste Alarme | |
| `alarm` | eine Zeitreihe pro nicht bestätigtem Alarm, Wert = sein Gesundheitscode, höchstens 64 | `alarm` (sein Name), `entity`, `entity_type`, `status` |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Die geltenden [eingebauten Regeln](../alerting/rules.md#vmware-vsphere):
vSphere host not responding, vSphere host health red, vSphere host
left in maintenance, vSphere datastore almost full, vSphere datastore
inaccessible, vSphere VM tools not running, vSphere red alarm, plus
Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt vSphere nie ab. Es sagt zuerst, was nicht stimmt
(nicht antwortende Hosts, rote Alarme, fast volle oder unzugängliche
Datastores), dann die Hosts mit ihrer Last, die Datastores mit ihrem
Füllstand, und die VMs nach Power-Zustand.

## Wie es validiert wurde

Gegen den govmomi-Simulator `vcsim` 0.56.0, im vCenter- und im
ESXi-Modus: Login und abgelehntes Passwort, Sitzungswiederverwendung
und -ablauf, Durchblättern des Inventars, ein nicht antwortender Host,
ein Host im Wartungsmodus, eine ausgeschaltete VM und ein Template. Der
Simulator löst keinen Alarm aus: Das Lesen ausgelöster Alarme ist gegen
die dokumentierte `AlarmState`-Struktur der Web Services API validiert.

## Einen nur-lesenden Benutzer in vSphere erstellen

1. Auf einem vCenter: Administration → Single Sign On → Users and
   Groups → Users, Domain vsphere.local → Add. Benennen Sie den
   Benutzer wie folgt und geben Sie ihm ein langes Zufallspasswort.

    ```
    dumbmonit
    ```

2. Weiterhin auf dem vCenter: Administration → Access Control → Global
   Permissions → Add. Wählen Sie diesen Benutzer, die Rolle Read-only,
   und kreuzen Sie Propagate to children an. Read-only sieht jeden
   Host, jede VM, jedes Datastore und jeden Alarm, und kann nichts
   ändern.

3. Auf einem eigenständigen ESXi-Host stattdessen: im Host-Client,
   Manage → Security & users → Users → Add user, mit demselben Namen.
   Dann Host → Actions → Permissions → Add user: wählen Sie ihn, die
   Rolle Read-only, und kreuzen Sie Propagate to all children an.

4. Geben Sie in DumbMonit die Adresse des vCenter oder des Hosts ein,
   zum Beispiel "vcenter.lan" oder "esxi1.lan", mit dem Benutzernamen
   wie erstellt: dumbmonit@vsphere.local auf einem vCenter, dumbmonit
   auf einem Host. Verwaltet ein vCenter die Hosts, fügen Sie nur das
   vCenter hinzu: Es meldet jeden Host, jede VM und jedes Datastore.

5. DumbMonit liest nur, über die vSphere-Web-Services-API (/sdk). Es
   schaltet nie eine VM ein oder aus, geht nie in den Wartungsmodus
   und bestätigt nie einen Alarm.

!!! warning
    Nutzen Sie nicht das Single-Sign-On-Domänenkonto oder das
    eingebaute Konto des Hosts wiederverwendet: Die obige Read-only-
    Rolle ist alles, was DumbMonit braucht. vCenter und ESXi nutzen
    standardmäßig ein selbstsigniertes Zertifikat: Aktivieren Sie
    "Accept an unverifiable certificate", sofern der DumbMonit-Host
    deren Zertifizierungsstelle nicht vertraut.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername / Passwort | Der dedizierte Benutzer mit der Read-only-Rolle: `dumbmonit@vsphere.local` auf einem vCenter, `dumbmonit` auf einem Host. |

Adresse: ein Hostname oder eine IP (`vcenter.lan`), `host:port`, oder
eine vollständige URL; ein abschließendes `/sdk` oder `/ui` wird
akzeptiert. Optionen: Port (443), Zertifikatsprüfung, Anfrage-Timeout
(20 s) und ausgelöste Alarme (gelesen).
