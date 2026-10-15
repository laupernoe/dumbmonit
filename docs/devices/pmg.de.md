# Proxmox Mail Gateway

Mail-Gateway: Postfix-Warteschlangen und das Alter der ältesten
wartenden Nachricht, heute gezählte und gefilterte Mail,
Quarantäne-Größen, das Alter der Antivirus- und Antispam-
Signaturdatenbanken, Dienste, Zertifikate und Cluster-Zustand.

!!! warning "Nur gegen eine Testinstanz getestet"

    Diese Integration wurde gegen ein zu diesem Zweck installiertes
    Proxmox Mail Gateway 9 gebaut und verifiziert, nicht gegen ein
    produktives. Jeder Aufruf ist ein Lesevorgang, und die Formen
    stammen von einem echten Mail-Gateway, aber noch niemand hat es
    wochenlang auf einem lebenden Mail-Gateway laufen lassen. Sagen
    Sie uns, was kaputt ist.

## Die Monitorseite

Über die generischen Diagramme hinaus zeigt ein
Proxmox-Mail-Gateway-Monitor drei Panels, aus dem gelesen, was die
Abfrage gespeichert hat — das Öffnen der Seite fragt das Gateway
selbst nie ab:

* **Mail queues** — die vier Postfix-Warteschlangen in der
  Reihenfolge, in der ein Mail-Admin sie liest: *incoming* (gerade
  angenommen), *active* (wird zugestellt), *deferred* (Zustellung
  fehlgeschlagen, wird wiederholt) und *hold* (von einer Regel
  zurückgehalten). Jede zeigt, wie viele Nachrichten sie hält, wie
  viele Zieldomains, die geschäftigste dieser Domains, und wie lange
  die älteste Nachricht wartet. Eine Warteschlange, die lediglich Mail
  enthält, ist normaler Verkehr; eine Warteschlange, die eine
  Nachricht länger als vier Stunden hält, wird als *Stuck* markiert.
* **Mail filtered today** — eingehende und ausgehende Nachrichten,
  Bytes, Spam, Viren, Bounces, durchschnittliche Verarbeitungszeit,
  und was vor der Zustellung abgelehnt wurde (Greylisting, Blocklisten,
  SPF, Pregreeting). Ein Balken pro Abschnitt des jüngeren Fensters
  zeichnet die Verkehrskurve, ohne die Zeitreihendatenbank abzufragen.
  Dann die drei Quarantänen mit ihrer Nachrichtenanzahl und
  Plattennutzung, und die heute gefangenen Viren namentlich.
* **Gateway** — gestoppte Dienste zuerst, dann die Cluster-Mitglieder
  und ihr Sync-Zustand, dann jeder Knoten: Uptime, CPU, Speicher,
  Platte, Uhrenabweichung, die Signaturdatenbanken mit ihrem Alter und
  Urteil, innerhalb zwei Wochen ablaufende Zertifikate, ausstehende
  Updates und der Abonnementstatus.

Nur Zählungen, nie Inhalt. DumbMonit liest, wie viele Nachrichten in
jeder Quarantäne liegen, nie einen Betreff, einen Absender, einen
Empfänger oder einen Body. Die Pro-Adresse-Statistik-Endpunkte
(`/statistics/sender`, `/statistics/receiver`, `/statistics/contact`)
werden überhaupt nicht aufgerufen.

## Heute, nicht die letzten vierundzwanzig Stunden

