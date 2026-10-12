# SecurityGateway for Email Servers

Los servicios de una pasarela de correo SecurityGateway, comprobados
desde fuera, y en SecurityGateway 12.5 o posterior, la versión y los
contadores de rendimiento que publica su API REST: cola de entrega,
cuarentena, sesiones SMTP.

!!! warning "Validado solo contra la documentación"

    Esta integración se construyó a partir de la documentación publicada
    por MDaemon Technologies sobre SecurityGateway y su API REST, no
    contra una pasarela real. Cada llamada es de lectura. La
    documentación no publica la ruta ni los nombres de los contadores de
    rendimiento, así que DumbMonit encuentra la ruta en la propia
    descripción de la API y conserva cada contador bajo el nombre que la
    API le da. Todavía nadie la ha ejecutado contra una pasarela en
    producción. Dinos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_securitygateway_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `service_up` | `service`, `port` | 1 si el puerto aceptó la conexión y, para SMTP, saludó con `220`. 0 si no respondió, o respondió para rechazar el servicio (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Tiempo hasta el saludo, o hasta la conexión para un puerto sin uno. |
| `api_up` | | Solo con una clave de API: 1 si la API REST respondió, 0 si no. |
| `api_response_seconds` | | Tiempo que tardó la API REST en responder. |
| `counters_available` | | 1 si se leyeron los contadores de rendimiento; 0 si la API no describe ninguno, o la cuenta de la clave no puede leerlos. |
| `counter` | `counter` | Una serie por cada valor numérico de los contadores de rendimiento, nombrada como la API la nombra, en `snake_case`: `deliveryQueue` se convierte en `delivery_queue`. Como máximo 64. |
| `info` (valor 1) | `version`, `source` | La versión dada por la descripción de la API (`source="openapi"`), o si no por el saludo SMTP cuando la pasarela lo anuncia ahí (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tiempo que tardó toda la comprobación. |

**Cómo se encuentran los contadores.** La API REST de SecurityGateway se
describe a sí misma en `/api/v1/openapi`. DumbMonit lee esa descripción,
toma la ruta de solo lectura más corta sin parámetros cuyo nombre
mencione contadores, la lee, y conserva cada número de su respuesta. Nada
se renombra y nada se asume: si una versión futura renombra un contador,
la serie sigue el nombre nuevo. Los contadores listados en la ayuda de
SecurityGateway son las sesiones SMTP de entrada y salida activas, la
cola de entrega, la cuarentena administrativa, el tiempo de actividad, los
recuentos de dominios y usuarios y el pool de conexiones de base de
datos; algunos de ellos solo cambian una vez por minuto.

**Un fallo parcial sigue siendo una comprobación exitosa.** Un servicio
detenido da `service_up 0` para ese servicio y los demás se siguen
midiendo. Solo cuando ningún servicio ni ninguna API responde en
absoluto, el dispositivo es *inaccesible*. Una clave rechazada es un
error de configuración, mostrado en la página del dispositivo y no
notificado.

**Qué no se lee.** La licencia y su caducidad, el espacio en disco y las
estadísticas de mensajes del panel no están descritos por la
documentación publicada de la API REST. Para vigilar el propio servicio
Windows de SecurityGateway y los discos del servidor, instala el
[agente](agent.md) en el servidor y lista el servicio en su ajuste
`services`.

Las [reglas integradas](../alerting/rules.md#mdaemon-and-securitygateway)
que se aplican: SecurityGateway service down, SecurityGateway API not
answering, SecurityGateway delivery queue growing, además de Device
unreachable.

## La página del dispositivo {#the-device-page}

Un dispositivo SecurityGateway muestra un panel **Mail services** sobre
las gráficas, leído de la última medición guardada: abrir la página nunca
se conecta a la pasarela. Lista cada servicio vigilado con su puerto, una
palabra y un color (Answering, Down) y su tiempo de respuesta, la
versión, y, con una clave de API, cada contador de rendimiento con su
último valor.

## Vigilar los servicios de la pasarela, y opcionalmente la API REST {#watch-the-gateway-services-and-optionally-the-rest-api}

1. Nada que instalar en el servidor. Sin ninguna credencial, DumbMonit se
   conecta a SMTP (25) y a la interfaz web (4000) desde fuera y lee el
   saludo SMTP. Cambia la lista en la opción Services, por ejemplo para
   comprobar HTTPS en lugar de HTTP.

    ```
    smtp,web_https
    ```

2. Para leer también los contadores de rendimiento, se necesita
   SecurityGateway 12.5 o posterior. Crea una cuenta dedicada con el
   nombre siguiente, con una contraseña larga usada en ningún otro
   sitio, y dale el rol Domain Administrator. Si los contadores siguen
   vacíos en la página del dispositivo, tu versión los reserva para el
   rol Global Administrator, y la cuenta necesita ese rol en su lugar.

    ```
    dumbmonit
    ```

3. Inicia sesión como esa cuenta y crea una clave en Setup/Users →
   Accounts → API Keys, con una fecha de caducidad. Una clave lleva los
   derechos de la cuenta que la crea, por eso debe crearse desde esta
   cuenta y no desde la tuya.

4. Restringe la cuenta a la dirección desde la que DumbMonit se conecta
   en sus restricciones de IP: las claves de API también las obedecen.

5. Comprueba desde el host de DumbMonit que la clave funciona. El comando
   imprime la descripción de la API, que también lista los contadores
   que DumbMonit lee.

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. En DumbMonit, introduce la dirección de la pasarela, por ejemplo
   «sg.example.com», y pega la clave. La API se alcanza en el puerto
   HTTPS de la interfaz web, 4443 por defecto, bajo /api/v1.

!!! warning
    SecurityGateway no tiene ningún rol de solo lectura ni una clave más
    limitada: la clave puede hacer todo lo que puede hacer su cuenta.
    Nunca la crees desde tu propia cuenta, mantenla solo en DumbMonit y
    dale una fecha de caducidad. Las versiones anteriores a 12.5 no
    tienen API REST: deja la credencial vacía y DumbMonit solo
    comprueba los servicios. La interfaz web a menudo usa un certificado
    autofirmado en 4443: si la conexión se rechaza por ese motivo, marca
    «Accept an unverifiable certificate» en las opciones.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Solo servicios | Nada: los puertos se comprueban sin iniciar sesión. |
| Clave de API (12.5 y posteriores) | La clave, enviada como `Authorization: Bearer …`. |

Dirección: el nombre de host o IP de la pasarela, por ejemplo
`sg.example.com`. Un puerto escrito en la dirección
(`sg.example.com:8443`) se toma como el puerto de la API.

## Opciones {#options}

| Opción | Por defecto | Efecto |
|---|---|---|
| `services` | `smtp,web` | Qué comprobar, separado por comas: `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `nombre:puerto` cambia un puerto; cualquier otro nombre con su puerto añade un servicio propio, comprobado solo por conexión. `none` no comprueba ningún puerto. |
| `request_timeout_seconds` | `10` | Tiempo permitido para cada conexión, saludo y llamada a la API, de 1 a 60. |
| `api_port` | `4443` | El puerto de la interfaz web que sirve `/api/v1`. Solo se usa con una clave de API. |
| `api_tls` | `true` | HTTPS hacia la API REST. Desmárcalo solo para la interfaz web por HTTP plano (puerto 4000 por defecto): la clave entonces cruza la red en claro. |
| `insecure_tls` | `false` | Aceptar un certificado autofirmado en la API REST. |
| `counters` | `true` | Leer los contadores de rendimiento. |
