# Sicherheitsmodell {#security-model}

Diese Seite beschreibt, was DumbMonit schützt, wem es vertraut, wie das Design den Schaden eines
Fehlers begrenzt und wo Sie in Code und Tests nachsehen können. Eine Schwachstelle melden Sie
gemäß der
[Sicherheitsrichtlinie](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Was auf dem Spiel steht {#what-is-at-stake}

DumbMonit läuft innerhalb Ihres Netzwerks. Es speichert die Zugangsdaten, die es zum Auslesen
jedes Geräts braucht (SNMP-Communities, API-Tokens, Passwörter), empfängt Messwerte von Agents
auf Ihren Rechnern und sendet Anfragen an Adressen, die Sie konfigurieren. Eine Kompromittierung
würde diese Geheimnisse preisgeben und einem Angreifer einen Brückenkopf im überwachten Netzwerk
verschaffen.

## Vertrauensgrenzen {#trust-boundaries}

| Grenze | Wer auf der anderen Seite steht | Vertrauen |
|---|---|---|
| Browser oder Skript zu `/api/*` | Admins, Operatoren und Betrachter | Authentifiziert per Session-Cookie oder API-Token; drei Rollen begrenzen, was jeder darf. |
| Agent zu `/api/ingest` | Rechner, auf denen Sie einen Agent installiert haben | Ein Token pro Rechner; ein geleaktes Token lässt sich einzeln widerrufen. |
| Server zu Geräten und Diensten | Fremdsysteme | Nicht vertrauenswürdig: Antworten werden defensiv geparst, Weiterleitungen bleiben auf demselben Host. |
| Server zu Benachrichtigungskanälen | Externe Dienste | Zugangsdaten sind im Ruhezustand verschlüsselt und werden von der API nie zurückgegeben. |
| Server zu VictoriaMetrics | Ein Kindprozess auf Loopback oder eine externe Instanz | Die eingebettete Instanz hat keine Authentifizierung und lauscht auf Loopback; bei einer externen liegt die Verantwortung beim Betreiber. |
| Volume `/data` und Host | Der Betreiber | Vertrauenswürdig. Ein Angreifer, der den Host oder das Volume kontrolliert, liegt außerhalb des Geltungsbereichs. |

## Angewandte Prinzipien für sicheres Design {#secure-design-principles-applied}

- **Minimale Rechte.** Der Container läuft als unprivilegierter Benutzer ohne Capability, mit
  schreibgeschütztem Root-Dateisystem und `no-new-privileges`. Drei Rollen (Betrachter,
  Operator, Admin) beschränken, was ein Benutzer ändern darf
  (`crates/server/src/auth/users.rs`; Test `operator_role_api.rs`).
- **Sicher von Haus aus.** Auf einer frischen Instanz antworten nur die Routen für Health,
  Setup, Login und Auth-Status, bis der erste Admin existiert (Test
  `api_open_security_api.rs`).
- **Sparsamkeit der Mechanismen.** Ein Binary, eine SQLite-Datei, ein Kindprozess auf Loopback
  und ein einziger Zugang für alle Clients: `/api/*`.
- **Sichere Standardwerte im Fehlerfall.** Eine Sicherheitsprüfung, für die Daten fehlen, ist
  `unknown`, nie bestanden; ein nicht erreichbares Gerät und ein Konfigurationsfehler sind
  verschiedene Zustände.
- **Vollständige Zugriffskontrolle.** Eine einzige Authentifizierungs-Middleware
  (`crates/server/src/auth/middleware.rs`) schützt die API-Routen.
- **Trennung der Privilegien.** Sessions, API-Tokens und Agent-Tokens sind verschiedene
  Zugangsdaten mit unterschiedlicher Reichweite.
- **Verteidigung in der Tiefe.** Verschlüsselung im Ruhezustand zusätzlich zur Zugriffskontrolle,
  ein CSRF-Header zusätzlich zu den Cookie-Flags, ein Rate-Limit zusätzlich zum Passwort-Hashing.
- **Kein Geheimnis auf dem Rückweg.** Die API liefert `has_secret` und `credential_kind`, nie
  das Geheimnis; wird ein Geheimnis bei `PUT` weggelassen, bleibt es erhalten.
- **Nur erprobte Kryptografie.** Passwörter werden mit Argon2 gehasht, Geheimnisse mit
  AES-256-GCM verschlüsselt (`crypto.rs`), die Zwei-Faktor-Authentifizierung nutzt TOTP mit
  gehashten Einmal-Wiederherstellungscodes, OpenID Connect verwendet State, Nonce und PKCE.
  Nichts davon ist Eigenbau.

## Gegenmaßnahmen nach Schwachstellenklasse {#countermeasures-by-class-of-weakness}

| Klasse (OWASP-Top-10-Thema) | Maßnahmen |
|---|---|
| Fehlerhafte Zugriffskontrolle | Eine Authentifizierungs-Middleware; Rollen; eine frische Instanz lehnt alles außer Setup, Login, Health und Auth-Status ab. |
| Kryptografische Fehler | AES-256-GCM im Ruhezustand, Schlüssel außerhalb der Datenbank; Argon2-Passwort-Hashes; der eingebaute Server spricht unverschlüsseltes HTTP, daher ist ein TLS-Reverse-Proxy dokumentiert. |
| Injection | SQL läuft über `sqlx` mit gebundenen Parametern; der VictoriaMetrics-Proxy leitet nur lesende Abfragen weiter (`tsdb/mod.rs`, `proxy_read`); Parser sind per Unit-Test abgedeckt. |
| Unsicheres Design | Das oben beschriebene Bedrohungsmodell; Geheimnisse gehen nie den Rückweg; eine Instanz für genau einen Zweck. |
| Sicherheitsfehlkonfiguration | Gehärtete Compose-Dateien, VictoriaMetrics nur auf Loopback, Härtungshinweise für Betreiber in der Sicherheitsrichtlinie. |
| Anfällige Komponenten | `cargo deny` prüft Lizenzen, Verbote und RustSec-Advisories; wöchentlicher Trivy-Scan der veröffentlichten Images; Dependabot; GitHub Actions per Commit fixiert. |
| Fehler bei Identifizierung und Authentifizierung | Login-Rate-Limit pro Client-Adresse und pro Konto (`auth/rate_limit.rs`), optionale Zwei-Faktor-Authentifizierung, OpenID Connect, ein Admin kann den zweiten Faktor eines Benutzers zurücksetzen und ihn abmelden. |
| Software- und Datenintegrität | Die Agent-Installer prüfen den SHA-256 des heruntergeladenen Binarys; Release-Images werden von der CI gebaut. |
| Protokollierung und Überwachung | Ein Sicherheitsprotokoll der Anmeldungen und sensiblen Änderungen (`auth/audit.rs`). |
| Server-Side Request Forgery | Integrationen und Benachrichtigungskanäle folgen Weiterleitungen nur auf demselben Host, damit ein Gerät eine Anfrage samt API-Schlüssel nicht anderswohin umlenken kann. |
| Cross-Site Request Forgery | Zustandsändernde Anfragen, die sich auf das Cookie stützen, müssen `X-Requested-With: DumbMonit` oder Same-Origin-Fetch-Metadaten tragen, sonst `403`; tokenauthentifizierte Anfragen tragen kein Cookie. |

## Verifizierung {#verification}

Jeder Push und Pull Request führt `clippy -D warnings`, die Rust-Testsuite (einschließlich API-,
Authentifizierungs- und Rollentests unter `crates/server/tests/`), die Web-Tests unter
`web/tests/`, Abhängigkeitsprüfungen und die CodeQL-Analyse aus. Der OpenSSF Scorecard läuft
zeitgesteuert. Sicherheitskorrekturen kommen mit einem Regressionstest und werden mit ihrer
CVE- oder GHSA-Kennung in den Release-Notes aufgeführt.

Der [Assurance Case](assurance-case.md) enthält das vollständige Bedrohungsmodell, das
Argument für sicheres Design und die Belege hinter jeder Aussage.

## Bekannte Grenzen {#known-limits}

- Der eingebaute Server spricht unverschlüsseltes HTTP; schalten Sie TLS davor.
- Geräteoptionen werden unverschlüsselt gespeichert. Legen Sie Geheimnisse in
  Zugangsdatenfeldern ab.
- Das eingebettete VictoriaMetrics hat keine Authentifizierung.
- DumbMonit ist eine Alpha; Routen und Felder können sich noch ändern.
