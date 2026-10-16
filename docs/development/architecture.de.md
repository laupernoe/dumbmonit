# Architektur {#architecture}

DumbMonit wird als ein einziger Container ausgeliefert. Darin läuft ein Rust-Prozess, der die
Web-Oberfläche und die API bereitstellt, sowie ein Kindprozess (VictoriaMetrics), den der
Rust-Prozess startet und überwacht. Konfiguration und Zustand liegen in SQLite.

## Komponenten {#components}

| Komponente | Ort | Aufgabe |
|---|---|---|
| Server | `crates/server` | Das einzige Rust-Binary: HTTP-API (axum), Authentifizierung, Scheduler, Alarmierungs-Engine, Benachrichtigungskanäle, Berichte. |
| Web-Oberfläche | `web/` | SvelteKit-Single-Page-App (Svelte 5, Tailwind 4, uPlot). Wird zu statischen Dateien gebaut und mit `rust_embed` in das Server-Binary eingebettet. |
| Zeitreihen | `crates/server/src/tsdb/` | Writer mit Batching und Query-Proxy für VictoriaMetrics. `embedded.rs` startet und überwacht den mitgelieferten VictoriaMetrics-Kindprozess (Daten in `/data/vm`); `DUMBMONIT_VM_URL` verweist stattdessen auf eine externe Instanz. |
| Konfiguration und Zustand | `crates/server/src/db/` | SQLite mit nummerierten Migrationen: Monitore, Zugangsdaten, Benutzer, Regeln, Alarmzustand, Stummschaltungen, Kanäle. |
| Collectors | `crates/collectors`, `crates/server/src/collectors/` | Eine Implementierung des Traits `Collector` pro Monitor- oder Diensttyp, in einer `Registry` registriert. Der Scheduler kennt nur den Trait. |
| Gemeinsame Typen | `crates/proto` | `Sample`, `Target`, `Credential`, der Trait `Collector` und `ProbeError` sowie das Agent-Protokoll. |
| Agent | `crates/agent` | Agent für Linux, macOS, FreeBSD und Windows, der Host-Metriken sendet; er kann auch als Relay arbeiten und Collectors für einen entfernten Standort ausführen. |
| Alarmierung | `crates/server/src/alerting/` | Regeln, Alarm-Zustandsautomat, Unterdrückung durch übergeordnete Monitore, Stummschaltungen, gelernte Baselines. |
| Benachrichtigungen | `crates/server/src/notify/` | Kanäle (Chat, Push, Bereitschaft, E-Mail, Webhooks), der Oberfläche über `catalog.rs` beschrieben. |
| Geheimnisse | `crates/server/src/crypto.rs` | AES-256-GCM für in SQLite gespeicherte Zugangsdaten und Tokens; der Schlüssel stammt aus `/data/secret.key` oder `DUMBMONIT_SECRET`. |

## Datenfluss {#data-flow}

```text
 devices / services ──(SNMP, HTTP APIs, probes)──► collectors ┐
                                                              │ samples
 agents ──(push, per-machine token)──► /api/ingest ───────────┤
 relay agent (collectors run at a remote site) ───────────────┤
                                                              ▼
                                           tsdb writer (batched) ──► VictoriaMetrics
                                                                           │
 browser / scripts / assistants ◄─ /api/* ◄─ query proxy ◄─────────────────┘
                                   │
                                   └─► alerting engine ──► notify channels

 SQLite (config, users, alert state, silences) is read and written by the
 API, the scheduler and the alerting engine.
```

1. Der Scheduler führt jeden aktivierten Monitor in seinem Intervall über die Collector-Registry
   aus. Agents und Relays senden ihre Messwerte mit einem Token pro Rechner an `/api/ingest`.
2. Collectors liefern Samples mit dem Namen `dumbmonit_<kind>_*` und einem Label `target`
   (die Monitor-ID). Der Writer schreibt sie gebündelt nach VictoriaMetrics.
3. Die Alarmierungs-Engine wertet Regeln gegen die gespeicherten Serien aus. `phase` ist der
   Zustand der Engine; `effective_phase` ist das, was Benutzer sehen (`suppressed`, wenn ein
   übergeordneter Monitor ausgefallen ist). Ein nicht erreichbarer Monitor löst einen Alarm aus;
   ein Konfigurationsfehler wird angezeigt, aber nicht gemeldet.
4. Benachrichtigungen verlassen das System über die in der Benachrichtigungsrichtlinie
   konfigurierten Kanäle.
5. Die Oberfläche und jedes Skript lesen alles über `/api/*`; es gibt keine private Hälfte der
   API.

## Erweiterungspunkte {#extension-points}

- Ein neuer Monitortyp ist eine `Collector`-Implementierung plus ein Aufruf von
  `Registry::register`. Die Formulare der Oberfläche werden aus `GET /api/collectors` erzeugt.
- Ein neuer Benachrichtigungskanal ist ein Sender plus ein Eintrag in `notify/catalog.rs`; die
  Oberfläche rendert das Formular aus `GET /api/notify/kinds`.
- Ein SNMP-Monitor, der nur ein Profil braucht, benötigt kein Rust: eine YAML-Datei unter
  `profiles/`.

Die Bauanleitung finden Sie unter [Entwicklung](../development.md), die Vertrauensgrenzen unter
[Sicherheitsmodell](security-model.md).
