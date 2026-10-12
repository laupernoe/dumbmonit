# Paperless-ngx

Ob Paperless-ngx noch importiert, was ihm geschickt wird: die Gesundheit
seiner Datenbank, von Redis, Celery-Workern, Suchindex, Klassifikator und
Sanity-Check, fehlgeschlagene Tasks, Updates, Plattenplatz und die
Anzahl der Dokumente.

DumbMonit liest Paperless-ngx' REST-API mit einem **dedizierten
Benutzer, der kein Dokument lesen kann**: drei View-Berechtigungen geben
ihm die Task-Liste, den Systemstatus und die globalen Zählungen, nichts
sonst.

Der beobachtenswerte Fehler: **Redis oder Celery gestoppt**. Die
Web-Oberfläche öffnet sich, und Dokumente können gelesen werden, aber
nichts, was im Consume-Ordner abgelegt oder per Mail abgeholt wird, wird
importiert, bis jemand den Stapel bemerkt. Der Systemstatus sagt das;
DumbMonit alarmiert darauf.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_paperless_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `update_available` | 1, wenn Paperless-ngx' eigener Versionscheck eine neuere Version kennt (fehlt, wenn der Check aus ist) | `latest_version` |
| `component_status` | 0 OK, 1 Warning, 2 Error, wie die Systemstatus-Seite es meldet | `component`: `database`, `redis`, `celery`, `index`, `classifier`, `sanity_check`, `llm_index` (nur wenn aktiviert) |
| `unapplied_migrations` | noch nicht angewendete Datenbankmigrationen | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | die Platte mit den Medien | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | alle Tasks der letzten 30 Tage: fehlgeschlagene, insgesamt, und noch ausstehende | |
| `tasks_failed_recent` | für diesen Benutzer sichtbare fehlgeschlagene Tasks, neuer als das Fenster für fehlgeschlagene Tasks und nicht verworfen | |
| `task_failed` | Wert 1 pro solchem Task (höchstens 32) | `task`, `file` |
| `documents`, `documents_inbox` | Dokumente in Paperless, und im Posteingang | |
| `status_readable` | 0, wenn dem Konto die Berechtigung "system monitoring" fehlt | |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Paperless hält die Tasks jedes Besitzers privat: der dedizierte Benutzer
sieht geplante Tasks (Sanity-Check, Index, Klassifikator, Mail) namentlich,
während Importe anderer Benutzer in `tasks_failed_window` gezählt
werden, das jeden Task umfasst.

Die geltenden [eingebauten Regeln](../alerting/rules.md#self-hosted-applications):
Paperless task queue down, Paperless index or sanity check error,
Paperless migrations pending, Paperless task failed, Paperless storage
almost full, Paperless update available, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite ruft Paperless nie auf. Es sagt zuerst, was nicht
stimmt (eine Komponente im Fehlerzustand, fehlgeschlagene Tasks,
ausstehende Migrationen), dann Updates, dann Dokumente, Posteingang,
ausstehende und fehlgeschlagene Tasks und Plattenplatz, dann jede
Komponente und die fehlgeschlagenen Tasks namentlich.

## Einen nur-lesenden Benutzer in Paperless-ngx erstellen

1. Erstellen Sie einen wie folgt benannten Benutzer, mit einem langen
   Zufallspasswort, ohne Superuser- oder Staff-Status, und nur drei
   View-Berechtigungen: tasks, system monitoring und global statistics.
   Mit der API und Ihrem eigenen Konto erstellen die beiden folgenden
   Befehle ihn und geben sein Token aus.

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. Oder stattdessen in der Web-Oberfläche: Users & Groups → Add user,
   dann in dessen Berechtigungen nur View bei den Zeilen PaperlessTask,
   SystemMonitoring und GlobalStatistics ankreuzen. Loggen Sie sich
   einmal als dieser Benutzer ein, öffnen Sie My Profile und erzeugen
   Sie sein API-Token.

3. Dieser Benutzer liest überhaupt kein Dokument: globale Statistiken
   geben die Zählungen, Systemüberwachung gibt die Gesundheit von
   Datenbank, Redis, Celery und Index, und Tasks zeigen die
   fehlgeschlagenen. Auf Paperless-ngx 2.x, wo die letzten beiden
   Berechtigungen nicht existieren, werden Gesundheit und Zählungen
   übersprungen und der Rest gelesen.

4. Geben Sie in DumbMonit die Adresse ein, zum Beispiel "paperless.lan"
   (Port 8000) oder "https://paperless.example.com" hinter einem Reverse
   Proxy, und fügen Sie das Token ein.

!!! warning
    Wenn Redis oder Celery stoppt, liefert Paperless weiter seine Seiten
    und Dokumente aus, aber nichts Neues wird importiert, weder aus dem
    Consume-Ordner noch per Mail, bis jemand hinsieht. DumbMonit
    alarmiert auf beides.

Ersetzen Sie `CHANGE-ME` durch ein langes Zufallspasswort, bevor Sie die
Befehle ausführen, und `YOUR_USER` durch ein Konto, das Benutzer
verwalten darf (curl fragt nach dessen Passwort). Die Abfrage des
Systemstatus kann mehrere Sekunden dauern, während Redis down ist:
Paperless wartet darauf, bevor es antwortet.

## Credentials

| Credential | Felder |
|---|---|
| API-Token (empfohlen) | Das Token des dedizierten Benutzers, als `Authorization: Token …` gesendet. |
| Benutzername / Passwort | Der dedizierte Benutzer selbst, als HTTP-Basic-Authentifizierung gesendet. |

Adresse: ein Hostname (`paperless.lan`), `host:port`, oder eine
vollständige URL mit Pfadpräfix; ein abschließendes `/api` wird
akzeptiert. Optionen: Protokoll (standardmäßig HTTP), Port (8000),
Zertifikatsprüfung, Anfrage-Timeout (15 s) und das Fenster für
fehlgeschlagene Tasks (24 Stunden, von 1 bis 720).

Validiert gegen Paperless-ngx 3.2.1 (offizielles Image, SQLite, Valkey).
