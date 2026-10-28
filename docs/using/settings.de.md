# Einstellungen {#settings}

![Einstellungen: sieben Tabs, hier Mein Konto mit Darstellung, Konto und Sicherheit und Push-Benachrichtigungen](../assets/screenshots/settings-light.png){ loading=lazy }

Die Einstellungen enthalten das Administrative: die Instanz selbst, Ihr Konto, wer sich wie anmelden kann, die Agenten, die Benachrichtigungskanäle, die Sicherung der Instanz und den Zustand des Servers. Sie sind in sieben Tabs gegliedert, jeder mit eigener Adresse. Eine Leiste auf breitem Bildschirm, eine Reihe von Chips auf dem Telefon und ein Suchfeld über den Abschnittsnamen helfen, einen Abschnitt zu finden.

| Tab | Adresse | Inhalt |
|---|---|---|
| Allgemein | `/settings/general` | Name der Instanz, Sprache und Zeitzone, der Einrichtungsassistent ([Oberfläche und Einrichtungsassistent](product-modes.md)) |
| Mein Konto | `/settings/account` | Sprache, Konto und Sicherheit (Passwort, Zwei-Faktor, Sicherheitsprotokoll), Push-Benachrichtigungen, Bereitschaftskontakte und Ruhezeiten ([Bereitschaft](../alerting/oncall.md)) |
| Team & Zugang | `/settings/team` | Benutzer, Teams, Kunden, Single Sign-on, SAML, SCIM, LDAP |
| Überwachung | `/settings/monitoring` | Agenten, Integrationspakete, [Sicherheitsbewertung](security-score.md), Standorte |
| Benachrichtigungen | `/settings/notifications` | Kanäle, Benachrichtigungsrichtlinie ([Benachrichtigungskanäle](../notifications.md)) |
| Wandanzeige | `/settings/wall` | Wandmodus: was er zeigt und wie man ihn öffnet ([Wandmodus](wall.md)) |
| System | `/settings/system` | Sicherung, Konfigurationsdateien, DumbMonit selbst, Daten und Aufbewahrung, Logs, API und Assistenten, Info |

Jeder Abschnitt behält seinen Anker: `/settings#agents` leitet an `/settings/monitoring#agents` weiter. Die alten Links (`/alerts#notifications`) leiten ebenfalls an die neue Stelle weiter. Statusseiten und Berichte sind nicht mehr in den Einstellungen: Sie haben eigene Seiten, **Statusseiten** (`/status`) und **Berichte** (`/reports`); `/settings/status` und `/settings/reports` leiten dorthin weiter. Musik, Nachrichten und Widgets der Wand gibt es nicht mehr: Ihre alten Links (`#music`, `#news`, `#widgets`) öffnen den Tab Wandanzeige.

## Darstellung {#appearance}

Die Sprache der Oberfläche, in diesem Browser gespeichert. Die Konsole hat ein
einziges dunkles Erscheinungsbild, es gibt also kein Theme zu wählen; öffentliche
Statusseiten behalten ihr helles und dunkles Theme.

## Wandanzeige {#wall}

**Wandmodus öffnen**: die Übersicht der Konsole im Vollbild, für einen Monitor
im Raum. ++esc++ verlässt ihn, die Befehlspalette öffnet ihn von überall. Die
Wand hat keine Musik, keine Nachrichten und keine Widgets mehr; auf Wunsch
steht eine nächtliche Stadt hinter ihrem Urteil. Siehe [Wandmodus](wall.md).

## Konto & Sicherheit {#account-security}

Das ist Ihr eigenes Konto. Das erste wird beim ersten Start erstellt, weitere lassen sich unten
hinzufügen. Hier ändern Sie Ihr Passwort: aktuelles Passwort, neues Passwort (mindestens 12
Zeichen; ein ganzer Satz ist sicherer als ein kompliziertes Wort), Bestätigung. Die Änderung
meldet jede andere Sitzung ab. Konten, die sich über den Identitätsanbieter anmelden, haben hier
kein Passwort. **Abmelden** beendet diese Sitzung.