`GET /statistics/mail` aggregiert nach **lokalem Tag auf dem
Gateway**: PMG ignoriert das Ende des angefragten Fensters, nur
`starttime` wählt den Tag. DumbMonit fragt daher nach "jetzt", was
die Totals seit Mitternacht ergibt — genau, was PMGs eigenes
Dashboard zeigt. Diese Zeitreihen sind Gauges, die um Mitternacht auf
null zurückfallen, keine Zähler: Wickeln Sie sie nicht in `rate()`
ein. Die jüngere Kurve (`/statistics/recent`) liest stattdessen die
Rohtabelle und rollt tatsächlich.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_pmg_`.

| Familie | Metriken | Labels |
|---|---|---|
| Knoten | `node_cpu_percent`, `node_iowait_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_used_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_clock_offset_seconds` (positiv, wenn das Gateway vor DumbMonit liegt), `node_insync`, `node_kernel_info`, `node_version_info` (die API-Version *dieses* Knotens — in einem Cluster zeigt sich hier der, der beim letzten Upgrade zurückblieb), `version_info` | `node` |
| Warteschlangen | `queue_messages`, `queue_domains`, `queue_oldest_age_seconds` | `queue` (`incoming`, `active`, `deferred`, `hold`) |
| Mail heute | `mail_count_in/out`, `mail_bytes_in/out`, `mail_spam_in/out`, `mail_virus_in/out`, `mail_bounces_in/out`, `mail_junk_in`, `mail_junk_out` (Ihre eigenen Benutzer, die Junk senden: das erste Zeichen eines kompromittierten Kontos), `mail_junk_percent` (fehlt an einem Tag ohne eingehende Mail), `mail_greylisted`, `mail_spf_rejects`, `mail_rbl_rejects`, `mail_pregreet_rejects`, `mail_avg_processing_seconds` | |
| Durchsatz | `mail_rate_in_per_minute`, `mail_rate_out_per_minute` (aus dem letzten vollständigen Abschnitt der jüngeren Kurve) | |
| Spam-Scores | `spam_score_messages` | `level` (`0` bis `10`; `10` aggregiert alles darüber) |
| Viren | `virus_detections` (Top zehn des Tages) | `virus` |
| Quarantänen | `quarantine_messages`, `quarantine_bytes`, `quarantine_avg_spam_level` | `kind` (`spam`, `virus`, `attachment`) |
| Dienste | `service_running` (1 oder 0; überhaupt keine Zeitreihe für eine nicht installierte Unit) | `node`, `service` |
| Signaturen | `signature_age_seconds`, `signature_count`, `signature_update_available` | `node`, `family` (`virus`, `spam`), `database` (`main`, `daily`, `bytecode`, oder der SpamAssassin-Kanal) |
| Zertifikate | `certificate_expires_in_seconds` | `node`, `certificate` |
| Cluster | `cluster_nodes`, `cluster_node_insync`, `cluster_node_healthy` (alle fehlen auf einem eigenständigen Gateway) | `node`, `role` |
| Abonnement | `subscription_active` (Labels tragen den genauen Status) | `node`, `status`, `level` |
| Updates | `node_updates_pending`, `node_updates_security_pending` | `node` |
| Sammlung | `up`, `scrape_errors`, `scrape_duration_seconds` | |

`queue_oldest_age_seconds` ist eine **untere Grenze**. `qshape`
meldet Altersbereiche (unter 5 Min, 5–10 Min, 10–20 Min, und so weiter,
verdoppelnd), daher behält DumbMonit das Minimum des höchsten
belegten Bereichs: Eine mit "mindestens 20 Minuten" gemeldete
Nachricht kann 39 Minuten alt sein. Zu niedrig zu irren bedeutet, dass
der Alarm später statt früher feuert, nie auf einer Nachricht, die
nicht tatsächlich verspätet ist.

`queue_domains` ist auf einem einzelnen Gateway exakt. Auf einem
Cluster werden die Pro-Knoten-Warteschlangen zusammengeführt, und die
Zählung wird über die geschäftigsten Domains jedes Knotens genommen,
begrenzt auf zwanzig pro Knoten: darüber hinaus unterzählt sie.
Deduplizieren schlägt hier Summieren, da zwei Knoten eines Clusters
Mail für dieselben Domains empfangen.

Ein eigenständiges Gateway antwortet auf `/config/cluster/status` mit
einer leeren Liste. Das ist ein Gateway ohne Cluster, kein degradierter
Cluster: Keine Zeitreihe wird veröffentlicht, und keine Cluster-Regel
kann feuern.

Nur ClamAVs **daily**-Datenbank wird nach ihrem Alter beurteilt, vom
Panel wie von der Regel gleichermaßen. `main` wird ein- oder zweimal
pro Jahr neu erstellt, und `bytecode` kaum öfter: Sie nach zwei Tagen
zu beurteilen würde das ganze Jahr "out of date" auf einem perfekt
aktuellen Gateway ausgeben. Sie zeigen trotzdem ihr Alter, ihre
Version und ihre Signaturzahl — ohne Urteil. Ebenso hat ein nie
abgerufener SpamAssassin-Kanal überhaupt kein Datum, und "never
updated" ist nicht "out of date": keine Zeitreihe, kein Urteil, kein
Alarm.

ClamAV schreibt sein Build-Datum in seinem eigenen Format in den
`.cvd`-Header (`16 Dec 2025 23-18 +0000`, mit einem Bindestrich, wo
alle anderen einen Doppelpunkt setzen), und PMG gibt es durch. Sollte
sich dieses Format jemals ändern, fehlt das Alter einfach, statt um
fünfzig Jahre falsch zu sein. Dieselbe Regel gilt überall: Ein Wert,
den die API nicht zurückgibt, erzeugt eine Metrik weniger, nie eine
Null, die sich als Messwert ausgibt.

Geltende eingebaute Regeln: Monitor unreachable, Mail queue growing,
Mail stuck in the queue, Mail gateway service stopped, Virus
signatures out of date, Spam rules out of date, Quarantine filling up,
Mail gateway cluster degraded, Mail gateway certificate expiring, Mail
gateway updates pending. Benachrichtigungen benennen die betroffene
Warteschlange, den Dienst, die Datenbank oder den Knoten.

## Was in Proxmox Mail Gateway vorzubereiten ist

Die Schritte unten sind die, die der Hinweis neben dem Formular zeigt.
Das Prinzip: ein für die Überwachung reservierter Benutzer, mit einer
nur-lesenden Rolle — nie das Konto, mit dem Sie sich einloggen.

1. In der Web-Oberfläche: Configuration → User Management → Users →
   Add. Benennen Sie das Konto wie folgt, geben Sie ihm ein nirgendwo
   sonst genutztes langes Passwort, und kreuzen Sie Enabled an.

    ```
    dumbmonit@pmg
    ```

2. Setzen Sie seine Rolle auf `Audit`. Das ist genau das nur-lesende
   Minimum für alles, was DumbMonit liest: Knotenstatus, Dienste,
   Postfix-Warteschlangen, Mail-Statistiken, Quarantäne-Zählungen,
   ClamAV- und SpamAssassin-Datenbankalter, Cluster-Status,
   Zertifikate, Abonnement und ausstehende Updates. `Audit` kann
   nichts ändern, nichts aus der Quarantäne freigeben und keine
   Nachricht lesen.

3. Lieber eine Shell? Dasselbe Konto in einem Befehl, dann das
   Passwort setzen.

    ```
    pmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment "DumbMonit monitoring"
    ```

4. Geben Sie in DumbMonit die Gateway-Adresse ein, zum Beispiel
   "mail.lan" oder "mail.lan:8006", dann den Benutzernamen dieses
   Kontos (Realm eingeschlossen) und das Passwort.

5. Proxmox Mail Gateway stellt keine API-Tokens aus, anders als
   Proxmox VE und Proxmox Backup Server: Benutzername und Passwort
   sind der Weg hinein. DumbMonit öffnet eine zweistündige Sitzung und
   erneuert sie, statt sich bei jeder Messung einzuloggen.

!!! warning

    Nutzen Sie nicht das Konto wiederverwendet, mit dem Sie sich
    einloggen: Die obige Audit-Rolle kann nur lesen, und kann keine
    Quarantäne-Nachricht freigeben oder eine Regel ändern. Proxmox
    Mail Gateway nutzt auch standardmäßig ein selbstsigniertes
    Zertifikat: Wird die Verbindung deshalb abgelehnt, kreuzen Sie
    "Accept an unverifiable certificate" in den Optionen an.

Adresse: `10.0.0.40`, `mail.lan`, `mail.lan:8006`, `[fd00::4]` oder
eine vollständige URL. Das Schema `https` und Port 8006 werden
hinzugefügt, falls fehlend.

## Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | API port | `8006` | Genutzt, falls die Adresse keinen Port angibt. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Proxmox Mail Gateway liefert standardmäßig ein selbstsigniertes Zertifikat: aktivieren Sie das, wenn die Verbindung deshalb abgelehnt wird. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Erlaubte Zeit für jeden API-Aufruf, von 1 bis 120. Das Lesen einer Warteschlange führt einen Postfix-Befehl auf dem Gateway aus und kann ein paar Sekunden dauern. |
| `node` | Monitored node | *(leer)* | Name des zu überwachenden Cluster-Knotens. Leer: jeder vom Gateway gelistete Knoten. |
| `recent_hours` | Traffic window (hours) | `12` | Wie weit die Verkehrskurve zurückreicht, von 1 bis 24. Die Tagestotals decken immer den aktuellen Tag ab. |
| `queues` | Watch the mail queues | `true` | Liest die incoming-, active-, deferred- und hold-Warteschlangen und das Alter der jeweils ältesten Nachricht. |
| `quarantine` | Count the quarantines | `true` | Liest, wie viele Nachrichten die Spam- und Virus-Quarantänen halten, und wie viel Platte sie nutzen. Nur Zählungen. |
| `attachment_quarantine` | Also count the attachment quarantine | `false` | Diese Quarantäne hat keinen Zähl-Aufruf: Sie muss gelistet werden, um gezählt zu werden. Standardmäßig aus; nur die Anzahl der Einträge wird behalten. |
| `signatures` | Watch the signature databases | `true` | Liest das Alter der ClamAV-Virendatenbanken und der SpamAssassin-Regelkanäle. |
| `services` | Watch the services | `true` | Liest den Zustand von postfix, pmg-smtp-filter, pmgpolicy und den anderen Units jedes Knotens. |
| `certificates` | Watch the certificates | `true` | Liest die von der Web-Oberfläche bedienten Zertifikate und warnt vor ihrem Ablauf. |
| `updates` | Count pending updates | `true` | Listet die auf ein Update wartenden Pakete jedes Knotens. |
| `subscription` | Read the subscription | `true` | Liest den Abonnementstatus jedes Knotens. |

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| ClamAV `main` zeigt ein riesiges Alter | Erwartet: Es wird ein- oder zweimal pro Jahr neu erstellt. Nur `daily` trägt ein Urteil. |
| Zertifikatsfehler | Selbstsigniertes Zertifikat: installieren Sie ein vertrauenswürdiges (ACME ist in PMG eingebaut) oder aktivieren Sie `insecure_tls`. |
| Authentifizierungsfehler | Falscher Realm im Benutzernamen — er muss `dumbmonit@pmg` lauten, nicht `dumbmonit` — oder das Konto ist nicht Enabled. Nur ein fehlschlagendes `GET /version` verurteilt die gesamte Abfrage; jeder andere fehlschlagende Aufruf wird in `scrape_errors` gezählt. |
| Unzureichende Rechte | Die Rolle des Kontos liegt unter `Audit`. `quser` sieht nur seine eigene Mail; `helpdesk` kann den Knoten oder die Statistiken nicht lesen. |
| Jede `mail_*`-Metrik ist null | Das Gateway hat seit Mitternacht tatsächlich nichts gefiltert. Die Totals decken den aktuellen lokalen Tag auf dem Gateway ab und setzen sich bei dessen Mitternacht zurück, nicht Ihrer. |
| Überhaupt keine `service_running`-Zeitreihe | Die Option `services` ist aus, oder die Rolle kann sie nicht auflisten. Eine in der Liste fehlende einzelne Unit ist auf diesem Knoten einfach nicht installiert (zum Beispiel `chrony` auf einer Maschine, die `systemd-timesyncd` nutzt) und wird absichtlich nicht als gestoppt gemeldet. |
| Kein `signature_age_seconds` für einen SpamAssassin-Kanal | Dieser Kanal wurde nie aktualisiert, daher gibt PMG kein Datum zurück. Führen Sie `sa-update` auf dem Gateway aus; ein konfigurierter, aber nie abgerufener sekundärer Kanal bleibt absichtlich undatiert. |
| Überhaupt keine ClamAV-Datenbank | ClamAV ist nicht installiert, oder `freshclam` hat nie einen Download abgeschlossen. Der Endpunkt antwortet mit einer leeren Liste, was kein Fehler ist. |
| Keine Cluster-Metriken | Das Gateway ist eigenständig. `/config/cluster/status` antwortet mit einer leeren Liste, und keine Cluster-Regel kann feuern. |
| "Mail stuck in the queue" feuert spät | `queue_oldest_age_seconds` ist das Minimum des höchsten belegten Altersbereichs von `qshape`, daher meldet es absichtlich zu niedrig. |
| `quarantine_messages{kind="attachment"}` fehlt | Die Option `attachment_quarantine` ist aus: Diese Quarantäne hat keinen Zähl-Endpunkt, daher bedeutet sie zu zählen, sie zu listen. |
