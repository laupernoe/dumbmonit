# FAQ

## Ich habe das Passwort verloren

Fragen Sie zuerst einen anderen Administrator: In Settings → **Users**
setzt **Edit** auf Ihrem Konto ein neues Passwort und zeigt es einmal,
und **Reset 2FA** in derselben Zeile löscht einen zweiten Faktor, der
Sie ausschließt. Das ist die alltägliche Antwort — eine Instanz mit
zwei Admin-Konten braucht den Rest dieser Seite nie.

Wenn sich niemand mehr einloggen kann, gibt es keine Wiederherstellungs-
E-Mail: Starten Sie den Server einmal mit `DUMBMONIT_RESET_PASSWORD=1`:

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
```

Jedes Konto und jede Sitzung wird beim Start entfernt, und die UI
zeigt wieder `/setup`. Erstellen Sie den ersten Administrator, starten
Sie dann erneut ohne die Variable (`docker compose up -d dumbmonit`,
mit der Variable in Ihrer Umgebung nicht gesetzt oder leer). Geräte,
Regeln, Kanäle und die Single-Sign-On-Einstellungen bleiben unberührt.

## Keine Daten nach dem Hinzufügen eines Geräts

Geben Sie ihm eine Minute: Die erste Abfrage läuft im Intervall des
Geräts (standardmäßig 60 s), und Schreibvorgänge erreichen
VictoriaMetrics alle 5 Sekunden (`DUMBMONIT_FLUSH_INTERVAL_SECS`).
Dann:

1. Öffnen Sie die Geräteseite und klicken Sie auf **Probe now**. Es
   meldet, wie viele Samples und Zeitreihen eine Abfrage erzeugt hat,
   oder den genauen Fehler.
2. *Misconfigured* mit einem Fehler unter der Adresse ist ein
   Konfigurationsproblem: falsche Community oder Passwort, abgelehntes
   Zertifikat (aktivieren Sie "Accept an unverifiable certificate"),
   fehlende Fähigkeit. Es wird angezeigt, nicht alarmiert.
3. *Unreachable* ohne Fehler bedeutet, dass das Gerät überhaupt nicht
   geantwortet hat. Bei SNMP sieht eine falsche Community genau wie ein
   ausgeschaltetes Gerät aus, weil das Gerät sie nie beantwortet.
   Prüfen Sie die Community, dass SNMP aktiviert ist, und dass das
   Gerät die Adresse des Servers erlaubt.
4. Prüfen Sie, dass der Container das Gerät erreichen kann:
   Docker-Netzwerke und Firewalls sind die üblichen Übeltäter.

## Ping sagt "configuration error"

Der Ping-Monitor braucht einen ICMP-Echo-Socket. Der Container läuft
ohne jede Capability, daher erlaubt die Compose-Datei diese Sockets mit
dem sysctl `net.ipv4.ping_group_range` unter dem Dienst `dumbmonit`:
Stellen Sie sicher, dass die `sysctls:`-Zeilen noch da sind, dann
`docker compose up -d`. Siehe
[ICMP-Ping ohne Capability](install/docker.md#icmp-ping-without-a-capability).

## Der Agent erscheint nicht

- Der Installer macht einen Test-Push, bevor er den Dienst startet,
  und stoppt mit einem expliziten Fehler, wenn URL oder Token falsch
  sind. Korrigieren Sie `/etc/dumbmonit/agent.yaml` und starten Sie
  `dumbmonit-agent` neu.
- Die URL im Installationsbefehl ist die, mit der Ihr Browser die UI
  erreicht hat. Von der überwachten Maschine aus muss diese URL den
  Server erreichen: `curl http://server:8080/api/health` von dort
  sagt es Ihnen.
- Hinter einem Reverse Proxy muss `/api/ingest` weitergeleitet und
  Bodies bis 16 MB erlaubt werden (`client_max_body_size 16m` in
  nginx).
- `journalctl -u dumbmonit-agent -f` zeigt jeden abgelehnten Push. Das
  Token selbst erscheint nie im Log.
- Ein widerrufenes Token stoppt den Agent beim nächsten Push; führen
  Sie den Installer mit einem neuen Token erneut aus.

Siehe [Linux-, macOS-, FreeBSD- und Windows-Agent](devices/agent.md).

## Den Port ändern

Der Container hört auf 8080; der Host-Port ist, was
`docker-compose.yml` veröffentlicht. Setzen Sie `DUMBMONIT_PORT` (in
einer `.env`-Datei neben der Compose-Datei, oder auf der Kommandozeile):

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Um den Port *innerhalb* des Containers zu ändern, setzen Sie
`DUMBMONIT_BIND`, zum Beispiel `0.0.0.0:9000`, und passen Sie das
Port-Mapping an.

## Warum ist der Alarm "Advisory", wenn die Regel "warning" sagt?

Die API speichert drei Schweregrade: `info`, `warning`, `critical`. Die
Oberfläche zeigt sie mit den Worten des Wetterberichts: Info, Advisory,
Warning. Siehe [Schweregrade](alerting/index.md#severities).

## Ein Alarm ist "Suppressed by parent"

Das Gerät hat ein übergeordnetes Gerät, das nicht erreichbar ist,
daher werden seine eigenen Alarme zurückgehalten statt gesendet.
Beheben Sie das übergeordnete Gerät; die Alarme des Kindes lösen sich
oder feuern danach von selbst, ohne eine zweite Benachrichtigung.
Siehe [Abhängigkeitsunterdrückung](alerting/index.md#dependency-suppression).

## "Unusual CPU" feuert nie

Es ist eine saisonale Baseline, die für ihre ersten 14 Tage auf jeder
Zeitreihe still bleibt. Während dieser Zeit zeigt sie nur, was sie
ausgelöst hätte. Siehe [Baseline-Lernen](alerting/index.md#baseline-learning).

## Kann ich mein eigenes VictoriaMetrics nutzen?

Ja: Setzen Sie `DUMBMONIT_VM_URL` auf dessen Adresse
(`http://host:8428`), und das eingebettete VictoriaMetrics wird nicht
gestartet. Jede Metrik hat das Präfix `dumbmonit_`, kann die Instanz
also mit anderen Tools teilen. `GET /api/health` meldet
`victoria.embedded: false`, sobald der Wechsel erfolgt ist.

## Muss DumbMonit aus dem Internet erreichbar sein?

Nein. Es greift auf Geräte und Benachrichtigungsdienste zu; nichts
ruft zurück, außer Agents, die die URL des Servers erreichen müssen.
Behalten Sie es in Ihrem LAN oder hinter einem VPN; wenn Sie es
exponieren, nutzen Sie einen Reverse Proxy mit TLS und setzen Sie
`DUMBMONIT_COOKIE_SECURE=1`.

## Gibt es ein API-Token?

Ja. Settings → **API & assistants** erstellt `dmt_…`-Tokens mit einem
Scope `read` oder `write`; senden Sie eines als
`Authorization: Bearer dmt_…` auf jeder Route, und kein Cookie oder
CSRF-Header wird benötigt. Dasselbe Token verbindet einen
[Assistant](using/assistant.md) über MCP. Tokens verwalten nie Konten,
Login-Einstellungen oder andere Tokens. Siehe die
[HTTP-API-Referenz](reference/api.md#authentication).
