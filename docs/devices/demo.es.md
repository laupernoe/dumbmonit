# Dispositivo de demostración {#demo-device}

Un dispositivo ficticio que genera mediciones, para explorar la herramienta
sin hardware. No se necesita hardware.

## Configuración {#setup}

1. Dale cualquier nombre y confirma.
2. Aparecerán mediciones falsas al instante, suficientes para recorrer la
   interfaz.

La dirección es libre (el placeholder dice `demo`), no hay credencial ni
opción alguna. El dispositivo de demostración produce una serie
`cpu_usage_percent`, así que las reglas «High CPU» y «Unusual CPU» se le
aplican, y puede usarse como padre o hijo en una [cadena de dependencia](../alerting/index.md#dependency-suppression)
para ver la supresión en acción.

Para mediciones reales sin hardware dedicado, apunta un dispositivo SNMP a
cualquier máquina que ejecute `snmpd`: consulta [Probar sin hardware](../install/docker.md#testing-without-hardware).
