# Nutanix AHV

Nutanix-Cluster, gelesen über die v4-REST-API von Prism Central. Ein Prism
Central deckt alle bei ihm registrierten Cluster ab.

Jede Minute liest DumbMonit mit HTTP-Basic-Authentifizierung auf Port 9440:

| Was | Pfad |
|---|---|
| Cluster | `/api/clustermgmt/v4.0/config/clusters` |
| Hosts | `/api/clustermgmt/v4.0/config/hosts` |
| AHV-VMs | `/api/vmm/v4.0/ahv/config/vms` |
| Storage-Container | `/api/clustermgmt/v4.0/config/storage-containers` |
| Füllstand jedes Containers | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Ungelöste Alarme | `/api/monitoring/v4.0/serviceability/alerts` |

Listen werden mit 100 Einträgen pro Seite gelesen, bis zu 1.000 Einträge je
Liste. Der Füllstand und die Alarme sind optional: Verweigert die Rolle des
Kontos sie, wird der Rest trotzdem erfasst. Nutanix hat die APIs v1, v2.0 und
v3 für veraltet erklärt; nur v4 wird weiterentwickelt, und sie erfordert
Prism Central pc.2024.3 oder neuer.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus den v4-API-Spezifikationen von Nutanix gebaut
    (Pfade und Felder, wie die offiziellen generierten Clients sie
    definieren) und gegen aufgezeichnete Antworten getestet, nicht gegen ein
    laufendes Prism Central. Sagen Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_nutanix_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `clusters` | | Cluster, ohne Prism Central selbst. |
| `cluster_info` | `cluster`, `version` | Wert 1: die AOS-Version jedes Clusters. |
| `cluster_available` | `cluster` | 1, wenn Prism Central den Cluster erreicht. |
| `hosts` | | Hosts aller Cluster. |
| `host_info` | `host`, `cluster`, `hypervisor` | Wert 1: der Hypervisor, den jeder Host ausführt. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Kapazität. |
| `host_degraded` | `host`, `cluster` | 1, wenn Nutanix den Host als beeinträchtigt markiert. |
| `host_maintenance` | `host`, `cluster` | 1, wenn der Host in den Wartungsmodus wechselt oder darin ist. |
| `vms`, `vms_running` | | AHV-VMs und wie viele eingeschaltet sind. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Wert 1: `on`, `off`, `paused` oder `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1, wenn die VM eingeschaltet ist. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Sockets × Kerne × Threads und konfigurierter Speicher. |
| `storage_containers` | | Storage-Container, ohne die internen von Nutanix. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Füllstand, aus den letzten Statistiken der vergangenen dreißig Minuten. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Ungelöste Alarme, alle und nach Schweregrad. |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#nutanix):

- **Nutanix-Host beeinträchtigt**: Ein Host ist fünf Minuten lang als
  beeinträchtigt markiert (Warnung).
- **Nutanix-Storage-Container fast voll**: Ein Container ist fünfzehn Minuten
  lang zu 90 % voll (Hinweis).
- **Kritische Nutanix-Alarme**: Prism Central meldet fünf Minuten lang
  mindestens einen ungelösten kritischen Alarm (Warnung).

## Einen schreibgeschützten Benutzer in Prism Central anlegen

1. Öffnen Sie in Prism Central Admin Center > IAM > Identities und fügen Sie einen lokalen Benutzer mit einem langen Zufallspasswort hinzu, wie folgt benannt.

    ```
    dumbmonit
    ```

2. Erstellen Sie unter Admin Center > IAM > Authorization Policies eine Richtlinie mit der eingebauten Rolle Prism Viewer für alle Entitäten und weisen Sie sie diesem Benutzer zu. Prism Viewer liest Cluster, Hosts, VMs, Storage-Container und Alarme und ändert nichts.

3. Geben Sie in DumbMonit die Adresse von Prism Central ein, zum Beispiel "prism-central.lan", mit diesem Benutzernamen und Passwort. Ein Prism Central meldet alle bei ihm registrierten Cluster.

4. DumbMonit liest nur die v4-API (clustermgmt, vmm und monitoring, Version 4.0): Es schaltet nie eine VM ein oder aus, löst nie einen Alarm auf und ändert nie einen Storage-Container.

!!! warning
    Die v4-API wird von Prism Central pc.2024.3 oder neuer bereitgestellt; ein nur von Prism Element oder von einem älteren Prism Central verwalteter Cluster wird nicht unterstützt. Prism Central liefert ein selbstsigniertes Zertifikat aus: Aktivieren Sie "Accept an unverifiable certificate", sofern der DumbMonit-Host seiner Zertifizierungsstelle nicht vertraut.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername und Passwort | Der lokale Benutzer `dumbmonit` mit der Rolle Prism Viewer. |

Adresse: der Hostname oder die IP von Prism Central, `host:port`, oder eine
URL. Optionen:

- **Port** (9440).
- **Accept an unverifiable certificate**: das eigene Zertifikat von Prism
  Central ist selbstsigniert.
- **Timeout pro Request** (15 s).

## Fehlerbehebung

"Nutanix Prism Central refused the credentials … (401)": falscher
Benutzername oder falsches Passwort.

"Nutanix Prism Central has no /api/clustermgmt/v4.0/…": Die Adresse ist ein
Prism Element oder ein Prism Central älter als pc.2024.3.

Kein `storage_container_used_percent`: Die Rolle erlaubt das Lesen der
Statistiken nicht; die Container werden trotzdem gezählt, mit ihrer
konfigurierten Kapazität.
