# Assurance Case {#assurance-case}

Diese Seite argumentiert, dass DumbMonit seine Sicherheitsanforderungen erfüllt: Sie nennt die
Bedrohungen, die Grenzen, an denen sich das Vertrauen ändert, warum das Design den Schaden eines
Fehlers begrenzt, wie jede verbreitete Schwachstellenklasse abgewehrt wird und welche Belege jede
Aussage stützen. Sie erweitert das [Sicherheitsmodell](security-model.md), das dieselben
Maßnahmen knapper aufführt. Eine Schwachstelle melden Sie gemäß der
[Sicherheitsrichtlinie](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

Dateipfade unten sind relativ zum Wurzelverzeichnis des Repositorys.

## Sicherheitsanforderungen {#security-requirements}

1. Nur ein authentifizierter Benutzer mit einer dafür berechtigten Rolle darf die Konfiguration,
   die Alarme oder die Messwerte lesen oder ändern.
2. Die Zugangsdaten, die DumbMonit für Ihre Monitore speichert, sind weder aus der Datenbankdatei
   noch aus einem Backup-Bundle ohne dessen Passphrase noch über die API lesbar.
3. Nur ein Rechner mit einem gültigen Token darf Messwerte einspeisen, und ein geleaktes Token
   lässt sich einzeln widerrufen.
4. DumbMonit kann nicht dazu missbraucht werden, Dienste zu erreichen, die es nicht erreichen
   soll, insbesondere seinen eigenen VictoriaMetrics-Kindprozess ohne Authentifizierung.
5. Eine an einen Browser ausgelieferte Seite kann kein Skript ausführen, das nicht mit ihr
   ausgeliefert wurde, und eine fremde Website kann einen angemeldeten Browser nichts ändern
   lassen.

## Bedrohungsmodell {#threat-model}

| Angreifer | Fähigkeit | Im Geltungsbereich |
|---|---|---|
| Anonymer Netzwerk-Client | Erreicht den HTTP-Port | Ja |
| Angemeldeter Benutzer mit niedriger Rolle | Gültige Session oder schreibgeschütztes API-Token | Ja (Rechteausweitung) |
| Bösartige Webseite | Läuft im Browser eines Admins | Ja (CSRF, XSS, Clickjacking) |
| Kompromittiertes oder bösartiges überwachter Monitor | Beantwortet Probes mit feindlichem Inhalt | Ja (Parser, Weiterleitungen) |
| Bösartiger oder gestohlener Agent | Besitzt ein Agent-Token | Ja (Reichweite eines einzelnen Tokens) |
| Admin, der eine feindliche URL einträgt | Konfiguriert einen Webhook oder eine Monitoradresse | Ja, für die eigenen Adressen des Servers |
| Jemand mit Zugriff auf den Host, das Volume `/data` oder die Container-Runtime | Volle Kontrolle | Nein: außerhalb des Geltungsbereichs, wie in der Sicherheitsrichtlinie |
| Schwachstellen in VictoriaMetrics oder in den überwachten Monitoren | Upstream-Code | Nein: beim Upstream melden |

## Vertrauensgrenzen {#trust-boundaries}

| Grenze | Was sie überquert | Wie sie geschützt ist | Beleg |
|---|---|---|---|
| Browser zu `/api/*` | Session-Cookie oder API-Token, JSON | Eine Authentifizierungs-Middleware, drei Rollen, CSRF-Header bei cookie-authentifizierten Schreibzugriffen | `crates/server/src/auth/middleware.rs`; Tests `auth_api.rs`, `operator_role_api.rs`, `api_open_security_api.rs`, `cors_policy_api.rs` |
| Browser zu ausgelieferten Seiten | HTML, Skript, Stil | Content Security Policy mit Nonce pro Antwort, `nosniff`, `Referrer-Policy: same-origin`, `X-Frame-Options: DENY` | `crates/server/src/api/mod.rs` (Security-Header-Layer) |
| Identity-Provider zu Server | OpenID-Connect-Redirect und ID-Token | State, Nonce und PKCE (S256); die Verknüpfung mit lokalen Konten ist explizit | `crates/server/src/auth/oidc/`; Test `oidc_api.rs` |
| Agent zu `/api/ingest` | Messwerte, Befehlsergebnisse | Token pro Agent, als SHA-256-Hash gespeichert (Präfix `dmon_`) | `crates/server/src/api/ingest.rs`, `crates/server/src/collectors/agent/token.rs`; Test `agent_binding_api.rs` |
| Relay-Agent zu Server | Probe-Ergebnisse aus einem anderen Netzwerk | Dieselbe Agent-Token-Prüfung (`agent_commands::authenticate`) | `crates/server/src/api/relay.rs`; Test `relay_api.rs` |
| Server zu überwachten Monitoren | Probe-Anfragen, feindliche Antworten | Weiterleitungen nur auf demselben Host, begrenzte Anzahl Weiterleitungen, Antwort-Parser mit Unit-Tests, Zertifikatsprüfung standardmäßig aktiv | `crates/collectors/src/http.rs` (`same_host_redirects`) |
| Server zu Benachrichtigungskanälen | Webhook- und SMTP-Anfragen | Dieselbe Weiterleitungsrichtlinie plus eine Schutzprüfung, die die eigenen Adressen des Servers ablehnt, vor dem Senden, bei jeder Weiterleitung und bei der DNS-Auflösung | `crates/server/src/notify/guard.rs`; Test `notify_guard_api.rs` |
| Server zu VictoriaMetrics | Schreib- und Leseabfragen | Der eingebettete Kindprozess lauscht standardmäßig auf `127.0.0.1:8428`; der Prometheus-kompatible Proxy leitet nur die Lese-Routen weiter (query, query_range, series, labels) | `crates/server/src/config.rs` (`DUMBMONIT_VM_LISTEN`), `crates/server/src/api/prometheus.rs` (`readable_segments`) |
| Server zu SQLite | Konfiguration und Zustand | Gebundene Parameter über `sqlx` | `crates/server/src/db/` |
| Server zu `/data` | Datenbank, geheimer Schlüssel | Geheimnisse mit AES-256-GCM verschlüsselt; der Container-Benutzer besitzt nur `/data` | `crates/server/src/crypto.rs`, `Dockerfile` |

## Argument für sicheres Design {#secure-design-argument}

Das Argument folgt den Entwurfsprinzipien von Saltzer und Schroeder.

- **Sparsamkeit der Mechanismen.** Ein Binary, eine SQLite-Datei, ein Kindprozess auf Loopback,
  ein Eingang für alle Clients (`/api/*`).
- **Sichere Standardwerte im Fehlerfall.** Auf einer frischen Instanz antwortet jede Route unter
  der Authentifizierung mit `401`; nur Health, Auth-Status, Setup und Login antworten, sodass
  niemand die Instanz vor ihrem Besitzer vorbereiten kann (`auth/middleware.rs`, Test
  `api_open_security_api.rs`). VictoriaMetrics lauscht auf Loopback, sofern der Betreiber das
  nicht ändert. TLS-Zertifikate überwachter Dienste werden geprüft, sofern der Betreiber nicht
  `insecure_tls` setzt (`crates/collectors/src/http.rs`).
- **Vollständige Zugriffskontrolle.** Die Authentifizierungs-Middleware läuft auf jeder
  API-Route, statt dass jeder Handler selbst prüft.
- **Offenes Design.** Der Code, die API-Beschreibung (`crates/server/src/api/openapi.yaml`) und
  diese Seite sind öffentlich. Die Sicherheit beruht nicht auf der Geheimhaltung des Designs.
- **Trennung der Privilegien.** Sessions, API-Tokens und Agent-Tokens sind verschiedene
  Zugangsdaten. API-Tokens tragen einen Scope `read` oder `write` (`auth/token.rs`), und
  Benutzer haben die Rolle Betrachter, Operator oder Admin.
- **Minimale Rechte.** Das Image läuft als UID 65532 (`Dockerfile`); die Compose-Datei entfernt
  alle Capabilities, setzt ein schreibgeschütztes Root-Dateisystem und `no-new-privileges`
  (`docker-compose.yml`).
- **Minimale gemeinsame Mechanismen.** Agents authentifizieren sich mit ihrem eigenen Token;
  den Zugang eines Rechners zu widerrufen berührt also weder einen Benutzer noch andere Agents.
- **Psychologische Akzeptanz.** Sicheres Verhalten ist der Standard und braucht keinen
  zusätzlichen Schritt: Das Cookie ist `HttpOnly` und `SameSite=Lax`, der CSRF-Header wird vom
  Web-Client hinzugefügt, Geheimnisse sind in der Oberfläche nur schreibbar.
- **Verteidigung in der Tiefe.** Verschlüsselung im Ruhezustand zusätzlich zur Zugriffskontrolle;
  der CSRF-Header zusätzlich zu `SameSite`; das Login-Rate-Limit zusätzlich zum Argon2-Hashing;
  eine Weiterleitungsrichtlinie zusätzlich zur Schutzprüfung für eigene Adressen.

### Kryptografie {#cryptography}

- Passwörter: Argon2 (`auth/password.rs`). Geheimnisse von Sessions und API-Tokens werden als
  SHA-256-Hashes zufälliger Werte gespeichert (`auth/session.rs`, `auth/token.rs`).
- Geheimnisse im Ruhezustand: AES-256-GCM mit frischer Nonce pro Verschlüsselung (`crypto.rs`).
  Der Schlüssel wird mit Argon2id aus `DUMBMONIT_SECRET` oder der erzeugten Schlüsseldatei
  abgeleitet, mit persistentem Salt; ein Geheimnis unter 16 Zeichen wird abgelehnt, und ein
  Canary-Wert erkennt beim Start einen falschen Schlüssel.
- Backups: Der Schlüssel wird mit Argon2id aus der Passphrase des Betreibers abgeleitet; die
  Parameter stehen im Bundle, sodass sich der Algorithmus ändern kann, ohne alte Bundles zu
  brechen (`backup/bundle.rs`).
- Zweiter Faktor: TOTP, mit gehashten Einmal-Wiederherstellungscodes (`auth/totp.rs`).
- OpenID Connect: State, Nonce und PKCE `S256` (`auth/oidc/pkce.rs`).
- Alle Primitive stammen aus gepflegten Bibliotheken (`aes-gcm`, `argon2`, `sha2`); nichts ist
  Eigenbau. Der eingebaute Listener spricht unverschlüsseltes HTTP; die Dokumentation verlangt
  einen TLS-Reverse-Proxy, und Cookies erhalten `Secure`, wenn die Anfrage über HTTPS eintrifft
  (`auth/cookie.rs`).

## Gegenmaßnahmen zu verbreiteten Schwachstellen {#countermeasures-to-common-weaknesses}

| Schwachstelle (OWASP Top 10, CWE) | Gegenmaßnahme | Beleg |
|---|---|---|
| Fehlerhafte Zugriffskontrolle, CWE-862, CWE-285 | Authentifizierungs-Middleware, Rollen, Token-Scopes | `operator_role_api.rs`, `api_tokens_api.rs`, `api_open_security_api.rs` |
| Kryptografische Fehler, CWE-327, CWE-312 | AES-256-GCM im Ruhezustand, Argon2, kein Geheimnis in API-Antworten | Unit-Tests in `crypto.rs`; `has_secret` / `credential_kind` statt des Geheimnisses |
| SQL-Injection, CWE-89 | Gebundene Parameter über `sqlx` | `crates/server/src/db/` |
| Cross-Site-Scripting, CWE-79 | Svelte maskiert die Ausgabe; CSP mit `default-src 'none'`, Skripte nur von `'self'` und mit Nonce; `base-uri 'none'` | `api/mod.rs` |
| Cross-Site Request Forgery, CWE-352 | Schreibzugriffe per Cookie brauchen `X-Requested-With: DumbMonit` oder Same-Origin-Fetch-Metadaten; CORS gewährt niemandem den Header; `SameSite=Lax` | `auth/middleware.rs`, `auth/cors.rs`; Test `cors_policy_api.rs` |
| Clickjacking, CWE-1021 | `X-Frame-Options: DENY` und `frame-ancestors 'none'`, außer bei öffentlichen Statusseiten (Opt-in), die zum Einbetten gedacht sind | `api/mod.rs` |
| Server-Side Request Forgery, CWE-918 | Nur Weiterleitungen auf demselben Host; die Benachrichtigungs-Schutzprüfung lehnt die eigenen Listen- und VictoriaMetrics-Adressen des Servers ab, auch nach der DNS-Auflösung (Rebinding). Loopback und private Bereiche bleiben absichtlich erreichbar, weil Homelab-Dienste dort liegen | `notify/guard.rs`; Test `notify_guard_api.rs` |
| Authentifizierungsfehler, CWE-307, CWE-287 | Login-Rate-Limit mit wachsender Verzögerung pro Adresse und pro Konto, optionales TOTP, OIDC mit PKCE, Zurücksetzen des zweiten Faktors durch einen Admin | `auth/rate_limit.rs`, `auth/totp.rs`, `auth/oidc/` |
| Unsichere Deserialisierung, Ressourcenerschöpfung, CWE-400 | Typisiertes JSON über `serde`, Request-Bodys auf 16 MiB begrenzt, begrenzte Weiterleitungen | `api/mod.rs`, `collectors/src/http.rs` |
| Speichersicherheit, CWE-787, CWE-416 | Rust; `unsafe` kommt nur im Code für Performance-Zähler des Windows-Agents vor (`crates/agent/src/collect/perf_counters.rs`) | grep nach `unsafe` |
| Anfällige Komponenten, CWE-1104 | `cargo deny` (Lizenzen, Verbote, Advisories); Dependabot; Trivy-Scan der veröffentlichten Images; Actions per Commit fixiert; Builds mit `--locked` | `.github/workflows/ci.yml`, `security.yml`, `.github/dependabot.yml`, `deny.toml` |
| Software- und Datenintegrität, CWE-494 | Die Agent-Installer prüfen den SHA-256 des heruntergeladenen Binarys | `crates/agent/install/install.sh`, `install.ps1` |
| Mängel bei Protokollierung und Überwachung, CWE-778 | Sicherheitsprotokoll der Anmeldungen und sensiblen Änderungen | `auth/audit.rs`; Test `security_audit_api.rs` |

Bekannte Restrisiken stehen im Sicherheitsmodell (HTTP-Listener ohne Verschlüsselung,
unverschlüsselte Monitoroptionen, eingebettetes VictoriaMetrics ohne Authentifizierung). Die CSP
erlaubt Inline-Stile (`style-src 'unsafe-inline'`), was sie gegen Style-Injection schwächt, nicht
aber gegen die Ausführung von Skripten. Es gibt ein akzeptiertes Advisory, `RUSTSEC-2023-0071`,
das in `deny.toml` mit Begründung ignoriert wird: Die Crate `rsa` prüft RS256-Signaturen mit dem
öffentlichen Schlüssel des Providers und verwendet nie einen privaten Schlüssel.

## Belege {#evidence}

- **Kontinuierliche Integration** (`.github/workflows/ci.yml`): `cargo fmt --check`,
  `clippy -D warnings`, die vollständige Rust-Testsuite, `cargo deny check licenses bans` und
  `cargo deny check advisories`, Typprüfung, Tests und Build des Webs sowie ein Build des
  Docker-Images, bei jedem Push und Pull Request.
- **Geplante Scans** (`.github/workflows/security.yml`): Trivy auf den veröffentlichten Server-
  und Agent-Images sowie der OpenSSF Scorecard, wöchentlich und nach jeder
  `:edge`-Veröffentlichung. Die Ergebnisse erscheinen im Security-Tab des Repositorys, wo auch
  das GitHub-Code-Scanning (CodeQL) berichtet.
- **Tests**: API-, Authentifizierungs-, Rollen-, CORS-, OIDC-, Guard- und Audit-Tests unter
  `crates/server/tests/`, Unit-Tests neben dem Code und Web-Tests unter `web/tests/`. Die mit
  `cargo-llvm-cov` gemessene Rust-Zeilenabdeckung beträgt 87,9 %.
- **Regressionsrichtlinie**: Eine Korrektur kommt mit einem Test, der ohne sie fehlschlägt
  (siehe die Testrichtlinie in `CONTRIBUTING.md`).
- **Öffentliches Protokoll**: Sicherheitskorrekturen werden als Advisories veröffentlicht und in
  den Release-Notes aufgeführt.
