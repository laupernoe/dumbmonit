# DumbMonit selbst überwachen {#watching-dumbmonit-itself}

Wer überwacht den Wächter? DumbMonit antwortet in zwei Teilen:

- ein **ausgehender Heartbeat** für das, was es von innen nicht sehen kann: der
  Host ausgefallen, der Container gestoppt, die Engine hängt;
- **integrierte Regeln** zur eigenen Gesundheit, für das, was es sehen kann: das
  Datenvolume läuft voll, die Metrikdatenbank startet in einer Schleife neu,
  Abfragen hängen, Schreibvorgänge schlagen fehl, die Datenbank wächst
  ungewöhnlich.

Beides liegt unter **Einstellungen → System → DumbMonit selbst**, das auch den
aktuellen Zustand der Engine zeigt.

## Der ausgehende Heartbeat {#the-outgoing-heartbeat}

DumbMonit ruft in regelmäßigen Abständen (standardmäßig 60 Sekunden) eine URL von
Ihnen auf, **nur solange die Engine gesund ist**:

- der Scheduler läuft (er hat in den letzten zwei Minuten eine Runde beendet);
- die SQLite-Datenbank antwortet;
- VictoriaMetrics antwortet.

Fällt eines davon aus, wird nichts gesendet. Ein „Totmannschalter“-Dienst wartet auf
diese Aufrufe und alarmiert Sie, wenn sie ausbleiben, ob die Ursache der ganze Host,
der Container oder eine einzelne Komponente ist.

### Einrichten {#setting-it-up}