Die Anmeldung ist pro Client-Adresse und pro Konto ratenbegrenzt: Nach fünf fehlgeschlagenen
Versuchen wartet jeder weitere länger (30 s, sich verdoppelnd, bis zu 5 Minuten). Setzen Sie
hinter einem Reverse-Proxy `DUMBMONIT_TRUSTED_PROXIES`, damit der Limiter die echte
Client-Adresse statt der des Proxys sieht (siehe [Konfiguration](../reference/configuration.md)).
Sitzungen dauern 30 Tage. Ein vergessenes Passwort wird mit `DUMBMONIT_RESET_PASSWORD=1`
zurückgesetzt: siehe die [FAQ](../faq.md#i-lost-the-password).

### Zwei-Faktor-Authentifizierung {#two-factor-authentication}

Konten, die sich mit einem Passwort anmelden, können einen zweiten Faktor hinzufügen: einen
sechsstelligen zeitbasierten Code (TOTP) aus einer Authenticator-App — Aegis, FreeOTP, Google
Authenticator, 1Password, Bitwarden … **Einrichten** fragt nach Ihrem Passwort, zeigt einen
QR-Code zum Scannen (oder den Schlüssel zum Eintippen) und aktiviert den zweiten Faktor, sobald
Sie einen ersten Code aus der App eingeben. Danach werden einmalig acht **Wiederherstellungscodes**
angezeigt: Bewahren Sie sie auf, jeder meldet Sie genau einmal an, falls das Smartphone verloren
geht. Der Anmeldebildschirm fragt nach dem Passwort nach dem Code (oder einem
Wiederherstellungscode); fünf falsche Codes brechen den Versuch ab, und Sie beginnen wieder beim
Passwort, wobei die übliche Anmelde-Ratenbegrenzung über Versuche hinweg weiterzählt. Ein Code
wird nur einmal akzeptiert: Der gerade verwendete (auch der, der den zweiten Faktor aktiviert
hat) meldet Sie nicht erneut an — warten Sie auf den nächsten.

**Deaktivieren** fragt erneut nach dem Passwort. Wiederherstellungscodes lassen sich nicht
einzeln neu erzeugen: Deaktivieren Sie und richten Sie neu ein, um einen frischen Satz zu
erhalten. Ein Admin kann den zweiten Faktor eines anderen Benutzers unter **Benutzer**
(*2FA zurücksetzen*) zurücksetzen, wenn Smartphone und Codes beide weg sind — das meldet den
Benutzer überall ab und lässt das Passwort unberührt. Das Geheimnis wird mit dem
Instanzgeheimnis verschlüsselt gespeichert; Codes werden gehasht. Konten mit Single Sign-On
erhalten ihren zweiten Faktor vom Identitätsanbieter, nicht von hier.

Admins finden in diesem Abschnitt auch das **Sicherheitslog**: Anmeldungen und Fehlschläge,
Passwort- und Zwei-Faktor-Änderungen, Token- und Kontoänderungen, mit der Client-Adresse. Es
behält die letzten 5 000 Einträge.

## Push-Benachrichtigungen {#push-notifications}

Native Benachrichtigungen auf diesem Smartphone oder Computer, auch bei geschlossenem DumbMonit:
**Auf diesem Gerät aktivieren**, dann **Test senden**. Jedes Konto verwaltet hier seine eigenen
Geräte (auch Betrachter); welche Alarme sie erreichen, legt ein Kanal **Web Push** unter Einstellungen →
Benachrichtigungen & Berichte fest. Die Schaltfläche **i** fasst die Voraussetzungen zusammen: HTTPS (oder
localhost), Installation auf dem Home-Bildschirm bei iPhone und iPad und die Domains der
Push-Dienste, die der Server erreichen muss. Vollständige Anleitung:
[Push-Benachrichtigungen (PWA)](push-notifications.md).

## Benutzer {#users}

Nur Admins. Die Konten, die sich anmelden können, und ihre Rolle:

| Rolle | Liest alles | Bearbeitet Alarme (bestätigen, zurückstellen, ignorieren, Wartungsfenster, Verlauf löschen) | Ändert die Konfiguration (Monitore, Regeln, Kanäle, Statusseiten, Einstellungen, Sicherungen, Tokens, Konten) |
|---|---|---|---|
| **Betrachter** | ja | nein | nein |
| **Operator** | ja | ja | nein |
| **Admin** | ja | ja | ja |

Ein Betrachter sieht jede Seite, wobei jedes Bedienelement, das etwas ändern würde, verborgen ist.
Einiges bleibt selbst zum Lesen Admins vorbehalten: API- und Agent-Tokens, E-Mail-Adressen der
Abonnenten von Statusseiten und Monitoroptionen, die wie Geheimnisse aussehen (eine Option, deren
Name `key`, `token`, `password` … enthält, die zusätzlichen Header einer HTTP-Prüfung, das
Passwort einer Adresse `https://user:pass@host`), die Nicht-Admins als `***` lesen. Ein Operator
ist die Bereitschaftsrolle: Die Alarmsteuerung ist da, die Konfigurationsbildschirme lesen
*Operator — Konfiguration nur für Admins*. Nur ein Admin erstellt Konten oder ändert eine Rolle —
ein Operator kann niemanden befördern, sich selbst eingeschlossen. Die Abschnitte *Benutzer*,
*Single Sign-On* und Sicherheitslog bleiben Admins vorbehalten. **Benutzer hinzufügen** fragt
nach einem Benutzernamen, einem optionalen Anzeigenamen, einer Rolle und einem Passwort; die
Schaltfläche neben dem Passwortfeld denkt sich ein zufälliges aus. Das Passwort wird einmalig
zur Übergabe angezeigt.

Jede Zeile trägt die Rolle, ob sich das Konto mit einem Passwort oder über den Identitätsanbieter
anmeldet, ob Zwei-Faktor aktiv ist, und die letzte Anmeldung. **Bearbeiten** ändert den
Anzeigenamen, die Rolle und das Passwort (ein Zurücksetzen meldet den Benutzer überall ab und
zeigt das neue Passwort einmalig an). Der Schalter **Aktiviert** sperrt ein Konto, ohne es zu
löschen; **2FA zurücksetzen** erscheint, wenn das Konto einen zweiten Faktor hat; **Löschen**
entfernt es endgültig. Der Server behält mindestens einen aktiven Admin, und die Bedienelemente
spiegeln diese Regel wider, statt sie als Fehler zu melden: Der letzte Admin kann nicht
herabgestuft, deaktiviert oder gelöscht werden, und Sie können Ihr eigenes Konto nicht
deaktivieren oder löschen.

## Single Sign-On {#single-sign-on}

Nur Admins. Anmeldung über einen OpenID-Connect-Anbieter (Authelia, Authentik, Keycloak, Pocket
ID, Google Workspace …). Füllen Sie den *Anbieternamen* (er beschriftet die Schaltfläche
„Weiter mit …“ auf dem Anmeldebildschirm), die *Issuer-URL* — Discovery wird aus
`{issuer}/.well-known/openid-configuration` gelesen —, die *Client-ID*, das *Client-Secret*
(verschlüsselt gespeichert, nie wieder angezeigt) und die *öffentliche URL* aus, unter der
Browser DumbMonit erreichen. Das Formular baut die Redirect-URI aus dieser öffentlichen URL:
Kopieren Sie sie und registrieren Sie sie bei Ihrem Anbieter.

**Discovery testen** liest die Konfiguration des Anbieters, ohne jemanden anzumelden, listet die
gefundenen Endpunkte auf und warnt, wenn der Anbieter weder mit RS256 noch mit ES256 signiert —
den einzigen beiden Signaturen, die DumbMonit akzeptiert. Hier gespeicherte Einstellungen haben
Vorrang vor den Umgebungsvariablen `DUMBMONIT_OIDC_*`, die nur gelten, solange nichts gespeichert
ist; **Gespeicherte Einstellungen vergessen** löscht sie und fällt auf die Variablen zurück,
sofern sie existieren.

**Erweitert.** Der Rest des Formulars liegt unter *Erweitert*: *Scopes* nennt, was beim Anbieter
angefragt wird, *Gruppen-Claim* den Token-Claim, der die Gruppen des Benutzers auflistet, und
*Konten bei der ersten Anmeldung erstellen* entscheidet, ob eine unbekannte Person ein Konto
erhält. Es ist **standardmäßig aus**: Nur bereits existierende Konten können sich anmelden, bis
Sie es einschalten.

**Rollen.** *Admin-Gruppen* listet die Gruppen auf, deren Mitglieder zu Admins werden;
*Operator-Gruppen* die Gruppen, deren übrige Mitglieder zu Operatoren werden; alle anderen sind
Betrachter. Lassen Sie beide leer, um Rollen weiter unter **Benutzer** zu verwalten. Ist nur
*Operator-Gruppen* ausgefüllt, berühren die Gruppen die Admin-Rolle nie: Admins bleiben Admins,
die anderen werden Operatoren oder Betrachter. Rollen werden bei jeder Anmeldung neu bewertet,
außer dass der letzte aktive Admin nie herabgestuft wird. Die entsprechenden Umgebungsvariablen
sind `DUMBMONIT_OIDC_ADMIN_GROUPS` und `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Auf welchem Konto eine Anmeldung landet.** Die Identität wird nach der ersten Anmeldung über
das Subject des Anbieters gemerkt. Bei einer ersten Anmeldung:

- Ein bestehendes Konto wird nur verknüpft, wenn der Anbieter `email_verified: true` zusichert
  und diese E-Mail der Benutzername des Kontos ist. Ein `preferred_username` oder eine
  unverifizierte E-Mail verknüpft nie: Bei den meisten Anbietern wählt der Benutzer diese selbst,
  daher darf ein `admin` vom Anbieter nicht *Ihr* `admin` werden.
- Ein lokaler **Admin mit Passwort wird nie automatisch verknüpft**, selbst bei verifizierter
  E-Mail: Die Anmeldung erstellt stattdessen ein eigenes Konto. Verwalten Sie diesen Admin weiter
  mit seinem Passwort oder erstellen Sie über die Admin-Gruppen einen separaten SSO-Admin.
- Andernfalls wird, wenn *Konten bei der ersten Anmeldung erstellen* an ist, ein neues Konto
  erstellt (Benutzername vom Anbieter, mit `-1`, `-2` … ergänzt, wenn vergeben). Ist es aus, kann
  sich nur über die oben beschriebene Verknüpfung per verifizierter E-Mail angemeldet werden.

**Kontoerstellung ist aus, sofern Sie sie nicht wählen.** Neue Installationen starten mit
*Konten bei der ersten Anmeldung erstellen* aus, ebenso jede Konfiguration, in der es nie gesetzt
wurde — vor der Option gespeicherte Einstellungen oder reine Umgebungs-Setups ohne
`DUMBMONIT_OIDC_AUTO_CREATE`. Frühere Versionen behandelten „nie gesetzt“ als an; das Panel
Single Sign-On sagt jetzt, wann dieser Standard gilt. Ein ausdrücklich gespeicherter Wert bleibt
unverändert.

## Agenten {#agents}

Nur was allen [Agenten](../devices/agent.md) gemeinsam ist, für Admins: der
verschlüsselte Agentenkanal (Zertifikate und ihre Zertifizierungsstelle, siehe
[Agentensicherheit](../install/agent-security.md)), die automatischen
Agenten-Updates und die bereits ausgegebenen Registrierungstokens. Jedes Token
zeigt seinen Geltungsbereich, das Erstellungsdatum und die letzte Nutzung;
**Widerrufen** stoppt jeden Agenten, der es nutzt, bei seiner nächsten
Übertragung, ohne die Monitore zu löschen.

Eine Maschine wird im Hinzufügen-Ablauf hinzugefügt (Token,
Installationsbefehl, erste Verbindung, Weg über ein Relay, Active-Directory-Kit):
**Neuer Monitor → Agent** (`/targets/new?kind=agent`), nicht hier. Die Agenten
selbst haben eigene Seiten.

## Sicherheitsbewertung {#security-score}

In **Einstellungen → Überwachung**, unter den Agenten: eine Bewertung, wie gut die Instanz und die überwachten Hosts abgesichert sind. Siehe [Sicherheitsbewertung](security-score.md).

## Berichte {#reports}

Unter **Einstellungen → Benachrichtigungen & Berichte**, unterhalb der Richtlinie: periodische Verfügbarkeits-E-Mails für das Team. Wählen Sie wöchentlich, täglich oder monatlich,
den Tag, die Uhrzeit und die Zeitzone, die Empfänger und den E-Mail-Kanal, dann **Vorschau
senden**, um das Ergebnis zu sehen. Nur Admins. Siehe [E-Mail-Berichte](reports.md).

## Sicherung {#backup}

Drei Dinge, in der Reihenfolge, in der Sie ihnen begegnen.

Eine **Warnung, die Sie nicht übersehen können**: `/data/secret.key` entschlüsselt die
Monitor-Zugangsdaten, und eine Kopie der Datenbank ohne sie stellt eine Instanz wieder her, die mit
nichts sprechen kann. Der Wortlaut ändert sich, wenn das Geheimnis stattdessen aus
`DUMBMONIT_SECRET` kommt — dann gibt es keine Datei zu kopieren, und Ihr Passwortmanager muss es
halten.

**Die Konfiguration exportieren** listet auf, was das Bundle enthalten würde, mit einer Zahl pro
Abschnitt, und fragt dann zweimal nach einer Passphrase. Die heruntergeladene Datei enthält alle
Zugangsdaten dieser Instanz, allein mit dieser Passphrase verschlüsselt: Bewahren Sie sie dort
auf, wo Sie Passwörter aufbewahren. Kontopasswörter und 2FA-Geheimnisse bleiben außen vor, sofern
Sie den Schalter nicht einschalten.

**Ein Bundle wiederherstellen** nimmt die Datei und ihre Passphrase und zeigt zuerst einen
**Probelauf**: eine Tabelle, was erstellt, aktualisiert und unangetastet bliebe, Abschnitt für
Abschnitt, mit einem Hinweis zu allem, was es nicht einordnen kann. Nichts wird geschrieben, bis
Sie *Wirklich wiederherstellen* drücken.

**Geplante lokale Sicherungen** zeigt, ob sie aktiv sind, wohin sie gehen, den letzten Lauf und
ob er erfolgreich war, und die aufbewahrten Dateien mit ihrer Größe. **Jetzt sichern** schreibt
sofort eine — das, was man vor einem Upgrade tun sollte.

Alles hier ist Admins vorbehalten. Siehe [Sicherung und Wiederherstellung](../install/backup.md).

## DumbMonit selbst {#self-monitoring}

Der aktuelle Zustand der Engine — Planer, SQLite-Datenbank, Metrikdatenbank —
mit dem freien Platz auf dem Datenvolume, der Datenbankgröße und, beim
eingebetteten VictoriaMetrics, der Zahl seiner Neustarts. Eingebaute Regeln
warnen Sie bereits, wenn etwas davon schiefgeht.

Darunter der **ausgehende Heartbeat**: eine URL von Ihnen (Healthchecks.io, ein
Push-Monitor von Uptime Kuma, Better Stack…), die DumbMonit regelmäßig aufruft,
nur solange es gesund ist, damit jemand von außen merkt, wenn dieser Host
verstummt. **Testen** sendet sofort einen Aufruf; **Ausfälle melden** ruft die
Fehler-URL auf, statt zu schweigen. Die URL wird verschlüsselt gespeichert und
nie wieder angezeigt, nur ihr Host. Ändern dürfen nur Administratoren. Siehe
[Watching DumbMonit itself](../install/self-monitoring.md).

## API & Assistenten {#api-assistants}

Eine Art Token für beides: MCP-Clients und die [REST-API](../reference/api.md). Benennen Sie ein
Token, wählen Sie seinen Umfang und erstellen Sie es — es wird einmalig angezeigt. Ein
**Lese**-Token kann nur ansehen; ein **Lese- und Schreib**-Token kann auch etwas ändern — einen
Monitor stummschalten, eine Prüfung ausführen, Monitore, Regeln und Kanäle hinzufügen oder bearbeiten
—, aber nie Konten, Anmeldeeinstellungen oder andere Tokens.

Unter **Verbinden** enthalten drei Tabs die Snippets, mit dem Token befüllt, solange es noch auf
dem Bildschirm steht: *Claude* (der Befehl `claude mcp add` und der Block für
`claude_desktop_config.json`), *ChatGPT* (die URL des MCP-Servers und der Authorization-Header,
mit einer Warnung, wenn diese Seite nicht über HTTPS ausgeliefert wird — ChatGPT verbindet sich
von den Servern von OpenAI aus, daher muss die Adresse aus dem Internet erreichbar sein) und
*Cursor / andere*, dieselbe Form für jeden Client, der Streamable HTTP mit Bearer-Header spricht.
**API-Zugriff** zeigt dasselbe Token an der REST-API als `curl`-Beispiel und verweist auf die
Routen, die ein Prometheus oder Grafana damit abfragt. Siehe [Einen Assistenten
verbinden](assistant.md) und [DumbMonit abfragen](../reference/metrics.md#scraping-dumbmonit).

## Info {#about}

Version des Servers und Zustand seiner beiden Speicher, der Datenbank (Monitoreinrichtung und
Alarmverlauf) und VictoriaMetrics (jede Messung in den Diagrammen), wie von `GET /api/health`
gemeldet und alle 15 Sekunden aktualisiert. Ein Speicher, der nicht mehr antwortet, zeigt eine
Warnung mit dem Fehler und dem, was zu prüfen ist. Der Abschnitt verlinkt außerdem diese
Dokumentation und nennt die Lizenz, MIT.
