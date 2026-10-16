# Plex Media Server

Was Plex gerade tut und ob es aktuell ist: jetzt laufende Streams, wie
viele transkodiert werden und ob in Hardware, die dafür reservierte
Bandbreite, Streams von außerhalb des Hauses, gerade gescannte
Bibliotheken, und Updates.

DumbMonit liest die eigene API des Servers auf Port 32400, nie plex.tv.
Es stoppt nie einen Stream, scannt nie eine Bibliothek und ändert nie
eine Einstellung.

Plex veröffentlicht nicht das Ergebnis seiner Wartungs-Tasks (der
"Butler" gibt nur ihren Zeitplan an), noch den Zustand seiner
Datenbank-Backups, daher gibt es dazu nichts zu sagen.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_plex_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `update_available` | 1, wenn Plex' eigener Update-Check eine neuere Version gefunden hat; fehlt, bis Plex einmal geprüft hat | `latest_version` |
| `claimed` | 1, wenn der Server mit einem Plex-Konto verknüpft ist | |
| `streams`, `streams_paused`, `streams_remote` | jetzige Wiedergaben; pausierte; von außerhalb des lokalen Netzwerks | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | neu kodierte Wiedergaben (Video oder Audio); mit neu kodiertem Video; mit Hardware-Beschleunigung | |
| `transcoder_active_video_sessions` | Video-Transkodierungen, die der Server selbst zählt | |
| `stream_bandwidth_bits_per_second` | von allen Wiedergaben reservierte Bandbreite | |
| `libraries`, `libraries_scanning` | Bibliotheken; jetzt gescannte | |
| `activities` | laufende Hintergrundaktivitäten (Scans, Analyse, Updates) | |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Die geltende [eingebaute Regel](../alerting/rules.md#self-hosted-applications):
Plex update available, plus Monitor unreachable.

## Die Monitorseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite ruft Plex nie auf. Es nennt den Update-Zustand und ob
der Server verknüpft ist, dann Streams, Transkodierungen,
Hardware-Transkodierungen, Remote-Streams, Bandbreite und Bibliotheken.

## DumbMonit Zugriff auf den Plex-Server geben

1. DumbMonit liest die eigene API des Servers auf Port 32400, nie
   plex.tv. Ein mit einem Plex-Konto verknüpfter Server verlangt ein
   Token (X-Plex-Token), und Plex kann kein eingeschränktes ausstellen:
   es ist das Token des Besitzers.

2. Entweder das Token vermeiden: In Plex, Settings → Network → "List of
   IP addresses and networks that are allowed without auth", die
   Adresse des DumbMonit-Servers hinzufügen, zum Beispiel wie folgt, und
   unten Kein Token wählen.

    ```
    10.0.0.20/32
    ```

3. Oder das Token nutzen: In Plex Web ein beliebiges Film öffnen → ⋯ →
   Get Info → View XML. Die Adresse der sich öffnenden Seite endet mit
   X-Plex-Token= gefolgt vom Token; diesen Wert kopieren.

4. Geben Sie in DumbMonit die Adresse des Servers ein, zum Beispiel
   "plex.lan" (Port 32400). DumbMonit liest nur Sitzungen, Bibliotheken,
   den Updater und die Aktivitätsliste; es stoppt nie einen Stream und
   ändert nie eine Einstellung.

!!! warning
    Das Token gehört Ihrem Plex-Konto: Wer es besitzt, kann den Server
    und jede Bibliothek verwalten. Bevorzugen Sie das erlaubte Netzwerk
    oben, wenn DumbMonit in Ihrem eigenen Netzwerk sitzt.

Das erlaubte Netzwerk ist auch das, was Plex ohne Token aus Docker
antworten lässt: Der Container von DumbMonit erreicht Plex von seiner
Docker-Netzwerkadresse aus, also listen Sie dieses Netzwerk, oder die
Adresse des Hosts, wenn Plex seinen Port auf dem Host veröffentlicht.

## Credentials

| Credential | Felder |
|---|---|
| X-Plex-Token | Im Header `X-Plex-Token` gesendet. |
| Kein Token (erlaubtes Netzwerk) | Nichts wird gesendet; Plex muss die Adresse von DumbMonit unter den ohne Authentifizierung erlaubten Netzwerken listen. |

Adresse: ein Hostname (`plex.lan`), `host:port`, oder eine vollständige
URL; ein abschließendes `/web` wird akzeptiert. Optionen: Protokoll
(standardmäßig HTTP; Plex' eigene Zertifikate werden für `*.plex.direct`-
Namen ausgestellt, daher braucht HTTPS per IP-Adresse die ausgeschaltete
Zertifikatsprüfung), Port (32400), Zertifikatsprüfung und Anfrage-Timeout
(15 s).

Validiert gegen Plex Media Server 1.43.4 (offizielles Image), einen
nicht verknüpften Server, der ohne Token antwortet, mit einer echten
transkodierten Wiedergabe. Der Token-Pfad folgt Plex' dokumentiertem
`X-Plex-Token`-Header und wurde nicht gegen einen verknüpften Server
getestet.
