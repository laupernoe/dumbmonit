# Veeam Backup & Replication

Ein Veeam-Backup-&-Replication-Server über seine REST-API: Jobs, deren
letzter Lauf fehlschlug oder mit einer Warnung endete, fehlgeschlagene
Sitzungen der letzten 24 Stunden, sich füllende Repositories, die
Lizenz, und — pro geschütztem Rechner oder VM — sein letztes
erfolgreiches Backup.

DumbMonit loggt sich mit einem Windows-Konto ein, das nur die Rolle
Veeam Backup Viewer hält (OAuth2 Password Grant auf
`/api/oauth2/token`, Port 9419) und behält das Token für seine
fünfzehnminütige Lebensdauer. Jeder Aufruf trägt den obligatorischen
Header `x-api-version`, standardmäßig `1.1-rev0`: die Revision, die
mit Version 12.0 kam.

Die Job-Zustände (`/api/v1/jobs/states`) sind das einzig erforderliche
Lesen. Die anderen — `/api/v1/serverInfo`, `/api/v1/sessions`
(in den letzten 24 Stunden erstellt),
`/api/v1/backupInfrastructure/repositories/states`, `/api/v1/license`
und `/api/v1/restorePoints` (jedes geschützten Objekts
Wiederherstellungspunkte, serverweit, neueste zuerst) — werden
gelesen, wenn die Rolle sie erlaubt. Veeam behält manche Routen der
Rolle Backup Administrator vor, die Lizenz auf aktuellen Versionen
darunter: Eine Ablehnung dort wird als "nicht lesbar" angezeigt, nie
als Fehler. Ein Server, der zu alt ist, um `/api/v1/restorePoints` zu
kennen — die Route erschien nach 12.0 — wird genauso behandelt: kein
Gerät, kein Fehler.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus Veeams REST-API-Referenz und Help
    Center gebaut (Authentifizierung, Versionierung, die Modelle für
    Job, Sitzung, Repository, Lizenz und Wiederherstellungspunkt),
    nicht gegen einen laufenden Backup-Server. Welche Routen die
    Viewer-Rolle lesen darf, variiert mit der Version: Sagen Sie uns,
    was Ihr Server ablehnt.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_veeam_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `version_info` | `version` | Wert 1: der Build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Pro Job. |
| `job_failed`, `job_warning` | `job`, `type` | 1, wenn der letzte Lauf eines aktivierten Jobs fehlschlug, oder mit einer Warnung endete. Ein deaktivierter Job behält sein letztes Ergebnis, das nichts mehr sagt: Er zählt für keines von beiden. |
| `job_result_info` | `job`, `type`, `result` | Wert 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Seit dem letzten Lauf; fehlt für einen nie gelaufenen Job. |
| `job_progress_percent` | `job`, `type` | Während der Job läuft. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Zählungen; höchstens 200 Jobs werden einzeln beschrieben. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | In den letzten 24 Stunden erstellte Sitzungen, nach Ergebnis. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Pro Repository. Ein Object-Storage-Repository hat keine Kapazität: nur sein genutzter Platz wird veröffentlicht. |
| `license_valid` | | 1, wenn der Lizenzstatus `Valid` ist. |
| `license_info` | `status`, `type`, `edition` | Wert 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Bis Lizenz und Support-Vertrag ablaufen; fehlt bei einer unbefristeten Lizenz. |
| `license_instances_licensed`, `license_instances_used` | | Nutzung der Instanzlizenz. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1, wenn das Konto es lesen darf, 0, wenn Veeam ablehnt. |

