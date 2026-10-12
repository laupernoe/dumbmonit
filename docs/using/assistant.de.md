# Einen Assistant verbinden

DumbMonit hat einen eingebauten [MCP](https://modelcontextprotocol.io)-
Server: das Model Context Protocol, der Weg, auf dem Assistants wie
Claude, ChatGPT, Cursor oder der Agent von VS Code externe Werkzeuge
aufrufen. Verbinden Sie einen, und fragen Sie dann Dinge wie:

- "Ist alles in Ordnung?"
- "Schalte das NAS für zwei Stunden stumm, ich tausche eine Platte."
- "Was ist letzte Nacht passiert?"
- "Wie voll ist der Backup-Server, und wird es schlimmer?"
- "Poste ein Update auf der Statusseite: der Mailserver ist zurück."

Der Assistant sieht, was die Oberfläche sieht — Geräte, Alarme,
Historie, Metriken, Agents, Statusseiten — und nutzt dieselben
Worte: *reporting*, *unreachable*, *waiting*, *advisory*, *warning*,
*suppressed by parent*. Er läuft innerhalb des Servers: nichts neben
DumbMonit zu installieren, und die Werkzeuge rufen denselben Code wie
die Web-UI auf.

## 1. Ein Token erstellen

Settings → **API & assistants** (`/settings#assistant`). Benennen Sie
das Token nach dem Assistant oder der Maschine, auf der er läuft
("Claude on my laptop"), wählen Sie dann:

| Einstellung | Wahlmöglichkeiten |
| --- | --- |
| **Scope** | **Read**: nachsehen — Status, Geräte, Alarme, Historie, Metriken, Regeln, Agents, Statusseiten. Kann nie etwas ändern. **Read and write**: auch handeln — stummschalten, bestätigen, abfragen, ein Gerät hinzufügen, einen Container neu starten, einen Vorfall posten… (die [Schreibwerkzeuge](#what-the-assistant-can-call)). |
| **Expires** | 30 Tage, 90 Tage (der Standard), 1 Jahr oder nie. Ein abgelaufenes Token funktioniert nicht mehr; erstellen Sie ein neues. |
| **Allowed networks** | Optional. Adressen oder CIDR-Bereiche, aus denen das Token genutzt werden darf (`192.168.1.0/24, 10.8.0.5`). Leer: überall. |

Das Token sieht aus wie `dmt_` gefolgt von 32 hexadezimalen Zeichen.
Es wird **einmal** direkt nach der Erstellung angezeigt; DumbMonit
behält nur einen Hash. Verloren? Widerrufen und ein neues erstellen.

Bevorzugen Sie ein Read-Token, sofern Sie den Assistant nicht
tatsächlich handeln lassen wollen.

Dasselbe Token öffnet auch die [REST-API](../reference/api.md#authentication)
für Skripte und Dashboards — `curl -H "Authorization: Bearer dmt_…"
https://monit.example.lan/api/targets` — wo `read` sieht, was ein
Viewer sieht, und `write` tut, was ein Administrator tut, Konten,
Tokens und Backups ausgenommen.

## 2. Das Snippet einfügen

Die Settings-Seite füllt diese mit der Adresse Ihres Servers und dem
neuen Token. Unten steht `https://monit.example.lan` für Ihren
Server.

### Claude Code

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

Fügen Sie `--scope user` hinzu, um es in jedem Projekt zu haben,
nicht nur im aktuellen.

### Claude Desktop

Claude Desktop startet lokale Server aus seiner Konfigurationsdatei;
[`mcp-remote`](https://www.npmjs.com/package/mcp-remote) überbrückt
zu DumbMonits HTTP-Endpunkt (benötigt
[Node.js](https://nodejs.org)). DumbMonit authentifiziert mit einem
Bearer-Header statt OAuth, fügen Sie es also über diese Datei hinzu,
statt über den Connectors-Bildschirm. Settings → Developer → **Edit
Config**, dann zu `claude_desktop_config.json` hinzufügen und Claude
Desktop neu starten:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote", "https://monit.example.lan/api/mcp",
        "--header", "Authorization:${DUMBMONIT_AUTH}"
      ],
      "env": { "DUMBMONIT_AUTH": "Bearer dmt_…" }
    }
  }
}
```

Der Header läuft über eine Umgebungsvariable, ohne Leerzeichen nach
dem Doppelpunkt, weil manche Plattformen Argumente an Leerzeichen
aufteilen. Für einen Server, der über einfaches HTTP in Ihrem
Netzwerk erreicht wird, fügen Sie `"--allow-http"` zu `args` hinzu —
die Settings-Seite tut das für Sie, wenn sie selbst über HTTP
bedient wird.

### VS Code

`.vscode/mcp.json` in einem Workspace (oder der Befehl **MCP: Add
Server** → HTTP):

```json
{
  "servers": {
    "dumbmonit": {
      "type": "http",
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

### Cursor und andere Clients

`.cursor/mcp.json`, und dieselbe Form für die meisten Clients, die
Streamable-HTTP-Server mit benutzerdefinierten Headern unterstützen:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

Ein Client, der nur lokale (stdio-)Server startet, kann die oben für
Claude Desktop gezeigte `mcp-remote`-Brücke nutzen.

### ChatGPT

Settings → Connectors → **Create** (Developer-Modus).
MCP-Server-URL: `https://monit.example.lan/api/mcp`,
Autorisierungs-Header: `Bearer dmt_…`.

ChatGPT verbindet sich von OpenAIs Servern aus, nicht von Ihrem
Browser: DumbMonit muss **aus dem Internet über HTTPS erreichbar**
sein. Siehe die [Sicherheitshinweise](#security-notes), bevor Sie
das tun.

### Jeder andere Client

Was ein Client wissen muss: der Endpunkt ist
`https://<server>/api/mcp`, der Transport ist Streamable HTTP, und
jeder Request trägt `Authorization: Bearer dmt_…`. Ein roher Aufruf,
um das Token und den Netzwerkpfad von einem Terminal zu prüfen:

```bash
curl -s https://monit.example.lan/api/mcp \
  -H 'Authorization: Bearer dmt_…' \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

## Was der Assistant aufrufen kann {#what-the-assistant-can-call}

Siebenundzwanzig Werkzeuge. Jedes hat einen Titel, eine Beschreibung,
ein Input-Schema, ein Output-Schema und Anmerkungen
(`readOnlyHint`, `destructiveHint`, `idempotentHint`), sodass ein
Client erkennen kann, welche nur nachsehen, und Sie fragen kann, bevor
er die anderen ausführt.

| Werkzeug | Scope | Tut |
| --- | --- | --- |
| `get_status` | read | Das Bulletin, die Antwort auf "ist alles in Ordnung?": Gerätezahlen, feuernde und sich aufbauende Alarme, ein Satz ("2 advisories, 1 unreachable."). |
| `list_devices` | read | Geräte und Services mit Zustand, übergeordnetem Gerät und Tags; filterbar nach Teilstring oder Zustand. |
| `get_device` | read | Ein Gerät: Konfiguration, Zustand, aktive Alarme, 24-Stunden-Zusammenfassung CPU/Speicher/vollste Platte (Verfügbarkeit und Latenz für einen Service-Check). |
| `list_alerts` | read | Aktive Alarme mit Schweregrad-Worten, seit wann, Wert, Unterdrückung, Bestätigung. |
| `alert_history` | read | Alarm-Übergänge, neueste zuerst (`since` oder `hours`, optionales Gerät, Limit). |
| `query_metrics` | read | Eine MetricsQL-Range-Abfrage, höchstens 60 Punkte pro Zeitreihe. |
| `list_silences` | read | Wartungsfenster und ob sie jetzt aktiv sind. |
| `list_rules` | read | Alarmregeln mit Art, Schweregrad, Schwellenwert, aktiviert. |
| `list_device_types` | read | Die Gerätearten, die dieser Server überwachen kann, mit den Optionen und Credentials, die jede erwartet — was `add_device` braucht. |
| `list_agents` | read | Maschinen mit laufendem Agent: OS, Agent-Version, Relay-Modus, Site, relayte Geräte, letzter Kontakt, Bindung. |
| `list_containers` | read | Die Docker-Container einer Maschine mit laufendem Agent: Zustand, Gesundheit, Neustarts, verfügbares Update, letzter Befehl. |
| `list_heartbeats` | read | Heartbeat-Monitore: erwartetes Intervall, letzter Aufruf, Urteil (waiting, on time, missed, reported down). Die geheime URL wird nie angezeigt. |
| `list_status_pages` | read | Statusseiten, veröffentlicht oder nicht, mit ihrem öffentlichen Pfad (`/s/<slug>`), den gezeigten Geräten und ihren offenen Vorfällen. |
| `list_channels` | read | Benachrichtigungskanäle: Art, aktiviert, letzter Fehler, letzte gesendete Nachricht. Nie ihre Secrets. |
| `list_packs` | read | Installierte Integrationspakete: Version, Geräteart, aktiviert, nutzende Geräte, Fehler. |
| `silence_device` | write | Ein einmaliges Wartungsfenster auf einem Gerät, jetzt beginnend (standardmäßig 1 Stunde, höchstens eine Woche). |
| `remove_silence` | write | Entfernt ein Wartungsfenster. |
| `schedule_maintenance` | write | Ein einmaliges Wartungsfenster zu einem gegebenen Datum, auf einem Gerät oder der gesamten Instanz, höchstens eine Woche. |
| `acknowledge_alert` | write | Bestätigt einen Alarm per Fingerprint für eine Anzahl Stunden (standardmäßig 4): Erinnerungen pausieren, die Behebung wird trotzdem gemeldet. `hours: 0` hebt es auf. |
| `probe_device` | write | Fragt ein Gerät sofort ab und meldet, was gemessen wurde. |
| `set_device_enabled` | write | Aktiviert oder deaktiviert die Überwachung eines Geräts. |
| `set_rule_enabled` | write | Aktiviert oder deaktiviert eine Alarmregel. |
| `add_device` | write | Fügt ein Gerät hinzu: Name, Art, Adresse, Intervall, übergeordnetes Gerät, Optionen, Credentials, Relay-Agent. |
| `discover_network` | write | Scannt ein Netzwerk (CIDR, höchstens 4096 Adressen) nach SNMP-Geräten und schlägt für jedes ein Profil vor. |
| `restart_container` | write | Bittet den Agent einer Maschine, einen ihrer Container neu zu starten. |
| `test_channel` | write | Sendet eine Testnachricht über einen Benachrichtigungskanal. |
| `post_incident` | write | Öffnet einen Vorfall auf einer Statusseite, oder postet ein Update zu einem bestehenden. |

Ein Read-Token, das ein Write-Werkzeug aufruft, erhält eine klare
Ablehnung als Werkzeugergebnis (`isError: true`); der Assistant
erklärt es, und nichts ändert sich. Dasselbe gilt für alles, was das
Werkzeug nicht kann — ein unbekanntes Gerät, ein mehrdeutiger Name,
ein Fenster länger als eine Woche: die Meldung sagt, was zu beheben
ist.

## Protokolldetails

`POST /api/mcp` implementiert MCP über den **Streamable-HTTP**-
Transport: JSON-RPC 2.0, ein Request pro `POST`, mit einfachem JSON
beantwortet (keine Server-Sent-Events). Es ist zustandslos: Keine
`Mcp-Session-Id` wird ausgestellt, eine von einem Client gesendete
wird ignoriert, und `GET` oder `DELETE` auf dem Endpunkt antworten mit
`405`. Nur die `tools`-Fähigkeit wird angeboten — keine Resources,
Prompts, Sampling oder OAuth.

Zwei Generationen des Protokolls werden auf demselben Endpunkt
bedient:

| Versionen | Wie ein Client es spricht |
| --- | --- |
| `2026-07-28` | Zustandslos. Jeder Request trägt `io.modelcontextprotocol/protocolVersion` und `io.modelcontextprotocol/clientCapabilities` in `params._meta`, und die Header `MCP-Protocol-Version`, `Mcp-Method` und, für `tools/call`, `Mcp-Name`, das mit dem Body übereinstimmen muss. Methoden: `server/discover`, `tools/list`, `tools/call`. Ergebnisse tragen `resultType: "complete"` und die Identität des Servers in `_meta` (`io.modelcontextprotocol/serverInfo`); `tools/list` trägt auch `ttlMs` und `cacheScope`. |
| `2025-11-25`, `2025-06-18`, `2025-03-26` | Der `initialize`-Handshake wählt die Version (eine unbekannte erhält die neueste dieser drei); `notifications/initialized` wird mit `202` bestätigt; dann `ping`, `tools/list`, `tools/call`. |

Fehler, neben dem `401`, `403` und `429` der [Token-Prüfungen](#security-notes),
die vor jedem JSON-RPC kommen:

| HTTP | JSON-RPC-Code | Wann |
| --- | --- | --- |
| `400` | `-32020` (Header-Diskrepanz) | Ein `2026-07-28`-Request ohne `MCP-Protocol-Version`, `Mcp-Method` oder `Mcp-Name`, oder mit Werten, die vom Body abweichen. |
| `400` | `-32022` (nicht unterstützte Protokollversion) | Eine Version, die dieser Server nicht spricht; `error.data` listet die `supported` und die `requested`. |
| `400` | `-32602` (ungültige Parameter) | Ein `2026-07-28`-Request ohne die erforderlichen `_meta`-Felder. |
| `404` | `-32601` (Methode nicht gefunden) | Ein `2026-07-28`-Request für eine von diesem Server nicht implementierte Methode. |
| `200` | `-32602` | Ein unbekanntes Werkzeug, oder `arguments`, das kein Objekt ist. |
| | `-32700`, `-32600` | Unlesbares JSON; kein JSON-RPC-2.0-Request (Batches werden nicht unterstützt). |
| `202` | — | Eine Notification: akzeptiert, kein Body. |

Ein Werkzeug, das läuft, aber fehlschlägt — fehlender Scope,
unbekanntes Gerät — ist kein Protokollfehler: Es antwortet normal mit
`isError: true` und einem Satz, den der Assistant weitergeben kann.

## Sicherheitshinweise {#security-notes}

- **Ein Token ist ein Passwort.** Wer es besitzt, liest alles, was
  die Oberfläche zeigt — und, mit einem Write-Token, ändert alles,
  was ein Administrator über die REST-API ändern kann (Geräte,
  Regeln, Kanäle), Konten, Tokens und Backups ausgenommen. Halten
  Sie es aus geteilten Chats, Screenshots und Repositories fern.
  Widerrufen Sie ein Token, bei dem Sie unsicher sind; erstellen Sie
  in einer Minute ein neues.
- **Bevorzugen Sie Read.** Die meisten Fragen ("ist alles in
  Ordnung?") brauchen keinen Write-Scope. Erstellen Sie ein
  Write-Token nur für einen Assistant, den Sie tatsächlich handeln
  lassen wollen, und benennen Sie es so, dass Sie es in der Liste
  erkennen. Die meisten Clients fragen, bevor sie ein nicht als
  nur-lesend markiertes Werkzeug ausführen; lassen Sie diese
  Bestätigung an.
- **Geben Sie ihm eine Lebensdauer und ein Netzwerk.** Ein Token, das
  in 90 Tagen abläuft und nur aus Ihrem LAN oder VPN funktioniert
  (`192.168.1.0/24`), ist für jeden, der es findet, wenig wert.
  Hinter einem Reverse Proxy erklären Sie den Proxy in
  `DUMBMONIT_TRUSTED_PROXIES`, sonst scheint jeder Aufruf vom Proxy
  zu kommen, und eine Netzwerkbeschränkung lehnt sie alle ab.
- **An seinen Ersteller gebunden.** Ein Token funktioniert nicht
  mehr, wenn das Konto, das es erstellt hat, deaktiviert wird,
  verliert seine Schreibkraft, wenn dieses Konto kein Admin mehr ist,
  und wird widerrufen, wenn dieses Konto gelöscht wird.
- **Nichts Geheimes kommt zurück.** Die Werkzeuge geben nie
  Gerätezugangsdaten zurück (nur ihre Art), Kanal-Secrets,
  Heartbeat-URLs oder Tokens. `add_device` nimmt zwar Zugangsdaten
  entgegen: Sie reisen durch die Konversation Ihres Assistants, und
  so durch dessen Anbieter. Für ein Gerät mit sensiblen Zugangsdaten
  bevorzugen Sie die Web-Oberfläche.
- **Browser und `Origin`.** Ein Request mit einem `Origin`-Header
  (ein browserbasierter Client) wird mit `403` abgelehnt, sofern
  diese Origin nicht die eigene des Servers ist oder von
  `DUMBMONIT_API_CORS_ORIGINS` erlaubt wird (standardmäßig jede
  Origin, da das Token ohnehin erforderlich ist). Listen Sie dort
  Ihre Origins, um es einzugrenzen; siehe
  [Cross-Origin-Requests](../reference/api.md#cross-origin-requests-cors).
- **Ratenlimit.** Jedes Token ist auf 120 Aufrufe pro Minute begrenzt,
  REST und MCP zusammen: Ein in einer Schleife festsitzender
  Assistant erhält `429` mit `Retry-After`, nicht die volle
  Aufmerksamkeit Ihres Servers.
- **HTTPS vor dem Internet.** ChatGPTs Connectors brauchen eine
  öffentliche URL. Stellen Sie DumbMonit hinter einen Reverse Proxy
  mit TLS (Caddy, Traefik, nginx…) und setzen Sie
  `DUMBMONIT_COOKIE_SECURE=1`. Exponieren Sie nie den einfachen
  HTTP-Port. Claude Desktop, Claude Code, VS Code und Cursor laufen
  auf Ihrer Maschine und können eine LAN-Adresse direkt erreichen,
  brauchen also überhaupt keine Exposition.
- **Beobachten Sie "last used".** Die Token-Liste zeigt, wann jedes
  Token zuletzt genutzt wurde und von welcher Adresse. Ein Token, das
  zu einer Zeit genutzt wurde, als Sie nicht mit Ihrem Assistant
  sprachen, oder von einer Adresse, die Sie nicht kennen, verdient
  einen Widerruf.
- **Logs.** Jeder Werkzeugaufruf wird auf `info`-Ebene mit dem
  Token-Namen und dem Werkzeug protokolliert, nie den Argumenten;
  jeder Schreibvorgang über die REST-API mit dem Token-Namen, der
  Methode, dem Pfad und dem Status, nie dem Body. Das Erstellen und
  Widerrufen von Tokens steht im Audit-Log (Settings → Security).

## Fehlerbehebung

| Symptom | Ursache und Behebung |
| --- | --- |
| `401` / "A valid API token is required." | Der Header fehlt oder ist falsch geschrieben (`Authorization: Bearer dmt_…`, ein Leerzeichen nach `Bearer`), oder das Token wurde widerrufen. Erstellen Sie ein neues. |
| `401` / "The API token "…" expired on …" | Erstellen Sie ein neues Token; wählen Sie eine längere Lebensdauer, falls sie zu kurz war. |
| `403` / "cannot be used from …" | Das Token ist auf andere Netzwerke beschränkt. Die Adresse in der Meldung ist die, die der Server sieht: Erklären Sie hinter einem Reverse Proxy den Proxy in `DUMBMONIT_TRUSTED_PROXIES`. |
| `403` bei jedem Aufruf von einem browserbasierten Client | Dessen `Origin` ist nicht erlaubt: fügen Sie sie zu `DUMBMONIT_API_CORS_ORIGINS` hinzu. |
| Ein Werkzeug antwortet "needs a token with the "write" scope" | Das Token ist nur-lesend, oder sein Ersteller ist kein Administrator mehr. Erstellen Sie ein Write-Token, wenn Sie den Assistant handeln lassen wollen. |
| `429` | 120 Aufrufe in einer Minute: Der Assistant loopt wahrscheinlich. Warten Sie auf `Retry-After`, stellen Sie dann eine engere Frage. |
| "Unsupported protocol version" | Der Client spricht eine Version, die DumbMonit nicht hat; aktualisieren Sie den Client. |
| Claude Desktop zeigt den Server als fehlgeschlagen | Node.js muss für `npx` installiert sein; über einfaches HTTP muss `--allow-http` in `args` stehen. |
| ChatGPT kann sich nicht verbinden | Es braucht eine öffentliche HTTPS-URL; eine LAN-Adresse oder einfaches HTTP funktioniert von OpenAIs Servern aus nicht. |
