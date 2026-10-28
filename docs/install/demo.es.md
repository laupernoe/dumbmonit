# Ejecutar una instancia de demostración {#run-a-demo-instance}

La demo pública en **[demo.dumbmonit.app](https://demo.dumbmonit.app)** es
una imagen de DumbMonit corriente iniciada con `DUMBMONIT_DEMO=1`. Puedes
ejecutar lo mismo tú mismo — para mostrar DumbMonit a un equipo antes de
instalarlo de verdad, o para poner una vitrina en tu propio sitio.

!!! warning "Nunca sobre un volumen de datos real"
    El modo demo **recrea la base de datos en cada inicio**. Dale un volumen
    propio, o ninguno: apuntado al `/data` de una instancia real,
    sustituiría tus monitores, reglas e historial por el patrimonio
    ficticio.

## Qué hace el modo demo {#what-demo-mode-does}

- **Inicia sesión con `demo` / `demo`.** La página de inicio de sesión lo
  indica y rellena los campos con un clic. La cuenta ve todas las pantallas
  que ve un administrador.
- **Solo lectura.** Todo cambio — monitores, reglas, silencios,
  confirmaciones, canales, tokens, usuarios, ajustes, copias de seguridad y
  restauraciones, escaneos de descubrimiento, inscripción de agentes,
  herramientas de escritura del endpoint del asistente — es rechazado por
  el servidor con `403` y `{"error": "This is a read-only demo…", "demo": true}`.
  Iniciar y cerrar sesión son las únicas escrituras permitidas. La interfaz
  muestra el mensaje donde ocurrió el clic.
- **Dos clientes ficticios.** Al arrancar, la base de datos se crea de nuevo y
  se llena con un homelab bajo `*.home.arpa` — Proxmox VE, un NAS Synology con
  un LUN iSCSI, Proxmox Backup Server, TrueNAS, OPNsense, UniFi, Kopia, un
  servidor Redfish con una pieza averiada, un switch SNMP, sondas HTTP, TLS,
  DNS y ping, un agente Linux con Docker — y, en modo MSP, con un pequeño
  estudio de arquitectura en Ginebra: un controlador de dominio Windows con su
  agente, Veeam Backup & Replication, una red Omada, Microsoft 365 y Entra ID,
  consultados a través de un agente relé instalado en la oficina. Los
  monitores los responden equipos simulados dentro del servidor, ligados a
  `127.0.0.1`, que reproducen respuestas de API grabadas: los colectores y
  paneles reales funcionan. Reiniciar da el mismo parque.
- **Treinta días de historial, y el resto del producto.** Al arrancar se
  escriben métricas sintéticas en VictoriaMetrics, por bloques acotados (la
  última semana con resolución de cinco minutos): gráficos, *Últimos 7 días*,
  disponibilidad a 30 días, previsiones, la matriz de copias de seguridad y el
  historial de alertas están llenos, y algunas alertas están activas. La
  siembra también rellena lo que normalmente se configura a mano: equipos y
  carpetas, cuentas y turnos de guardia con políticas de escalado, silencios,
  procedimientos, dos paneles, informes programados, canales de notificación
  (nunca usados), sitios en el mapa mundial, vulnerabilidades conocidas, cuatro
  páginas de estado (una por escena nocturna) con un incidente en curso, y un
  escaneo de red con un dispositivo nuevo. Todo se calcula a partir de una
  semilla fija: no se descarga nada de la red, ni siquiera las fuentes de
  vulnerabilidades. La búsqueda de registros queda apagada: la demo no tiene
  almacén de registros.
- **Nada sale del servidor.** Nunca se envía ninguna notificación, y
  ninguna sonda va a una dirección que un visitante haya escrito.
- **Un recorrido guiado** se abre tras el primer inicio de sesión; el
  banner lo vuelve a abrir.

La limitación de tasa del inicio de sesión funciona como siempre. Pon la
demo detrás de tu proxy inverso habitual con HTTPS y define
`DUMBMONIT_COOKIE_SECURE=1` y `DUMBMONIT_TRUSTED_PROXIES` como para
cualquier instancia pública.

## Iniciarla {#start-it}

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

O con Compose, sin volumen para que cada reinicio empiece limpio:

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/laupernoe/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

Descarga la imagen con regularidad (o deja que una herramienta como
Watchtower lo haga): la demo mostrará siempre la última versión con los
mismos datos.
