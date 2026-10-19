# Reputation des Mailservers {#mail-server-reputation}

Eine Maildomain oder die öffentliche Adresse eines Mailservers, geprüft gegen die
DNS-Blocklisten, die noch darüber entscheiden, ob Mail zugestellt wird: Spamhaus,
Barracuda, SpamCop und einige kleinere. Für jede Liste, die eine Adresse führt,
nennt die Monitorseite den Antwortcode der Liste, ihren veröffentlichten Grund und
einen direkten Link zu ihrer Abfrage- und Entfernungsseite.

**Kein API-Schlüssel nötig.** Lassen Sie die Zugangsdaten auf „No key (recommended)“:
Die Listen werden über den DNS-Resolver des Servers abgefragt oder über den
Resolver, den Sie unter **Resolver** eintragen. Nur Spamhaus kann ablehnen, und nur,
wenn dieser Resolver ein öffentlicher ist; siehe [Resolver, Ablehnungen und Zeitüberschreitungen](#resolvers-refusals-and-timeouts).

Geben Sie eine Domain ein (`example.com`), und DumbMonit prüft die Adressen ihrer
MX-Hosts (A- und AAAA-Einträge), die Adressen der Domain selbst und alle
**Zusätzlichen Adressen**, die Sie ergänzen (ein ausgehendes Relay, das kein MX
ist). Die Domain wird außerdem gegen die Domain-Blocklisten geprüft. Geben Sie eine
IP-Adresse ein, wird nur diese Adresse geprüft. Um einen ganzen Adressbereich zu
beobachten, nutzen Sie [Blockliste für IP-Bereiche](ip_blacklist.md).

## Gmail, Outlook.com und Yahoo {#gmail-outlook-com-and-yahoo}

Die großen Postfachanbieter veröffentlichen **keine abfragbare Blockliste**: Was
sie von einem Absender halten, bleibt intern. DumbMonit tut nicht so, als würde es
sie fragen. Es prüft, was bei ihrer Entscheidung ins Gewicht fällt, und verlinkt
ihre Werkzeuge:

- **Reverse-DNS**: Jede sendende Adresse sollte einen PTR-Eintrag haben, dessen
  Name wieder auf dieselbe Adresse auflöst (Forward-confirmed Reverse DNS,
  FCrDNS). Gmail, Outlook.com und Yahoo lehnen Mails von Adressen ohne ihn ab oder
  sortieren sie als Junk aus.
- **SPF und DMARC**: genau ein SPF-Eintrag auf der Domain und ein DMARC-Eintrag
  unter `_dmarc.<domain>`. Gmail und Yahoo verlangen beides von Massenversendern.
- **Die unten stehenden Blocklisten**, die auch ihre Filter lesen.

DKIM wird nicht geprüft: Sein Selektor wird von jedem Absender selbst gewählt und
lässt sich nicht erraten. Die Monitorseite verlinkt:

| Anbieter | Werkzeuge |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/) (Reputation einer verifizierten Domain), [Absenderrichtlinien](https://support.google.com/a/answer/81126), [Formular für Zustellprobleme](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/) (Reputationsdaten pro Adresse), [Absenderrichtlinien](https://sendersupport.olc.protection.outlook.com/pm/), [Support-Formular für Absender](https://olcsupport.office.com/) (Entfernung bei Outlook.com), [Entfernungsportal von Microsoft 365](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/), [Best Practices](https://senders.yahooinc.com/best-practices/), [Support-Formular für Absender](https://senders.yahooinc.com/contact/) |

## Die Blocklisten {#the-blocklists}

Geprüft am 16. Oktober 2026. SORBS wurde im Juni 2024 eingestellt und wird nicht
abgefragt. Jede Liste wird nach einem `A`-Eintrag unter ihrer Zone gefragt. Das
Formular zeigt sie als Checkliste namens **Blocklists** (Name, ob sie Adressen oder
die Domain prüft, groß oder nicht, und eine kurze Beschreibung); die **großen**
Listen sind bei einem neuen Monitor angekreuzt. Die Id in der ersten Spalte ist das,
was die Option `lists` speichert und wie API und Metriken die Liste nennen.

| Id | Liste | Zone | Prüft | IPv6 | Groß | Nutzung und Entfernung |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | Adressen | ja | ja | Kostenlose öffentliche Spiegel für geringes, nichtkommerzielles Volumen; lehnt öffentliche und offene Resolver ab. Entfernung über den [Spamhaus-Checker](https://check.spamhaus.org/). Die PBL führt Endnutzer-Bereiche per Richtlinie: Entfernen Sie eine Adresse nur dann daraus, wenn sie wirklich einen Mailserver betreibt. |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | Adressen | nein | ja | Kostenlos; Barracuda bittet darum, die abfragenden DNS-Server mit einem kostenlosen Konto auf barracudacentral.org zu registrieren. [Entfernungsantrag](https://www.barracudacentral.org/rbl/removal-request). |
| `spamcop` | SpamCop | `bl.spamcop.net` | Adressen | nein | ja | Kostenlos. Einträge laufen etwa einen Tag nach dem Ende der Meldungen von selbst ab; die [Abfrageseite](https://www.spamcop.net/bl.shtml) zeigt den Grund und bietet die Entfernung an. |
| `mailspike` | Mailspike | `bl.mailspike.net` | Adressen | nein | nein | Kostenlos bei geringem Volumen. [Abfrage und Entfernung](https://mailspike.io/ip_verify). |
| `uceprotect_1` | UCEPROTECT Level 1 | `dnsbl-1.uceprotect.net` | Adressen | nein | nein | Abfrage kostenlos. Einträge laufen nach sieben Tagen ohne neuen Spam ab; die [Prüfseite](https://www.uceprotect.net/en/rblcheck.php) verkauft auch eine Express-Entfernung, die nie nötig ist. Wenige Empfänger nutzen sie. |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | Adressen | nein | nein | Kostenlos. Entfernung in Eigenregie auf der [Listing-Seite](https://psbl.org/). |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | Adressen | nein | nein | Kostenlos bei geringem Volumen. RATS-NoPtr führt Adressen ohne Reverse-DNS. [Abfrage](https://www.spamrats.com/lookup.php) und [Entfernung](https://www.spamrats.com/removal.php). |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | die Domain | — | ja | Dieselben Bedingungen wie ZEN. Entfernung über den [Spamhaus-Checker](https://check.spamhaus.org/). |
| `surbl` | SURBL (multi) | `multi.surbl.org` | die Domain | — | nein | Kostenlos bei geringem Volumen; lehnt öffentliche und stark ausgelastete Resolver ab. [Abfrage und Entfernung](https://www.surbl.org/surbl-analysis). |
| `uribl` | URIBL (multi) | `multi.uribl.com` | die Domain | — | nein | Kostenlos bei geringem Volumen; lehnt öffentliche und stark ausgelastete Resolver ab. [Abfrage](https://lookup.uribl.com/), [Entfernung](https://admin.uribl.com/). |

Ein Eintrag auf einer **großen** Liste löst einen kritischen Alarm aus, einer auf
den anderen Listen einen Hinweis-Alarm. IPv6-Adressen des MX werden nur gegen
Spamhaus ZEN geprüft, die einzige Liste hier, die für IPv6 antwortet; bei den
anderen erscheinen sie als nicht abgedeckt.

### Eigene Listen {#your-own-lists}

Fügen Sie unter **Custom lists** am Ende der Checkliste die DNSBL-Zonen jeder
anderen Liste hinzu, die Sie nutzen oder selbst betreiben (zum Beispiel
`dnsbl.example.org`). Jede hat einen **Type**: `ip` (die Zone wird zu jeder Adresse
gefragt, als `1.2.0.192.dnsbl.example.org`) oder `domain` (zur Mail-Domain) und einen
Schalter **Major** für eine Liste, deren Eintrag den kritischen Alarm auslösen soll.
Die Zone muss ein gültiger Domainname sein, keine IP-Adresse, und darf keine andere
Zone und keine der obigen Listen wiederholen; höchstens 20 eigene Listen. Jede
Antwort `127.x.x.x` zählt als gelistet, `NXDOMAIN` als sauber.

Der Monitor fragt genau die angekreuzten Listen plus die eigenen. Ein Monitor, der
vor diesen Optionen angelegt wurde, hat keine Einstellung und fragt weiter jede
obige Liste. Die Option `lists` enthält die angekreuzten Ids durch Kommas getrennt
(`-`, wenn keine angekreuzt ist) und `custom_lists` ein JSON-Array wie
`[{"zone": "dnsbl.example.org", "subject": "ip", "major": false}]`.

## Resolver, Ablehnungen und Zeitüberschreitungen {#resolvers-refusals-and-timeouts}

Die Listen werden über DNS abgefragt, und wie sie antworten, hängt davon ab,
**welcher Resolver fragt**:

- **Spamhaus lehnt Anfragen ab, die über öffentliche Resolver kommen** (Google
  `8.8.8.8`, Cloudflare `1.1.1.1`, Quad9), über Resolver, die an sie weiterleiten,
  oder über Resolver ohne zuordenbares Reverse-DNS. Es antwortet dann
  `127.255.255.254` statt `NXDOMAIN` (`127.255.255.255`, wenn ein Resolver zu
  viele Anfragen sendet, `127.255.255.252` bei einer fehlerhaften Anfrage oder
  einem falschen DQS-Schlüssel). Das sieht aus wie ein Eintrag und ist keiner:
  DumbMonit markiert die Spamhaus-Liste als **abgelehnt** („Spamhaus refuses this
  resolver“ auf der Monitorseite), nie als „gelistet“, und die anderen Listen
  behalten ihre Ergebnisse. Nur wenn keine andere ausgewählte Liste geantwortet hat,
  zeigt der Monitor statt eines Ergebnisses einen **Konfigurationsfehler** (der
  keinen Ausfallalarm sendet).
- **URIBL und SURBL antworten `127.0.0.1`** auf Resolver, die sie ablehnen. Diese
  Liste erscheint dann als „Abgelehnt“ (unbekannt), nicht als Eintrag.
- **Eine Liste, die nicht rechtzeitig antwortet, ist unbekannt, nie sauber.** Nur
  `NXDOMAIN` bedeutet „nicht gelistet“. Eine Antwort außerhalb von `127.0.0.0/8`
  (ein Resolver, der `NXDOMAIN` in die Adresse einer Suchseite umschreibt) ist
  ebenfalls unbekannt.

Nutzen Sie also **einen eigenen rekursiven Resolver**: Unbound, Knot Resolver,
BIND oder PowerDNS Recursor, die ab den Root-Servern auflösen (nicht an einen
öffentlichen Resolver weiterleiten), und tragen Sie seine Adresse unter
**Resolver** ein, wenn er nicht der System-Resolver ist. Pi-hole oder AdGuard Home
vor Unbound funktioniert; vor `1.1.1.1` nicht.

Oder, nur wenn Sie auch Spamhaus wollen und den Resolver nicht ändern können, legen
Sie ein kostenloses Konto beim [Spamhaus Data Query Service](https://www.spamhaus.com/data-access/free-data-query-service/)
an (nichtkommerzielle Nutzung, geringes Volumen) und tragen Sie seinen Schlüssel
als Zugangsdaten ein („Spamhaus DQS key (optional)“): Spamhaus wird dann unter
`<key>.zen.dq.spamhaus.net` und `<key>.dbl.dq.spamhaus.net` gefragt, die über jeden
Resolver antworten. Der Schlüssel wird verschlüsselt gespeichert und nie wieder
angezeigt.

## Tempo {#pace}

Blocklisten begrenzen, wie oft ein Resolver fragen darf, und sperren Resolver, die
sie durchkämmen. DumbMonit fragt jede Liste einmal pro **Refresh interval**
(standardmäßig 6 Stunden), höchstens **Queries per second** (standardmäßig 10) bei
acht gleichzeitigen Anfragen, und zeigt dazwischen bei jeder Prüfung das letzte
Ergebnis erneut, sodass die Alarmregeln eine durchgehende Reihe sehen. Eine Prüfung
läuft im Hintergrund: Das erste Ergebnis erscheint einige Sekunden nach dem
Hinzufügen des Monitors, und das Ändern der Adresse, einer Option oder des
Schlüssels startet sofort eine neue Prüfung. Eine Prüfung mit einem Fehler als
Ergebnis wird nach zehn Minuten wiederholt.

## Einen Mailserver gegen die Blocklisten prüfen {#check-a-mail-server-against-the-blocklists}

1. Geben Sie die Domain ein, für die Sie Mail versenden, zum Beispiel „example.com“: DumbMonit prüft die Adressen ihres MX und der Domain selbst. Oder geben Sie die öffentliche IP-Adresse ein, von der der Server sendet. Verlässt Mail das Netz über ein Relay, das kein MX ist, ergänzen Sie dessen Adresse unter Extra addresses.

2. Kein API-Schlüssel nötig: Lassen Sie die Zugangsdaten auf „No key“. Ohne Schlüssel werden die Blocklisten über den DNS-Resolver dieses Servers abgefragt oder über den, den Sie unter Resolver eintragen.

3. Kreuzen Sie unter Blocklists die abzufragenden Blocklisten an (die großen sind standardmäßig angekreuzt) und ergänzen Sie unter Custom lists die DNSBL-Zonen jeder anderen Liste, die Sie nutzen.

4. Nur Spamhaus kann eine Anfrage ablehnen, und nur, wenn der Resolver ein öffentlicher ist (Google, Cloudflare, Quad9) oder an einen weiterleitet. Der Monitor zeigt dann „Spamhaus refuses this resolver“, und jede andere Liste arbeitet weiter. Um auch Spamhaus zu bekommen, setzen Sie Resolver auf einen eigenen rekursiven Resolver (ein Unbound, das ab den Root-Servern auflöst, genügt) oder wählen Sie als Zugangsdaten den optionalen Spamhaus-DQS-Schlüssel: ein kostenloses Konto (nichtkommerzielle Nutzung, geringes Volumen), dessen Schlüssel Spamhaus über jeden Resolver antworten lässt.

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

5. Um die Einrichtung zu prüfen, fügen Sie einen Monitor für 127.0.0.2 hinzu, die Testadresse, die jede Blockliste führt: Jede sollte sie als gelistet zeigen.

6. Gmail, Outlook.com und Yahoo veröffentlichen keine abfragbare Blockliste. DumbMonit prüft stattdessen, was sie gewichten (ein Reverse-DNS, das auf die Adresse zurückführt, SPF- und DMARC-Einträge), und verlinkt auf der Monitorseite ihre Absenderwerkzeuge.

!!! warning
    Eine von Spamhaus abgelehnte Antwort (127.255.255.254, ein öffentlicher Resolver) erscheint nie als Eintrag: Diese Liste wird als abgelehnt markiert, und die anderen behalten ihr Ergebnis. Nur wenn keine andere Liste antwortet, meldet der Monitor einen Konfigurationsfehler. Eine Blockliste, die nicht antwortet, erscheint als unbekannt, nie als sauber. Die Listen werden standardmäßig alle 6 Stunden gefragt: Häufigeres Fragen bringt nichts und führt zur Sperrung von Resolvern.

## Was beobachtet wird {#what-it-watches}

Alle Metriken tragen das Präfix `dumbmonit_blacklist_`.

| Metrik | Was | Labels |
|---|---|---|
| `listed_major_lists` | große Listen (Spamhaus ZEN und DBL, Barracuda, SpamCop), die eine Adresse oder die Domain führen | |
| `listed_lists` | alle Listen, die eine Adresse oder die Domain führen | |
| `addresses_checked` | geprüfte Adressen | |
| `addresses_listed` | Adressen, die auf mindestens einer Liste stehen | |
| `domain_listed` | 1, wenn die Domain auf einer Domain-Blockliste steht | |
| `unknown_checks` | Prüfungen ohne Antwort (Zeitüberschreitung, Ablehnung) | |
| `check` | eine Reihe pro Adresse (oder Domain) und Liste: 1 gelistet, 0 sauber, −1 keine Antwort, −2 abgelehnt, −3 nicht abgedeckt | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | pro Liste: gefragte Objekte, gelistete, ohne Antwort | `list` |
| `list_info` | Wert 1, eine Reihe pro tatsächlich abgefragter Liste (auch eigene): was die Monitorseite zeigt | `list`, `name`, `zone`, `subject`, `major` |
| `address_info` | Wert 1, woher jede Adresse stammt | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 1, wenn das Reverse-DNS auf die Adresse zurückauflöst | `address`, `ptr` |
| `mx_hosts` | MX-Hosts der Domain | |
| `spf_records` | `v=spf1`-Einträge auf der Domain (1 ist richtig) | |
| `dmarc_records` | `v=DMARC1`-Einträge unter `_dmarc.<domain>` | |
| `dmarc_policy_info` | Wert 1 | `policy` |
| `mail_auth_missing` | 0 bis 2: SPF fehlt oder doppelt, DMARC fehlt | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | die angezeigte Prüfung: ihr Alter, ihre Dauer, ob eine neue läuft | |

Die [integrierten Regeln](../alerting/rules.md#dns-blocklists), die greifen: Auf
einer großen Blockliste geführt, Auf einer Blockliste geführt, Reverse-DNS des
Mailservers passt nicht, SPF oder DMARC fehlt, dazu Monitor nicht erreichbar, wenn
der Resolver keine der Anfragen beantwortet.

## Die Monitorseite {#the-monitor-page}

Das Panel liest, was die Prüfung gespeichert hat; das Öffnen der Seite fragt nie
eine Blockliste ab. Es zeigt die Zähler, dann die tatsächlich abgefragten Listen (die
angekreuzten und Ihre eigenen, jede mit der Zahl der von ihr geführten Objekte und
„Spamhaus refuses this resolver“, wenn das zutrifft), dann jedes Ergebnis mit dem
Code der Liste, ihrem veröffentlichten Grund und Links zu ihren Abfrage- und
Entfernungsseiten, dann das Reverse-DNS jeder Adresse, SPF und DMARC sowie die
Absenderwerkzeuge von Gmail, Outlook.com und Yahoo.

Die Seite hat außerdem einen Link **Also check on MXToolbox**. Er öffnet in einem
neuen Tab die öffentliche Blocklist-Abfrage von mxtoolbox.com für die überwachte
Adresse oder Domain; DumbMonit ruft deren API nicht auf.

## Zugangsdaten {#credentials}

Keine nötig („No key (recommended)“, die Vorgabe), oder optional ein
Spamhaus-DQS-Schlüssel (siehe oben), der nur für Spamhaus eine Rolle spielt.

Adresse: eine Domain (`example.com`) oder eine öffentliche IP-Adresse. Optionen:
Resolver, Blocklists (die großen sind bei einem neuen Monitor angekreuzt, bei einem
früher angelegten alle), Custom lists, Extra addresses, Refresh interval (6 Stunden,
von 1 bis 168), Queries per second (10, von 1 bis 50) und Timeout per query (5 s,
von 1 bis 30). Private Adressen werden nie geprüft: Blocklisten kennen nur das
öffentliche Internet. `127.0.0.2` wird als Testadresse akzeptiert.
