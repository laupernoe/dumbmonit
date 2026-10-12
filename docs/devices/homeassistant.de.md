# Home Assistant

Das Smart Home hinter dem Dashboard: ob Home Assistant läuft oder im
Recovery-Modus gestartet ist, Entitäten, die nach Domäne nicht
verfügbar oder unbekannt sind, schwache Akkus, wartende Updates (Core,
Betriebssystem, Add-ons, Gerätefirmware) und die offenen Repairs.

DumbMonit liest Home Assistants REST-API mit dem langlebigen
Access-Token eines dedizierten Benutzers **ohne Administratorrechte**,
und die Liste der Repairs über die WebSocket-API. Es ruft nie einen
Service, löst nie ein Event aus und ändert nie einen Zustand.

Der beobachtenswerte Fehler ist nicht "Home Assistant antwortet nicht"
(das sieht schon ein [Website-Check](services.md#http)) sondern
**Home Assistant antwortet, und etwas dahinter hat aufgehört zu
funktionieren**: eine Integration, deren Entitäten nach einem Update
alle nicht verfügbar wurden, ein Zigbee-Stick, der vom USB-Bus fiel,
ein Türsensor, dessen Akku 5 % erreicht hat, ein seit Wochen wartender
Repair.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_homeassistant_`.

| Metrik | Was | Labels |
|---|---|---|
| `info` | Wert 1 | `version` |
| `running` | 1, wenn der Core-Zustand `RUNNING` ist, 0, während er startet oder stoppt | |
| `recovery_mode` | 1 im Recovery-Modus (oder Safe Mode auf älteren Versionen): die Konfiguration konnte nicht geladen werden | |
| `entities` | Entitäten nach Domäne | `domain` |
| `entities_unavailable`, `entities_unknown` | Entitäten in diesem Zustand, nach Domäne | `domain` |
| `entity_unavailable` | Wert 1 pro nicht verfügbarer Entität, höchstens 50 | `entity`, `name`, `domain` |
| `batteries` | Akku-Sensoren und Akku-Binärsensoren | |
| `batteries_low` | Akkus unter dem Schwellenwert, und Akku-Binärsensoren, die an sind | |
| `battery_low` | eine Zeitreihe pro schwachem Akku, Wert = sein Stand in Prozent (0 für einen Binärsensor), höchstens 50 | `entity`, `name` |
| `updates_available` | eingeschaltete `update.*`-Entitäten | |
| `update_available` | Wert 1 pro wartendem Update, höchstens 50 | `entity`, `name`, `installed`, `latest` |
| `repairs` | offene Repairs (weder ignoriert noch verworfen) nach Schweregrad: `critical`, `error`, `warning` | `severity` |
| `repair` | Wert 1 pro offenem Repair, höchstens 50 | `issue`, `domain`, `severity` |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Manche Domänen ruhen von Natur aus im Zustand `unknown`: eine nie
gedrückte Taste, ein nie empfangenes Event, ein Benachrichtigungs-
oder Sprachdienst. Sie werden nicht als unknown gezählt: `button`,
`input_button`, `event`, `scene`, `notify`, `tts`, `stt`,
`conversation`, `wake_word`, `ai_task`, `person`. Die Option
**Domains left out** entfernt weitere Domänen (zum Beispiel
`device_tracker` oder `media_player` für oft ausgeschaltete Geräte)
aus den Zählungen von unavailable und unknown.

Repairs sind nicht in der REST-API: DumbMonit öffnet die WebSocket-API,
sendet `repairs/list_issues` und schließt sie. Ein Benutzer ohne
Administratorrechte darf sie auflisten. Die Option **Read the
repairs** schaltet das ab.

Die geltenden [eingebauten Regeln](../alerting/rules.md#home-assistant):
Home Assistant in recovery mode, Home Assistant not running, Home
Assistant entities went unavailable, Home Assistant battery low, Home
Assistant update available, Home Assistant repair to address, plus
Device unreachable.

## Geräte

Jedes bei der Companion-App eingeloggte Telefon und Tablet (die
`mobile_app`-Integration) wird unter der generischen Familie
veröffentlicht, die sich jede DumbMonit-Integration teilt, die
Client-Geräte verfolgt — `dumbmonit_client_device_*`, ohne
`homeassistant`-Präfix, vollständig dokumentiert unter
[Client-Geräte](../alerting/rules.md#client-devices):

| Metrik | Was | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Letztes Update der `device_tracker`-Entität des Geräts | `device`, `type` (`Mobile`), `os` (leer), `user` (die verknüpfte Person, falls vorhanden), `kind` |
| `stale_seconds` | Positiv, sobald ein Gerät `device_stale_days` (standardmäßig 3) ohne Update überschritten hat; treibt die Regel [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (immer `connection`: Home Assistant hat kein Backup zu melden) |

**Wie ein Gerät erkannt wird**: `/api/states` sagt nicht, welche
Integration eine Entität erstellt hat — das braucht die Entitäts-
Registry, die die REST-API nicht bedient. Der `device_tracker` der
Companion-App ist der einzige, der ein `battery_level`-Attribut
setzt; Drittanbieter-Standort-Integrationen (Life360, OwnTracks, die
beim Testen genutzte Demo-Integration) setzen höchstens `battery`.
Eine Drittanbieter-Integration, die genau diesen Attributnamen
zufällig kopiert hätte, würde mit der Companion-App verwechselt;
umgekehrt fehlt ein Companion-App-Gerät mit ausgeschalteter
Akku-Meldung in der Tabelle, statt falsch angezeigt zu werden. Das
Konto des Geräts, wenn bekannt, kommt von einer `person.*`-Entität,
deren `device_trackers`-Attribut es benennt.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Home Assistant nie ab. Es sagt zuerst, was
nicht stimmt (Recovery-Modus, Repairs der Stufe Error, nicht
verfügbare Entitäten), dann die schwachen Akkus nach Stand, die
wartenden Updates mit ihren Versionen, die Entitäten nach Domäne, und
die Tabelle der Companion-App-Geräte mit ihrer letzten Verbindung.

## Wie es validiert wurde

Gegen Home Assistant 2026.9.4, mit dem langlebigen Token eines
Benutzers ohne Administratorrechte: Konfiguration, Entitätszustände
der Demo-Integration und von Template-Sensoren (Akkustände, ein
Akku-Binärsensor, ein nicht verfügbarer Sensor), Update-Entitäten und
die Repairs-Liste über die WebSocket-API. Die Attribute
(`battery_level`, `source_type`) des `device_tracker` der
Companion-App wurden gegen den Quellcode der `mobile_app`-Integration
geprüft, nicht gegen ein echtes Telefon.

## Einen dedizierten Benutzer und ein langlebiges Token in Home Assistant erstellen

1. In Home Assistant: Settings → People → Users → Add user (schalten
   Sie Advanced mode in Ihrem Profil ein, wenn der Users-Tab fehlt).
   Benennen Sie den Benutzer wie folgt, geben Sie ihm ein langes
   Zufallspasswort und lassen Sie den Administrator-Schalter aus. Ein
   normaler Benutzer kann jede Entität und die Repairs-Liste lesen,
   und kann die Konfiguration nicht ändern.

    ```
    dumbmonit
    ```

2. Loggen Sie sich einmal als dieser Benutzer in Home Assistant ein,
   öffnen Sie sein Profil (der Name unten in der Seitenleiste) →
   Security → Long-lived access tokens → Create token. Benennen Sie es
   nach DumbMonit und kopieren Sie es jetzt: Es wird nur einmal
   angezeigt.

3. Geben Sie in DumbMonit die Adresse von Home Assistant ein, zum
   Beispiel "homeassistant.local" (Port 8123) oder
   "https://ha.example.net" hinter einem Reverse Proxy, und fügen Sie
   das Token ein. Akkus unter 20 % zählen als schwach; die Option Low
   battery threshold ändert das.

4. DumbMonit liest nur /api/config, /api/states und, über die
   WebSocket-API, die Repairs-Liste. Es ruft nie einen Service, löst
   nie ein Event aus und ändert nie einen Zustand.

!!! warning
    Ein langlebiges Token trägt alle Rechte seines Benutzers und
    bleibt zehn Jahre gültig: Erstellen Sie es für den dedizierten
    Benutzer, nie für Ihren eigenen. Home Assistant liefert einfaches
    HTTP, sofern TLS nicht konfiguriert ist, und das Token reist mit
    jedem Request mit: Nutzen Sie über ein nicht vertrauenswürdiges
    Netzwerk HTTPS. Jedes bei der Companion-App eingeloggte Telefon
    und Tablet (die mobile_app-Integration) erscheint in der
    Devices-Tabelle unten mit seiner letzten Aktivität; es ist
    optional und bleibt leise ohne diese App.

## Credentials

| Credential | Felder |
|---|---|
| Langlebiges Access-Token | Ein Token des dedizierten Benutzers, als `Authorization: Bearer` gesendet. |

Adresse: ein Hostname oder eine IP (`homeassistant.local`),
`host:port`, oder eine vollständige URL (`https://ha.example.net`, mit
Pfadpräfix hinter einem Reverse Proxy); ein abschließendes `/api` wird
akzeptiert. Optionen: Protokoll (standardmäßig HTTP), Port (8123),
Zertifikatsprüfung, Anfrage-Timeout (10 s), Low-Battery-Schwellenwert
(20 %), ausgelassene Domänen (keine), Repairs (gelesen) und
Geräte-Veralterung (`device_stale_days`, standardmäßig 3 Tage).
