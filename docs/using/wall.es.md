# Modo pared {#wall-mode-and-music}

`/wall` muestra el tablero propio de la consola, a pantalla completa, para un
monitor que permanece encendido en una sala. Se actualiza solo y mantiene la
pantalla despierta.

De arriba abajo:

- el **veredicto** en pocas palabras («All good», «3 problems») con la fecha y el
  reloj, y cuándo se actualizó por última vez;
- las **cinco cifras** del resumen: monitores activos, disponibilidad a 30 días,
  tiempo de respuesta p95, alertas abiertas y tiempo medio de resolución;
- el **río de salud**, la gran imagen del tablero: una franja por monitor, los
  peores arriba;
- las **alertas abiertas**, las más graves primero;
- **quién está de guardia** ahora, con el calendario y el fin del turno.

Sin música y sin widgets: fondo negro, la tipografía de la consola,
estados escritos junto a una forma y nunca solo con color.

## Abrir y salir

Ábrelo con el icono **Wall mode** de la barra superior, desde la paleta de
comandos («Wall mode») o escribiendo la URL. ++esc++ o **Exit** vuelve a la
consola. Una cuenta limitada a algunos equipos o clientes no tiene el modo pared,
porque muestra toda la flota.

## Fondo {#backdrop}

La pared comparte el aspecto oscuro de la consola. El botón **Fondo** (arriba a
la derecha, junto a Exit) puede dibujar una de las ciudades de noche de las
[páginas de estado](status-pages.md) detrás del veredicto y el reloj: París,
Nueva York, Tokio o Dubái, o *Auto* para la más cercana a la zona horaria del
navegador. Está desactivado por defecto y se recuerda por navegador;
`?city=paris` (o `newyork`, `tokyo`, `dubai`, `none`) en un enlace lo elige solo
para esa pantalla. El cielo sigue al tablero: estrellas cuando todo va bien,
nubes con un aviso, tormenta con una alerta. En un teléfono, o con la pestaña
oculta, la ciudad se queda quieta. Un `zermatt` guardado o en un enlace cae ahora
en Dubái.
