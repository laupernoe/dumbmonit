# Proxmox Datacenter Manager

Die Konsole, die mehrere Proxmox-VE-Cluster und
Proxmox-Backup-Server-Instanzen föderiert: welche Instanzen sie
erreicht, der gesamte Bestand auf einen Blick, die Tasks, die
irgendwo fehlschlugen, und die eigene Gesundheit der Konsole.

!!! warning "Nur gegen eine Testinstanz getestet"

    Diese Integration wurde gegen ein zu diesem Zweck installiertes
    Proxmox Datacenter Manager 1.1 gebaut und verifiziert, nicht gegen
    ein produktives. Jeder Aufruf ist ein Lesevorgang, und die Formen
    stammen von einem echten Datacenter Manager, aber noch niemand hat
    es wochenlang auf einem lebenden Datacenter Manager laufen lassen.
    Sagen Sie uns, was kaputt ist.

## Ein Monitor für den Bestand, oder ein Monitor pro Cluster?

Ein Datacenter-Manager-Monitor ist kein Ersatz für [Proxmox-VE](proxmox.md)-
und [Proxmox-Backup-Server](pbs.md)-Monitore. Er beantwortet eine
andere Frage.

| | Datacenter-Manager-Monitor | Ein Monitor pro Cluster und pro Backup-Server |
|---|---|---|
| Zu erstellende Monitore | eines | eines pro Cluster, eines pro Backup-Server |
| Zu verwaltende Zugangsdaten | ein Token, auf der Konsole | ein Token pro Instanz |
| Last auf jedem Cluster | keine von uns: Wir lesen den eigenen Cache der Konsole | eine Abfrage pro Intervall, pro Cluster |
| Was Sie bekommen | Erreichbarkeit, Version, Gast- und Knotenzahlen, CPU-, Speicher- und Storage-Totals, fehlgeschlagene Tasks | all das plus Gäste einzeln, Snapshots, Backup-Jobs, HA, Ceph, Replikation, Platten und SMART, ZFS-Pools, der 30-Tage-Backup-Kalender |
| Was Sie nicht bekommen | Pro-Gast-Zeitreihen, Pro-Datastore-Details, Platten, Snapshots, Task-Logs | ein einziges bestandsweites Total |
| Hauptfehler | die Konsole kann eine Instanz nicht erreichen | der Cluster antwortet uns nicht |

**Unsere Empfehlung.** Fügen Sie den Datacenter-Manager-Monitor hinzu,
wenn Sie bereits eine Konsole betreiben und eine Seite wollen, die
sagt, ob der Bestand vollständig ist — es ist der günstigste Weg zu
bemerken, dass ein entfernter Standort weggefallen ist, und es kostet
die Cluster nichts. Fügen Sie einen dedizierten Monitor für jeden Cluster
und Backup-Server hinzu, den Sie tatsächlich betreiben: Dort liegt
die Tiefe, und das sagt Ihnen, *welche* VM stoppte oder *welches*
Backup fehlschlug. Die beiden überlappen, ohne zu kollidieren; ein
fehlgeschlagenes Backup hebt dann einen Alarm vom PBS-Monitor (mit dem
Task-Log) und einen von der Konsole (die die Instanz benennt). Schalten
Sie die Task-Regel der Konsole stumm, wenn Sie das redundant finden,
oder begrenzen Sie sie auf die Instanzen, für die Sie keinen dedizierten
Monitor haben.

Betreiben Sie überhaupt keine Konsole, installieren Sie keine nur
zur Überwachung: dedizierte Monitore sehen strikt mehr.

## Die Monitorseite

Über die generischen Diagramme hinaus zeigt ein Datacenter-Manager-
Monitor drei Panels, aus dem gelesen, was die Abfrage gespeichert hat —
das Öffnen der Seite fragt weder die Konsole noch die von ihr
verwalteten Cluster ab:

