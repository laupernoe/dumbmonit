# Veeam Backup & Replication {#veeam-backup-replication}

Ein Veeam-Backup-&-Replication-Server über seine REST-API: Jobs, deren letzter Lauf
fehlgeschlagen ist oder mit einer Warnung endete, fehlgeschlagene Sitzungen der letzten 24
Stunden, sich füllende Repositories, die Lizenz und — pro geschütztem Computer oder VM — das
letzte erfolgreiche Backup.

DumbMonit meldet sich mit einem Windows-Konto an, das nur die Rolle Veeam Backup Viewer besitzt
(OAuth2 Password Grant auf `/api/oauth2/token`, Port 9419), und behält das Token für seine
Lebensdauer von fünfzehn Minuten. Jeder Aufruf trägt den obligatorischen Header
`x-api-version`, standardmäßig `1.1-rev0`: die Revision, die mit Version 12.0 kam.

Die Jobzustände (`/api/v1/jobs/states`) sind der einzige erforderliche Lesezugriff. Die anderen —
`/api/v1/serverInfo`, `/api/v1/sessions` (in den letzten 24 Stunden erstellt),
`/api/v1/backupInfrastructure/repositories/states`, `/api/v1/license` und
`/api/v1/restorePoints` (die Wiederherstellungspunkte jedes geschützten Objekts, serverweit,
neueste zuerst) — werden gelesen, wenn die Rolle es erlaubt. Veeam behält einige Routen der Rolle
Backup Administrator vor, darunter bei aktuellen Versionen die Lizenz: Eine Ablehnung dort wird
als „nicht lesbar“ angezeigt, nie als Fehler. Ein Server, der zu alt ist, um
`/api/v1/restorePoints` zu kennen — die Route erschien nach 12.0 —, wird genauso behandelt: kein
Gerät, kein Fehler.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht auf einem echten System getestet"

    Diese Integration wurde aus der REST-API-Referenz und dem Help Center von Veeam gebaut
    (Authentifizierung, Versionierung, die Modelle für Job, Sitzung, Repository, Lizenz und
    Wiederherstellungspunkt), nicht gegen einen laufenden Backup-Server. Welche Routen die
    Rolle Viewer lesen darf, hängt von der Version ab: Sagen Sie uns, was Ihr Server ablehnt.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_veeam_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `version_info` | `version` | Wert 1: der Build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Pro Job. |
| `job_failed`, `job_warning` | `job`, `type` | 1, wenn der letzte Lauf eines aktivierten Jobs fehlgeschlagen ist oder mit einer Warnung endete. Ein deaktivierter Job behält sein letztes Ergebnis, das nichts mehr aussagt: Er zählt als keines von beiden. |
| `job_result_info` | `job`, `type`, `result` | Wert 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Seit dem letzten Lauf; fehlt bei einem Job, der nie lief. |
| `job_progress_percent` | `job`, `type` | Solange der Job läuft. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Zahlen; höchstens 200 Jobs werden einzeln beschrieben. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | In den letzten 24 Stunden erstellte Sitzungen, nach Ergebnis. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Pro Repository. Ein Objektspeicher-Repository hat keine Kapazität: Nur sein belegter Platz wird veröffentlicht. |
| `license_valid` | | 1, wenn der Lizenzstatus `Valid` ist. |
| `license_info` | `status`, `type`, `edition` | Wert 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Bis zum Ablauf von Lizenz und Supportvertrag; fehlt bei einer unbefristeten Lizenz. |
| `license_instances_licensed`, `license_instances_used` | | Nutzung der Instanzlizenz. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1, wenn das Konto es lesen darf, 0, wenn Veeam es abgelehnt hat. |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#veeam-backup--replication):

- **Veeam-Job fehlgeschlagen**: Der letzte Lauf eines aktivierten Jobs ist fehlgeschlagen
  (Warnung, täglich erinnert).
- **Veeam-Job mit Warnung beendet** (Hinweis, täglich erinnert).
- **Veeam-Repository fast voll**: mehr als 90 % dreißig Minuten lang (Hinweis).
- **Veeam-Lizenz läuft ab**: weniger als 30 Tage verbleibend (Hinweis, täglich erinnert). Nur,
  wenn das Konto die Lizenz lesen kann.
