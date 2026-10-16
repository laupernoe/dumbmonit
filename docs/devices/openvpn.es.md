# Servidor OpenVPN

El proceso servidor de OpenVPN expone su propio canal de control, la **interfaz
de gestión** (*management interface*): un listener TCP local, separado de la VPN
en sí, pensado para una herramienta de monitorización y no para un cliente VPN.
DumbMonit la lee como cualquier cliente de gestión: la versión del servidor, si
se declara conectado y cada cliente que está en sesión en ese momento, con su
dirección real y virtual y su tráfico. Nunca se envía nada que cambie el estado
del servidor o desconecte a un cliente.

Los clientes son transitorios por naturaleza: un portátil que se suspende
simplemente desaparece de la lectura siguiente. DumbMonit no intenta recordar
quién estaba conectado hace cinco minutos; informa solo de quién está conectado
ahora, además del número de clientes como métrica que merece la pena graficar
a lo largo del tiempo.

## Qué vigila {#what-it-watches}

Todas las métricas llevan el prefijo `dumbmonit_openvpn_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `server_info` | valor 1 | `version` |
| `server_up` | 1 cuando la interfaz de gestión informa del servidor como `CONNECTED`, 0 en caso contrario | `state` |
| `server_state_age_seconds` | tiempo desde que el servidor entró en su estado actual | |
| `clients_connected` | número de clientes en sesión en este momento | |
| `client_bytes_received`, `client_bytes_sent` | tráfico de un cliente (contadores) | `common_name`, `real_address` |
| `client_connected_seconds` | tiempo desde que empezó la sesión de ese cliente | `common_name`, `real_address` |

`real_address` nunca incluye el puerto de origen del cliente: cambia en cada
reconexión detrás de un NAT y multiplicaría las series sin motivo. Como máximo
256 clientes tienen cada uno una serie detallada; `clients_connected` sigue
siendo exacto más allá de ese límite.

Reglas integradas que se aplican: **Monitor unreachable**, cuando no se puede
establecer la propia conexión de gestión o se rechaza la contraseña.

## La página del monitor {#the-device-page}

El panel situado sobre los gráficos muestra lo que almacenó la sonda; abrir la
página nunca se conecta a la interfaz de gestión. Muestra la versión del
servidor y si se declara conectado, el número de clientes y una lista de los
clientes en sesión en ese momento con su dirección y su tráfico.

## Activar la interfaz de gestión {#turn-on-the-management-interface}

1. La interfaz de gestión es un listener aparte, desactivado por defecto: añade esto a la configuración del servidor (server.conf, o as.conf en OpenVPN Access Server, bajo otro nombre) y reinícialo. Enlázala a una dirección que el host de DumbMonit pueda alcanzar, o a 127.0.0.1 y accede a ella mediante un túnel SSH o un agente relay en la misma máquina.

    ```
    management 0.0.0.0 7505
    ```

2. Opcional pero recomendado: la interfaz de gestión no tiene cuenta propia, solo una contraseña compartida. Pon una en un archivo legible únicamente por el proceso de OpenVPN y añádela como tercer argumento.

    ```
    management 0.0.0.0 7505 /etc/openvpn/management-password
    ```

3. En DumbMonit, introduce la dirección del servidor, por ejemplo «vpn.lan» (puerto 7505), y la contraseña de gestión si hay una.

4. DumbMonit solo lee: envía version, state y status 3, nunca signal, kill ni client-kill. No puede desconectar a un cliente, cambiar un ajuste ni detener el servidor.

!!! warning
    Cualquiera que alcance el puerto de gestión y conozca la contraseña puede desconectar clientes, leer todas sus direcciones IP reales y reconfigurar el servidor en ejecución: mantenlo fuera de la red pública, tras una regla de firewall que permita solo al host de DumbMonit, y con una contraseña definida.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Sin contraseña | Solo para una interfaz de gestión iniciada sin el argumento de archivo de contraseña. |
| Contraseña de gestión | La contraseña compartida definida por el tercer argumento de la directiva «management» del servidor. |

Dirección: un nombre de host o IP, opcionalmente con `:puerto`; el puerto por
defecto es 7505, una convención de la propia documentación de OpenVPN, no un
estándar que el servidor imponga. Opciones: puerto (7505) y tiempo de espera
por intercambio (10 s).

Una conexión rechazada o la falta de respuesta cuenta como servidor
inaccesible. Una contraseña incorrecta se muestra como error de credenciales;
ninguna de las dos despierta a nadie por la noche. La conexión es TCP plano:
nada de lo que se lee es secreto, pero la contraseña, si la hay, cruza la red
en claro en cada comprobación — accede a la interfaz solo desde una red de
confianza o mediante un agente relay en la misma máquina que el servidor.
