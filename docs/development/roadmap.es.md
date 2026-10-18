# Hoja de ruta {#roadmap}

Lo que está publicado, en curso y previsto se sigue en el tablero público del
proyecto: [DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2).
Las prioridades y los plazos pueden cambiar; el tablero es un plan vivo, no una
promesa.

- **Los cambios publicados** están en el
  [registro de cambios](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md)
  y en las notas de cada
  [versión de GitHub](https://github.com/laupernoe/dumbmonit/releases).
- **Las ideas y los errores** viven en el
  [gestor de incidencias](https://github.com/laupernoe/dumbmonit/issues). Abre
  una incidencia antes de escribir código para algo grande.
- **Las correcciones de seguridad** se anuncian en las notas de la versión y
  como avisos de seguridad de GitHub, con su identificador CVE o GHSA.

DumbMonit está en una versión previa a la 1.0: espera que la API y el modelo de
datos sigan evolucionando hasta que se anuncie una línea estable. Consulta la
[guía de contribución](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
y la
[gobernanza](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)
para participar.

## Dirección para los próximos doce meses {#direction-for-the-next-twelve-months}

Son intenciones, no compromisos, más o menos por orden.

- **Hacia la 1.0.** Fijar la API HTTP y el esquema de la base de datos para poder
  anunciar una línea estable; hasta entonces las versiones menores aún pueden
  cambiarlos, con notas de actualización en el registro de cambios.
- **Más personas con acceso.** Un segundo administrador del repositorio, del
  registro de imágenes, del alojamiento de la documentación y de los dominios
  (consulta la
  [gobernanza](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)).
- **Cadena de suministro.** Compilaciones de versiones reproducibles
  ([estado actual](reproducible-builds.md)) y etiquetas de versión firmadas.
- **Imágenes arm64.** Las imágenes del servidor son solo amd64 por ahora; arm64
  volverá con ejecutores de compilación nativos.
- **Integraciones y paquetes.** Más integraciones de dispositivos y servicios y
  más perfiles SNMP, guiados por contribuciones y peticiones.
- **Documentación.** Mantener los nueve idiomas de la interfaz y de la
  documentación al día con el original en inglés.

## Lo que DumbMonit no pretende hacer {#what-dumbmonit-does-not-intend-to-do}

- Convertirse en una plataforma de registros, trazas o APM: monitoriza
  dispositivos y servicios.
- Ofrecer una edición de pago o un servicio alojado: sigue siendo 100 % código
  abierto.
- Enviar telemetría de tu instancia a ninguna parte.
- Apuntar a flotas de miles de dispositivos: está pensado para homelabs y
  equipos pequeños, en un solo contenedor.
