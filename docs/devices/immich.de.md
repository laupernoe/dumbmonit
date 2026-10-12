# Immich

Ob Immich mit dem mithält, was hochgeladen wird: Job-Warteschlangen,
die stocken oder pausiert sitzen, Updates, die Platte mit der
Bibliothek, wie viele Fotos und Videos sie hält, und — für jedes
Telefon, Tablet und Desktop, das im Konto eingeloggt ist — seine letzte
Verbindung und sein letztes Backup.

DumbMonit liest Immichs API mit einem **auf acht Lese-Berechtigungen
begrenzten API-Key**. Der Key kann kein Foto, kein Album und keine
Person sehen, und kann nichts ändern.

Der beobachtenswerte Fehler: **Jobs, die nicht mehr laufen**. Eine
während eines großen Imports pausierte und vergessene Warteschlange,
oder ein gestoppter Job-Worker, und neue Fotos erhalten kein Thumbnail,
keine Metadaten, kein Gesicht und keine intelligente Suche, während
Uploads weiter gelingen. Immich zeigt das nur auf seiner Jobs-Seite.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_immich_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `update_available` | 1, wenn Immichs eigener Versionscheck eine neuere Version kennt (fehlt, wenn der Check ausgeschaltet ist) | `latest_version` |
| `maintenance_mode` | 1, während Immich im Wartungsmodus ist | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | die Platte mit der Bibliothek | |
| `photos`, `videos`, `usage_bytes`, `users` | Bibliotheksgröße, alle Benutzer zusammen | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | alle Warteschlangen zusammen | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | pro Warteschlange: wartend (einschließlich verzögerter und durch eine Pause gehaltener), laufend, fehlgeschlagen, pausiert | `queue` |
| `statistics_readable` | 0, wenn der Key zu einem gewöhnlichen Konto gehört und Zählungen und Warteschlangen übersprungen werden | |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Neuere Immich-Versionen entfernen fehlgeschlagene Jobs aus der
Warteschlange, sobald sie protokolliert sind, daher liest sich
`queue_failed` meist als 0; ein fehlgeschlagener Job zeigt sich in
Immichs eigenem Log.