* **Federated instances** — ein Streifen mit Bestandstotals
  (Instanzen, Knoten online, laufende und gestoppte Gäste, Kerne,
  Speicher, Storage), dann eine Zeile pro föderierter Instanz.
  Instanzen, die die Konsole nicht erreichen kann, sortieren zuerst,
  dann die mit fehlgeschlagenen Tasks, dann die mit Versionsrückstand.
  Jede Zeile gibt das Produkt (Proxmox VE oder Backup Server), die
  Version, Knoten- und Gastzahlen, Speicher- und Storage-Nutzung, den
  Abonnementstatus, die konfigurierte Adresse, wann die Konsole
  zuletzt von ihr sammelte — und, wenn es fehlschlug, die Meldung, die
  die Konsole selbst empfing. Diese Meldung ist das Einzige, das den
  Ausfall erklärt, daher wird sie wortgetreu angezeigt.
* **Failures** — jeder im Bestand in den letzten 14 Tagen
  fehlgeschlagene Task, neueste zuerst, mit der Instanz, auf der er
  lief, der Art des Tasks, wann er begann, wie lange er dauerte und
  der Fehlermeldung. Das Task-Log selbst bleibt auf der Instanz: Öffnen
  Sie es dort, oder fügen Sie diesen Cluster als eigenen Monitor hinzu,
  wo *Show log* es abruft.
* **Console host** — die Maschine, die die Konsole betreibt: CPU,
  Speicher, Root-Dateisystem, Uptime und Kernel, seine Zertifikate mit
  den verbleibenden Tagen bis zum Ablauf, die auf ein Update
  wartenden Pakete, und der Abonnementstatus der verwalteten Knoten.
  Das ganze Panel ist optional: Ein Token ohne `Sys.Audit` liest
  nichts davon, und das Panel sagt das, statt Nullen zu zeigen.

