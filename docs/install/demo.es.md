# Ejecutar una instancia de demostración {#run-a-demo-instance}

La demo pública en **[demo.dumbmonit.app](https://demo.dumbmonit.app)** es
una imagen de DumbMonit corriente iniciada con `DUMBMONIT_DEMO=1`. Puedes
ejecutar lo mismo tú mismo — para mostrar DumbMonit a un equipo antes de
instalarlo de verdad, o para poner una vitrina en tu propio sitio.

!!! warning "Nunca sobre un volumen de datos real"
    El modo demo **recrea la base de datos en cada inicio**. Dale un volumen
    propio, o ninguno: apuntado al `/data` de una instancia real,
    sustituiría tus dispositivos, reglas e historial por el patrimonio
    ficticio.

## Qué hace el modo demo {#what-demo-mode-does}

- **Inicia sesión con `demo` / `demo`.** La página de inicio de sesión lo
  indica y rellena los campos con un clic. La cuenta ve todas las pantallas
  que ve un administrador.
- **Solo lectura.** Todo cambio — dispositivos, reglas, silencios,
  confirmaciones, canales, tokens, usuarios, ajustes, copias de seguridad y
  restauraciones, escaneos de descubrimiento, inscripción de agentes,
  herramientas de escritura del endpoint del asistente — es rechazado por
  el servidor con `403` y `{"error": "This is a read-only demo…", "demo": true}`.
  Iniciar y cerrar sesión son las únicas escrituras permitidas. La interfaz
  muestra el mensaje donde ocurrió el clic.
- **Un patrimonio fijo y ficticio.** Al iniciar, la base de datos se crea
  de nuevo y se puebla con un pequeño homelab bajo `*.home.arpa`: un
  clúster Proxmox VE, un NAS Synology, un Proxmox Backup Server, un
  TrueNAS, un cortafuegos OPNsense, un servidor Redfish con una pieza
  fallando, algunas comprobaciones HTTP, TLS y ping, un host con agente y
  una página de estado. Los dispositivos son respondidos por respondedores
  falsos dentro del servidor, ligados a `127.0.0.1` y que reproducen
  respuestas de API grabadas, así que los recolectores y paneles de
  dispositivo reales se ejecutan de verdad. Reiniciar da el mismo
  patrimonio.
- **Siete días de historial.** Se escriben métricas sintéticas en
  VictoriaMetrics al iniciar, así que las gráficas, *Last 7 days*, los
  pronósticos y el historial de alertas están poblados, y hay algunas
  alertas activas para que *Needs you* tenga contenido.
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
