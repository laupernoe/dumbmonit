# Caso de aseguramiento

Esta página argumenta que DumbMonit cumple sus requisitos de seguridad: expone
las amenazas, los límites donde cambia la confianza, por qué el diseño limita el
daño de un error, cómo se contrarresta cada clase habitual de debilidad y qué
pruebas respaldan cada afirmación. Amplía el [modelo de seguridad](security-model.md),
que enumera las mismas medidas de forma más breve. Para informar de una
vulnerabilidad, sigue la
[política de seguridad](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

Las rutas de archivo que siguen son relativas a la raíz del repositorio.

## Requisitos de seguridad

1. Solo un usuario autenticado, con un rol que lo permita, puede leer o cambiar
   la configuración, las alertas o las mediciones.
2. Las credenciales que DumbMonit guarda para tus dispositivos no se pueden leer
   desde el archivo de la base de datos, desde un paquete de copia de seguridad
   sin su frase de contraseña ni desde la API.
3. Solo una máquina con un token válido puede aportar mediciones, y un token
   filtrado puede revocarse por sí solo.
4. DumbMonit no puede convertirse en un medio para llegar a servicios a los que
   no debería, en particular su propio VictoriaMetrics hijo, sin autenticación.
5. Una página servida a un navegador no puede ejecutar scripts que no trajera
   consigo, y un sitio de terceros no puede hacer que un navegador con la sesión
   iniciada cambie nada.

## Modelo de amenazas

| Atacante | Capacidad | En alcance |
|---|---|---|
| Cliente de red anónimo | Llega al puerto HTTP | Sí |
| Usuario con sesión iniciada y rol bajo | Sesión válida o token de API de solo lectura | Sí (escalada de privilegios) |
| Página web maliciosa | Se ejecuta en el navegador de un administrador | Sí (CSRF, XSS, clickjacking) |
| Dispositivo monitorizado comprometido o malicioso | Responde a las sondas con contenido hostil | Sí (analizadores, redirecciones) |
| Agente malicioso o robado | Posee un token de agente | Sí (alcance de un único token) |
| Administrador que introduce una URL hostil | Configura un webhook o la dirección de un dispositivo | Sí, para las direcciones propias del servidor |
| Quien tenga el host, el volumen `/data` o el entorno de ejecución de contenedores | Control total | No: fuera de alcance, como en la política de seguridad |
| Vulnerabilidades dentro de VictoriaMetrics o de los dispositivos monitorizados | Código de terceros | No: informar al proyecto original |

## Límites de confianza

| Límite | Qué lo cruza | Cómo se protege | Evidencia |
|---|---|---|---|
| Navegador hacia `/api/*` | Cookie de sesión o token de API, JSON | Un único middleware de autenticación, tres roles, cabecera CSRF en las escrituras autenticadas por cookie | `crates/server/src/auth/middleware.rs`; pruebas `auth_api.rs`, `operator_role_api.rs`, `api_open_security_api.rs`, `cors_policy_api.rs` |
| Navegador hacia páginas servidas | HTML, script, estilo | Content Security Policy con nonce por respuesta, `nosniff`, `Referrer-Policy: same-origin`, `X-Frame-Options: DENY` | `crates/server/src/api/mod.rs` (capa de cabeceras de seguridad) |
| Proveedor de identidad hacia el servidor | Redirección de OpenID Connect y token de ID | State, nonce y PKCE (S256); la vinculación de cuentas locales es explícita | `crates/server/src/auth/oidc/`; prueba `oidc_api.rs` |
| Agente hacia `/api/ingest` | Mediciones, resultados de comandos | Token por agente, almacenado como hash SHA-256 (prefijo `dmon_`) | `crates/server/src/api/ingest.rs`, `crates/server/src/collectors/agent/token.rs`; prueba `agent_binding_api.rs` |
| Agente relé hacia el servidor | Resultados de sondas de otra red | La misma comprobación del token de agente (`agent_commands::authenticate`) | `crates/server/src/api/relay.rs`; prueba `relay_api.rs` |
| Servidor hacia dispositivos monitorizados | Peticiones de sonda, respuestas hostiles | Redirecciones seguidas solo dentro del mismo host, número de redirecciones acotado, analizadores de respuestas con pruebas unitarias, verificación de certificados activada por defecto | `crates/collectors/src/http.rs` (`same_host_redirects`) |
| Servidor hacia canales de notificación | Peticiones de webhook y SMTP | La misma política de redirecciones más una protección que rechaza las direcciones propias del servidor, comprobada antes de enviar, en cada redirección y en la resolución DNS | `crates/server/src/notify/guard.rs`; prueba `notify_guard_api.rs` |
| Servidor hacia VictoriaMetrics | Escrituras y consultas de lectura | El hijo incluido escucha por defecto en `127.0.0.1:8428`; el proxy compatible con Prometheus solo reenvía las rutas de lectura (query, query_range, series, labels) | `crates/server/src/config.rs` (`DUMBMONIT_VM_LISTEN`), `crates/server/src/api/prometheus.rs` (`readable_segments`) |
| Servidor hacia SQLite | Configuración y estado | Parámetros enlazados mediante `sqlx` | `crates/server/src/db/` |
| Servidor hacia `/data` | Base de datos, clave secreta | Secretos cifrados con AES-256-GCM; el usuario del contenedor es propietario únicamente de `/data` | `crates/server/src/crypto.rs`, `Dockerfile` |

## Argumento de diseño seguro

El argumento sigue los principios de diseño de Saltzer y Schroeder.

- **Economía de mecanismo.** Un binario, un archivo SQLite, un proceso hijo en
  loopback, una única entrada para todos los clientes (`/api/*`).
- **Valores predeterminados a prueba de fallos.** En una instancia nueva todas
  las rutas bajo la protección de autenticación responden `401`; solo responden
  salud, estado de autenticación, configuración inicial e inicio de sesión, de
  modo que nadie puede preparar la instancia antes que su propietario
  (`auth/middleware.rs`, prueba `api_open_security_api.rs`). VictoriaMetrics
  escucha en loopback salvo que el operador lo cambie. Los certificados TLS de
  los servicios monitorizados se verifican salvo que el operador active
  `insecure_tls` (`crates/collectors/src/http.rs`).
- **Mediación completa.** El middleware de autenticación se ejecuta en cada ruta
  de la API, en lugar de que cada manejador lo compruebe por su cuenta.
- **Diseño abierto.** El código, la descripción de la API
  (`crates/server/src/api/openapi.yaml`) y esta página son públicos. La
  seguridad no depende del secreto del diseño.
- **Separación de privilegios.** Las sesiones, los tokens de API y los tokens de
  agente son credenciales distintas. Los tokens de API llevan un ámbito `read` o
  `write` (`auth/token.rs`) y los usuarios tienen un rol de lector, operador o
  administrador.
- **Mínimo privilegio.** El servidor se ejecuta con el UID 65532 (o `PUID`):
  arrancado como root, solo entrega el directorio de datos a ese usuario, sin
  seguir enlaces, y luego abandona root de forma definitiva y comprueba que no
  puede recuperarlo, antes de abrir ninguna otra cosa (`privileges.rs`). El
  archivo Compose conserva únicamente las cuatro capabilities que necesita este
  paso, establece un sistema de archivos raíz de solo lectura y
  `no-new-privileges` (`docker-compose.yml`).
- **Mínimo mecanismo común.** Los agentes se autentican con su propio token, de
  modo que revocar una máquina no afecta a ningún usuario ni a otros agentes.
- **Aceptabilidad psicológica.** El comportamiento seguro es el predeterminado y
  no requiere pasos adicionales: la cookie es `HttpOnly` y `SameSite=Lax`, el
  cliente web añade la cabecera CSRF, los secretos son de solo escritura en la
  interfaz.
- **Defensa en profundidad.** Cifrado en reposo además del control de acceso; la
  cabecera CSRF además de `SameSite`; el límite de intentos de inicio de sesión
  además del hash Argon2; una política de redirecciones además de la protección
  contra direcciones propias.

### Criptografía

- Contraseñas: Argon2 (`auth/password.rs`). Los secretos de sesión y de token de
  API se almacenan como hashes SHA-256 de valores aleatorios (`auth/session.rs`,
  `auth/token.rs`).
- Secretos en reposo: AES-256-GCM con un nonce nuevo en cada cifrado
  (`crypto.rs`). La clave se deriva con Argon2id a partir de `DUMBMONIT_SECRET`
  o del archivo de clave generado, con una sal persistente; se rechaza un
  secreto de menos de 16 caracteres, y un valor canario detecta una clave
  incorrecta al arrancar.
- Copias de seguridad: la clave se deriva de la frase de contraseña del operador
  con Argon2id, con los parámetros almacenados en el paquete, de modo que el
  algoritmo puede cambiar sin romper los paquetes antiguos (`backup/bundle.rs`).
- Segundo factor: TOTP, con códigos de recuperación de un solo uso almacenados
  como hash (`auth/totp.rs`).
- OpenID Connect: state, nonce y PKCE `S256` (`auth/oidc/pkce.rs`).
- Todas las primitivas proceden de bibliotecas mantenidas (`aes-gcm`, `argon2`,
  `sha2`); ninguna es de fabricación propia. El listener integrado habla HTTP
  sin cifrar; la documentación exige un proxy inverso con TLS, y las cookies
  reciben `Secure` cuando la petición llega por HTTPS (`auth/cookie.rs`).

## Contramedidas a las debilidades habituales

| Debilidad (OWASP Top 10, CWE) | Contramedida | Evidencia |
|---|---|---|
| Control de acceso defectuoso, CWE-862, CWE-285 | Middleware de autenticación, roles, ámbitos de token | `operator_role_api.rs`, `api_tokens_api.rs`, `api_open_security_api.rs` |
| Fallos criptográficos, CWE-327, CWE-312 | AES-256-GCM en reposo, Argon2, ningún secreto devuelto por la API | pruebas unitarias de `crypto.rs`; `has_secret` / `credential_kind` en lugar del secreto |
| Inyección SQL, CWE-89 | Parámetros enlazados mediante `sqlx` | `crates/server/src/db/` |
| Cross-site scripting, CWE-79 | Svelte escapa la salida; CSP con `default-src 'none'`, scripts solo desde `'self'` y un nonce; `base-uri 'none'` | `api/mod.rs` |
| Cross-site request forgery, CWE-352 | Las escrituras por cookie necesitan `X-Requested-With: DumbMonit` o metadatos de fetch del mismo origen; CORS no concede la cabecera a nadie; `SameSite=Lax` | `auth/middleware.rs`, `auth/cors.rs`; prueba `cors_policy_api.rs` |
| Clickjacking, CWE-1021 | `X-Frame-Options: DENY` y `frame-ancestors 'none'`, salvo en las páginas de estado públicas opcionales pensadas para incrustarse | `api/mod.rs` |
| Falsificación de peticiones del lado del servidor, CWE-918 | Solo redirecciones al mismo host; la protección de notificaciones rechaza las direcciones de escucha del propio servidor y de VictoriaMetrics, también tras la resolución DNS (rebinding). El loopback y los rangos privados siguen siendo accesibles a propósito, porque ahí viven los servicios de un homelab | `notify/guard.rs`; prueba `notify_guard_api.rs` |
| Fallos de autenticación, CWE-307, CWE-287 | Límite de intentos de inicio de sesión con retardo creciente por dirección y por cuenta, TOTP opcional, OIDC con PKCE, restablecimiento del segundo factor por un administrador | `auth/rate_limit.rs`, `auth/totp.rs`, `auth/oidc/` |
| Deserialización insegura, agotamiento de recursos, CWE-400 | JSON tipado mediante `serde`, cuerpos de petición limitados a 16 MiB, redirecciones acotadas | `api/mod.rs`, `collectors/src/http.rs` |
| Seguridad de memoria, CWE-787, CWE-416 | Rust; `unsafe` aparece solo en el código de contadores de rendimiento del agente de Windows (`crates/agent/src/collect/perf_counters.rs`) | grep de `unsafe` |
| Componentes vulnerables, CWE-1104 | `cargo deny` (licencias, prohibiciones, avisos); Dependabot; análisis con Trivy de las imágenes publicadas; acciones fijadas por commit; compilaciones con `--locked` | `.github/workflows/ci.yml`, `security.yml`, `.github/dependabot.yml`, `deny.toml` |
| Integridad del software y de los datos, CWE-494 | Los instaladores del agente verifican el SHA-256 del binario descargado | `crates/agent/install/install.sh`, `install.ps1` |
| Fallos de registro y monitorización, CWE-778 | Registro de seguridad de los inicios de sesión y los cambios sensibles | `auth/audit.rs`; prueba `security_audit_api.rs` |

Los riesgos residuales conocidos se enumeran en el modelo de seguridad (listener
HTTP sin cifrar, opciones de dispositivo sin cifrar, VictoriaMetrics incluido sin
autenticación). La CSP permite estilos en línea (`style-src 'unsafe-inline'`), lo
que la debilita frente a la inyección de estilos pero no frente a la ejecución de
scripts. Existe un aviso aceptado, `RUSTSEC-2023-0071`, ignorado en `deny.toml`
con su motivo: el crate `rsa` verifica firmas RS256 con la clave pública del
proveedor y nunca usa una clave privada.

## Evidencias

- **Integración continua** (`.github/workflows/ci.yml`): `cargo fmt --check`,
  `clippy -D warnings`, la batería completa de pruebas de Rust,
  `cargo deny check licenses bans` y `cargo deny check advisories`, comprobación
  de tipos de la web, pruebas y compilación de la web, y una compilación de la
  imagen Docker, en cada push y pull request.
- **Análisis programados** (`.github/workflows/security.yml`): Trivy sobre las
  imágenes publicadas del servidor y del agente, y el OpenSSF Scorecard, cada
  semana y tras cada publicación de `:edge`. Los resultados aparecen en la
  pestaña Security del repositorio, donde también informa el escaneo de código
  de GitHub (CodeQL).
- **Pruebas**: pruebas de API, autenticación, roles, CORS, OIDC, protección y
  auditoría en `crates/server/tests/`, pruebas unitarias junto al código y
  pruebas web en `web/tests/`. La cobertura de líneas de Rust medida con
  `cargo-llvm-cov` es del 87,9 %.
- **Política de regresión**: una corrección incluye una prueba que falla sin
  ella (véase la política de pruebas en `CONTRIBUTING.md`).
- **Registro público**: las correcciones de seguridad se publican como avisos y
  se enumeran en las notas de la versión.
