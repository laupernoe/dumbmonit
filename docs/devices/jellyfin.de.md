# Jellyfin

Ob Jellyfin seine Bibliotheken aktuell hält und seine Zuschauer bedient:
geplante Tasks, die beim letzten Lauf fehlschlugen (zuerst der
Bibliotheksscan), Plugins, die nicht mehr laden, ein wartender Neustart,
und was gerade läuft, transkodiert oder nicht.

DumbMonit liest Jellyfins API mit einem im Dashboard erstellten
**API-Key**. Es startet nie einen Task, stoppt nie eine Wiedergabe und
ändert nie eine Einstellung.

Der beobachtenswerte Fehler: **der Bibliotheksscan, der jede Nacht
fehlschlägt**. Eine ausgehängte Netzwerkfreigabe, eine volle Platte, und
neue Filme und Episoden tauchen nicht mehr auf; Jellyfin selbst spielt
weiterhin, was es schon kennt, und die einzige Spur ist eine Zeile in
Scheduled Tasks.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_jellyfin_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `pending_restart` | 1, wenn Jellyfin auf einen Neustart wartet (nach einer Plugin-Installation oder einem Update) | |
| `shutting_down` | 1, während Jellyfin herunterfährt | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | Tasks, deren letzter Lauf fehlschlug oder abgebrochen wurde; jetzt laufende Tasks | |
| `scheduled_task_failed` | Wert 1 pro fehlgeschlagenem Task (höchstens 32) | `task`, `error` (erste Zeile der Meldung) |
| `library_scan_last_ok`, `library_scan_age_seconds` | letzter Lauf von "Scan Media Library": erfolgreich oder nicht, und wie lange er zurückliegt | |
| `plugins`, `plugins_broken` | installierte Plugins; solche, die nicht laden oder diese Version nicht unterstützen | |
| `plugin_broken` | Wert 1 pro defektem Plugin | `plugin`, `status` |
| `sessions`, `streams`, `streams_paused`, `transcodes` | Client-Sitzungen; die, die etwas abspielen, pausierte, und transkodierende | |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Jellyfin prüft nicht selbst auf eigene Updates (das übernimmt das Paket
oder das Image), daher gibt es keine Update-Metrik.

Die geltenden [eingebauten Regeln](../alerting/rules.md#self-hosted-applications):
Jellyfin scheduled task failed, Jellyfin plugin broken, Jellyfin restart
pending, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite ruft Jellyfin nie auf. Es sagt zuerst, was nicht stimmt
(fehlgeschlagene Tasks, defekte Plugins, ein wartender Neustart), dann
Streams, Transkodierungen, Sitzungen und das Alter des letzten
Bibliotheksscans, dann die fehlgeschlagenen Tasks und defekten Plugins
namentlich mit Jellyfins eigenem Fehler.

## Einen API-Key in Jellyfin erstellen

1. In Jellyfin: Dashboard → API Keys (unter Advanced) → +, und den Key
   wie folgt benennen. Kopieren Sie ihn.

    ```
    DumbMonit
    ```

2. Jellyfin kann nicht einschränken, was ein Key darf: Jeder Key hat
   volle Rechte auf dem Server. Es ist trotzdem der einzige Weg, geplante
   Tasks und die Wiedergabe aller zu lesen; ein normaler Benutzer sieht
   beides nicht.

3. Geben Sie in DumbMonit Jellyfins Adresse ein, zum Beispiel
   "jellyfin.lan" (Port 8096) oder "https://media.example.com" hinter
   einem Reverse Proxy, und fügen Sie den Key ein.

4. DumbMonit liest nur /System/Info, /ScheduledTasks, /Sessions und
   /Plugins. Es startet nie einen Task, stoppt nie eine Wiedergabe und
   ändert nie eine Einstellung.

!!! warning
    Da der Key alle Rechte trägt, behalten Sie ihn in einem Netzwerk, dem
    Sie vertrauen, und stellen Sie Jellyfin über das Internet hinter
    HTTPS. Wenn er jemals durchsickert, widerrufen Sie ihn auf derselben
    Seite: nichts anderes nutzt ihn.

## Credentials

| Credential | Felder |
|---|---|
| API key | Im Header `Authorization: MediaBrowser … Token="…"` gesendet, die von Jellyfin 10.11 und später standardmäßig akzeptierte Form. |

Adresse: ein Hostname (`jellyfin.lan`), `host:port`, oder eine
vollständige URL mit Pfadpräfix (`https://example.com/jellyfin`); ein
abschließendes `/web` wird akzeptiert. Optionen: Protokoll (standardmäßig
HTTP), Port (8096), Zertifikatsprüfung und Anfrage-Timeout (15 s).

Validiert gegen Jellyfin 12.1.0 (offizielles Image).
