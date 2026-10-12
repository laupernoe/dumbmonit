# DumbMonit aktualisieren

DumbMonit sagt Ihnen, wenn eine neuere Version existiert, und führt Sie
durch das Update. Es installiert nie etwas von selbst.

## Eine neue Version entdecken

Höchstens einmal täglich fragt der Server die öffentliche Releases-API
von GitHub (`api.github.com/repos/laupernoe/dumbmonit/releases`) nach der
neuesten Version und vergleicht sie mit der von Ihnen betriebenen. Vorab-
Veröffentlichungen wie `0.1.0-alpha.7` werden korrekt verglichen:
`alpha.10` ist neuer als `alpha.9`, und `0.1.0` ist neuer als jedes
`0.1.0-alpha.N`. Entwürfe werden ignoriert, und einer Instanz auf einer
stabilen Version wird nie eine Vorab-Veröffentlichung angeboten.

Wenn eine neuere Version existiert, erscheint **Update available: vX**
unten rechts auf jeder Seite und unter **Settings → About**, wo **How to
update** die folgenden Schritte öffnet und **See what's new** das Fenster
*What's new* erneut öffnet. **Check now** prüft sofort erneut (höchstens
einmal pro Minute).

!!! note "Was gesendet wird"
    Ein anonymer `GET`-Request mit einem `DumbMonit/<version>`-User-Agent.
    Nichts über Ihre Instanz, Ihre Geräte oder Ihr Netzwerk wird gesendet,
    und die Adresse ist fest im Code verankert.

### Die Prüfung ausschalten

Für Air-Gapped-Netzwerke schalten Sie **Check for new versions** unter
**Settings → About** (Administratoren) aus, oder setzen Sie die
Umgebungsvariable, die über den Schalter gewinnt:

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

Dieselbe Information steht Skripten und API-Tokens unter
`GET /api/update` zur Verfügung.

## Aktualisieren

1. **Sichern.** Laden Sie ein Backup unter **Settings → Backup** herunter,
   und behalten Sie eine Kopie von `/data/secret.key` (siehe
   [Backup und Wiederherstellung](backup.md)). Ohne diesen Schlüssel
   können gespeicherte Passwörter und Tokens nicht entschlüsselt werden.
2. **Pullen und neu starten**, aus dem Ordner, der Ihre
   `docker-compose.yml` enthält:

    ```bash
    docker compose pull && docker compose up -d
    ```

    Ihre Daten liegen im `/data`-Volume und bleiben erhalten;
    Datenbankmigrationen laufen beim Start.
3. **Seite neu laden.** **Settings → About** zeigt die neue Version und
   *You are up to date*.

Um eine Version festzupinnen, statt `:latest` zu folgen, ändern Sie den
Image-Tag in der Compose-Datei (zum Beispiel
`ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`) und führen Sie denselben
Befehl aus.

Um zurückzugehen, stellen Sie das in Schritt 1 erstellte Backup auf dem
vorherigen Image wieder her.
