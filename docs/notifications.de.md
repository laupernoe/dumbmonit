# DumbMonit-Benachrichtigungen

Diese Anleitung erklärt, wie Sie DumbMonit-Alarme auf dem Dienst
Ihrer Wahl erhalten. Sie erfordert keine Programmierkenntnisse: Jeder
Abschnitt sagt, wo die zu kopierende Adresse oder das Token zu finden
ist, und was in das Formular einzutragen ist.

---

## Inhalt

- [Wie es funktioniert](#how-it-works)
- [Einen Kanal erstellen und testen](#creating-and-testing-a-channel)
- [Settings oder Secrets: der Unterschied](#settings-or-secrets-the-difference)
- [Der benutzerdefinierte Kanal](#the-custom-channel)
  - [Verfügbare Variablen](#available-variables)
  - [Beispiel 1 — eine interne JSON-API](#example-1--an-internal-json-api)
  - [Beispiel 2 — ein Dienst, der ein Formular erwartet](#example-2--a-service-that-expects-a-form)
  - [Beispiel 3 — eine einfache SMS über einen Carrier](#example-3--a-plain-text-sms-through-a-carrier)
  - [Beispiel 4 — ein GET-Request ohne Body](#example-4--a-get-request-without-a-body)
  - [Häufige Template-Fehler](#common-template-mistakes)
- [Unterstützte Kanäle](#supported-channels)
- [Fehlerbehebung](#troubleshooting)

---

## Wie es funktioniert {#how-it-works}

Ein **Kanal** ist ein Ziel: ein Discord-Kanal, eine Telefonnummer,
ein Postfach. Sie erstellen so viele wie Sie wollen, und Ihre
Alarmregeln wählen, welche benachrichtigt werden.

Drei Prinzipien gelten für jeden Kanal:

1. **Alarme werden gruppiert.** Eine Nachricht pro Gerät und Zyklus,
   selbst wenn fünf Platten gleichzeitig voll werden. Ohne das würden
   Sie Benachrichtigungen innerhalb einer Woche stummschalten.
2. **Ein defekter Kanal blockiert nie einen anderen.** Der Fehler
   wird in der Spalte "last error" des Kanals aufgezeichnet, und die
   Zustellung an die anderen läuft weiter.
3. **Ein nicht antwortender Dienst wird nach 15 Sekunden
   aufgegeben.** Eine eine Minute zu spät ankommende Benachrichtigung
   ist nicht mehr nützlich, und die Überwachungsschleife darf nicht
   hinter einem nicht erreichbaren Server warten.

---

## Einen Kanal erstellen und testen {#creating-and-testing-a-channel}

1. Öffnen Sie **Alerts → Notifications → Add channel**
   (`/alerts#notifications`).
2. Geben Sie ihm einen aussagekräftigen Namen ("Discord — family",
   "SMS — on-call"). Das ist der Name, den Sie später in Ihren
   Alarmregeln wählen.
3. Wählen Sie die Kanalart, füllen Sie dann die unten beschriebenen
   Felder aus.
4. Speichern, dann auf **Send test** klicken.

Die Testnachricht nimmt **genau denselben Weg** wie ein echter
Alarm: derselbe Request, dasselbe Token, dieselbe Formatierung.
Kommt sie an, funktioniert der Kanal. Schlägt sie fehl, benennt die
Fehlermeldung die zu behebende Einstellung.

> **Ein Token wird nie wieder angezeigt.** Einmal gespeichert, ist
> es in der Datenbank verschlüsselt, und die Oberfläche sagt nur
> "configured". Um einen Kanal zu bearbeiten, ohne sein Token erneut
> einzutippen, lassen Sie das Feld einfach leer.

---

## Settings oder Secrets: der Unterschied {#settings-or-secrets-the-difference}

Jeder Kanal hat zwei Gruppen von Feldern.

| | **Settings** | **Secrets** |
|---|---|---|
| Inhalt | Serveradresse, Kanalname, Thema | Tokens, Passwörter, Webhook-URLs |
| Speicherung | Klartext | verschlüsselt |
| Von der Oberfläche zurückgelesen | ja | nie |

Die Oberfläche **verweigert**, ein sensibles Feld unter den Settings
zu speichern, und sagt Ihnen das. Das ist keine Formalität: Settings
werden unverändert von der API zurückgegeben, daher würde ein dort
gespeichertes Token jedes Mal veröffentlicht, wenn die Seite geöffnet
wird.

Die Tabellen im Abschnitt [Unterstützte Kanäle](#supported-channels)
sagen, für jeden Kanal, was wohin gehört.

---

## Der benutzerdefinierte Kanal {#the-custom-channel}

Der Kanal **`webhook`** ist DumbMonits Notausgang. Sie beschreiben
den ganzen HTTP-Request — Methode, Adresse, Header, Content-Type
und Body —, was Sie fast jeden Dienst anbinden lässt, einschließlich
einer internen API, die DumbMonit nie kennen wird.

**Ohne besondere Konfiguration** sendet er schon eine strukturierte
JSON-Payload, die ein Skript nutzen kann:

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

### Einstellungen des benutzerdefinierten Kanals

| Einstellung | Erforderlich | Standard | Rolle |
|---|---|---|---|
| `url` | ja (hier oder als Secret) | — | Aufzurufende Adresse. Kann Variablen enthalten. |
| `method` | nein | `POST` | `POST`, `PUT` oder `GET`. |
| `content_type` | nein | `json` | `json`, `form` oder `text`. Bestimmt auch, wie Variablen escaped werden. |
| `body_template` | nein | — | Body-Template. Fehlt es, wird die obige Payload gesendet. |
| `headers` | nein | — | Zusätzliche Header, als Objekt `{"X-Key": "value"}`. |
| `base_url` | nein | — | Öffentliche Adresse Ihres DumbMonit, sodass `{{link}}` einen klickbaren Link gibt. |
| `username` | nein | — | Benutzername für HTTP-"Basic"-Authentifizierung. Braucht das Secret `password`. |

| Secret | Rolle |
|---|---|
| `url` | Nutzen Sie es statt der Einstellung `url`, wenn die Adresse schon ein Token enthält. |
| `token` | Als `Authorization: Bearer …`-Header gesendet, sofern Ihr Template nicht `{{token}}` nutzt. |
| `password` | Passwort für "Basic"-Authentifizierung. |

### Verfügbare Variablen {#available-variables}

Eine Variable wird zwischen **doppelten geschweiften Klammern**
geschrieben: `{{title}}`. Leerzeichen um den Namen werden toleriert
(`{{ title }}`). Einzelne geschweifte Klammern in einem JSON-Body
bleiben wörtlich, daher können Sie Ihr JSON normal schreiben.

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
| `{{value_raw}}` | Blanker Wert, nutzbar als JSON-**Zahl** | `95` |
| `{{threshold}}` | Überschrittener Schwellenwert, Einheit eingeschlossen | `90 %` |
| `{{threshold_raw}}` | Blanker Schwellenwert, nutzbar als JSON-**Zahl** | `90` |
| `{{unit}}` | Einheit des Werts | `%` |
| `{{operator}}` | Schwellenwert-Vergleich | `>` |
| `{{count}}` | Anzahl in dieser Nachricht gruppierter Alarme | `3` |
| `{{fingerprint}}` | Stabile Kennung des Alarms | `nas/disk-full` |
| `{{timestamp}}` | ISO-8601-Zeitstempel | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Zeitstempel in Sekunden seit 1970 | `1788358325` |
| `{{date}}` | Lesbarer Zeitstempel, in UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Priorität von 1 (niedrig) bis 5 (hoch) | `4` |
| `{{color}}` | Akzentfarbe, ohne Hash | `D98A00` |
| `{{color_hex}}` | Akzentfarbe, mit Hash | `#D98A00` |
| `{{emoji}}` | Status-Piktogramm | `🔴`, `⚠️`, `ℹ️` oder `✅` |
| `{{link}}` | Link zum Gerät in DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Immer `dumbmonit` | `dumbmonit` |
| `{{token}}` | Das `token`-Secret des Kanals | *(Ihr Token)* |

Zwei nützliche Bemerkungen:

- **Manche Variablen können leer sein.** Ein Verfügbarkeitsalarm
  ("host unreachable") hat weder einen Wert noch einen Schwellenwert:
  `{{value}}` und `{{value_raw}}` rendern dann einen leeren String,
  nie `null` oder `NaN`. Platzieren Sie sie als JSON-Zahl, behandeln
  Sie diesen Fall — oder nutzen Sie `"{{value}}"` zwischen
  Anführungszeichen.
- **`{{link}}` bleibt leer**, bis Sie `base_url` ausfüllen. Ein
  falscher Link würde mehr Zeit kosten als ein Alarm ohne einen.

**Sie müssen nie selbst etwas escapen.** Je nach gewähltem
`content_type` schützt DumbMonit ersetzte Werte automatisch:
Anführungszeichen und Zeilenumbrüche in JSON escaped,
Percent-Encoding in einem Formular, keine Transformation in
Klartext. Eine Regel namens `"System" disk full` wird Ihr
JSON-Template nicht kaputt machen.

### Beispiel 1 — eine interne JSON-API {#example-1--an-internal-json-api}

Eine selbst gebaute Anwendung exponiert `POST /incidents` und
erwartet ein Token in einem benutzerdefinierten Header.

**Settings**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "headers": {
    "X-Application": "dumbmonit"
  },
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Secrets**

```json
{ "token": "your-application-token" }
```

Das Token wird automatisch als `Authorization: Bearer …`-Header
gesendet. So sieht die Anwendung es:

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

Viele SMS-Gateways und ältere APIs erwarten
`application/x-www-form-urlencoded` statt JSON, mit dem Token im
Body.

**Settings**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Secrets**

```json
{ "token": "gateway-key" }
```

Da das Template bereits `{{token}}` nutzt, **fügt** DumbMonit
**keinen** `Authorization`-Header hinzu: manche Dienste weisen einen
Request mit zwei Authentifizierungen ab. Werte werden automatisch
codiert, Leerzeichen werden zu `+` und akzentuierte Zeichen zu ihrem
Percent-Encoding-Äquivalent.

### Beispiel 3 — eine einfache SMS über einen Carrier {#example-3--a-plain-text-sms-through-a-carrier}

Manche Dienste, wie die SMS-API eines Carriers oder eine selbst
gebaute Automatisierung, wollen eine einzige Textzeile.

**Settings**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

Die gesendete Nachricht, ohne jedes Escaping:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Beispiel 4 — ein GET-Request ohne Body {#example-4--a-get-request-without-a-body}

Akzeptiert der Dienst nur URL-Parameter, legen Sie die Variablen in
`url` und wählen Sie die Methode `GET`. Ein `GET`-Request hat keinen
Body: `body_template` wird dann abgelehnt, damit niemand glaubt, es
tue etwas.

**Settings**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Secrets**

```json
{ "token": "api-key" }
```

Nur die **ersetzten Werte** werden codiert; der Rest der Adresse
wird unverändert kopiert. Der tatsächlich gesendete Request sieht
so aus:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Häufige Template-Fehler {#common-template-mistakes}

Ein fehlerhaftes Template wird **beim Speichern abgelehnt**,
während Sie das Formular ansehen, nicht in der Nacht, in der der
Vorfall passiert.

| Was Sie schreiben | Was passiert |
|---|---|
| `{{severite}}` | Abgelehnt: unbekannte Variable "severite", gefolgt von der Liste gültiger Namen. Der korrekte Name ist `severity`. |
| `{"t": "{{title"}` | Abgelehnt: "unclosed braces". Das `}}` fehlt. |
| `{{}}` oder `{{  }}` | Abgelehnt: eine Variable ohne Namen bedeutet nichts. |
| `{"a": {"b": 1}}` | **Akzeptiert.** Einzelne geschweifte Klammern sind wörtlich: Schreiben Sie Ihr JSON normal. |
| `{{token}}` ohne ein `token`-Secret | Abgelehnt: das Template verlangt ein Token, das der Kanal nicht hat. |
| `method: GET` mit `body_template` | Abgelehnt, mit der Alternative: Legen Sie die Variablen in `url`. |
| `method: DELETE` | Abgelehnt: nur `POST`, `PUT` und `GET` werden akzeptiert. |

---

## Unterstützte Kanäle {#supported-channels}

### Discord

In Discord: **Channel settings → Integrations → Webhooks → New
webhook**, dann **Copy webhook URL**. Diese URL enthält das Token:
Sie gehört in die Secrets.

- **Secrets** — `webhook_url`

### Slack

Auf <https://api.slack.com/apps>, erstellen Sie eine App, aktivieren
Sie **Incoming Webhooks**, dann **Add New Webhook to Workspace** und
wählen Sie den Kanal. Kopieren Sie die
`https://hooks.slack.com/services/…`-URL.

- **Secrets** — `webhook_url`

### Telegram

Sie brauchen zwei Dinge: das **Bot-Token** (Secret) und die
**Chat-ID** (`chat_id`), an die es schreiben muss. Das erste dauert
eine Minute; das zweite hängt davon ab, wo Sie die Alarme empfangen
wollen.

#### 1. Den Bot erstellen und sein Token erhalten

1. Suchen Sie in Telegram **@BotFather** und klicken Sie *Start*.
2. Senden Sie `/newbot`, beantworten Sie dann die Fragen (ein Name,
   dann ein Benutzername, der mit `bot` enden muss).
3. BotFather endet mit einer Nachricht wie dieser:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   Die Zeile `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` ist das
   **Bot-Token**. Teilen Sie es nicht: Es gibt volle Kontrolle über
   den Bot.

In den Beispielen unten ersetzen Sie `<TOKEN>` durch dieses Token.
Beachten Sie das Präfix `bot`, das in URLs direkt davor klebt:
`https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Die `chat_id` finden

**Privater Chat** (der Bot schreibt direkt an Sie)

1. Öffnen Sie Ihren Bot (`t.me/my_homelab_bot`) und klicken Sie
   *Start*, oder senden Sie ihm eine beliebige Nachricht. Ohne das
   kann er Ihnen nie schreiben.
2. Öffnen Sie in einem Browser:
   `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Sie erhalten ein JSON-Dokument; suchen Sie nach `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   Die `chat_id` ist der Wert von `chat.id`: hier `21xxxxx38` (eine
   positive Zahl).

   Ist `result` leer (`"result":[]`), senden Sie dem Bot eine weitere
   Nachricht und laden Sie die Seite neu.

**Kanal**

1. Fügen Sie den Bot als Administrator des Kanals hinzu (Manage
   channel → Administrators → Add).
2. Posten Sie eine Nachricht im Kanal.
3. Öffnen Sie `https://api.telegram.org/bot<TOKEN>/getUpdates`: die
   Antwort enthält einen `channel_post`-Block mit
   `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   Die `chat_id` ist diese negative Zahl, `-100` eingeschlossen:
   `-1001xxxxxx062`.

**Gruppe**

Der einfachste Weg läuft über die Desktop-App:

1. Fügen Sie den Bot zur Gruppe hinzu.
2. Senden Sie eine Nachricht in der Gruppe, rechtsklicken Sie darauf
   → *Copy message link*. Sie erhalten `https://t.me/c/194xxxx987/13`
   (oder `https://t.me/c/194xxxx987/11/13`, wenn die Gruppe Topics
   hat).
3. Die Zahl direkt nach `/c/` ist die interne Kennung der Gruppe:
   `194xxxx987`. Die einzugebende `chat_id` ist diese Zahl
   **mit dem Präfix `-100`**: `-100194xxxx987`.

Ohne die Desktop-App funktioniert die obige Methode `getUpdates`
auch: `message.chat.id` ist direkt die richtige Zahl (schon negativ).
Sieht der Bot die Nachrichten der Gruppe nicht, bitten Sie BotFather
um `/setprivacy` → *Disable*, entfernen Sie den Bot dann aus der
Gruppe und fügen Sie ihn erneut hinzu.

**Gruppen-Topic** (Gruppen im "Topics"-Modus)

Die `chat_id` ist die der Gruppe. Füllen Sie zusätzlich **Group
topic** (`message_thread_id`) mit der mittleren Zahl des kopierten
Links aus: In `https://t.me/c/194xxxx987/11/13` ist das Topic `11`.
Ohne diese Einstellung landet die Nachricht im *General*-Thread.

#### 3. Vor dem Speichern prüfen

Öffnen Sie im Browser:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(fügen Sie `&message_thread_id=11` für ein Topic hinzu). Kommt
`test123` an, sind Token und `chat_id` richtig: Geben Sie sie in
DumbMonit ein und klicken Sie *Test*.

- **Settings** — `chat_id` (erforderlich), `message_thread_id`
  (Topic, optional), `api_base` (Standard
  `https://api.telegram.org`, nur ändern, um über ein Relay zu
  gehen)
- **Secrets** — `bot_token`

> **"Telegram cannot find this chat"** — bei einem privaten Chat hat
> der Bot nie eine Nachricht von Ihnen erhalten: senden Sie ihm
> `/start`. Bei einer Gruppe oder einem Kanal fehlt wahrscheinlich das
> Präfix `-100`, oder der Bot wurde nicht hinzugefügt.
>
> **"thread not found"** — die `message_thread_id` passt zu keinem
> Topic der Gruppe, oder die Gruppe ist nicht im "Topics"-Modus.

### Microsoft Teams

**Achtung: Die Methode hat sich geändert.** Die alten "Office
365-Konnektoren" wurden im Mai 2026 abgeschaltet; eine
`outlook.office.com/webhook/…`-URL funktioniert nicht mehr.

Das aktuelle Verfahren:

1. Im Teams-Kanal, **⋯ → Workflows**.
2. Wählen Sie das Template **"Post to a channel when a webhook
   request is received"**.
3. Bestätigen Sie die Schritte, **kopieren Sie dann die am Ende
   gezeigte HTTPS-POST-URL**.

- **Secrets** — `webhook_url`

DumbMonit sendet eine verpackte *Adaptive Card*, das einzige Format,
das diese Flows korrekt anzeigen. Erhalten Sie einen Fehler, der
Power Automate erwähnt, stammt die URL wahrscheinlich von einem
alten Konnektor.

### Matrix

1. Erstellen Sie ein für DumbMonit dediziertes Konto auf Ihrem
   Homeserver, und **laden Sie es in den Raum ein**. Ein Konto, das
   kein Mitglied ist, kann nicht schreiben.
2. Holen Sie sein Access-Token. In Element: **Settings → Help &
   About → Advanced → Access token**.
3. Holen Sie die **interne** Kennung des Raums — nicht seinen
   Anzeigenamen. In Element: **Room settings → Advanced → Internal
   room ID**. Sie beginnt mit `!` (zum Beispiel
   `!aBcDeF:matrix.org`).

- **Settings** — `server_url` (zum Beispiel `https://matrix.org`),
  `room_id`
- **Secrets** — `token`

### Mattermost

**Integrations → Incoming Webhooks → Add**. Wählen Sie den
Standardkanal, kopieren Sie dann die URL.

- **Settings** — `channel` (optional, um woanders als dem
  Standardkanal zu schreiben), `username` (optional, Standard
  `DumbMonit`)
- **Secrets** — `webhook_url`

### Rocket.Chat

**Administration → Integrations → New integration → Incoming
webhook**. Wählen Sie den Raum, aktivieren Sie die Integration,
kopieren Sie dann die *Webhook URL*.

- **Settings** — `channel` (optional), `alias` (optional, Standard
  `DumbMonit`)
- **Secrets** — `webhook_url`

### Google Chat

Im Google-Chat-Space: **Space-Name → Apps & integrations → Webhooks
→ Add webhook**. Kopieren Sie die URL, die schon den Schlüssel und
das Token enthält.

- **Secrets** — `webhook_url`

### Zulip

1. Erstellen Sie einen **Bot**: **Personal settings → Bots → Add a
   new bot**, Typ "Generic". Merken Sie sich seine E-Mail-Adresse
   und seinen API-Key.
2. Abonnieren Sie den Bot für den Ziel-Stream, sonst wird das Senden
   abgelehnt.

- **Settings** — `server_url` (zum Beispiel
  `https://your-org.zulipchat.com`), `email` (die Adresse des Bots),
  `stream` (der Stream-Name), `topic` (optional, Standard
  `DumbMonit`)
- **Secrets** — `api_key`

### ntfy

Der einfachste Kanal von allen: Wählen Sie einen schwer zu
erratenden Topic-Namen, abonnieren Sie ihn dann aus der ntfy-App.

- **Settings** — `topic`, `server_url` (optional, Standard
  `https://ntfy.sh`)
- **Secrets** — `token` (nur, wenn Ihr Server Authentifizierung
  verlangt)

> Auf der öffentlichen Instanz kann **jeder, der den Namen eines
> Topics kennt, es lesen**. Wählen Sie einen langen, zufälligen
> Namen, oder hosten Sie Ihren eigenen Server.

### Gotify

In der Gotify-Oberfläche: **Apps → Create Application**. Kopieren
Sie das gezeigte Token.

- **Settings** — `server_url` (zum Beispiel `https://gotify.home`)
- **Secrets** — `token`

### Pushover

1. Auf <https://pushover.net> wird Ihr **User Key** direkt auf der
   Startseite angezeigt.
2. Erstellen Sie dann eine Anwendung: **Create an
   Application/API Token**. Sie erhalten ein **Application-Token**.

- **Settings** — `priority` (optional, von -2 bis 2), `sound`
  (optional), `retry` und `expire` (nur für `priority: 2`)
- **Secrets** — `token` (Application-Token), `user_key` (User-Key)

Ohne eine `priority`-Einstellung wählt DumbMonit nach Schweregrad:
`-1` für ein Advisory oder eine Behebung, `0` für ein Warning, `1`
für einen kritischen Alarm. Ein Advisory lässt das Telefon daher
nicht klingeln.

> Priorität `2` ("emergency") wiederholt die Benachrichtigung, bis
> sie bestätigt wird. DumbMonit fügt dann automatisch `retry` und
> `expire` hinzu, ohne die Pushover die Nachricht abweisen würde.

### Pushbullet

Auf <https://www.pushbullet.com/#settings/account>, klicken Sie
**Create Access Token**.

- **Settings** — `device_iden` (optional; ohne es werden alle Ihre
  Geräte benachrichtigt)
- **Secrets** — `token`

### Bark (iOS)

Öffnen Sie die Bark-App auf Ihrem iPhone: Sie zeigt eine URL mit
Ihrem **Device Key**, dem langen Zeichenstring nach dem
Domainnamen.

- **Settings** — `server_url` (optional, Standard
  `https://api.day.app`), `group` (optional), `level` (optional),
  `sound` (optional)
- **Secrets** — `token` (der Device Key)

Ohne eine `level`-Einstellung wählt DumbMonit: `passive` für ein
Advisory, `active` für ein Warning, `timeSensitive` für einen
kritischen Alarm. Die Stufe `critical`, die "Do Not Disturb"
durchbricht, wird nie automatisch gewählt — setzen Sie sie explizit,
wenn Sie sie wollen.

### Web Push

Native Benachrichtigungen auf Ihren Telefonen und Computern, direkt
von Ihrem eigenen DumbMonit, ohne App zu installieren und ohne
Drittanbieterkonto: Der Browser (installiert als App oder nicht)
erhält die Alarme, auch mit jedem geschlossenen Tab.

1. Öffnen Sie auf jedem Telefon oder Computer DumbMonit und gehen
   Sie zu **Settings → Push notifications → Enable on this device**,
   dann **Send a test**. Auf iPhone und iPad fügen Sie DumbMonit
   zuerst dem Homescreen hinzu. Die Schritt-für-Schritt-Anleitung
   steht in
   [Push-Benachrichtigungen (PWA)](using/push-notifications.md).
2. Erstellen Sie hier einen **Web-Push**-Kanal. Er hat kein Secret:
   Seine Empfänger sind die in Schritt 1 abonnierten Geräte.

- **Settings** — `users` (optional; ein Konto pro Zeile, leer:
  jedes abonnierte Gerät jedes Kontos), `urgency` (standardmäßig
  `high`, `normal` oder `low`), `ttl` (Sekunden, die der Push-
  Dienst die Nachricht für ein ausgeschaltetes Gerät behält,
  Standard `86400`), `contact` (optionale `mailto:`- oder
  `https:`-Adresse, Push-Diensten gegeben; leer:
  `DUMBMONIT_PUBLIC_URL`, wenn es HTTPS ist, sonst die Projektseite)
- **Secrets** — keine. Das VAPID-Schlüsselpaar des Servers wird beim
  ersten Start generiert und verschlüsselt gespeichert; der private
  Schlüssel verlässt den Server nie.

Der Titel der Benachrichtigung ist der Schweregrad und das Gerät
(`Critical · nas01`), der Body eine kurze Zusammenfassung; ein Tap
öffnet die **Alerts**-Seite. DumbMonit braucht ausgehendes HTTPS zu
den Push-Diensten (`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`); Nachrichten
sind Ende-zu-Ende verschlüsselt, diese Dienste können sie nicht
lesen. Ein vom Push-Dienst als verschwunden gemeldetes Abonnement
(HTTP 404 oder 410, zum Beispiel nach Entfernung der App) wird
automatisch gelöscht.

### Apprise

Apprise ist ein **Gateway**: mit einer einzigen Konfiguration gibt
es Zugang zu Dutzenden Diensten, die DumbMonit nicht selbst
implementiert. Deployen Sie den Container `caronc/apprise`, wählen
Sie dann einen der zwei Modi.

**Empfohlener Modus — Konfiguration in Apprise gespeichert.**
Erstellen Sie eine Konfiguration in der Apprise-Oberfläche, merken
Sie sich ihren Key, und legen Sie hier nichts sonst ab.

- **Settings** — `server_url`, `config_key`

**Direkter Modus — DumbMonit liefert die Ziele.** Apprise-URLs
enthalten oft ein Passwort (`mailto://user:password@…`): Sie
gehören daher in die Secrets.

- **Settings** — `server_url`
- **Secrets** — `urls` (eine oder mehrere Apprise-URLs,
  kommagetrennt)

Optionale gemeinsame Einstellungen: `tag` (um nur einen Teil der
Ziele einer Konfiguration anzusprechen) und `format` (standardmäßig
`text`, oder `markdown`).

> Antwortet der Test mit "Apprise has no configuration under this
> config_key", passt der Key zu nichts. DumbMonit meldet das, statt
> vorzugeben, die Nachricht sei gesendet worden.

### Home Assistant

1. In Home Assistant: **Ihr Profil → Security → Long-lived access
   tokens → Create token**. Kopieren Sie es, es wird nicht wieder
   angezeigt.
2. Wählen Sie den aufzurufenden Dienst. Standardmäßig erstellt
   DumbMonit eine persistente Benachrichtigung auf dem Dashboard.

- **Settings** — `server_url` (zum Beispiel
  `http://homeassistant.local:8123`), `service` (optional, Standard
  `persistent_notification.create`), `target` (optional), `data`
  (optional, Objekt, in den Aufruf eingemischt)
- **Secrets** — `token`

Ein paar nützliche Dienste:

| `service` | Wirkung |
|---|---|
| `persistent_notification.create` | Banner in der Home-Assistant-Oberfläche |
| `notify.notify` | Alle konfigurierten Benachrichtigungsziele |
| `notify.mobile_app_<device>` | Die mobile App eines bestimmten Geräts |
| `light.turn_on` | Schaltet eine Lampe ein — nutzen Sie `data` für Entität und Farbe |

Die Einstellung `data` wird **zuletzt** eingemischt: Sie kann daher
den Titel oder die Nachricht überschreiben, die DumbMonit
vorbereitet hat.

### E-Mail (SMTP)

Der einzige Kanal, der von keinem Drittanbieterdienst abhängt.

- **Settings** — `host`, `from`, `to` (Liste von Adressen),
  `security` (standardmäßig `starttls`, oder `tls`, oder `none`),
  `port` (aus `security` abgeleitet: 587, 465 oder 25), `username`
  (optional)
- **Secrets** — `password`

> Mit Gmail funktioniert ein Kontopasswort nicht: Erstellen Sie ein
> **App-Passwort** in den Sicherheitseinstellungen Ihres
> Google-Kontos.

### Signal

Läuft über eine
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api)-
Instanz, die Sie hosten und wo Sie zuerst Ihre Nummer registrieren.

- **Settings** — `server_url` (zum Beispiel
  `http://signal.home:8080`), `number` (die registrierte
  Absendernummer), `recipients` (Liste von Nummern oder
  Gruppenkennungen), `username` (optional)
- **Secrets** — `password` (optional, falls Ihre Instanz geschützt
  ist)

Alle Nummern werden im **internationalen Format** geschrieben,
`+33612345678`.

### SMS über Twilio

In der Twilio-Konsole zeigt die Startseite Ihre **Account SID** und
Ihr **Auth Token**. Kaufen Sie dann eine SMS-fähige Nummer.

- **Settings** — `account_sid`, `from` (Ihre Twilio-Nummer) **oder**
  `messaging_service_sid`, `to` (Liste von Empfängern)
- **Secrets** — `token` (das *Auth Token*)

Der SMS-Body wird bei 300 Zeichen abgeschnitten: darüber hinaus
teilt Twilio ihn in separat abgerechnete Segmente.

> Auf einem Testkonto sendet Twilio nur an von Ihnen verifizierte
> Nummern.

### PagerDuty

In PagerDuty: **Services → Ihr Service → Integrations → Add
integration**, und wählen Sie **Events API v2**. Kopieren Sie den
*Integration Key*.

- **Settings** — `region` (`eu`, wenn Ihr Konto in Europa gehostet
  ist), `source` (optional, Standard `dumbmonit`)
- **Secrets** — `token` (der *Integration Key*)

Ein Alarm öffnet einen Vorfall; seine Behebung **schließt ihn
automatisch**. Das Matching erfolgt über den Fingerprint des
Alarms, mit `dumbmonit-` vorangestellt, daher öffnet eine Erinnerung
nie einen zweiten Vorfall.

> Ein Key eines anderen Typs ("Events API v1", "REST API") erhält
> eine knappe Ablehnung. Prüfen Sie, dass die Integration
> tatsächlich vom Typ *Events API v2* ist.

### Opsgenie

**Teams → Ihr Team → Integrations → Add integration → API**.
Kopieren Sie den API-Key.

- **Settings** — `region` (`eu` für ein europäisches Konto),
  `responders` (Liste von Team-Namen, optional), `tags` (Liste,
  optional)
- **Secrets** — `api_key`

Wie bei PagerDuty schließt eine Behebung den Alarm, über seinen
Alias identifiziert.

> Atlassian hat das **Ende des Dienstes für Opsgenie am 5. April
> 2027** festgelegt. Dieser Kanal bleibt für bestehende
> Installationen; für eine neue bevorzugen Sie PagerDuty oder einen
> benutzerdefinierten Kanal zu Jira Service Management.

### Benutzerdefinierter Webhook

Siehe [Der benutzerdefinierte Kanal](#the-custom-channel) oben.

---

## Fehlerbehebung {#troubleshooting}

**Der Test schlägt sofort fehl, mit "missing setting …".**
Ein erforderliches Feld ist leer. Die Meldung benennt genau die
Einstellung.

**Der Test schlägt fehl mit "no answer within 15 s".**
Der Zielserver ist von DumbMonit aus nicht erreichbar. Ist es ein
Dienst in Ihrem lokalen Netzwerk, prüfen Sie, dass der
DumbMonit-Container ihn erreichen kann — das ist häufig ein
Docker-Netzwerkproblem, kein Kanal-Konfigurationsproblem.

**Der Test gelingt, aber ich erhalte nichts.**
Der Dienst hat die Nachricht angenommen und anderswohin gesendet:
falscher Kanal, falsches ntfy-Topic, falsches Pushbullet-Gerät.
Prüfen Sie die Ziel-Einstellung.

**Die Fehlermeldung enthält `***`.**
Das ist beabsichtigt: Ihr Token wurde aus der Nachricht entfernt,
bevor sie aufgezeichnet wurde. Kein Secret erscheint je in den Logs,
in der Spalte "last error", oder in einer API-Antwort.

**Wo sehe ich den letzten Fehler eines Kanals?**
In der Kanalliste: Jede Zeile zeigt das Datum der letzten
erfolgreichen Zustellung und, falls vorhanden, den Fehler des
letzten Fehlschlags.
