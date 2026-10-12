# Caducidad de dominio {#domain-expiry}

Un dominio registrado, leído desde su registro vía RDAP: los días
restantes antes de que caduque, una retención (hold) que lo saca del DNS,
un período de redención, su registrador y si su delegación está firmada
con DNSSEC.

RDAP es el sucesor de WHOIS: los mismos datos de registro, en JSON, con un
formato estándar entre registros. DumbMonit encuentra el registro de la
extensión en el archivo de arranque RDAP de la IANA
(`https://data.iana.org/rdap/dns.json`, leído como máximo una vez al día),
y luego le pregunta `/domain/example.com`. No se necesita ninguna
credencial, y no se pide nada más que el registro del dominio.

Dos fallos valen la pena vigilar además de la propia fecha de caducidad.
Un dominio en `clientHold` o `serverHold` ha sido sacado del DNS por el
registrador o el registro, normalmente por una factura impagada o una
dirección de contacto que nunca se verificó: cada sitio y buzón bajo él se
detiene de golpe. Un dominio en `redemptionPeriod` o `pendingDelete` ya ha
caducado y será liberado a menos que se restaure, con un coste.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_domain_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `expiry_days` | días restantes antes de la caducidad, fraccional; negativo una vez caducado | |
| `expiry_timestamp_seconds` | la fecha de caducidad, en segundos Unix | |
| `expiry_published` | 0 cuando el registro no publica la fecha de caducidad | |
| `on_hold` | 1 en `clientHold` o `serverHold` | |
| `redemption` | 1 en `redemptionPeriod`, `pendingDelete` o `pendingRestore` | |
| `status` | valor 1, una serie por código de estado, en forma EPP (`clientTransferProhibited`) | `status` |
| `registrar_info` | valor 1 | `registrar` |
| `dnssec` | 1 cuando la delegación está firmada | |
| `rdap_ok` | 0 cuando la última consulta al registro falló y se muestra una respuesta anterior | |
| `rdap_age_seconds` | antigüedad de la respuesta mostrada | |

Se consulta al registro una vez por intervalo de actualización (12 horas
por defecto); entre medias, los días restantes se recalculan en cada
comprobación. Cuando el registro no responde, la última respuesta se sigue
usando hasta siete días, con `rdap_ok` a 0, así que una caída del registro
no se parece a un problema del dominio.

Las [reglas integradas](../alerting/rules.md#reverse-proxies-and-domains)
que se aplican: Domain expiring (menos de 30 días), Domain about to expire
(menos de 7 días o caducado), Domain on hold, Domain in redemption, además
de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta el registro. Indica la fecha de caducidad y los días
restantes, los códigos de estado (o el hold y la redención en una frase),
el registrador y DNSSEC, y si la última consulta al registro falló.

## Vigilar el registro de un dominio {#watch-a-domains-registration}

1. Introduce el dominio registrado, por ejemplo «example.com», no un
   nombre bajo él como www.example.com, que el registro no conoce. Nada
   que configurar y ninguna credencial: RDAP, el sucesor de WHOIS, es
   público.

2. DumbMonit encuentra el registro de la extensión en el archivo de
   arranque RDAP de la IANA, y luego le pide la fecha de caducidad, los
   códigos de estado y el registrador, dos veces al día por defecto. Los
   registros limitan las consultas RDAP, y la fecha de caducidad cambia
   una vez al año: un intervalo de actualización más corto no aporta nada.

3. Algunos registros ejecutan RDAP sin estar listados por la IANA, los de
   .ch y .li por ejemplo: define RDAP server con su dirección. Una
   extensión cuyo registro no ejecuta ningún servicio RDAP público no se
   puede vigilar.

    ```
    https://rdap.nic.ch/
    ```

4. Algunos registros no publican la fecha de caducidad (.ch es uno de
   ellos): DumbMonit lo indica entonces, y sigue vigilando los códigos de
   estado.

!!! warning
    Un dominio en clientHold o serverHold se saca del DNS: cada sitio y
    buzón bajo él se detiene, normalmente por una factura impagada o una
    dirección de contacto no verificada. DumbMonit avisa 30 días antes de
    la caducidad, lo eleva 7 días antes, y alerta de inmediato ante un hold
    o un período de redención.

Un dominio que el registro no conoce (un subdominio, una errata, o un
dominio que fue eliminado) se reporta como un error de configuración, no
como una caída. Los nombres con acentos se aceptan y se consultan en su
forma ASCII (`xn--…`).

## Credenciales {#credentials}

Ninguna: RDAP es público.

Dirección: el dominio registrado (`example.com`); una URL pegada se reduce
a su nombre de host. Opciones: RDAP server (vacío: se encuentra a través
de la IANA), intervalo de actualización (12 horas, de 1 a 168) y tiempo de
espera de la petición (10 s). El intervalo de comprobación del propio
dispositivo puede quedarse en su valor por defecto: solo recalcula los días
restantes entre dos consultas al registro.
