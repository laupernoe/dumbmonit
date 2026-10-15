# XCP-ng / XenServer

Ein XCP-ng- oder XenServer-Pool (Citrix Hypervisor), gelesen über XAPI, die
Verwaltungs-API, die jeder Host ohnehin bereitstellt. Xen Orchestra wird nicht
benötigt.

Jede Minute sendet DumbMonit JSON-RPC-Aufrufe an
`https://<Koordinator>/jsonrpc`: ein `session.login_with_password` (die
Session wird dann von einer Abfrage zur nächsten beibehalten und einmal neu
geöffnet, wenn der Pool sie für ungültig erklärt) und pro Klasse ein
`get_all_records`: `host`, `host_metrics`, `VM`, `VM_metrics`, `SR` und
`pool`. Nichts wird je verändert: Das Konto hat die eingebaute Rolle
`read-only`, die XAPI bei jedem Aufruf durchsetzt.

Fügen Sie den Pool einmal hinzu, über die Adresse seines Koordinators: Er
meldet alle Hosts, VMs und Storage Repositories. Ist die eingegebene Adresse
die eines Pool-Mitglieds, antwortet es mit `HOST_IS_SLAVE` und der Adresse des
Koordinators; DumbMonit folgt ihr einmal, mit demselben Protokoll und Port,
sodass Sie den Monitor bei einem Koordinatorwechsel nicht bearbeiten müssen.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus der XAPI-Dokumentation gebaut (dem
    JSON-RPC-Protokoll, den Klassen `host`, `VM`, `SR` und `pool`) und gegen
    aufgezeichnete Antworten getestet, nicht gegen einen laufenden Pool. Sagen
    Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_xcpng_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `hosts`, `hosts_enabled` | | Hosts im Pool und wie viele davon neue VMs annehmen. |
| `host_info` | `host`, `version` | Wert 1: die Produktversion jedes Hosts. |
| `host_enabled` | `host` | 0, wenn der Host deaktiviert ist (Wartung oder evakuiert). |
| `host_live` | `host` | 1, wenn der Heartbeat des Hosts den Koordinator erreicht, 0, wenn er offline ist. |
| `host_cpu_cores`, `host_memory_total_bytes`, `host_memory_used_bytes` | `host` | Kapazität und genutzter Speicher. |
| `vms`, `vms_running` | | VMs, ohne Templates, Snapshots und Control Domains. |
| `vm_power_state_info` | `vm`, `state` | Wert 1: `running`, `halted`, `paused` oder `suspended`. |
| `vm_running` | `vm` | 1, wenn die VM läuft. |
| `vm_vcpus`, `vm_memory_bytes` | `vm` | vCPUs beim Start; genutzter Speicher bei laufender VM, sonst das konfigurierte Maximum. |
| `vm_ha_protected_down` | `vm` | Nur für VMs mit Neustartpriorität in einem Pool mit aktivierter Hochverfügbarkeit: 1, wenn die VM angehalten ist. |
| `srs` | | Storage Repositories. |
| `sr_size_bytes`, `sr_used_bytes`, `sr_used_percent` | `sr` | Füllstand (ISO-Bibliotheken werden gezählt, nicht detailliert). |
| `sr_shared` | `sr` | 1 für ein geteiltes Repository (NFS, iSCSI …). |
| `pool_info` | `pool`, `master` | Wert 1: der Pool und sein aktueller Koordinator. |
| `pool_ha_enabled` | `pool` | 1, wenn die Hochverfügbarkeit aktiviert ist. |
| `pool_ha_overcommitted` | `pool` | Bei aktivierter HA: 1, wenn der Pool seine geschützten VMs nach den Host-Ausfällen, die er tolerieren soll, nicht mehr neu starten kann. |
| `pool_ha_host_failures_to_tolerate` | `pool` | Die Anzahl der Host-Ausfälle, die die HA überstehen soll. |

