# DumbMonit-Benachrichtigungen {#dumbmonit-notifications}

Diese Anleitung erklärt, wie Sie DumbMonit-Alarme über den Dienst Ihrer Wahl erhalten.
Programmierkenntnisse sind nicht nötig: Jeder Abschnitt sagt, wo Sie die zu kopierende Adresse
oder das Token finden und was Sie in das Formular eintragen.

---

## Inhalt {#contents}

- [Wie es funktioniert](#how-it-works)
- [Einen Kanal erstellen und testen](#creating-and-testing-a-channel)
- [Einstellungen oder Geheimnisse: der Unterschied](#settings-or-secrets-the-difference)
- [Selbstsignierte Zertifikate](#self-signed-certificates)
- [Der benutzerdefinierte Kanal](#the-custom-channel)
  - [Verfügbare Variablen](#available-variables)
  - [Beispiel 1 — eine interne JSON-API](#example-1--an-internal-json-api)
  - [Beispiel 2 — ein Dienst, der ein Formular erwartet](#example-2--a-service-that-expects-a-form)
  - [Beispiel 3 — eine Klartext-SMS über einen Anbieter](#example-3--a-plain-text-sms-through-a-carrier)
  - [Beispiel 4 — eine GET-Anfrage ohne Body](#example-4--a-get-request-without-a-body)
  - [Häufige Fehler in Vorlagen](#common-template-mistakes)
- [Unterstützte Kanäle](#supported-channels)
- [Fehlerbehebung](#troubleshooting)

---

## Wie es funktioniert {#how-it-works}

Ein **Kanal** ist ein Ziel: ein Discord-Kanal, eine Telefonnummer, ein Postfach. Sie erstellen
beliebig viele, und Ihre Alarmregeln wählen, welche benachrichtigt werden.

Drei Prinzipien gelten für jeden Kanal:

1. **Alarme werden gruppiert.** Eine Nachricht pro Gerät und Zyklus, auch wenn fünf Platten
   gleichzeitig volllaufen. Ohne das würden Sie die Benachrichtigungen binnen einer Woche
   stummschalten.
2. **Ein defekter Kanal blockiert nie einen anderen.** Der Fehler wird in der Spalte „Letzter
   Fehler“ des Kanals festgehalten, und die Zustellung an die anderen läuft weiter.
3. **Ein Dienst, der nicht antwortet, wird nach 15 Sekunden aufgegeben.** Eine
   Benachrichtigung, die eine Minute zu spät kommt, nützt nichts mehr, und die
   Überwachungsschleife darf nicht hinter einem nicht erreichbaren Server warten.

---

## Einen Kanal erstellen und testen {#creating-and-testing-a-channel}

1. Öffnen Sie **Einstellungen → Benachrichtigungen & Berichte → Kanal hinzufügen** (`/settings/notifications`).
2. Geben Sie ihm einen aussagekräftigen Namen („Discord — Familie“, „SMS — Bereitschaft“).
   Diesen Namen wählen Sie später in Ihren Alarmregeln.
3. Wählen Sie den Kanaltyp und füllen Sie dann die unten beschriebenen Felder aus.
4. Speichern Sie, dann klicken Sie auf **Test senden**.

Die Testnachricht nimmt **exakt denselben Weg** wie ein echter Alarm: dieselbe Anfrage, dasselbe
Token, dieselbe Formatierung. Kommt sie an, funktioniert der Kanal. Scheitert sie, nennt die
Fehlermeldung die zu korrigierende Einstellung.

> **Ein Token wird nie wieder angezeigt.** Nach dem Speichern ist es in der Datenbank
> verschlüsselt, und die Oberfläche sagt nur „konfiguriert“. Um einen Kanal zu bearbeiten, ohne
> sein Token neu einzugeben, lassen Sie das Feld einfach leer.

---

## Einstellungen oder Geheimnisse: der Unterschied {#settings-or-secrets-the-difference}

Jeder Kanal hat zwei Gruppen von Feldern.

| | **Einstellungen** | **Geheimnisse** |
|---|---|---|
| Inhalt | Serveradresse, Kanalname, Topic | Tokens, Passwörter, Webhook-URLs |
| Speicherung | Klartext | verschlüsselt |
| Von der Oberfläche zurückgelesen | ja | nie |

Die Oberfläche **weigert sich**, ein sensibles Feld unter den Einstellungen zu speichern, und
sagt es Ihnen. Das ist keine Formalität: Einstellungen werden von der API unverändert
zurückgegeben, ein dort gespeichertes Token würde also bei jedem Öffnen der Seite
veröffentlicht.

Die Tabellen im Abschnitt [Unterstützte Kanäle](#supported-channels) sagen für jeden Kanal, was
wohin gehört.

---

## Selbstsignierte Zertifikate {#self-signed-certificates}

Standardmäßig lehnt DumbMonit jeden Server ab, dessen Zertifikat es nicht prüfen kann. Für einen
selbst gehosteten Dienst, der mit einem selbstsignierten Zertifikat oder einem einer privaten
Zertifizierungsstelle ausgeliefert wird, haben die Kanäle für Matrix, Mattermost, Rocket.Chat,
Zulip, ntfy, Gotify, Bark, Apprise, Home Assistant, Signal und den benutzerdefinierten Webhook
die Einstellung **Nicht prüfbares Zertifikat akzeptieren** (`insecure_tls`, ein Boolean,
standardmäßig aus).

Das Einschalten hält die Verbindung verschlüsselt, prüft aber die Identität des Servers nicht
mehr; verwenden Sie es also nur für einen Server, den Sie kontrollieren, in einem Netzwerk, dem
Sie vertrauen. Ist ein Zertifikat der Grund für einen Fehler, nennt es die für den Kanal
angezeigte Fehlermeldung (zum Beispiel „invalid peer certificate: UnknownIssuer“).

---

## Der benutzerdefinierte Kanal {#the-custom-channel}

Der Kanal **`webhook`** ist der Notausgang von DumbMonit. Sie beschreiben die gesamte
HTTP-Anfrage — Methode, Adresse, Header, Inhaltstyp und Body —, womit sich fast jeder Dienst
anbinden lässt, auch eine interne API, die DumbMonit nie kennen wird.

**Ohne besondere Konfiguration** sendet er bereits eine strukturierte JSON-Payload, die ein
Skript verwenden kann:

```json
{
  "source": "dumbmonit",
  "target": "nas",
  "rule": "Disk full",
  "severity": "warning",
  "status": "firing",
  "title": "nas — Disk full",
  "text": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "value": 95.0,
  "threshold": 90.0,
  "unit": "%",
  "fingerprint": "nas/disk-full",
  "count": 1,
  "at": "2026-09-01T14:12:05+00:00",
  "link": ""
}
```

### Einstellungen des benutzerdefinierten Kanals {#custom-channel-settings}

| Einstellung | Erforderlich | Standard | Rolle |
|---|---|---|---|
| `url` | ja (hier oder als Geheimnis) | — | Aufzurufende Adresse. Darf Variablen enthalten. |
| `method` | nein | `POST` | `POST`, `PUT` oder `GET`. |
| `content_type` | nein | `json` | `json`, `form` oder `text`. Bestimmt auch, wie Variablen maskiert werden. |
| `body_template` | nein | — | Body-Vorlage. Fehlt sie, wird die obige Payload gesendet. |
| `base_url` | nein | — | Öffentliche Adresse Ihres DumbMonit, damit `{{link}}` einen anklickbaren Link liefert. |
| `username` | nein | — | Benutzername für die HTTP-„Basic“-Authentifizierung. Erfordert das Geheimnis `password`. |

| Geheimnis | Rolle |
|---|---|
| `url` | Verwenden Sie es statt der Einstellung `url`, wenn die Adresse bereits ein Token enthält. |
| `token` | Wird als Header `Authorization: Bearer …` gesendet, sofern Ihre Vorlage nicht `{{token}}` verwendet. |
| `password` | Passwort für die „Basic“-Authentifizierung. |
| `headers` | Zusätzliche Header als JSON-Objekt `{"X-Api-Key": "value"}`. Wie das Token verschlüsselt gespeichert und nie wieder angezeigt, daher gehören API-Schlüssel und `Authorization`-Header hierher. |

`headers` ist ein Geheimnis. Bei Kanälen, die mit 0.1.0-alpha.6 oder früher gespeichert wurden
und sie in den Einstellungen hielten, werden sie beim ersten Start in die verschlüsselten
Geheimnisse verschoben, und die API gibt sie nicht mehr zurück. Wie jedes Geheimnis bleiben sie
erhalten, wenn Sie den Kanal bearbeiten, ohne sie erneut einzutippen; das Senden von `secrets`
ersetzt den gesamten Satz.

DumbMonit folgt einer Weiterleitung nur zum selben Host (oder von `http` zu `https` auf diesem
Host): Header und Token eines Kanals gehen nie anderswohin als an die von Ihnen eingegebene
Adresse.

### Verfügbare Variablen {#available-variables}

Eine Variable wird zwischen **doppelten geschweiften Klammern** geschrieben: `{{title}}`.
Leerzeichen um den Namen werden toleriert (`{{ title }}`). Einfache geschweifte Klammern in einem
JSON-Body bleiben wörtlich, Sie können Ihr JSON also normal schreiben.

| Variable | Was sie enthält | Beispiel |
|---|---|---|
| `{{title}}` | Vollständiger Nachrichtentitel | `nas — Disk full` |
| `{{message}}` | Klartext-Body, eine Zeile pro Alarm | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | Derselbe Body in Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Name der Alarmregel | `Disk full` |
| `{{target}}` | Name des überwachten Geräts | `nas` |
| `{{target_id}}` | Numerische Kennung des Geräts | `42` |
| `{{severity}}` | Technischer Schweregrad | `info`, `warning` oder `critical` |
| `{{severity_fr}}` | Schweregrad als lesbares Wort | `information`, `avertissement`, `critique` |
| `{{status}}` | Technischer Status | `firing` oder `resolved` |
| `{{status_fr}}` | Status als lesbares Wort | `en cours` oder `résolu` |
| `{{value}}` | Gemessener Wert, Einheit eingeschlossen | `95 %` |
| `{{value_raw}}` | Nackter Wert, als JSON-**Zahl** verwendbar | `95` |
| `{{threshold}}` | Überschrittener Schwellenwert, Einheit eingeschlossen | `90 %` |
| `{{threshold_raw}}` | Nackter Schwellenwert, als JSON-**Zahl** verwendbar | `90` |
| `{{unit}}` | Einheit des Werts | `%` |
| `{{operator}}` | Schwellenwertvergleich | `>` |
| `{{count}}` | Anzahl der in dieser Nachricht gruppierten Alarme | `3` |
| `{{fingerprint}}` | Stabile Kennung des Alarms | `nas/disk-full` |
| `{{timestamp}}` | ISO-8601-Zeitstempel | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Zeitstempel in Sekunden seit 1970 | `1788358325` |
| `{{date}}` | Lesbarer Zeitstempel, in UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Priorität von 1 (niedrig) bis 5 (hoch) | `4` |
| `{{color}}` | Akzentfarbe, ohne Raute | `D98A00` |
| `{{color_hex}}` | Akzentfarbe, mit Raute | `#D98A00` |
| `{{emoji}}` | Statuspiktogramm | `🔴`, `⚠️`, `ℹ️` oder `✅` |
| `{{link}}` | Link zum Gerät in DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Immer `dumbmonit` | `dumbmonit` |
| `{{token}}` | Das Geheimnis `token` des Kanals | *(Ihr Token)* |

Zwei nützliche Hinweise:

- **Manche Variablen können leer sein.** Ein Verfügbarkeitsalarm („Host nicht erreichbar“) hat
  weder Wert noch Schwellenwert: `{{value}}` und `{{value_raw}}` ergeben dann eine leere
  Zeichenkette, nie `null` oder `NaN`. Setzen Sie sie als JSON-Zahl ein, behandeln Sie diesen
  Fall — oder verwenden Sie `"{{value}}"` in Anführungszeichen.
- **`{{link}}` bleibt leer**, bis Sie `base_url` ausfüllen. Ein falscher Link würde mehr Zeit
  kosten als ein Alarm ohne einen.

**Sie müssen nie selbst etwas maskieren.** Je nach gewähltem `content_type` schützt DumbMonit
eingesetzte Werte automatisch: Anführungszeichen und Zeilenumbrüche werden in JSON maskiert,
in einem Formular prozentkodiert, in Klartext nicht umgewandelt. Eine Regel namens
`"System" disk full` bricht Ihre JSON-Vorlage nicht.

### Beispiel 1 — eine interne JSON-API {#example-1--an-internal-json-api}

Eine selbst gebaute Anwendung stellt `POST /incidents` bereit und erwartet ein Token in einem
benutzerdefinierten Header.

**Einstellungen**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Geheimnisse**

```json
{
  "token": "your-application-token",
  "headers": "{\"X-Application\": \"dumbmonit\"}"
}
```

Das Token wird automatisch als Header `Authorization: Bearer …` gesendet. Das erhält die
Anwendung:

```json
{
  "title": "nas — Disk full",
  "device": "nas",
  "severity": "warning",
  "state": "firing",
  "value": 95,
  "threshold": 90,
  "detail": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "link": "https://dumbmonit.home/targets/42",
  "seen_at": "2026-09-01T14:12:05+00:00"
}
```

### Beispiel 2 — ein Dienst, der ein Formular erwartet {#example-2--a-service-that-expects-a-form}

Viele SMS-Gateways und ältere APIs erwarten `application/x-www-form-urlencoded` statt JSON, mit
dem Token im Body.

**Einstellungen**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Geheimnisse**

```json
{ "token": "gateway-key" }
```

Da die Vorlage bereits `{{token}}` verwendet, **fügt DumbMonit keinen** `Authorization`-Header
hinzu: Manche Dienste lehnen eine Anfrage mit zwei Authentifizierungen ab. Werte werden
automatisch kodiert, Leerzeichen werden zu `+` und Umlaute zu ihrem prozentkodierten Äquivalent.

### Beispiel 3 — eine Klartext-SMS über einen Anbieter {#example-3--a-plain-text-sms-through-a-carrier}

Manche Dienste, etwa die SMS-API eines Anbieters oder eine selbst gebaute Automatisierung,
wollen eine einzelne Textzeile.

**Einstellungen**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

Die gesendete Nachricht, ohne jegliche Maskierung:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Beispiel 4 — eine GET-Anfrage ohne Body {#example-4--a-get-request-without-a-body}

Akzeptiert der Dienst nur URL-Parameter, setzen Sie die Variablen in `url` und wählen die
Methode `GET`. Eine `GET`-Anfrage hat keinen Body: `body_template` wird dann abgelehnt, damit
niemand glaubt, es bewirke etwas.

**Einstellungen**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Geheimnisse**

```json
{ "token": "api-key" }
```

Nur die **eingesetzten Werte** werden kodiert; der Rest der Adresse wird unverändert kopiert.
Die tatsächlich gesendete Anfrage sieht so aus:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Häufige Fehler in Vorlagen {#common-template-mistakes}

Eine fehlerhafte Vorlage wird **beim Speichern abgelehnt**, solange Sie auf das Formular
schauen, nicht in der Nacht, in der der Vorfall eintritt.

| Was Sie schreiben | Was passiert |
|---|---|
| `{{severite}}` | Abgelehnt: unbekannte Variable „severite“, gefolgt von der Liste gültiger Namen. Der richtige Name ist `severity`. |
| `{"t": "{{title"}` | Abgelehnt: „unclosed braces“. Das `}}` fehlt. |
| `{{}}` oder `{{  }}` | Abgelehnt: Eine Variable ohne Namen bedeutet nichts. |
| `{"a": {"b": 1}}` | **Akzeptiert.** Einfache geschweifte Klammern sind wörtlich: Schreiben Sie Ihr JSON normal. |
| `{{token}}` ohne Geheimnis `token` | Abgelehnt: Die Vorlage verlangt ein Token, das der Kanal nicht hat. |
| `method: GET` mit `body_template` | Abgelehnt, mit der Alternative: Setzen Sie die Variablen in `url`. |
| `method: DELETE` | Abgelehnt: Nur `POST`, `PUT` und `GET` werden akzeptiert. |

---

## Unterstützte Kanäle {#supported-channels}

### Discord {#discord}

In Discord: **Kanaleinstellungen → Integrationen → Webhooks → Neuer Webhook**, dann
**Webhook-URL kopieren**. Diese URL enthält das Token: Sie gehört in die Geheimnisse.

- **Geheimnisse** — `webhook_url`

### Slack {#slack}

Erstellen Sie auf <https://api.slack.com/apps> eine App, aktivieren Sie **Incoming Webhooks**,
dann **Add New Webhook to Workspace** und wählen Sie den Kanal. Kopieren Sie die URL
`https://hooks.slack.com/services/…`.

- **Geheimnisse** — `webhook_url`

### Telegram {#telegram}

Sie brauchen zwei Dinge: das **Bot-Token** (Geheimnis) und die **Chat-ID** (`chat_id`), in die er
schreiben soll. Das erste ist in einer Minute besorgt; das zweite hängt davon ab, wo Sie die
Alarme empfangen möchten.

#### 1. Den Bot erstellen und sein Token holen {#1-create-the-bot-and-get-its-token}

1. Suchen Sie in Telegram nach **@BotFather** und klicken Sie auf *Start*.
2. Senden Sie `/newbot` und beantworten Sie die Fragen (ein Name, dann ein Benutzername, der auf
   `bot` enden muss).
3. BotFather schließt mit einer Nachricht wie dieser:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   Die Zeile `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` ist das **Bot-Token**. Geben Sie es nicht
   weiter: Es gibt die volle Kontrolle über den Bot.

Ersetzen Sie in den folgenden Beispielen `<TOKEN>` durch dieses Token. Beachten Sie das in URLs
direkt davor geklebte Präfix `bot`: `https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Die `chat_id` finden {#2-find-the-chat_id}

**Privater Chat** (der Bot schreibt direkt an Sie)

1. Öffnen Sie Ihren Bot (`t.me/my_homelab_bot`) und klicken Sie auf *Start*, oder senden Sie ihm
   eine beliebige Nachricht. Ohne das kann er Ihnen nie schreiben.
2. Öffnen Sie in einem Browser: `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Sie erhalten ein JSON-Dokument; suchen Sie `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   Die `chat_id` ist der Wert von `chat.id`: hier `21xxxxx38` (eine positive Zahl).

   Ist `result` leer (`"result":[]`), senden Sie dem Bot noch eine Nachricht und laden Sie die
   Seite neu.

**Kanal**

1. Fügen Sie den Bot als Administrator des Kanals hinzu (Kanal verwalten → Administratoren →
   Hinzufügen).
2. Posten Sie eine Nachricht im Kanal.
3. Öffnen Sie `https://api.telegram.org/bot<TOKEN>/getUpdates`: Die Antwort enthält einen Block
   `channel_post` mit `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   Die `chat_id` ist diese negative Zahl, `-100` eingeschlossen: `-1001xxxxxx062`.

**Gruppe**

Am einfachsten geht es über die Desktop-App:

1. Fügen Sie den Bot zur Gruppe hinzu.
2. Senden Sie eine Nachricht in der Gruppe, Rechtsklick darauf → *Nachrichtenlink kopieren*. Sie
   erhalten `https://t.me/c/194xxxx987/13` (oder `https://t.me/c/194xxxx987/11/13`, wenn die
   Gruppe Themen hat).
3. Die Zahl direkt nach `/c/` ist die interne Kennung der Gruppe: `194xxxx987`. Die
   einzutragende `chat_id` ist diese Zahl, **mit dem Präfix `-100`**: `-100194xxxx987`.

Ohne die Desktop-App funktioniert auch die obige Methode `getUpdates`: `message.chat.id` ist
direkt die richtige Zahl (bereits negativ). Sieht der Bot die Nachrichten der Gruppe nicht,
bitten Sie BotFather um `/setprivacy` → *Disable*, entfernen Sie dann den Bot aus der Gruppe und
fügen Sie ihn erneut hinzu.

**Gruppenthema** (Gruppen im Modus „Themen“)

Die `chat_id` ist die der Gruppe. Füllen Sie zusätzlich **Gruppenthema** (`message_thread_id`)
mit der mittleren Zahl des kopierten Links: In `https://t.me/c/194xxxx987/11/13` ist das Thema
`11`. Ohne diese Einstellung landet die Nachricht im Thread *Allgemein*.

#### 3. Vor dem Speichern prüfen {#3-check-before-saving}

Öffnen Sie im Browser:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(ergänzen Sie `&message_thread_id=11` für ein Thema). Kommt `test123` an, stimmen Token und
`chat_id`: Tragen Sie sie in DumbMonit ein und klicken Sie auf *Test*.

- **Einstellungen** — `chat_id` (erforderlich), `message_thread_id` (Thema, optional),
  `api_base` (Standard `https://api.telegram.org`, ändern Sie es nur, um über ein Relay zu gehen)
- **Geheimnisse** — `bot_token`

> **„Telegram cannot find this chat“** — bei einem privaten Chat hat der Bot nie eine Nachricht
> von Ihnen erhalten: Senden Sie ihm `/start`. Bei einer Gruppe oder einem Kanal fehlt
> wahrscheinlich das Präfix `-100`, oder der Bot wurde nicht hinzugefügt.
>
> **„thread not found“** — die `message_thread_id` passt zu keinem Thema der Gruppe, oder die
> Gruppe ist nicht im Modus „Themen“.

### Microsoft Teams {#microsoft-teams}

**Achtung: Die Methode hat sich geändert.** Die alten „Office-365-Connectors“ wurden im Mai 2026
eingestellt; eine URL `outlook.office.com/webhook/…` funktioniert nicht mehr.

Das aktuelle Vorgehen:

1. Im Teams-Kanal **⋯ → Workflows**.
2. Wählen Sie die Vorlage **„Post to a channel when a webhook request is received“**.
3. Bestätigen Sie die Schritte, dann **kopieren Sie die HTTPS-POST-URL**, die am Ende angezeigt
   wird.

- **Geheimnisse** — `webhook_url`

DumbMonit sendet eine eingebettete *Adaptive Card*, das einzige Format, das diese Flows korrekt
darstellen. Erhalten Sie einen Fehler, der Power Automate erwähnt, stammt die URL wahrscheinlich
von einem alten Connector.

### Matrix {#matrix}

1. Erstellen Sie auf Ihrem Homeserver ein dediziertes Konto für DumbMonit und **laden Sie es in
   den Raum ein**. Ein Konto, das kein Mitglied ist, kann nicht schreiben.
2. Holen Sie sich sein Zugriffstoken. In Element: **Einstellungen → Hilfe & Info → Erweitert →
   Zugriffstoken**.
3. Holen Sie sich die **interne** Kennung des Raums — nicht seinen Anzeigenamen. In Element:
   **Raumeinstellungen → Erweitert → Interne Raum-ID**. Sie beginnt mit `!` (zum Beispiel
   `!aBcDeF:matrix.org`).

- **Einstellungen** — `server_url` (zum Beispiel `https://matrix.org`), `room_id`
- **Geheimnisse** — `token`

### Mattermost {#mattermost}

**Integrationen → Eingehende Webhooks → Hinzufügen**. Wählen Sie den Standardkanal, dann
kopieren Sie die URL.

- **Einstellungen** — `channel` (optional, um woanders als im Standardkanal zu schreiben),
  `username` (optional, Standard `DumbMonit`)
- **Geheimnisse** — `webhook_url`

### Rocket.Chat {#rocketchat}

**Administration → Integrationen → Neue Integration → Eingehender Webhook**. Wählen Sie den
Raum, aktivieren Sie die Integration, dann kopieren Sie die *Webhook-URL*.

- **Einstellungen** — `channel` (optional), `alias` (optional, Standard `DumbMonit`)
- **Geheimnisse** — `webhook_url`

### Google Chat {#google-chat}

Im Google-Chat-Space: **Name des Space → Apps & Integrationen → Webhooks → Webhook
hinzufügen**. Kopieren Sie die URL, die bereits Schlüssel und Token enthält.

- **Geheimnisse** — `webhook_url`

### Zulip {#zulip}

1. Erstellen Sie einen **Bot**: **Persönliche Einstellungen → Bots → Neuen Bot hinzufügen**, Typ
   „Generic“. Notieren Sie seine E-Mail-Adresse und seinen API-Schlüssel.
2. Abonnieren Sie den Bot auf den Ziel-Stream, sonst wird das Senden abgelehnt.

- **Einstellungen** — `server_url` (zum Beispiel `https://your-org.zulipchat.com`), `email` (die
  Adresse des Bots), `stream` (der Stream-Name), `topic` (optional, Standard `DumbMonit`)
- **Geheimnisse** — `api_key`

### ntfy {#ntfy}

Der einfachste Kanal von allen: Wählen Sie einen schwer zu erratenden Topic-Namen und abonnieren
Sie ihn dann in der ntfy-App.

- **Einstellungen** — `topic`, `server_url` (optional, Standard `https://ntfy.sh`)
- **Geheimnisse** — `token` (nur wenn Ihr Server Authentifizierung verlangt)

> Auf der öffentlichen Instanz kann **jeder, der den Namen eines Topics kennt, es lesen**. Wählen
> Sie einen langen, zufälligen Namen oder hosten Sie Ihren eigenen Server.

### Gotify {#gotify}

In der Gotify-Oberfläche: **Apps → Anwendung erstellen**. Kopieren Sie das angezeigte Token.

- **Einstellungen** — `server_url` (zum Beispiel `https://gotify.home`)
- **Geheimnisse** — `token`

### Pushover {#pushover}

1. Auf <https://pushover.net> wird Ihr **User Key** direkt auf der Startseite angezeigt.
2. Erstellen Sie dann eine Anwendung: **Create an Application/API Token**. Sie erhalten ein
   **Anwendungstoken**.

- **Einstellungen** — `priority` (optional, von -2 bis 2), `sound` (optional), `retry` und
  `expire` (nur für `priority: 2`)
- **Geheimnisse** — `token` (Anwendungstoken), `user_key` (User Key)

Ohne Einstellung `priority` wählt DumbMonit nach Schweregrad: `-1` für einen Hinweis oder eine
Auflösung, `0` für eine Warnung, `1` für einen kritischen Alarm. Ein Hinweis lässt das
Smartphone daher nicht klingeln.

> Priorität `2` („Notfall“) wiederholt die Benachrichtigung, bis sie bestätigt wird. DumbMonit
> ergänzt dann `retry` und `expire` automatisch, ohne die Pushover die Nachricht ablehnen würde.

### Pushbullet {#pushbullet}

Klicken Sie auf <https://www.pushbullet.com/#settings/account> auf **Create Access Token**.

- **Einstellungen** — `device_iden` (optional; ohne sie werden alle Ihre Geräte benachrichtigt)
- **Geheimnisse** — `token`

### Bark (iOS) {#bark-ios}

Öffnen Sie die Bark-App auf Ihrem iPhone: Sie zeigt eine URL mit Ihrem **Device Key**, der langen
Zeichenfolge nach dem Domainnamen.

- **Einstellungen** — `server_url` (optional, Standard `https://api.day.app`), `group`
  (optional), `level` (optional), `sound` (optional)
- **Geheimnisse** — `token` (der Device Key)

Ohne Einstellung `level` wählt DumbMonit: `passive` für einen Hinweis, `active` für eine
Warnung, `timeSensitive` für einen kritischen Alarm. Die Stufe `critical`, die „Nicht stören“
durchbricht, wird nie automatisch gewählt — setzen Sie sie ausdrücklich, wenn Sie sie wollen.

### Web Push {#web-push}

Native Benachrichtigungen auf Ihren Smartphones und Computern, direkt von Ihrem eigenen
DumbMonit, ohne App-Installation und ohne Drittanbieter-Konto: Der Browser (als App installiert
oder nicht) empfängt die Alarme auch bei geschlossenen Tabs.

1. Öffnen Sie auf jedem Smartphone oder Computer DumbMonit und gehen Sie zu **Einstellungen →
   Push-Benachrichtigungen → Auf diesem Gerät aktivieren**, dann **Test senden**. Auf iPhone und
   iPad fügen Sie DumbMonit zuerst zum Home-Bildschirm hinzu. Die Schritt-für-Schritt-Anleitung
   steht unter [Push-Benachrichtigungen (PWA)](using/push-notifications.md).
2. Erstellen Sie hier einen Kanal **Web Push**. Er hat kein Geheimnis: Seine Empfänger sind die
   in Schritt 1 abonnierten Geräte.

- **Einstellungen** — `users` (optional; ein Konto pro Zeile, leer: jedes abonnierte Gerät jedes
  Kontos), `urgency` (standardmäßig `high`, `normal` oder `low`), `ttl` (Sekunden, die der
  Push-Dienst die Nachricht für ein ausgeschaltetes Gerät aufbewahrt, Standard `86400`),
  `contact` (optionale Adresse `mailto:` oder `https:`, die an Push-Dienste gegeben wird; leer:
  `DUMBMONIT_PUBLIC_URL`, wenn sie HTTPS ist, sonst die Projektseite)
- **Geheimnisse** — keine. Das VAPID-Schlüsselpaar des Servers wird beim ersten Start erzeugt und
  verschlüsselt gespeichert; der private Schlüssel verlässt den Server nie.

Der Titel der Benachrichtigung ist der Schweregrad und das Gerät (`Kritisch · nas01`), der Body
eine kurze Zusammenfassung; ein Tipp öffnet die Seite **Alarme**. DumbMonit braucht ausgehendes
HTTPS zu den Push-Diensten (`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`); Nachrichten sind Ende-zu-Ende-
verschlüsselt, diese Dienste können sie nicht lesen. Ein Abonnement, das der Push-Dienst als
verschwunden meldet (HTTP 404 oder 410, zum Beispiel nach dem Entfernen der App), wird
automatisch gelöscht.

### Apprise {#apprise}

Apprise ist ein **Gateway**: Mit einer einzigen Konfiguration gibt es Zugriff auf Dutzende
Dienste, die DumbMonit nicht selbst implementiert. Stellen Sie den Container `caronc/apprise`
bereit und wählen Sie dann einen der beiden Modi.

**Empfohlener Modus — in Apprise gespeicherte Konfiguration.** Erstellen Sie in der
Apprise-Oberfläche eine Konfiguration, notieren Sie ihren Schlüssel und tragen Sie hier sonst
nichts ein.

- **Einstellungen** — `server_url`, `config_key`

**Direkter Modus — DumbMonit liefert die Ziele.** Apprise-URLs enthalten oft ein Passwort
(`mailto://user:password@…`): Sie gehören daher in die Geheimnisse.

- **Einstellungen** — `server_url`
- **Geheimnisse** — `urls` (eine oder mehrere Apprise-URLs, durch Kommas getrennt)

Optionale allgemeine Einstellungen: `tag` (um nur einen Teil der Ziele einer Konfiguration
anzusprechen) und `format` (standardmäßig `text`, oder `markdown`).

> Antwortet der Test „Apprise has no configuration under this config_key“, passt der Schlüssel zu
> nichts. DumbMonit meldet es, statt so zu tun, als wäre die Nachricht gesendet worden.

### Home Assistant {#home-assistant}

1. In Home Assistant: **Ihr Profil → Sicherheit → Langlebige Zugriffstoken → Token erstellen**.
   Kopieren Sie es, es wird nicht wieder angezeigt.
2. Wählen Sie den aufzurufenden Dienst. Standardmäßig erstellt DumbMonit eine dauerhafte
   Benachrichtigung auf dem Dashboard.

- **Einstellungen** — `server_url` (zum Beispiel `http://homeassistant.local:8123`), `service`
  (optional, Standard `persistent_notification.create`), `target` (optional), `data` (optional,
  in den Aufruf eingemischtes Objekt)
- **Geheimnisse** — `token`

Einige nützliche Dienste:

| `service` | Wirkung |
|---|---|
| `persistent_notification.create` | Banner in der Home-Assistant-Oberfläche |
| `notify.notify` | Alle konfigurierten Benachrichtigungsziele |
| `notify.mobile_app_<device>` | Die Mobile App eines bestimmten Geräts |
| `light.turn_on` | Schaltet ein Licht ein — verwenden Sie `data` für Entität und Farbe |

Die Einstellung `data` wird **zuletzt** eingemischt: Sie kann daher den Titel oder die Nachricht
überschreiben, die DumbMonit vorbereitet hat.

### E-Mail (SMTP) {#email-smtp}

Der einzige Kanal, der von keinem Drittanbieterdienst abhängt.

- **Einstellungen** — `host`, `from`, `to` (Liste von Adressen), `security` (standardmäßig
  `starttls`, oder `tls`, oder `none`), `port` (aus `security` abgeleitet: 587, 465 oder 25),
  `username` (optional)
- **Geheimnisse** — `password`

> Bei Gmail funktioniert das Kontopasswort nicht: Erstellen Sie in den Sicherheitseinstellungen
> Ihres Google-Kontos ein **App-Passwort**.

### Signal {#signal}

Läuft über eine Instanz von
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api), die Sie selbst hosten
und in der Sie zuerst Ihre Nummer registrieren.

- **Einstellungen** — `server_url` (zum Beispiel `http://signal.home:8080`), `number` (die
  registrierte Absendernummer), `recipients` (Liste von Nummern oder Gruppenkennungen),
  `username` (optional)
- **Geheimnisse** — `password` (optional, wenn Ihre Instanz geschützt ist)

Alle Nummern werden im **internationalen Format** geschrieben, `+33612345678`.

### SMS über Twilio {#sms-via-twilio}

In der Twilio-Konsole zeigt die Startseite Ihre **Account SID** und Ihr **Auth Token**. Kaufen
Sie dann eine Nummer, die SMS senden kann.

- **Einstellungen** — `account_sid`, `from` (Ihre Twilio-Nummer) **oder**
  `messaging_service_sid`, `to` (Liste der Empfänger)
- **Geheimnisse** — `token` (das *Auth Token*)

Der SMS-Body wird bei 300 Zeichen abgeschnitten: Darüber teilt Twilio ihn in getrennt
abgerechnete Segmente.

> Bei einem Testkonto sendet Twilio nur an Nummern, die Sie verifiziert haben.

### PagerDuty {#pagerduty}

In PagerDuty: **Services → Ihr Service → Integrations → Add integration**, und wählen Sie
**Events API v2**. Kopieren Sie den *Integration Key*.

- **Einstellungen** — `region` (`eu`, wenn Ihr Konto in Europa gehostet wird), `source`
  (optional, Standard `dumbmonit`)
- **Geheimnisse** — `token` (der *Integration Key*)

Ein Alarm öffnet einen Vorfall; seine Auflösung **schließt ihn automatisch**. Die Zuordnung
erfolgt über den Fingerprint des Alarms mit dem Präfix `dumbmonit-`, sodass eine Erinnerung nie
einen zweiten Vorfall öffnet.

> Ein Schlüssel eines anderen Typs („Events API v1“, „REST API“) erhält eine knappe Ablehnung.
> Prüfen Sie, ob die Integration tatsächlich vom Typ *Events API v2* ist.

### Opsgenie {#opsgenie}

**Teams → Ihr Team → Integrations → Add integration → API**. Kopieren Sie den API-Schlüssel.

- **Einstellungen** — `region` (`eu` für ein europäisches Konto), `responders` (Liste von
  Teamnamen, optional), `tags` (Liste, optional)
- **Geheimnisse** — `api_key`

Wie bei PagerDuty schließt eine Auflösung den Alarm, der über seinen Alias identifiziert wird.

> Atlassian hat das **Ende des Services für Opsgenie auf den 5. April 2027** festgelegt. Dieser
> Kanal bleibt für bestehende Installationen; für eine neue bevorzugen Sie PagerDuty oder einen
> benutzerdefinierten Kanal, der auf Jira Service Management zeigt.

### Benutzerdefinierter Webhook {#custom-webhook}

Siehe oben [Der benutzerdefinierte Kanal](#the-custom-channel).

---

## Fehlerbehebung {#troubleshooting}

**Der Test scheitert sofort mit „missing setting …“.**
Ein erforderliches Feld ist leer. Die Meldung nennt die genaue Einstellung.

**Der Test scheitert mit „no answer within 15 s“.**
Der Zielserver ist von DumbMonit aus nicht erreichbar. Handelt es sich um einen Dienst in Ihrem
lokalen Netzwerk, prüfen Sie, ob der DumbMonit-Container ihn erreichen kann — das ist häufig ein
Docker-Netzwerkproblem, kein Problem der Kanalkonfiguration.

**Der Test scheitert mit einem Zertifikatsfehler.**
Ihr Server verwendet ein selbstsigniertes Zertifikat oder eines einer privaten CA. Siehe
[Selbstsignierte Zertifikate](#self-signed-certificates).

**Der Test gelingt, aber ich erhalte nichts.**
Der Dienst hat die Nachricht angenommen und anderswohin gesendet: falscher Kanal, falsches
ntfy-Topic, falsches Pushbullet-Gerät. Prüfen Sie die Einstellung für das Ziel.

**Die Fehlermeldung enthält `***`.**
Das ist beabsichtigt: Ihr Token wurde aus der Nachricht entfernt, bevor sie aufgezeichnet wurde.
Kein Geheimnis erscheint je in den Logs, in der Spalte „Letzter Fehler“ oder in einer
API-Antwort.

**Wo sehe ich den letzten Fehler eines Kanals?**
In der Kanalliste: Jede Zeile zeigt das Datum der letzten erfolgreichen Zustellung und gegebenenfalls
den Fehler des letzten Fehlschlags.