- **Client-Gerät veraltet**: Ein geschützter Computer oder eine VM hat `device_stale_days` ohne
  erfolgreiches Backup überschritten (Hinweis, täglich erinnert); siehe [Geräte](#devices)
  unten.

## Geräte {#devices}

Jedes Objekt mit mindestens einem Wiederherstellungspunkt — eine VM, ein physischer oder
virtueller, per Agent geschützter Computer, eine Dateifreigabe — wird unter der generischen
Familie veröffentlicht, die jede DumbMonit-Integration teilt, die geschützte Geräte verfolgt —
`dumbmonit_client_device_*`, mit Präfix ohne `veeam`, ausführlich dokumentiert unter
[Client-Geräte](../alerting/rules.md#client-devices):

| Metrik | Was | Labels |
|---|---|---|
| `last_backup_timestamp_seconds` | Der jüngste Wiederherstellungspunkt des Objekts | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positiv, sobald ein Objekt `device_stale_days` (standardmäßig 3) ohne neuen Wiederherstellungspunkt überschritten hat; speist die Regel [Client-Gerät veraltet](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (immer `backup`) |

**Wissenswerte Grenzen**: Ein Job schützt seine Objekte als Gruppe (`jobs/states` liefert nur
eine Zahl, `objectsCount`); das Datum pro Objekt stammt stattdessen aus
`/api/v1/restorePoints`, serverweit in einem Aufruf gelesen. Veeam erzeugt einen
Wiederherstellungspunkt erst, wenn ein Backup mindestens teilweise erfolgreich war, daher ist
der jüngste pro Objekt bereits „das letzte erfolgreiche Backup“ — hier wird nichts nach dem
Ergebnis gefiltert. Veeam stellt keine vom Backup selbst getrennte „letzte Verbindung“ bereit,
daher werden `last_seen_timestamp_seconds` und das Signal `connection` nie veröffentlicht; ein
Gerät zeigt `last_connection` in der Tabelle der Geräteseite als „—“. Der `type` des Geräts ist
der eigene Plattformname von Veeam (`VSphere`, `WindowsPhysical`, `HyperV` …), unverändert
angezeigt.

## Die Geräteseite {#the-device-page}

Das Panel Backups nennt in einem Satz, was nicht stimmt, zeigt die fehlgeschlagenen Jobs, die
fehlgeschlagenen Sitzungen der letzten 24 Stunden, die laufenden Jobs und die Lizenz und listet
dann die aktivierten Jobs — Fehlschläge zuerst — mit Typ und letztem Lauf sowie die Repositories
mit Füllstand und freiem Platz. Danach folgt die Tabelle der geschützten Geräte (Name,
Plattform, letztes Backup, Status) aus der obigen generischen Familie.

## DumbMonit ein Veeam-Backup-Viewer-Konto geben {#give-dumbmonit-a-veeam-backup-viewer-account}

1. Erstellen Sie auf dem Backup-Server ein lokales Windows-Konto für DumbMonit mit einem langen zufälligen Passwort und ohne weitere Rechte auf dem Rechner (ein Domänenkonto funktioniert ebenfalls).

    ```
    net user dumbmonit * /add
    ```

2. Öffnen Sie in der Veeam-Konsole das Hauptmenü > Benutzer und Rollen > Sicherheit, klicken Sie auf Hinzufügen, geben Sie das Konto ein (zum Beispiel VBR01\dumbmonit) und weisen Sie ihm ausschließlich die Rolle Veeam Backup Viewer zu.

3. Geben Sie in DumbMonit die Adresse des Backup-Servers ein, zum Beispiel „vbr.lan“, und das Konto als HOST\dumbmonit oder DOMAIN\dumbmonit mit seinem Passwort. Die REST-API lauscht auf Port 9419 mit einem selbstsignierten Zertifikat: Haken Sie Nicht prüfbares Zertifikat akzeptieren an, sofern Sie kein eigenes installiert haben.

!!! warning
    Veeam behält einige Routen der REST-API der Rolle Backup Administrator vor, darunter bei aktuellen Versionen die Lizenz. Mit der Rolle Viewer liest DumbMonit die Jobs, Sitzungen, Repositories und Wiederherstellungspunkte und zeigt die Lizenz als nicht lesbar an: Es verlangt nie mehr.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Benutzername und Passwort | Das Konto mit der Rolle Veeam Backup Viewer, als `HOST\dumbmonit` oder `DOMAIN\dumbmonit`. |

Adresse: der Hostname oder die IP des Backup-Servers, `host:port` oder eine URL. Optionen:

- **Protokoll** (`https`) und **Port** (9419): die des REST-API-Dienstes.
- **API-Version** (`1.1-rev0`): als `x-api-version` gesendet. Ändern Sie sie nur, wenn der
  Server sie ablehnt; ein v13-Server dokumentiert `1.3-rev1`.
- **Nicht prüfbares Zertifikat akzeptieren**: Das eigene Zertifikat der REST-API ist
  selbstsigniert.
- **Timeout pro Anfrage** (15 s).
- **Geräteveralterung** (`device_stale_days`, standardmäßig 3 Tage): siehe [Geräte](#devices)
  oben.

## Fehlerbehebung {#troubleshooting}

„Veeam hat die Anmeldung abgelehnt“: Der Benutzername braucht seinen Host oder seine Domäne
(`VBR01\dumbmonit`), oder das Konto hat keine Veeam-Rolle.

„Veeam hat das Auflisten der Jobs abgelehnt (403)“: Das Konto hat eine Rolle, die keine Jobs
lesen kann; geben Sie ihm Veeam Backup Viewer.

„Veeam hat die API-Version abgelehnt“: Setzen Sie die Option API-Version auf eine, die der Server
unter `https://<server>:9419/swagger` aufführt.

Die Lizenz zeigt „Nicht lesbar“: erwartet mit der Rolle Viewer bei aktuellen Versionen; DumbMonit
verlangt nicht mehr.

Kein Gerät, oder das Panel Geräte ist leer: `restore_points` zeigt „Nicht lesbar“, wahrscheinlich
weil der Server älter ist als die Route (alles vor den späteren Revisionen von 12.0) und nicht
wegen eines Rollenproblems — Wiederherstellungspunkte sind gewöhnliches Inventar, keine Seite nur
für Administratoren.
