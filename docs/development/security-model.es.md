# Modelo de seguridad

Esta página describe qué protege DumbMonit, en quién confía, cómo el diseño
limita el daño de un error y dónde mirar en el código y en las pruebas. Para
informar de una vulnerabilidad, sigue la
[política de seguridad](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Qué está en juego

DumbMonit funciona dentro de tu red. Guarda las credenciales que necesita para
leer cada dispositivo (comunidades SNMP, tokens de API, contraseñas), recibe
mediciones de los agentes instalados en tus máquinas y envía peticiones a las
direcciones que configuras. Una vulneración expondría esos secretos y daría a un
atacante un punto de apoyo en la red monitorizada.

## Límites de confianza

| Límite | Quién está al otro lado | Confianza |
|---|---|---|
| Navegador o script hacia `/api/*` | Administradores, operadores y lectores | Autenticados por una cookie de sesión o un token de API; tres roles limitan lo que cada uno puede hacer. |
| Agente hacia `/api/ingest` | Máquinas en las que instalaste un agente | Un token por máquina; un token filtrado puede revocarse por separado. |
| Servidor hacia dispositivos y servicios | Sistemas de terceros | Sin confianza: las respuestas se analizan de forma defensiva y las redirecciones se quedan en el mismo host. |
| Servidor hacia canales de notificación | Servicios externos | Las credenciales se cifran en reposo y la API nunca las devuelve. |
| Servidor hacia VictoriaMetrics | Un proceso hijo en loopback, o una instancia externa | La incluida no tiene autenticación y escucha en loopback; una externa es responsabilidad del operador. |
| Volumen `/data` y host | El operador | De confianza. Un atacante que controle el host o el volumen queda fuera del alcance. |

## Principios de diseño seguro aplicados

- **Mínimo privilegio.** El contenedor se ejecuta como usuario sin privilegios,
  sin ninguna capability, con sistema de archivos raíz de solo lectura y
  `no-new-privileges`. Tres roles (lector, operador, administrador) restringen
  lo que un usuario puede cambiar (`crates/server/src/auth/users.rs`; prueba
  `operator_role_api.rs`).
- **Seguro por defecto.** En una instancia nueva solo responden las rutas de
  salud, configuración inicial, inicio de sesión y estado de autenticación
  hasta que existe el primer administrador (prueba
  `api_open_security_api.rs`).
- **Economía de mecanismo.** Un binario, un archivo SQLite, un proceso hijo en
  loopback y una sola entrada para todos los clientes: `/api/*`.
- **Valores predeterminados a prueba de fallos.** Una comprobación de seguridad
  sin datos es `unknown`, nunca un éxito; un dispositivo inaccesible y un error
  de configuración son estados distintos.
- **Mediación completa.** Un único middleware de autenticación
  (`crates/server/src/auth/middleware.rs`) protege las rutas de la API.
- **Separación de privilegios.** Las sesiones, los tokens de API y los tokens de
  agente son credenciales distintas con alcance distinto.
- **Defensa en profundidad.** Cifrado en reposo además del control de acceso,
  una cabecera CSRF además de las banderas de la cookie, un límite de tasa
  además del hash de contraseñas.
- **Ningún secreto de ida y vuelta.** La API devuelve `has_secret` y
  `credential_kind`, nunca el secreto; omitir un secreto en un `PUT` lo
  conserva.
- **Solo criptografía contrastada.** Las contraseñas se hashean con Argon2, los
  secretos se cifran con AES-256-GCM (`crypto.rs`), la autenticación en dos
  factores usa TOTP con códigos de recuperación de un solo uso almacenados como
  hash, y OpenID Connect usa state, nonce y PKCE. Nada es de fabricación
  propia.

## Contramedidas por clase de debilidad

| Clase (tema del OWASP Top 10) | Medidas |
|---|---|
| Control de acceso defectuoso | Un único middleware de autenticación; roles; una instancia nueva rechaza todo salvo configuración inicial, inicio de sesión, salud y estado de autenticación. |
| Fallos criptográficos | AES-256-GCM en reposo, clave guardada fuera de la base de datos; hashes de contraseña Argon2; el servidor integrado habla HTTP sin cifrar, por lo que se documenta un proxy inverso con TLS. |
| Inyección | El SQL pasa por `sqlx` con parámetros enlazados; el proxy de VictoriaMetrics solo reenvía consultas de lectura (`tsdb/mod.rs`, `proxy_read`); los analizadores tienen pruebas unitarias. |
| Diseño inseguro | El modelo de amenazas anterior; los secretos nunca hacen el viaje de ida y vuelta; una instancia de propósito único. |
| Configuración de seguridad incorrecta | Archivos Compose reforzados, VictoriaMetrics solo en loopback, notas de refuerzo para el operador en la política de seguridad. |
| Componentes vulnerables | `cargo deny` comprueba licencias, prohibiciones y avisos de RustSec; análisis semanal con Trivy de las imágenes publicadas; Dependabot; GitHub Actions fijadas por commit. |
| Fallos de identificación y autenticación | Límite de intentos de inicio de sesión por dirección de cliente y por cuenta (`auth/rate_limit.rs`), autenticación en dos factores opcional, OpenID Connect, un administrador puede restablecer el segundo factor de un usuario y cerrar sus sesiones. |
| Fallos de integridad del software y de los datos | Los instaladores del agente verifican el SHA-256 del binario descargado; las imágenes de las versiones las construye la CI. |
| Fallos de registro y monitorización | Un registro de seguridad con los inicios de sesión y los cambios sensibles (`auth/audit.rs`). |
| Falsificación de peticiones del lado del servidor | Las integraciones y los canales de notificación siguen las redirecciones solo dentro del mismo host, de modo que un dispositivo no puede desviar una petición y su clave de API a otro lugar. |
| Falsificación de petición en sitios cruzados | Las peticiones que modifican estado y dependen de la cookie deben llevar `X-Requested-With: DumbMonit` o metadatos de fetch del mismo origen; de lo contrario, `403`; las peticiones autenticadas con token no llevan cookie. |

## Verificación

Cada push y cada pull request ejecuta `clippy -D warnings`, la batería de
pruebas de Rust (incluidas las pruebas de API, autenticación y roles en
`crates/server/tests/`), las pruebas web en `web/tests/`, las comprobaciones de
dependencias y el análisis de CodeQL. El OpenSSF Scorecard se ejecuta de forma
programada. Las correcciones de seguridad incluyen una prueba de regresión y se
enumeran, con su identificador CVE o GHSA, en las notas de la versión.

El [caso de aseguramiento](assurance-case.md) ofrece el modelo de amenazas
completo, el argumento de diseño seguro y las pruebas que respaldan cada
afirmación.

## Límites conocidos

- El servidor integrado habla HTTP sin cifrar; pon TLS delante.
- Las opciones de los dispositivos se guardan sin cifrar. Pon los secretos en
  los campos de credenciales.
- El VictoriaMetrics incluido no tiene autenticación.
- DumbMonit está en una versión previa a la 1.0; las rutas y los campos aún pueden cambiar.