XenAPI kennt keinen Betriebszustand „abgestürzt“. Eine geschützte VM, die
angehalten vorgefunden wird, ist das nächstliegende Signal, das die API für
eine VM liefert, die gegen Ihren Willen gestoppt wurde; wenn Sie eine
geschützte VM absichtlich herunterfahren, entfernen Sie zuerst ihre
Neustartpriorität.

Die zutreffenden [integrierten Regeln](../alerting/rules.md#xcp-ng-xenserver):

- **XCP-ng-Host offline**: Der Heartbeat eines Hosts hat den Koordinator
  fünf Minuten lang nicht erreicht (Warnung).
- **XCP-ng-SR fast voll**: Ein Storage Repository ist fünfzehn Minuten lang zu
  90 % voll (Hinweis).
- **HA-geschützte XCP-ng-VM ausgefallen**: Eine durch die Hochverfügbarkeit
  geschützte VM ist zwei Minuten lang angehalten (Warnung).
- **XCP-ng-Pool-HA überbucht**: Der Pool ist fünf Minuten lang überbucht
  (Hinweis).

## DumbMonit ein schreibgeschütztes XAPI-Konto geben

1. XAPI akzeptiert andere Konten als seinen eingebauten Superuser nur, wenn der Pool eine externe Authentifizierung verwendet. Ist der Pool bereits in Active Directory eingebunden, legen Sie dort ein Konto dumbmonit an und fahren Sie mit Schritt 3 fort.

2. Legen Sie andernfalls auf jedem Host des Pools ein lokales Konto mit einem langen Zufallspasswort an und aktivieren Sie dann einmalig die PAM-Authentifizierung, vom Pool-Koordinator aus. Solange keine Rolle vergeben ist, kann das Konto nichts aufrufen.

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. Registrieren Sie das Konto auf dem Pool-Koordinator als Subject und geben Sie diesem Subject dann die eingebaute Rolle read-only, mit der UUID, die der erste Befehl ausgibt.

    ```
    xe subject-add subject-name=dumbmonit
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

4. Geben Sie in DumbMonit die Adresse des Pool-Koordinators ein, zum Beispiel "xcp01.lan", mit diesem Benutzernamen und Passwort. Fügen Sie den Pool einmal hinzu: Der Koordinator meldet alle Hosts, VMs und Storage Repositories. Ein Pool-Mitglied antwortet mit der Adresse des Koordinators, der DumbMonit dann folgt.

5. DumbMonit liest nur, über die JSON-RPC-Schnittstelle von XAPI (/jsonrpc): Es startet, stoppt oder migriert nie eine VM und ändert nie ein Storage Repository oder die Hochverfügbarkeit des Pools.

!!! warning
    Geben Sie DumbMonit nie das Superuser-Passwort der Hosts: Mit der Rolle read-only kann ein durchgesickertes Passwort nichts verändern. XCP-ng und XenServer liefern ein selbstsigniertes Zertifikat aus: Aktivieren Sie "Accept an unverifiable certificate", sofern der DumbMonit-Host dem vom Pool präsentierten Zertifikat nicht vertraut.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername und Passwort | Das Konto `dumbmonit`, als Subject mit der Rolle `read-only` registriert. |

Adresse: der Hostname oder die IP des Pool-Koordinators, `host:port`, oder
eine URL. Optionen:

- **Port** (443).
- **Accept an unverifiable certificate**: das eigene Zertifikat der Hosts ist
  selbstsigniert.
- **Timeout pro Request** (15 s).

## Fehlerbehebung

"XCP-ng refused the user name or password": falsches Passwort, oder das Konto
ist nicht als Subject registriert (`xe subject-list`).

"XCP-ng accepted the account but refused the call": Das Subject hat keine
Rolle oder eine Rolle ohne Lesezugriff; geben Sie ihm `read-only`.

"This host is a pool member, not the coordinator": DumbMonit konnte den vom
Mitglied genannten Koordinator nicht erreichen; geben Sie die Adresse des
Koordinators selbst ein.
