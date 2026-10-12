# MDaemon Email Server

Die Mail-Dienste eines MDaemon-Servers, von außen geprüft: SMTP, IMAP,
POP3, Webmail und Remote Administration akzeptieren Verbindungen und
grüßen wie erwartet. Mit einem Konto liest DumbMonit zusätzlich die
genaue Version über die XML-API. Mit dem auf dem MDaemon-Server
installierten [Agent](agent.md) zeigt dieselbe Seite auch die
Mail-Warteschlangen, Sitzungen und Nachrichtenzahlen, die MDaemon nur
als Windows-Leistungszähler veröffentlicht.

!!! warning "Nur gegen die Dokumentation validiert"

    Diese Integration wurde aus MDaemon Technologies' veröffentlichter
    Dokumentation und dem dort gezeigten XML-API-Antwortformat gebaut,
    nicht gegen einen echten MDaemon-Server. Jeder Aufruf ist ein
    Lesevorgang, und die Mail-Port-Prüfungen folgen den
    Protokollstandards, aber noch niemand hat es gegen einen laufenden
    MDaemon getestet. Dasselbe gilt für die vom Agent gelesenen
    Warteschlangenzähler: Ihre Namen stammen aus einer veröffentlichten
    Liste der Leistungszähler von MDaemon, nicht von einem laufenden
    Server. Sagen Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_mdaemon_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `service_up` | `service`, `port` | 1, wenn der Port die Verbindung angenommen hat und, für SMTP, POP3 und IMAP, mit `220`, `+OK` oder `* OK` grüßte. 0, wenn er nicht antwortete, oder antwortete, um den Dienst zu verweigern (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Zeit bis zur Begrüßung, oder bis zur Verbindung bei einem Port ohne eine. |
| `api_up` | | Nur mit einem Konto: 1, wenn die XML-API antwortete, 0, wenn nicht. |
| `api_response_seconds` | | Zeit, die die XML-API zum Antworten brauchte. |
| `api_operation_ok` | | 1, wenn das Konto den Aufruf ausführen durfte, 0, wenn MDaemon mit einem Fehlerstatus antwortete. |
| `info` (Wert 1) | `version`, `build`, `source` | Die Version: von der XML-API (`source="xml_api"`, `build` ist der Dienst-Build), oder sonst von der SMTP-Begrüßung, wenn der Server sie dort ankündigt (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Zeit, die die gesamte Prüfung brauchte. |

**Ein teilweiser Ausfall ist weiterhin eine erfolgreiche Prüfung.**
Ein gestoppter Dienst gibt für diesen Dienst `service_up 0`, und die
anderen werden weiterhin gemessen. Nur wenn kein Dienst und keine API
überhaupt antwortet, ist das Gerät *unreachable*.

**Ein abgelehntes Konto ist ein Konfigurationsfehler, kein Ausfall.**
Ein falsches Passwort, eine von der XML-API nicht erlaubte Adresse,
oder ein falscher Port wird auf der Geräteseite mit dem Grund
angezeigt und nicht benachrichtigt.

**Warteschlangen kommen vom Agent, nicht von diesem Gerät.** MDaemon
veröffentlicht seine Warteschlangengrößen (Inbound, Local, Remote,
Retry, Bad, Holding, Quarantine), seine Sitzungszahlen, seine
Nachrichtenstatistiken und seine Uptime als Windows-Leistungszähler,
im Objekt namens MDaemon, und in seinem eigenen Queue and Statistics
Manager. Keine von MDaemons veröffentlichter Dokumentation beschreibt
einen XML-API-Aufruf, der sie zurückgibt, daher rät dieses Gerät
keinen: Der auf dem MDaemon-Server installierte [Agent](agent.md)
liest stattdessen die Leistungszähler. Siehe
[Mail-Warteschlangen über den Agent](#mail-warteschlangen-ber-den-agent).
Die Lizenz und der Plattenplatz des Mail-Stores werden über keine
dokumentierte Schnittstelle oder einen Leistungszähler exponiert und
nicht gelesen. Der Agent beobachtet auch die Platte mit dem
Mail-Store; listen Sie den MDaemon-Dienst in seiner
`services`-Einstellung, um den Windows-Dienst selbst zu beobachten.

Die geltenden [eingebauten Regeln](../alerting/rules.md#mdaemon-and-securitygateway):
MDaemon mail service down, MDaemon XML API not answering, plus Device
unreachable; mit dem Agent, MDaemon mail queue growing, MDaemon Bad
queue not empty und MDaemon Retry queue high.

## Die Geräteseite

Ein MDaemon-Gerät zeigt ein **Mail services**-Panel über den
Diagrammen, aus der letzten gespeicherten Messung gelesen: Das Öffnen
der Seite verbindet sich nie mit dem Server. Es listet jeden
beobachteten Dienst mit seinem Port, einem Wort und einer Farbe
(Answering, Down), und seiner Antwortzeit; dann die Version und wo sie
gelesen wurde. Gestoppte Dienste kommen zuerst.

Wenn der Agent auf demselben Server MDaemons Zähler meldet, folgt ein
**Mail queues**-Panel: die Anzahl der Nachrichten in jeder
Warteschlange (eine eingefrorene Warteschlange, eine nicht leere
Bad-Warteschlange und mehr als fünfzig Nachrichten in Retry werden
wörtlich markiert), die aktiven Sitzungen, die Nachrichten und Spam-,
Virus- und DNSBL-Urteile der letzten 24 Stunden, MDaemons Uptime, und
die von MDaemon als inaktiv gemeldeten internen Server. Dasselbe
Panel erscheint auf der eigenen Seite des Agents.

## Mail-Warteschlangen über den Agent {#mail-warteschlangen-ber-den-agent}

1. Installieren Sie den [Agent](agent.md#install) auf dem
   Windows-Server, der MDaemon ausführt. Nichts sonst ist nötig: Der
   Agent bemerkt den Windows-Dienst namens `MDaemon` und liest die
   Zähler des MDaemon-Leistungsobjekts bei jeder Sampling-Periode. Um
   es zu erzwingen oder abzuschalten, setzen Sie `mdaemon: true` oder
   `mdaemon: false` in `agent.yaml`.
2. Die Warteschlangen erscheinen auf der Seite des Agents. Um sie
   auch auf diesem MDaemon-Gerät anzuzeigen, sucht DumbMonit nach dem
   Agent auf derselben Maschine: der als übergeordnetes Gerät dieses
   Geräts gesetzte Agent, oder der es abfragende Relay-Agent, oder der
   Agent, dessen Hostname zur Adresse dieses Geräts passt (`mail` für
   `mail.example.com`), oder, wenn es ein MDaemon-Gerät und einen
   MDaemon-Zähler meldenden Agent gibt, dieser Agent. Passt keiner,
   bearbeiten Sie dieses Gerät und wählen Sie den Agent als sein
   **Parent device**: Der Agent und MDaemon teilen sich die Maschine,
   daher werden, wenn der Agent verstummt, auch die Alarme dieses
   Geräts zurückgehalten.

Die Zeitreihen gehören dem Agent, mit seinem `target`-Label. Alle mit
dem Präfix `dumbmonit_mdaemon_`:

| Metrik | Labels | Gelesene Leistungszähler |
|---|---|---|
| `queue_messages` | `queue`: `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | `Inbound queue messages`, `Local queue messages`, `Remote queue messages`, `Retry queue messages`, `Bad queue messages`, `Holding queue messages`, `LAN queue messages`, `Quarantine queue messages`, `RAW queue messages` |
| `queue_frozen` (1 eingefroren) | `queue`: `inbound`, `local`, `remote` | `Inbound queue frozen`, `Local queue frozen`, `Remote queue frozen` |
| `sessions_active` | `protocol`: `smtp_in`, `smtp_out`, `pop3_in`, `pop3_out`, `imap`, `webmail` | `Active SMTP (in) sessions`, `Active SMTP (out) sessions`, `Active POP3 (in) sessions`, `Active POP3 (out) sessions`, `Active IMAP sessions`, `Active Webmail sessions` |
| `sessions_total` (Zähler) | `protocol`: `smtp_in`, `smtp_out`, `pop3`, `imap` | `SMTP sessions (in) total`, `SMTP sessions (out) total`, `POP3 sessions total`, `IMAP sessions total` |
| `messages_total` (Zähler) | `protocol`: `smtp_in`, `smtp_out`, `domainpop_in` | `SMTP messages (in) total`, `SMTP messages (out) total`, `DomainPOP messages (in) total` |
| `filtered_messages_total` (Zähler) | `filter`: `spam`, `virus`, `dnsbl`; `verdict`: `accepted`, `refused` | `spam accepted total`, `spam refused total`, `Viruses accepted total`, `Viruses refused total`, `DNSBL accepted total`, `DNSBL refused total` |
| `server_active` (1 aktiv) | `server`: `smtp`, `pop3`, `imap`, `webmail`, `webadmin`, `activesync`, `antispam`, `antivirus`, `minger`, `multipop`, `domainpop` | `SMTP server state`, `POP3 server state`, `IMAP server state`, `Web Mail server state`, `Web Admin server state`, `ActiveSync server state`, `AntiSpam server state`, `AntiVirus server state`, `Minger server state`, `MultiPOP server state`, `DomainPOP server state` |
| `running` (1 laufend) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

Totals zählen seit dem letzten Start von MDaemon: Wenden Sie
`increase()` oder `rate()` in Abfragen an, wie die Seite es für ihre
24-Stunden-Werte tut. Die Pro-Sekunde-Zähler (`SMTP messages
(in)/sec`…) werden nicht gelesen, da die Totals dieselben Raten
geben. Ein Zähler, den Ihre MDaemon-Version nicht hat, oder anders
benennt, wird einmal im Log des Agents gemeldet (`performance counter
not available`), und die anderen werden weiter gelesen; um einen vom
Preset fehlenden zu lesen, listen Sie ihn unter `perf_counters` in
`agent.yaml` (siehe
[Windows-Leistungszähler](agent.md#windows-performance-counters)).

## Die Mail-Dienste beobachten, und optional die XML-API

1. Nichts auf dem Server zu installieren. Ohne Credential verbindet
   sich DumbMonit von außen mit den Mail-Ports und liest jede
   Begrüßung: standardmäßig SMTP (25), IMAP (143) und Webmail (3000).
   Fügen Sie POP3, MSA, die TLS-Ports oder Remote Administration in
   der Option Services hinzu, als Liste wie diese.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. Um auch die Version über die XML-API zu lesen, erstellen Sie ein
   dediziertes Konto in MDaemon, zum Beispiel wie folgt benannt, mit
   einem nirgendwo sonst genutzten langen Passwort. Geben Sie ihm das
   niedrigste von Ihrem MDaemon akzeptierte Level für die XML-API, und
   nicht mehr: DumbMonit ruft nur GetVersionInfo auf, das kein
   Postfach liest und nichts ändert.

    ```
    dumbmonit@example.com
    ```

3. Seit MDaemon 24 beantwortet die XML-API nur die ihr erlaubten
   Adressen. Öffnen Sie Setup → XML API Service → Address
   Restrictions (Setup → XML API Management auf MDaemon 26) und
   erlauben Sie die Adresse, von der aus DumbMonit sich verbindet.

4. Prüfen Sie vom DumbMonit-Host aus, dass das Konto die API erreicht.
   Der Befehl fragt nach dem Passwort und gibt eine Antwort aus, die
   mit `<MDaemon><API productversion=…>` beginnt.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. Geben Sie in DumbMonit die Serveradresse ein, zum Beispiel
   "mail.example.com", dann die vollständige E-Mail-Adresse des
   Kontos als Benutzername, und sein Passwort. Die XML-API wird auf
   dem HTTPS-Port der Remote Administration erreicht, standardmäßig
   444.

!!! warning
    MDaemon hat keine nur-lesende Rolle für die XML-API: Nutzen Sie
    nie Ihr eigenes Konto wiederverwendet, und halten Sie dieses
    Passwort von jedem anderen Tool fern. Dynamic Screening kann eine
    Adresse nach wiederholten fehlgeschlagenen Logins blockieren,
    daher kann ein falsches Passwort hier den DumbMonit-Host
    blockieren lassen: Testen Sie zuerst mit dem obigen Befehl. Remote
    Administration nutzt oft ein selbstsigniertes Zertifikat: Wird die
    Verbindung deshalb abgelehnt, kreuzen Sie "Accept an unverifiable
    certificate" in den Optionen an. Mail-Warteschlangengrößen werden
    von diesem Gerät nicht gelesen: MDaemon veröffentlicht sie nur als
    Windows-Leistungszähler, die der DumbMonit-Agent liest, wenn er
    auf dem Mail-Server installiert ist.

Die XML-API wird vom Remote-Administration-Webserver bedient:
standardmäßig Port 1000 über HTTP und 444 über HTTPS, unter
`/MdMgmtWS/`, oder von IIS, wenn Remote Administration dort läuft.
Ihre vollständige Referenz wird mit dem Server geliefert: Öffnen Sie
`https://mail.example.com:444/MdMgmtWS` in einem Browser, oder sehen
Sie in `\MDaemon\Docs\API\XML API\`.

## Credentials

| Credential | Felder |
|---|---|
| Nur Dienste | Nichts: Die Mail-Ports werden ohne Login geprüft. |
| XML-API-Konto | Die vollständige E-Mail-Adresse des Kontos, und sein Passwort, als HTTP-Basic-Authentifizierung gesendet. |

Adresse: der Hostname oder die IP des Servers, zum Beispiel
`mail.example.com`. Ein in der Adresse geschriebener Port
(`mail.example.com:8444`) wird als XML-API-Port genommen.

## Optionen

| Option | Standard | Wirkung |
|---|---|---|
| `services` | `smtp,imap,webmail` | Was geprüft wird, kommagetrennt: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `name:port` ändert einen Port; jeder andere Name mit seinem Port fügt einen eigenen Dienst hinzu, nur per Verbindung geprüft. `none` prüft keinen Port. |
| `request_timeout_seconds` | `10` | Erlaubte Zeit für jede Verbindung, Begrüßung und API-Aufruf, von 1 bis 60. |
| `api_port` | `444` | Der Remote-Administration-Port, der `/MdMgmtWS/` bedient. Nur mit einem Konto genutzt. |
| `api_tls` | `true` | HTTPS zur XML-API. Nur für Remote Administration über einfaches HTTP (standardmäßig Port 1000) entfernen: Das Passwort durchläuft dann das Netzwerk im Klartext. |
| `insecure_tls` | `false` | Ein selbstsigniertes Zertifikat auf der XML-API akzeptieren. |

Die TLS-Ports (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) werden
nur per Verbindung geprüft. Um den Ablauf ihrer Zertifikate zu
beobachten, fügen Sie für jeden ein
[TLS-Zertifikat](services.md#tls)-Gerät hinzu.
