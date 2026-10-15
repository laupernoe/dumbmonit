# Integrationspakete

Ein Integrationspaket (Pack) fügt DumbMonit einen Monitortyp hinzu, ohne
ein neues Release. Es ist eine einzige YAML-Datei, die sagt, wo
gelesen wird (eine HTTP-API oder eine Prometheus-`/metrics`-Seite auf
dem Monitor), was extrahiert wird, und welche Alarmregeln mitkommen.
Einmal installiert, zeigt sich die Art des Pakets in **Add monitor**
neben den eingebauten, und seine Monitore werden abgefragt, geplottet und
alarmiert wie jede andere.

Ein Pack ist deklarativ: kein Code, keine Skripte. DumbMonit führt
jeden Request selbst aus, unter Regeln, die das Pack nicht ändern
kann.

## Was ein Pack kann und nicht kann

| Ein Pack kann | Ein Pack kann nicht |
|---|---|
| `GET`- oder `POST`-Pfade auf der eigenen Adresse des Monitors aufrufen | Einen anderen Host aufrufen, einer Weiterleitung zu einem anderen Host folgen, oder eine absolute URL nutzen |
| JSON (JSONPath), Klartext (reguläre Ausdrücke) oder das Prometheus-Textformat lesen | Code, Shell-Befehle oder Templates jenseits von `{{option.x}}` und `{{credential.x}}` ausführen |
| Das Credential des Monitors in einem Header oder als Basic-/Bearer-Authentifizierung senden | Ein Credential in eine URL oder einen Request-Body legen |
| Im Monitordialog angezeigte Optionen deklarieren (ein Schaltkanal, ein Schwellenwert) | Metriken außerhalb des eigenen `dumbmonit_<pack>_`-Präfixes schreiben |
| Alarmregeln auf seinen eigenen Metriken mitbringen | Einen eingebauten Monitortyp oder eingebaute Regel ändern |
| SNMP-Profile mitbringen, im Format der eingebauten | Mehr als 4 MiB pro Antwort lesen, mehr als 16 Requests pro Abfrage senden, oder mehr als 5.000 Zeitreihen pro Abfrage erzeugen |

Loopback- und Link-Local-Adressen (`127.0.0.1`, `169.254.x.x`) werden
abgelehnt, sofern die Option **Allow loopback and link-local targets**
des Monitors nicht an ist, genau wie bei den HTTP-Checks. Private
LAN-Adressen sind immer erlaubt.

## Referenzpakete

Das Repository liefert drei Pakete unter
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs), jedes
mit seinen Fixtures und erwarteter Ausgabe:

| Pack | Liest | Regeln |
|---|---|---|
| `node-exporter` | eine kuratierte Teilmenge von Prometheus node_exporter (CPU, Speicher, Dateisysteme, Platten, Netzwerk) | Dateisystem fast voll, Speicher fast erschöpft |
| `shelly-plug` | Shelly-Gen2+-Steckdosen und -Relais, `/rpc/Switch.GetStatus` | Überhitzung |
| `speedtest-tracker` | das neueste Ergebnis der v1-API von Speedtest Tracker | Internet unter Erwartung, Speedtest fehlgeschlagen |

## Ein Pack installieren

In der Oberfläche, als Administrator: **Settings → Integration packs**.
Fügen Sie das YAML des Pakets ein oder laden Sie sein `pack.yaml` hoch,
dann **Install the pack**. Ein Pack, das die Prüfungen nicht besteht,
wird mit der Liste der zu behebenden Punkte abgelehnt; eines, das
bestanden hat, wird mit seinen Hinweisen, falls vorhanden, gespeichert.
Dieselbe Seite schaltet Pakete ein und aus und deinstalliert sie.
Installierte Arten erscheinen unter **Community packs**, wenn Sie ein
Monitor hinzufügen, markiert als *Pack*.

Über die HTTP-API brauchen Sie eine Administrator-Sitzung oder ein
API-Token mit dem Scope `write` (Settings → **API & assistants**).

```bash
# Install, or update to a newer version of the same pack
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @packs/shelly-plug/pack.yaml http://localhost:8080/api/packs

# List what is installed
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/api/packs
```

Die Antwort sagt, was passiert ist (`created`, `updated` oder
`unchanged`), die vom Pack erzeugten Metriken und wie viele Alarmregeln
hinzugefügt wurden. Ein Pack, das die Validierung nicht besteht, wird
mit der vollständigen Liste der Probleme abgelehnt, eines pro Zeile.

| Aufruf | Tut |
|---|---|
| `GET /api/packs` | installierte Pakete: Version, Art, Metriken, Regeln, Monitore, die es nutzen |
| `GET /api/packs/{id}` | ein Pack |
| `POST /api/packs` | installieren oder aktualisieren; Body: das YAML (`application/yaml`), oder `{"yaml": "…"}` in JSON |
| `PUT /api/packs/{id}/disable` | die Monitore des Pakets nicht mehr abfragen, alles andere behalten |
| `PUT /api/packs/{id}/enable` | sie wieder abfragen |
| `DELETE /api/packs/{id}` | deinstallieren, mit seinen Regeln; abgelehnt (`409`), solange Monitore es noch nutzen |

Mit einem Session-Cookie statt einem Token fügen Sie jedem `POST`,
`PUT` und `DELETE` `X-Requested-With: DumbMonit` hinzu, wie für den
Rest der [API](../reference/api.md).

Die Installation wirkt sofort: Die neue Art ist im Monitordialog ohne
Neustart verfügbar. Die einzige Ausnahme ist ein Pack, das
SNMP-Profile mitbringt; die Antwort sagt dann
`"restart_required": true`, weil der SNMP-Katalog beim Start gelesen
wird.

### Updates und Ihre Änderungen

Das Aktualisieren eines Pakets (dieselbe `id`, neuer Inhalt) ersetzt
seine Extraktion und seine Beschreibung. Seine Alarmregeln werden nur
jemals *hinzugefügt*: eine von Ihnen geänderte, deaktivierte oder
gelöschte Regel wird von einem Update nie überschrieben.

Das Deaktivieren eines Pakets zieht seinen Monitortyp zurück und behält
alles andere: Monitore, Historie, Regeln. Seine Monitore werden nicht
mehr abgefragt (jedes zeigt einen Konfigurationsfehler) und, wie jeder
Monitor, der aufhört zu melden, lösen sie nach drei Minuten *Monitor
unreachable* aus: Pausieren Sie sie auch, wenn Sie das Pack für eine
Weile deaktivieren.

### Backups

Installierte Pakete reisen in [Backups](../install/backup.md) mit
ihrem YAML, und werden vor den sie nutzenden Monitoren wiederhergestellt.

## Ein Pack vor der Installation prüfen

Das Server-Binary validiert und testet Pakete offline:

```bash
# Schema and consistency checks, without a server or a database
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# Replay fixtures/ through the real extraction and compare with expected.prom
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` schreibt `expected.prom` aus den Fixtures neu
(das Verzeichnis lesend-schreibend einhängen und mit
`--user "$(id -u)"` ausführen, damit die Datei Ihnen gehört).

Weiter: [Ihr erstes Pack schreiben](first-pack.md) in etwa zehn
Minuten, oder die [Format-Referenz](format.md) lesen.
