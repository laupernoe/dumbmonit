# Arquitectura

DumbMonit se entrega como un único contenedor. Dentro hay un proceso Rust que
sirve la interfaz web y la API, más un proceso hijo (VictoriaMetrics) que el
proceso Rust inicia y supervisa. La configuración y el estado viven en
SQLite.

## Componentes

| Componente | Dónde | Función |
|---|---|---|
| Servidor | `crates/server` | El único binario Rust: API HTTP (axum), autenticación, planificador, motor de alertas, canales de notificación, informes. |
| Interfaz web | `web/` | Aplicación de una sola página SvelteKit (Svelte 5, Tailwind 4, uPlot). Se compila a archivos estáticos y se incrusta en el binario del servidor con `rust_embed`. |
| Series temporales | `crates/server/src/tsdb/` | Escritor por lotes y proxy de consultas para VictoriaMetrics. `embedded.rs` inicia y supervisa el proceso hijo de VictoriaMetrics incluido (datos en `/data/vm`); `DUMBMONIT_VM_URL` apunta en su lugar a una instancia externa. |
| Configuración y estado | `crates/server/src/db/` | SQLite con migraciones numeradas: monitores, credenciales, usuarios, reglas, estado de alertas, silencios, canales. |
| Recolectores | `crates/collectors`, `crates/server/src/collectors/` | Una implementación del trait `Collector` por tipo de monitor o servicio, registrada en un `Registry`. El planificador solo conoce el trait. |
| Tipos compartidos | `crates/proto` | `Sample`, `Target`, `Credential`, el trait `Collector` y `ProbeError`, y el protocolo del agente. |
| Agente | `crates/agent` | Agente para Linux, macOS, FreeBSD y Windows que envía métricas del equipo; también puede actuar como relé que ejecuta recolectores para un sitio remoto. |
| Alertas | `crates/server/src/alerting/` | Reglas, máquina de estados de las alertas, supresión por padre, silencios, referencias aprendidas. |
| Notificaciones | `crates/server/src/notify/` | Canales (chat, push, guardia, correo, webhooks), descritos a la interfaz por `catalog.rs`. |
| Secretos | `crates/server/src/crypto.rs` | AES-256-GCM para las credenciales y los tokens guardados en SQLite; la clave procede de `/data/secret.key` o de `DUMBMONIT_SECRET`. |

## Flujo de datos

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

1. El planificador ejecuta cada monitor activado en su intervalo a través
   del registro de recolectores. Los agentes y los relés envían sus mediciones
   a `/api/ingest` con un token por máquina.
2. Los recolectores devuelven muestras llamadas `dumbmonit_<kind>_*` con una
   etiqueta `target` (el id del monitor). El escritor las agrupa en
   lotes hacia VictoriaMetrics.
3. El motor de alertas evalúa las reglas sobre las series almacenadas. `phase`
   es el estado del motor; `effective_phase` es lo que ven los usuarios
   (`suppressed` cuando un monitor padre está caído). Un monitor
   inaccesible genera una alerta; un error de configuración se muestra pero no
   se notifica.
4. Las notificaciones salen por los canales configurados en la política de
   notificación.
5. La interfaz y cualquier script leen todo a través de `/api/*`; no existe una
   mitad privada de la API.

## Puntos de extensión

- Un nuevo tipo de monitor es una implementación de `Collector` más una
  llamada a `Registry::register`. Los formularios de la interfaz se generan a
  partir de `GET /api/collectors`.
- Un nuevo canal de notificación es un emisor más una entrada en
  `notify/catalog.rs`; la interfaz construye el formulario a partir de
  `GET /api/notify/kinds`.
- Un dispositivo SNMP que solo necesita un perfil no requiere Rust: basta un
  archivo YAML en `profiles/`.

Consulta [Desarrollo](../development.md) para las instrucciones de compilación
y [Modelo de seguridad](security-model.md) para los límites de confianza.
