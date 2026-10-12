# Settings

![Settings: Konto, Benutzer, Single Sign-On, Agents, Assistant, Erscheinungsbild, About](../assets/screenshots/settings-light.png){ loading=lazy }

Settings hält alles Administrative: Ihr Konto, wer sich wie einloggen
kann, die Tokens, die Agents und Assistants einlassen, die Sicherung
der Instanz, das Theme, und die Gesundheit des Servers. Eine Seite,
ein Abschnitt pro Thema, mit einer Leiste aus Anker-Links auf einem
breiten Bildschirm und einem Streifen aus Chips auf einem Telefon.
Jeder Abschnitt ist verlinkbar (`/settings#agents`).

Zwei Dinge lebten früher hier und sind umgezogen, weil sie keine
Administration sind:

| Gesucht | Jetzt unter | Alter Link |
|---|---|---|
| Benachrichtigungskanäle, Ruhezeiten, Benachrichtigungsrichtlinie | **Alerts → Notifications** ([Alerts-Seite](alerts.md#notifications)) | `/settings#notifications`, `/settings#notifications-policy` |
| Statusseiten und Ankündigungen | **Status** ([Statusseiten](status-pages.md)) | `/settings#status` |

Die alten Links funktionieren weiterhin: Sie leiten zum neuen Ort weiter.

## Account & security

Das ist Ihr eigenes Konto. Das erste wird beim ersten Start erstellt,
weitere können unten hinzugefügt werden. Hier ändern Sie Ihr Passwort:
aktuelles Passwort, neues Passwort (mindestens 12 Zeichen; ein ganzer
Satz ist sicherer als ein kompliziertes Wort), Bestätigung. Es zu
ändern loggt jede andere Sitzung aus. Konten, die sich über den
Identity-Provider einloggen, haben hier kein Passwort. **Sign out**
beendet diese Sitzung.

Der Login ist pro Client-Adresse und pro Konto ratenbegrenzt: Nach
fünf fehlgeschlagenen Versuchen wartet jeder weitere Versuch länger
(30 s, verdoppelnd, bis zu 5 Minuten). Setzen Sie hinter einem Reverse
Proxy `DUMBMONIT_TRUSTED_PROXIES`, damit der Limiter die echte
Client-Adresse sieht statt die des Proxys (siehe
[Konfiguration](../reference/configuration.md)). Sitzungen dauern 30
Tage. Ein vergessenes Passwort wird mit
`DUMBMONIT_RESET_PASSWORD=1` zurückgesetzt: siehe die
[FAQ](../faq.md#i-lost-the-password).

### Zwei-Faktor-Authentifizierung

Konten, die sich mit einem Passwort einloggen, können einen zweiten
Faktor hinzufügen: einen sechsstelligen zeitbasierten Code (TOTP)
aus einer Authenticator-App — Aegis, FreeOTP, Google Authenticator,
1Password, Bitwarden… **Set up** fragt nach Ihrem Passwort, zeigt
einen zu scannenden QR-Code (oder den zu tippenden Schlüssel), und
aktiviert den zweiten Faktor, sobald Sie einen ersten Code aus der
App eingeben. Acht **Recovery-Codes** werden dann einmal angezeigt:
Speichern Sie sie, jeder loggt Sie einmal ein, falls das Telefon
verloren geht. Der Login-Bildschirm fragt nach dem Code (oder einem
Recovery-Code) nach dem Passwort; fünf falsche Codes brechen den
Versuch ab, und Sie starten erneut beim Passwort, wobei die übliche
Login-Ratenbegrenzung über die Versuche hinweg weiterzählt. Ein Code
wird einmal akzeptiert: der, den Sie gerade genutzt haben
(einschließlich dem, der den zweiten Faktor aktiviert hat), loggt Sie
nicht erneut ein — warten Sie auf den nächsten.

**Disable** fragt erneut nach dem Passwort. Recovery-Codes können
nicht für sich allein neu erzeugt werden: deaktivieren und erneut
einrichten für einen frischen Satz. Ein Admin kann den zweiten Faktor
eines anderen Benutzers unter **Users** zurücksetzen (*Reset 2FA*),
wenn sowohl das Telefon als auch die Codes weg sind — das loggt den
Benutzer überall aus und lässt das Passwort unberührt. Das Secret wird
mit dem Instanz-Secret verschlüsselt gespeichert; Codes sind gehasht.
Single-Sign-On-Konten erhalten ihren zweiten Faktor vom
Identity-Provider, nicht hier.

Admins finden in diesem Abschnitt auch das **Security Log**: Logins
und Fehlschläge, Passwort- und Zwei-Faktor-Änderungen,
Token- und Kontoänderungen, mit der Client-Adresse. Es behält die
letzten 5.000 Einträge.

## Push notifications

Native Benachrichtigungen auf diesem Telefon oder Computer, auch mit
geschlossenem DumbMonit: **Enable on this device**, dann **Send a
test**. Jedes Konto verwaltet hier seine eigenen Geräte (Viewer
auch); welche Alarme sie erreichen, wird von einem **Web-Push**-Kanal
unter Alerts → Notifications festgelegt. Die **i**-Schaltfläche fasst
die Anforderungen zusammen: HTTPS (oder localhost),
Homescreen-Installation auf iPhone und iPad, und die
Push-Dienst-Domains, die der Server erreichen muss. Vollständige
Anleitung: [Push-Benachrichtigungen (PWA)](push-notifications.md).

## Users

Nur Admins. Die Konten, die sich einloggen können, und ihre Rolle:

| Rolle | Liest alles | Bearbeitet Alarme (Ack, Snooze, Ignore, Wartungsfenster, Historie löschen) | Ändert Konfiguration (Geräte, Regeln, Kanäle, Statusseiten, Einstellungen, Backups, Tokens, Konten) |
|---|---|---|---|
| **Viewer** | ja | nein | nein |
| **Operator** | ja | ja | nein |
| **Admin** | ja | ja | ja |

Ein Viewer sieht jede Seite mit jeder Steuerung, die etwas verborgen
ändern würde. Ein Operator ist die Bereitschafts-Rolle: Die
Alarm-Steuerungen sind da, die Konfigurationsbildschirme lesen sich
*Operator — config is admin only*. Nur ein Admin erstellt Konten oder
ändert eine Rolle — ein Operator kann niemanden befördern, sich
selbst eingeschlossen. Die Abschnitte *Users*, *Single sign-on* und
das Security Log bleiben nur für Admins. **Add user** fragt nach
einem Benutzernamen, einem optionalen Anzeigenamen, einer Rolle und
einem Passwort; die Schaltfläche neben dem Passwortfeld erzeugt ein
zufälliges. Das Passwort wird einmal angezeigt, zur Weitergabe.

Jede Zeile trägt die Rolle, ob das Konto sich mit einem Passwort oder
über den Identity-Provider einloggt, ob Zwei-Faktor an ist, und den
letzten Login. **Edit** ändert den Anzeigenamen, die Rolle und das
Passwort (ein Reset loggt diesen Benutzer überall aus und zeigt das
neue Passwort einmal). Der Schalter **Enabled** suspendiert ein Konto
ohne es zu löschen; **Reset 2FA** erscheint, wenn das Konto einen
zweiten Faktor hat; **Delete** entfernt es endgültig. Der Server
behält mindestens einen aktiven Admin, und die Steuerungen spiegeln
diese Regel wider, statt sie als Fehler zu melden: Der letzte Admin
kann nicht degradiert, deaktiviert oder gelöscht werden, und Sie
können Ihr eigenes Konto nicht deaktivieren oder löschen.

## Single sign-on

Nur Admins. Login über einen OpenID-Connect-Provider (Authelia,
Authentik, Keycloak, Pocket ID, Google Workspace…). Füllen Sie den
*Provider-Namen* aus (er beschriftet die "Continue with …"-
Schaltfläche am Login-Bildschirm), die *Issuer-URL* — Discovery wird
von `{issuer}/.well-known/openid-configuration` gelesen — die
*Client-ID*, das *Client-Secret* (verschlüsselt gespeichert, nie
wieder angezeigt) und die *öffentliche URL*, über die Browser
DumbMonit erreichen. Das Formular baut die Redirect-URI aus dieser
öffentlichen URL: Kopieren Sie sie und registrieren Sie sie bei Ihrem
Provider.

**Test discovery** liest die Konfiguration des Providers, ohne
irgendjemanden einzuloggen, listet die gefundenen Endpunkte, und warnt,
wenn der Provider weder mit RS256 noch mit ES256 signiert — den
einzigen zwei Signaturen, die DumbMonit akzeptiert. Hier gespeicherte
Einstellungen haben Vorrang vor den Umgebungsvariablen
`DUMBMONIT_OIDC_*`, die nur gelten, solange nichts gespeichert ist;
**Forget saved settings** löscht sie und fällt auf die Variablen
zurück, falls vorhanden.

**Advanced.** Der Rest des Formulars sitzt unter *Advanced*: *Scopes*
benennt, wonach beim Provider gefragt wird, *Groups claim* den
Token-Claim, der die Gruppen des Benutzers listet, und *Create
accounts on first sign-in* entscheidet, ob eine unbekannte Person ein
Konto erhält. Es ist **standardmäßig aus**: Nur bereits existierende
Konten können sich einloggen, bis Sie es einschalten.

**Roles.** *Admin groups* listet die Gruppen, deren Mitglieder zu
Admins werden; *Operator groups* die Gruppen, deren andere Mitglieder
zu Operatoren werden; alle anderen sind Viewer. Lassen Sie beide leer,
um die Rollen weiter unter **Users** zu verwalten. Mit nur
ausgefülltem *Operator groups* berühren die Gruppen nie die
Admin-Rolle: Admins bleiben Admins, die anderen werden Operatoren
oder Viewer. Rollen werden bei jedem Login neu bewertet, außer dass
der letzte aktive Admin nie degradiert wird. Die passenden
Umgebungsvariablen sind `DUMBMONIT_OIDC_ADMIN_GROUPS` und
`DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Auf welchem Konto ein Login landet.** Die Identität wird nach dem
ersten Login anhand ihres Provider-Subjects gemerkt. Bei einem ersten
Login:

- Ein existierendes Konto wird nur verknüpft, wenn der Provider
  `email_verified: true` behauptet und diese E-Mail der Benutzername
  des Kontos ist. Ein `preferred_username` oder eine nicht
  verifizierte E-Mail verknüpft nie: Bei den meisten Providern wählt
  das der Benutzer, daher darf das `admin` vom Provider nicht zu
  *Ihrem* `admin` werden.
- Ein lokaler **Admin mit Passwort wird nie automatisch verknüpft**,
  selbst bei verifizierter E-Mail: Der Login erstellt stattdessen ein
  eigenständiges Konto. Verwalten Sie diesen Admin weiter mit seinem
  Passwort, oder erstellen Sie einen separaten SSO-Admin über die
  Admin-Gruppen.
- Sonst, mit eingeschaltetem *Create accounts on first sign-in*, wird
  ein neues Konto erstellt (Benutzername vom Provider, mit `-1`,
  `-2`… ergänzt, falls vergeben). Mit ausgeschaltetem kann sich nur
  die oben beschriebene verifizierte E-Mail-Verknüpfung einloggen.

**Kontoerstellung ist aus, sofern Sie sie nicht wählen.** Neue
Installationen starten mit ausgeschaltetem *Create accounts on first
sign-in*, ebenso jede Konfiguration, in der es nie gesetzt wurde —
vor Existenz der Option gespeicherte Einstellungen, oder nur auf
Umgebungsvariablen beruhende Setups ohne `DUMBMONIT_OIDC_AUTO_CREATE`.
Frühere Releases behandelten "nie gesetzt" als an; das
Single-Sign-On-Panel sagt jetzt, wann dieser Standard gilt. Ein von
Ihnen explizit gespeicherter Wert wird so behalten, wie er ist.

## Agents

Enrollment-Tokens für den
[Linux-, macOS-, FreeBSD- und Windows-Agent](../devices/agent.md).
Erstellen Sie eines mit einem Namen ("File server", "Home fleet"):
Das Token wird einmal angezeigt, mit den fertigen Installationsbefehlen
für Linux und Windows. Jeder Befehl trägt die SHA-256-Prüfsummen der
von diesem Server gelieferten Agent-Binaries, und der Installer lehnt
einen nicht passenden Download ab; die Prüfsummen stehen außerdem
unter den Befehlen, zum händischen Vergleich. Die Liste zeigt das
Präfix, Erstellungsdatum, letzte Nutzung und ob jedes Token
widerrufen wurde. **Revoke** stoppt jeden damit laufenden Agent bei
seinem nächsten Push.

Ein Token kann mehrere Maschinen anmelden. Es zu widerrufen löscht
die Geräte nicht. Viewer sehen die Liste, und der Abschnitt liest sich
*Viewer — read only*; Erstellen und Widerrufen sind für Admins.

## API & assistants

Eine Art Token für beides: MCP-Clients und die
[REST-API](../reference/api.md). Benennen Sie ein Token, wählen Sie
seinen Scope und erstellen Sie es — es wird einmal angezeigt. Ein
**read**-Token kann nur nachsehen; ein **read and write**-Token kann
auch Dinge ändern — ein Gerät stummschalten, eine Abfrage ausführen,
Geräte, Regeln und Kanäle hinzufügen oder bearbeiten — aber nie
Konten, Login-Einstellungen oder andere Tokens.

Unter **Connect** halten drei Tabs die Snippets, mit dem Token
gefüllt, während es noch auf dem Bildschirm steht: *Claude* (der
Befehl `claude mcp add` und der
`claude_desktop_config.json`-Block), *ChatGPT* (die
MCP-Server-URL und der Autorisierungs-Header, mit einer Warnung,
wenn diese Seite nicht über HTTPS bedient wird — ChatGPT verbindet
sich von OpenAIs Servern aus, daher muss die Adresse aus dem
Internet erreichbar sein) und *Cursor / other*, dieselbe Form für
jeden Client, der Streamable HTTP mit einem Bearer-Header spricht.
**API access** zeigt dasselbe Token auf der REST-API als
`curl`-Beispiel, und verweist auf die Routen, die ein Prometheus
oder ein Grafana damit abfragt. Siehe
[Einen Assistant verbinden](assistant.md) und
[DumbMonit abfragen](../reference/metrics.md#scraping-dumbmonit).

## Backup

Drei Dinge, in der Reihenfolge, in der Sie ihnen begegnen.

Eine **nicht zu übersehende Warnung**: `/data/secret.key` ist, was
Gerätezugangsdaten entschlüsselt, und eine Kopie der Datenbank ohne
sie stellt eine Instanz wieder her, die mit nichts sprechen kann. Der
Wortlaut ändert sich, wenn das Secret stattdessen aus
`DUMBMONIT_SECRET` kommt — es gibt dann keine Datei zum Kopieren, und
Ihr Passwort-Manager muss es halten.

**Export the configuration** listet, was das Bündel enthalten würde,
mit einer Zahl pro Abschnitt, und fragt dann zweimal nach einer
Passphrase. Die herunterkommende Datei hält jedes Credential dieser
Instanz, allein mit dieser Passphrase verschlüsselt: Bewahren Sie sie
auf, wo Sie Passwörter aufbewahren. Kontopasswörter und 2FA-Secrets
werden ausgelassen, sofern Sie den Schalter nicht einschalten.

**Restore a bundle** nimmt die Datei und ihre Passphrase und zeigt
zuerst einen **Dry Run**: eine Tabelle, was erstellt, aktualisiert
und unberührt gelassen würde, Abschnitt für Abschnitt, mit einer
Notiz für alles, was nicht platziert werden kann. Nichts wird
geschrieben, bis Sie *Restore for real* drücken.

**Scheduled local backups** zeigt, ob sie an sind, wohin sie gehen,
den letzten Lauf und ob er gelang, und die behaltenen Dateien mit
ihrer Größe. **Back up now** schreibt sofort eines — das, was vor
einem Upgrade zu tun ist.

Alles hier ist nur für Admins. Siehe
[Backup und Wiederherstellung](../install/backup.md).

## Reports

Periodische Verfügbarkeits-E-Mails für das Team: wöchentlich, täglich
oder monatlich wählen, den Tag, die Stunde und Zeitzone, die
Empfänger und den E-Mail-Kanal, dann **Send a preview**, um das
Ergebnis zu sehen. Nur für Admins. Siehe
[E-Mail-Berichte](reports.md).

## Appearance

Drei Kacheln, jede mit Vorschau ihres eigenen Themes: **System**
(folgt Ihrem Gerät und wechselt damit), **Day** (das
"Diagrammpapier"-Theme) und **Night** (das "Radar"-Theme). Die Wahl
wird im Browser gespeichert. Der Umschalter im Header und *Toggle
theme* der Befehlspalette wechseln zwischen Tag und Nacht.

**Open wall mode** sitzt am Ende des Abschnitts: nur das Bulletin,
im Vollbild, für einen Monitor im Raum. ++esc++ verlässt es, und die
Befehlspalette öffnet es von überall — siehe
[Wall-Modus](overview.md#wall-mode).

## About

Version des Servers und die Gesundheit seiner zwei Speicher, der
Datenbank (Geräte-Setup und Alarmhistorie) und VictoriaMetrics (jede
Messung in den Diagrammen), wie von `GET /api/health` gemeldet und
alle 15 Sekunden aktualisiert. Ein Speicher, der aufhört zu
antworten, zeigt eine Warnung mit dem Fehler und was zu prüfen ist.
Der Abschnitt verlinkt auch auf diese Dokumentation und benennt die
Lizenz, Apache 2.0.