1. Legen Sie bei einem Dienst Ihrer Wahl einen Check an und kopieren Sie seine
   Ping-URL:
   - **Healthchecks.io** (oder ein selbst gehostetes Healthchecks): ein Check mit
     einer Periode gleich dem Intervall und einer etwas längeren Karenzzeit
     (`https://hc-ping.com/<uuid>`);
   - **Uptime Kuma**: ein Monitor vom Typ *Push*
     (`https://kuma.example.com/api/push/<token>?status=up&msg=OK&ping=`);
   - **Better Stack**: ein Heartbeat (`https://uptime.betterstack.com/api/v1/heartbeat/<token>`);
   - jeder Dienst, der ein einfaches `GET` annimmt und alarmiert, wenn es ausbleibt
     (Cronitor, Dead Man's Snitch…).
2. Fügen Sie unter **Einstellungen → System → DumbMonit selbst** die URL ein, wählen
   Sie das Intervall (30 bis 3600 Sekunden), schalten Sie **Heartbeat senden** ein
   und speichern Sie.
3. Drücken Sie **Testen**: Sofort geht ein Aufruf hinaus, und das Ergebnis wird
   angezeigt. Der Dienst sollte den Check als aktiv markieren.

Der Aufruf ist ein `GET` mit 10 Sekunden Timeout. Er folgt demselben Schutz wie die
Benachrichtigungskanäle: Er wird nie an die eigene Weboberfläche von DumbMonit oder an
sein VictoriaMetrics gesendet.

!!! note "ntfy"
    ntfy liefert jede Nachricht aus, die es erhält. Zeigt der Heartbeat auf ein
    ntfy-Topic, bekommen Sie also jede Minute eine Benachrichtigung, statt wenn die
    Aufrufe ausbleiben. Nutzen Sie einen Totmannschalter-Dienst und behalten Sie ntfy
    als Kanal für die Alarme selbst.

### Fehlersignal {#failure-signal}

Standardmäßig hört eine kranke Engine einfach auf zu rufen, und der Dienst alarmiert,
sobald seine Karenzzeit verstrichen ist. Schalten Sie **Fehler signalisieren** ein, um
stattdessen die Fehler-URL aufzurufen, die der Dienst sofort meldet:

| Dienst | Fehler-URL |
|---|---|
| Healthchecks.io, Better Stack | `/fail` an den Pfad angehängt: `https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | `status=up` in der Query durch `status=down` ersetzt |

Lassen Sie es bei jedem anderen Dienst aus: Eine unbekannte URL mit `/fail` am Ende
würde meist 404 antworten, was ohnehin wie „es wird nichts gesendet“ aussieht.

### Die URL ist ein Secret {#the-url-is-a-secret}

Eine Ping-URL enthält meist ihr eigenes Token: Wer sie kennt, kann Ihren Check
gesund aussehen lassen. DumbMonit speichert sie mit dem Instanz-Secret verschlüsselt,
gibt sie nie über die API zurück (nur ihr Host wird gezeigt) und lässt sie aus den
Logs und Fehlermeldungen heraus. Um sie zu ändern, geben Sie die neue ein; um sie zu
behalten, lassen Sie das Feld leer. **URL entfernen** vergisst sie und schaltet den
Heartbeat aus.

Die Einstellung ist nicht Teil des verschlüsselten Konfigurationsbündels: Setzen Sie
sie nach einer Wiederherstellung auf einer neuen Instanz erneut.

## Integrierte Regeln {#built-in-rules}

Jede Minute schreibt DumbMonit einige Messwerte über sich selbst nach VictoriaMetrics,
und die integrierten Regeln der Gruppe *DumbMonit selbst* lesen sie. Sie verhalten
sich wie jede andere Regel: dieselben Kanäle, dieselben Stummschaltungen, dieselben
Schwellen pro Regel unter **Warnungen → Regeln**.

| Regel | Löst aus, wenn | Standard |
|---|---|---|
| DumbMonit-Datenvolume fast voll | Weniger als 10 % frei auf dem Volume, das `/data` hält | 15 Min., Warnung |
| Metrikdatenbank von DumbMonit startet in einer Schleife neu | Das eingebettete VictoriaMetrics wurde in 30 Minuten 3-mal oder öfter neu gestartet | 1 Min., kritisch |
| DumbMonit-Scheduler hängt | Seit mehr als zwei Minuten keine Planungsrunde | 5 Min., kritisch |
| Abfragen von DumbMonit sind im Verzug | Monitore bleiben nach jeder Runde 15 Minuten lang überfällig | 15 Min., Warnung |
| Metrik-Schreibvorgänge von DumbMonit schlagen fehl | Batches von VictoriaMetrics abgelehnt oder Samples verworfen, in den letzten 15 Minuten | 5 Min., Warnung |
| Datenbank von DumbMonit wächst schnell | `dumbmonit.db` hat sich an einem Tag mehr als verdoppelt und ist über 100 MB | 1 Std., Warnung |
| DumbMonit-Backup lief nicht | Seit zwei Tagen kein erfolgreiches lokales Backup ([Details](backup.md#scheduled-local-backups)) | 1 Std., Warnung |

Diese Regeln brauchen VictoriaMetrics, um ausgewertet zu werden. Ist VictoriaMetrics
selbst dauerhaft ausgefallen oder der ganze Host, kann keine Regel auslösen: Das ist
die Aufgabe des Heartbeats.

### Messwerte {#measurements}

| Metrik | Bedeutung |
|---|---|
| `dumbmonit_instance_data_free_bytes` | Freier Platz auf dem Volume, das `/data` hält, für einen unprivilegierten Benutzer. |
| `dumbmonit_instance_data_size_bytes` | Größe dieses Volumes. |
| `dumbmonit_instance_database_bytes` | Größe von `dumbmonit.db` und ihrem Write-Ahead-Log. |
| `dumbmonit_instance_scheduler_lag_seconds` | Sekunden seit der letzten abgeschlossenen Runde des Schedulers (normalerweise unter 2). |
| `dumbmonit_instance_scheduler_backlog` | Monitore, die nach der letzten Runde noch für eine Abfrage fällig sind. |
| `dumbmonit_instance_sample_writes_failed_total` | Batches, die VictoriaMetrics abgelehnt oder nicht beantwortet hat, seit dem Serverstart. |
| `dumbmonit_instance_samples_dropped_total` | Samples, die vor dem Schreibpuffer verworfen wurden, seit dem Serverstart. |
| `dumbmonit_instance_vm_restarts_total` | Neustarts des eingebetteten VictoriaMetrics seit dem Serverstart (nur eingebettet). |

Sie tragen kein Label `target`: Die Alarme, die sie auslösen, hängen an keinem
Monitor. Für ein eigenes Prometheus liefert `GET /metrics` ähnliche Zahlen, siehe
[DumbMonit scrapen](../reference/metrics.md#scraping-dumbmonit).

## API {#api}

| Methode | Route | |
|---|---|---|
| `GET` | `/api/self-monitoring` | Zustand der Engine, Datenvolume, Datenbankgröße, Heartbeat-Einstellung und letzter Heartbeat. |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`. Fehlendes `url` behält sie, `""` löscht sie. |
| `POST` | `/api/self-monitoring/heartbeat/test` | Ein Aufruf jetzt, an `{ "url" }` oder an die gespeicherte URL. |

Die beiden Schreibaufrufe sind für Administratoren (oder ein Token mit dem Scope
`write`).
