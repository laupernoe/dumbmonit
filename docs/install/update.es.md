# Actualizar DumbMonit {#update-dumbmonit}

DumbMonit te avisa cuando existe una versión más reciente y te guía a través
de la actualización. Nunca instala nada por sí mismo.

## Detectar una nueva versión {#spotting-a-new-version}

Como máximo una vez al día, el servidor consulta la API pública de releases
de GitHub (`api.github.com/repos/laupernoe/dumbmonit/releases`) para
obtener la última versión y la compara con la que estás ejecutando. Las
versiones previas como `0.1.0-alpha.7` se comparan correctamente:
`alpha.10` es más reciente que `alpha.9`, y `0.1.0` es más reciente que
cualquier `0.1.0-alpha.N`. Los borradores se ignoran, y a una instancia en
una versión estable nunca se le ofrece una versión previa.

Cuando existe una versión más reciente, aparece **Update available: vX**
en la parte inferior derecha de cada página y en **Settings → About**, donde
**How to update** abre los pasos siguientes y **See what's new** reabre la
ventana *What's new*. **Check now** vuelve a comprobarlo de inmediato (como
máximo una vez por minuto).

!!! note "Qué se envía"
    Una única petición `GET` anónima con un user agent `DumbMonit/<version>`.
    No se envía nada sobre tu instancia, tus dispositivos ni tu red, y la
    dirección está fija en el código.

### Desactivar la comprobación {#turning-the-check-off}

Para redes aisladas (air-gapped), desactiva **Check for new versions** en
**Settings → About** (administradores), o define la variable de entorno,
que prevalece sobre el interruptor:

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

La misma información está disponible para scripts y tokens de API en
`GET /api/update`.

## Actualizar {#updating}

1. **Haz una copia de seguridad.** Descarga una copia de seguridad desde
   **Settings → Backup**, y guarda una copia de `/data/secret.key` (consulta
   [Copia de seguridad y restauración](backup.md)). Sin esa clave, las
   contraseñas y tokens guardados no se pueden descifrar.
2. **Descarga y reinicia**, desde la carpeta que contiene tu
   `docker-compose.yml`:

    ```bash
    docker compose pull && docker compose up -d
    ```

    Tus datos viven en el volumen `/data` y se conservan; las migraciones de
    base de datos se ejecutan al iniciar.
3. **Recarga la página.** **Settings → About** muestra la nueva versión y
   *You are up to date*.

Para fijar una versión en lugar de seguir `:latest`, cambia la etiqueta de
la imagen en el archivo Compose (por ejemplo
`ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`) y ejecuta el mismo comando.

Para volver atrás, restaura la copia de seguridad hecha en el paso 1 sobre
la imagen anterior.