Die geltenden [eingebauten Regeln](../alerting/rules.md#veeam-backup-replication):

- **Veeam job failed**: der letzte Lauf eines aktivierten Jobs ist
  fehlgeschlagen (Warning, täglich erinnert).
- **Veeam job ended with a warning** (Advisory, täglich erinnert).
- **Veeam repository nearly full**: mehr als 90 % seit dreißig
  Minuten (Advisory).
- **Veeam license expiring**: weniger als 30 Tage übrig (Advisory,
  täglich erinnert). Nur wenn das Konto die Lizenz lesen kann.
- **Client device stale**: ein geschützter Rechner oder eine VM hat
  `device_stale_days` ohne erfolgreiches Backup überschritten
  (Advisory, täglich erinnert); siehe [Geräte](#geräte) unten.

## Geräte

Jedes Objekt mit mindestens einem Wiederherstellungspunkt — eine VM,
ein physischer oder virtueller, agent-geschützter Rechner, eine
Dateifreigabe — wird unter der generischen Familie veröffentlicht, die
sich jede DumbMonit-Integration teilt, die geschützte Geräte verfolgt
— `dumbmonit_client_device_*`, ohne `veeam`-Präfix, vollständig
dokumentiert unter [Client-Geräte](../alerting/rules.md#client-devices):

| Metrik | Was | Labels |
|---|---|---|
| `last_backup_timestamp_seconds` | Der neueste Wiederherstellungspunkt des Objekts | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positiv, sobald ein Objekt `device_stale_days` (standardmäßig 3) ohne neuen Wiederherstellungspunkt überschritten hat; treibt die Regel [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (immer `backup`) |

**Wissenswerte Einschränkungen**: Ein Job schützt seine Objekte als
Gruppe (`jobs/states` gibt nur eine Zählung, `objectsCount`); das
Datum pro Objekt kommt stattdessen von `/api/v1/restorePoints`, für
den ganzen Server in einem Aufruf gelesen. Veeam erstellt einen
Wiederherstellungspunkt nur, wenn ein Backup mindestens teilweise
gelungen ist, daher ist der neueste pro Objekt bereits "das letzte
erfolgreiche Backup" — hier wird nichts nach dem Ergebnis gefiltert.
Veeam stellt keine "letzte Verbindung" getrennt vom Backup selbst
bereit, daher werden `last_seen_timestamp_seconds` und das Signal
`connection` nie veröffentlicht; ein Gerät zeigt `last_connection` als
"—" in der Tabelle der Geräteseite. Der `type` des Geräts ist Veeams
eigener Plattformname (`VSphere`, `WindowsPhysical`, `HyperV`…), so
wie er ist angezeigt.

## Die Geräteseite

Das Backups-Panel sagt in einem Satz, was nicht stimmt, zeigt die
fehlgeschlagenen Jobs, die fehlgeschlagenen Sitzungen der letzten 24
Stunden, die laufenden Jobs und die Lizenz, listet dann die
aktivierten Jobs — Fehlschläge zuerst — mit ihrer Art und dem letzten
Lauf, und die Repositories mit ihrem Füllstand und freiem Platz. Die
Tabelle der geschützten Geräte (Name, Plattform, letztes Backup,
Status) folgt, aus der generischen Familie oben.

## DumbMonit ein Veeam-Backup-Viewer-Konto geben

1. Erstellen Sie auf dem Backup-Server ein lokales Windows-Konto für
   DumbMonit mit einem langen Zufallspasswort und keinem weiteren
   Recht auf der Maschine (ein Domänenkonto funktioniert auch).

    ```
    net user dumbmonit * /add
    ```

2. Öffnen Sie in der Veeam-Konsole das Hauptmenü > Users and Roles >
   Security, klicken Sie auf Add, geben Sie das Konto ein (zum
   Beispiel VBR01\dumbmonit) und geben Sie ihm nur die Rolle Veeam
   Backup Viewer.

3. Geben Sie in DumbMonit die Adresse des Backup-Servers ein, zum
   Beispiel "vbr.lan", und das Konto als HOST\dumbmonit oder
   DOMAIN\dumbmonit mit seinem Passwort. Die REST-API hört auf Port
   9419 mit einem selbstsignierten Zertifikat: Kreuzen Sie "Accept an
   unverifiable certificate" an, sofern Sie kein eigenes installiert
   haben.

!!! warning
    Veeam behält manche REST-API-Routen der Rolle Backup Administrator
    vor, die Lizenz auf aktuellen Versionen darunter. Mit der
    Viewer-Rolle liest DumbMonit die Jobs, Sitzungen, Repositories und
    Wiederherstellungspunkte, und zeigt die Lizenz als nicht lesbar:
    Es fragt nie nach mehr.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername und Passwort | Das Konto mit der Rolle Veeam Backup Viewer, als `HOST\dumbmonit` oder `DOMAIN\dumbmonit`. |

Adresse: der Hostname oder die IP des Backup-Servers, `host:port`,
oder eine URL. Optionen:

- **Protocol** (`https`) und **Port** (9419): die des REST-API-Diensts.
- **API version** (`1.1-rev0`): als `x-api-version` gesendet. Nur
  ändern, wenn der Server sie ablehnt; ein v13-Server dokumentiert
  `1.3-rev1`.
- **Accept an unverifiable certificate**: das eigene Zertifikat der
  REST-API ist selbstsigniert.
- **Timeout pro Request** (15 s).
- **Device staleness** (`device_stale_days`, standardmäßig 3 Tage):
  siehe [Geräte](#geräte) oben.

## Fehlerbehebung

"Veeam refused the login": Der Benutzername braucht seinen Host oder
seine Domäne (`VBR01\dumbmonit`), oder das Konto hat keine
Veeam-Rolle.

"Veeam refused to list the jobs (403)": Das Konto hat eine Rolle, die
Jobs nicht lesen kann; geben Sie ihm Veeam Backup Viewer.

"Veeam refused the API version": Setzen Sie die Option API version
auf eine, die der Server unter `https://<server>:9419/swagger`
aufführt.

Die Lizenz zeigt "Not readable": erwartet mit der Viewer-Rolle auf
aktuellen Versionen; DumbMonit fragt nicht nach mehr.

Kein Gerät, oder das Devices-Panel ist leer: `restore_points` liest
"Not readable", wahrscheinlich weil der Server älter als die Route
ist (alles vor 12.0s späteren Revisionen) statt ein Rollen-Problem —
Wiederherstellungspunkte sind gewöhnliches Inventar, keine
nur-administrative Seite.