Die Task-Historie wird in SQLite gehalten (`pdm_task_history`, 19
Tage, höchstens 5000 Tasks pro Monitor), sodass die Fehlerliste
Neustarts überlebt, obwohl jede Abfrage nur das Task-Fenster betrachtet.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_pdm_`.

| Familie | Metriken | Labels |
|---|---|---|
| Bestand | `remotes_total`, `remotes_failed`, `nodes_online`, `nodes_offline`, `cpu_used_cores`, `cpu_total_cores`, `memory_used_bytes`, `memory_total_bytes`, `memory_used_percent`, `storage_used_bytes`, `storage_total_bytes`, `storage_used_percent`, `datastores_total` | |
| Bestandsgäste | `guests_running`, `guests_stopped` | `guest_type` (`qemu`, `lxc`) |
| Instanzen | `remote_reachable` (1: die Konsole erhielt eine Antwort, 0: nicht), `remote_version_info` (Vorhandensein-Zeitreihe, Label `version`), `remote_version_behind`, `remote_nodes_online`, `remote_nodes_offline`, `remote_guests_running`, `remote_guests_stopped`, `remote_cpu_used_cores`, `remote_cpu_total_cores`, `remote_cpu_used_percent`, `remote_memory_used_bytes`, `remote_memory_total_bytes`, `remote_memory_used_percent`, `remote_storage_used_bytes`, `remote_storage_total_bytes`, `remote_storage_used_percent`, `remote_datastores`, `remote_subscription_active`, `remote_updates_pending`, `remote_last_collection_age_seconds`, `remote_tasks_failed` | `remote`, `type` (`pve`, `pbs`) |
| Tasks im Fenster | `tasks_total`, `tasks_failed`, `tasks_running` | |
| Console host | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`, `node_load1`, `node_memory_used_bytes`, `node_memory_total_bytes`, `node_memory_used_percent`, `node_swap_used_bytes`, `node_swap_total_bytes`, `node_rootfs_used_bytes`, `node_rootfs_total_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `node_updates_pending`, `version_info` | |
| Konsolen-Zertifikate | `node_certificate_expiry_days` (negativ nach Ablauf) | `file` (`proxy.pem`) |
| Abonnement | `subscription_active` (Label `status`), `subscription_active_nodes`, `subscription_total_nodes` | |
| Sammlung | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Ein Task, der mit `WARNINGS: n` endet, zählt als erfolgreich. Nur ein
fehlschlagendes `GET /version` verurteilt die gesamte Abfrage; jeder
andere fehlschlagende Aufruf wird in `scrape_errors` gezählt, und eine
von der Konsole nicht erreichbare Instanz ist eine Messung
(`remote_reachable = 0`), kein Sammlungsfehler.

`remote_version_behind` vergleicht die föderierten Instanzen
**miteinander**, Produkt für Produkt: Es ist 1, wenn ein anderer
Proxmox-VE-Cluster — oder ein anderer Backup-Server — im selben
Bestand eine neuere Version läuft. Es ist kein Vergleich mit dem, was
Proxmox veröffentlicht; eine Abfrage hat keinen Grund, ins Internet
zu greifen. Die Zeitreihe existiert nur für eine Instanz, deren
Version gelesen wurde, daher gilt eine nicht erreichbare Instanz nie
als aktuell.

Storage-Totals zählen jeden Storage-Namen einmal pro Instanz: Ein von
fünf Knoten eines Clusters geteilter Storage wird einmal gezählt,
nicht fünfmal.

Geltende eingebaute Regeln: Monitor unreachable, Federated instance
unreachable, Federated instance behind, Task failed on a federated
instance, Datacenter console disk almost full, Datacenter console
certificate expiring, Datacenter console updates pending.
Benachrichtigungen benennen die betroffene Instanz.

## Was in Proxmox Datacenter Manager vorzubereiten ist

Die Schritte unten sind die, die der Hinweis neben dem Formular zeigt.
Das Prinzip: ein für die Überwachung reservierter Benutzer, ein Token
mit einer nur-lesenden Rolle — nie das Konto, mit dem Sie sich
einloggen.

1. Öffnen Sie in der Konsole Configuration → Access Control → Users
   und klicken Sie auf Add. Benennen Sie das Konto wie folgt und
   lassen Sie das Passwort leer: Das Token ist, was sich einloggt.

    ```
    dumbmonit@pdm
    ```

2. Öffnen Sie weiterhin unter Access Control API Tokens → Add, wählen
   Sie diesen Benutzer und benennen Sie das Token "monitor". Die
   Konsole zeigt das Secret einmal: Kopieren Sie es jetzt.

    ```
    dumbmonit@pdm!monitor
    ```

3. Geben Sie sowohl dem Benutzer als auch dem Token die Rolle
   `Auditor` auf `/` (die Spitze des Baums, also der gesamte Bestand).
   `Auditor` trägt `System.Audit`, `Resource.Audit` und
   `Access.Audit`, und kann nichts ändern. Ein Token hat nie mehr
   Rechte als sein Benutzer, daher wird die Berechtigung zweimal
   gewährt: Permissions → Add → User Permission, dann erneut mit API
   Token Permission.
4. Fügen Sie die Token-ID in DumbMonits Feld Token ID ein und das
   Secret in sein Feld Secret.
5. Geben Sie in DumbMonit die Konsolenadresse ein, zum Beispiel
   "dc.lan" oder "dc.lan:8443".

!!! warning
    Nutzen Sie nicht das Konto wiederverwendet, mit dem Sie sich
    einloggen: Ein durchgesickertes Token würde dann jeden von der
    Konsole föderierten Cluster auf einmal kontrollieren. Die Rolle
    `Auditor` kann nur lesen. Die Konsole nutzt auch standardmäßig ein
    selbstsigniertes Zertifikat: Wird die Verbindung deshalb
    abgelehnt, kreuzen Sie "Accept an unverifiable certificate" in den
    Optionen an. Zwei Dinge brauchen mehr als `Auditor` und werden
    einfach ohne es übersprungen: der Pro-Instanz-Metrik-Sammlungs-
    status, und die Update-Zusammenfassung der föderierten Instanzen.

Bevorzugen Sie die Kommandozeile?
`proxmox-datacenter-manager-admin` verwaltet Remotes, keine Benutzer;
erstellen Sie das Konto, das Token und die zwei Berechtigungen in der
Web-Oberfläche.

## Credentials

| Credential | Felder |
|---|---|
| API-Token (empfohlen) | **Token ID**: Benutzer, Realm und Token-Name, wie die Konsole sie zeigt, `dumbmonit@pdm!monitor`. **Secret**: die einmal beim Erstellen des Tokens gezeigte UUID. DumbMonit sendet sie als den von der Konsole erwarteten Header `PDMAPIToken=user@pdm!name:secret`. |

Es gibt keine Benutzername-/Passwort-Option, und das ist Absicht:
Seit ihrer ersten Version gibt die Konsole ihr Session-Ticket nur in
einem `HttpOnly`-Cookie zurück — der JSON-Body trägt ein unbrauchbares
`ticket-info`. Ein Login würde gegenüber einem Token nichts gewinnen,
während er bei jeder Abfrage eine eingeloggte Sitzung öffnet. Tokens
sind ohnehin, was Proxmox für Automatisierung empfiehlt.

Rechte, nur-lesend. Am einfachsten und ausreichend: die eingebaute
Rolle `Auditor` auf `/`, dem Benutzer *und* dem Token gewährt. Pro
Endpunkt:

| Endpunkt | Braucht |
|---|---|
| `GET /version` | jeder authentifizierte Benutzer |
| `GET /remotes/remote` | gibt die Instanzen zurück, die der Aufrufer auditieren darf |
| `GET /remotes/remote/{id}/version` | `Resource.Audit` auf `/resource/{id}` |
| `GET /resources/status`, `GET /resources/list` | `Resource.Audit` auf `/resource` |
| `GET /resources/subscription` | `Resource.Audit` auf `/resource` |
| `GET /remotes/tasks/list` | `Resource.Audit` auf `/resource/{remote}`, pro Instanz |
| `GET /nodes/{node}/status` | `System.Audit` auf `/system/status` |
| `GET /nodes/{node}/certificates/info`, `GET /nodes/{node}/subscription` | jeder authentifizierte Benutzer |
| `GET /nodes/{node}/apt/update` | `System.Audit`; ein 403 wird toleriert: keine `node_updates_pending`-Zeitreihe, kein Scrape-Fehler |
| `GET /remotes/metric-collection/status` | mehr als `Auditor`; ein 403 wird toleriert: keine `remote_last_collection_age_seconds`-Zeitreihe, kein Scrape-Fehler |
| `GET /remotes/updates/summary` | `Resource.Modify` — ein Schreibrecht. Deshalb standardmäßig aus (`remote_updates`) |

Beachten Sie, dass die Konsole die für ihre Instanzen gehaltenen
Access-Tokens zensiert (`GET /remotes/remote` gibt ein leeres `token`
zurück), und DumbMonit liest dieses Feld ohnehin nie: Kein
Instanz-Secret durchläuft je den Collector, die gespeicherte Ansicht
oder die API.

Adresse: `10.0.0.40`, `dc.lan`, `dc.lan:8443`, `[fd00::2]` oder eine
vollständige URL. Das Schema `https` und Port 8443 werden
hinzugefügt, falls fehlend.

## Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | API port | `8443` | Genutzt, falls die Adresse keinen Port angibt. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Proxmox Datacenter Manager liefert standardmäßig ein selbstsigniertes Zertifikat: aktivieren Sie das, wenn die Verbindung deshalb abgelehnt wird. |
| `request_timeout_seconds` | Timeout per request (seconds) | `20` | Erlaubte Zeit für jeden API-Aufruf, von 1 bis 120. Die Konsole leitet Aufrufe an jede föderierte Instanz weiter, daher hält ein langsamer Standort die gesamte Antwort auf. |
| `node` | Console node name | `localhost` | Name des die Konsole betreibenden Knotens. Ein Datacenter Manager hat einen einzigen Knoten und nennt ihn "localhost". |
| `task_lookback_hours` | Task window (hours) | `24` | Ältere Tasks werden nicht unter den Fehlschlägen gezählt, von 1 bis 8760. |
| `max_age_seconds` | Accepted cache age (seconds) | `60` | Wie veraltet das eigene Inventar der Konsole sein darf, bevor sie die föderierten Instanzen erneut abfragt, von 0 bis 3600. Null fragt bei jeder Abfrage nach frischen Daten, was jeden Cluster wieder auf die Leitung bringt. |
| `remotes` | Monitored instances | *(leer)* | Namen der zu überwachenden föderierten Instanzen, kommagetrennt. Leer: jede Instanz. |
| `max_remotes` | Instance version limit | `100` | Maximale Anzahl Instanzen, nach deren Version bei jeder Abfrage gefragt wird, von 1 bis 1000. |
| `versions` | Read each instance version | `true` | Ein Aufruf pro föderierter Instanz, um ihre Version zu melden und die hinter ihren Peers zurückgebliebenen zu markieren. Eine nicht antwortende Instanz zählt als nicht erreichbar. |
| `tasks` | Watch tasks across instances | `true` | Liest die Task-Liste jeder föderierten Instanz, um die Fehlschläge zu melden. |
| `node_status` | Watch the console host | `true` | Liest CPU, Speicher, Root-Dateisystem, Uptime, Zertifikate und Abonnement der die Konsole betreibenden Maschine. |
| `updates` | Count pending updates | `true` | Listet die auf ein Update wartenden Pakete der Konsole selbst. |
| `remote_updates` | Count updates on each instance | `false` | Liest die Update-Zusammenfassung der Konsole für die föderierten Instanzen. Standardmäßig aus: Die Konsole schützt diesen Pfad mit `Resource.Modify`, einem Schreibrecht, das ein Monitoring-Konto keinen Grund hat zu halten. |

`max_age_seconds` ist das, was es zu verstehen lohnt. Die Konsole
hält ihr eigenes Inventar, von ihrer eigenen Sammelschleife
aktualisiert. Bei jeder Abfrage nach frischen Daten zu fragen lässt
die Konsole jeden Cluster erneut aufrufen, zusätzlich zu ihrem
normalen Rhythmus — genau die Last, die ein einziges Konsolen-Monitor
vermeiden soll. Sechzig Sekunden sind immer noch frischer als das
Standard-Abfrageintervall.

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| Zertifikatsfehler | Selbstsigniertes Zertifikat: installieren Sie ein vertrauenswürdiges (ACME ist in die Konsole eingebaut) oder aktivieren Sie `insecure_tls`. |
| Authentifizierungsfehler | Secret in Token ID eingefügt (oder umgekehrt), oder die Berechtigung wurde dem Benutzer gewährt, aber nicht dem Token — ein Token hat nie mehr Rechte als sein Benutzer, und die zwei ACLs sind getrennte Einträge. |
| Jede Instanz zeigt sich als nicht erreichbar, aber die eigene Seite der Konsole ist in Ordnung | Das Token hat `Auditor` am Benutzer, aber nicht am Token, oder nur auf `/system`: `Resource.Audit` auf `/resource` ist, was die Konsole für ihre Instanzen antworten lässt. |
| Eine Instanz ist nicht erreichbar, und die Meldung erwähnt einen Fingerprint | Die Konsole verifiziert das Zertifikat jeder von ihr verwalteten Instanz; beheben Sie es in der Konsole (Remotes → die Instanz), nicht hier. |
| Keine `remote_last_collection_age_seconds`-Zeitreihe | `GET /remotes/metric-collection/status` braucht mehr als `Auditor` und wird still übersprungen. Nichts sonst ist betroffen. |
| Keine `remote_updates_pending`-Zeitreihe | `remote_updates` ist standardmäßig aus, weil dieser Pfad `Resource.Modify` braucht. Schalten Sie es nur ein, wenn Sie bereit sind, ein Schreibrecht zu gewähren. |
| Kein Console-Host-Panel | Dem Token fehlt `System.Audit` auf `/system/status`, oder `node_status` ist aus. |
| "Federated instance behind" feuert nie | Es vergleicht Instanzen desselben Produkts miteinander. Eine Konsole, die einen einzigen Proxmox-VE-Cluster föderiert, hat nichts, womit sie ihn vergleichen könnte. |
| Bestandstotals sind alle Gedankenstriche | Die Konsole antwortete, zählte aber nichts — meist weil jede Instanz nicht erreichbar ist. Die Instanzzeilen sagen warum. |