Die geltenden [eingebauten Regeln](../alerting/rules.md#self-hosted-applications):
Immich jobs stalled, Immich queue paused, Immich storage almost full,
Immich update available, plus Device unreachable.

## Geräte

Jedes im Konto, das den Key besitzt, eingeloggte Gerät, mit seiner
letzten Verbindung und seinem letzten Backup, wird unter einer
generischen Familie veröffentlicht, die sich jede DumbMonit-
Integration teilt, die Client-Geräte verfolgt — `dumbmonit_client_device_*`,
ohne `immich`-Präfix, vollständig dokumentiert unter
[Client-Geräte](../alerting/rules.md#client-devices):

| Metrik | Was | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Letzte Verbindung (die letzte Aktivität der Sitzung) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | Letztes Backup; siehe die Einschränkung unten | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positiv, sobald ein Gerät `device_stale_days` (standardmäßig 3) überschritten hat, ohne sich zu verbinden oder zu sichern; treibt die Regel [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (`connection` oder `backup`) |

**Wissenswerte Einschränkung**: Immichs API verzeichnet nicht, welches
Gerät ein bestimmtes Foto hochgeladen hat, daher ist
`last_backup_timestamp_seconds` der neueste Upload der gesamten
Bibliothek — derselbe Wert für jedes Gerät, veröffentlicht unter einem
Gerät namens `library` statt einem echten. Es erkennt, wenn jedes
Gerät gleichzeitig gestoppt hat (Storage voll, das Konto überall
ausgeloggt); es erkennt nicht, wenn ein einzelnes Gerät still wird,
während ein anderes weiter hochlädt. Nur `last_seen_timestamp_seconds`
— die letzte Aktivität der Sitzung jedes Geräts — ist wirklich pro
Gerät.

Die Tabelle der Geräteseite zeigt diese Werte für jedes Gerät, mit
einem Status von OK oder Stale.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite ruft Immich nie auf. Es sagt zuerst, was nicht stimmt
(festsitzende Warteschlangen, pausierte Warteschlangen, Wartung), dann
Updates, dann Fotos, Videos, Bibliotheksgröße, genutzte Platte und
wartende und laufende Jobs, dann die Warteschlangen mit Arbeit,
namentlich, dann die Tabelle der Geräte (Name, Art, Betriebssystem,
letzte Verbindung, letztes Backup, Status).

## Einen nur-lesenden API-Key in Immich erstellen

1. Loggen Sie sich mit einem Konto ein, das Administration öffnen kann:
   Immich lässt nur diese Konten Fotozahlen und Job-Warteschlangen
   lesen. Öffnen Sie Account Settings → API Keys → New API Key,
   benennen Sie ihn wie folgt, und kreuzen Sie diese acht
   Berechtigungen an: die ersten fünf für Job-Warteschlangen, Zählungen
   und Version, die letzten drei, um die Geräte des Kontos und ihre
   letzte Verbindung und Backup aufzulisten.

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read session.read asset.read user.read
    ```

2. Kopieren Sie den Key jetzt: Immich zeigt ihn nur einmal. Er liest
   diese acht Seiten und nichts sonst: kein Foto, kein Album, keine
   Einstellung. Bietet Ihr Immich kein queue.read, kreuzen Sie
   stattdessen job.read an.

3. Ein von einem gewöhnlichen Konto erstellter Key funktioniert auch:
   Version, Updates, Plattenplatz und Geräte werden gelesen,
   Fotozahlen und Job-Warteschlangen werden übersprungen.

4. Geben Sie in DumbMonit Immichs Adresse ein, zum Beispiel
   "immich.lan" (Port 2283) oder "https://photos.example.com" hinter
   einem Reverse Proxy, und fügen Sie den Key ein.

!!! warning
    Eine pausierte Warteschlange ist leicht zu vergessen: neue Fotos
    erhalten kein Thumbnail, keine Metadaten und kein Gesicht, und
    Immich sagt das nur auf seiner Jobs-Seite. DumbMonit warnt, wenn
    Jobs in einer nicht pausierten Warteschlange warten, die nichts
    ausführt. session.read, asset.read und user.read listen jedes
    Gerät des Kontos, das den Key besitzt — seine Telefone, Tablets und
    Browser — mit der letzten Verbindung jedes einzelnen. Immichs API
    verzeichnet nicht, welches Gerät ein bestimmtes Foto hochgeladen
    hat, daher ist "letztes Backup" der neueste Upload der gesamten
    Bibliothek, für jedes Gerät gleich angezeigt: Stoppt nur ein Gerät
    das Sichern, während die anderen weitermachen, zeigt sich seine
    Verbindung als veraltet, aber die geteilte Backup-Zahl sieht
    weiterhin frisch aus.

Was jede Berechtigung öffnet: `server.about` die Version
(`/api/server/about`), `server.versionCheck` die neueste Immich
bekannte Version, `server.storage` die Platte, `server.statistics`
Foto- und Videozahlen, `queue.read` die Job-Warteschlangen
(`/api/queues`; ältere Versionen antworten auf `/api/jobs` mit
`job.read`), `session.read` die Geräte des Kontos (`/api/sessions`),
`asset.read` den neuesten Upload der Bibliothek
(`/api/search/metadata`), `user.read` den Anzeigenamen des Kontos
(`/api/users/me`).

## Credentials

| Credential | Felder |
|---|---|
| API-Key | Im Header `x-api-key` gesendet. |

Adresse: ein Hostname (`immich.lan`), `host:port`, oder eine
vollständige URL mit Pfadpräfix; ein abschließendes `/api` wird
akzeptiert. Optionen: Protokoll (standardmäßig HTTP), Port (2283),
Zertifikatsprüfung, Anfrage-Timeout (15 s), und Geräte-Veralterung
(`device_stale_days`, standardmäßig 3 Tage).

Validiert gegen Immich v3.2.4 (offizielle Images, Machine Learning
aus); die Geräte-Endpunkte wurden gegen den `v3.2.4`-Tag von Immichs
eigener OpenAPI-Spezifikation geprüft (`immich-app/immich`), was
bestätigte, dass `/api/search/metadata` keinen `deviceId`-Filter mehr
akzeptiert — der Grund, warum "letztes Backup" bibliotheksweit statt
pro Gerät ist.
