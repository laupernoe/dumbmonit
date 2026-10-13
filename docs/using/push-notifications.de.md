# Push-Benachrichtigungen (PWA) {#push-notifications-pwa}

DumbMonit kann Ihr Smartphone oder Ihren Computer selbst klingeln lassen, so wie eine native
App, ohne eine App zu installieren oder irgendwo ein Konto anzulegen: Der Push-Dienst des
Browsers (Google, Mozilla, Apple, Microsoft) transportiert eine Ende-zu-Ende-verschlüsselte
Nachricht von Ihrem Server zum Gerät, und die Benachrichtigung erscheint auch bei geschlossenen
DumbMonit-Tabs. Das ist der Standard **Web Push**; die Website lässt sich zusätzlich als App
installieren (eine *Progressive Web App*, PWA), sodass sie wie jede andere über ein Symbol
startet.

Zwei Schritte: Push auf jedem Gerät aktivieren (diese Seite), dann mit einem **Web-Push**-Kanal
festlegen, welche Alarme dorthin gehen ([Benachrichtigungskanäle](../notifications.md#web-push)).

## Bevor Sie beginnen {#before-you-start}

- **HTTPS ist erforderlich.** Browser bieten Push nur für einen sicheren Origin an. Stellen Sie
  DumbMonit hinter einen Reverse-Proxy mit Zertifikat (Caddy, Traefik, Nginx Proxy Manager …)
  und öffnen Sie es über diese Adresse. `http://localhost` wird ebenfalls akzeptiert, was nur
  zum Testen auf dem Server selbst nützlich ist. Bei einfachem `http://192.168.x.x` zeigt der
  Abschnitt **HTTPS erforderlich**.
- **Der Server braucht ausgehendes HTTPS** zu den Push-Diensten: `fcm.googleapis.com` (Chrome
  und Edge auf Android), `*.notify.windows.com` (Edge unter Windows),
  `updates.push.services.mozilla.com` (Firefox) und `*.push.apple.com` (Safari, iPhone, iPad).
  Es kommt nichts herein: Ihr DumbMonit muss aus dem Internet nicht erreichbar sein, das
  Smartphone muss es nur erreichen, wenn Sie auf eine Benachrichtigung tippen.
- **Setzen Sie `DUMBMONIT_PUBLIC_URL`** auf die verwendete HTTPS-Adresse: Sie wird den
  Push-Diensten als Kontakt des Absenders mitgeteilt und ist die Adresse, auf die die Links in
  anderen Kanälen zeigen.

## Auf einem Smartphone oder Computer aktivieren {#enable-on-a-phone-or-computer}

### Android, Windows, macOS, Linux {#android-windows-macos-linux}

1. Öffnen Sie DumbMonit in Chrome, Edge, Firefox oder Safari und melden Sie sich an.
2. Optional: als App installieren (Adressleiste → **Installieren**, oder Browsermenü →
   **Zum Startbildschirm hinzufügen**). Benachrichtigungen funktionieren in beiden Fällen.
3. **Einstellungen → Push-Benachrichtigungen → Auf diesem Gerät aktivieren** und
   Benachrichtigungen erlauben, wenn der Browser fragt.
4. **Test senden.** Eine *DumbMonit — Testbenachrichtigung* sollte innerhalb weniger Sekunden
   erscheinen.

### iPhone und iPad {#iphone-and-ipad}

Apple erlaubt Push nur für Websites, die zum Home-Bildschirm hinzugefügt wurden (iOS und iPadOS
16.4 oder neuer):

1. Öffnen Sie DumbMonit in **Safari**.
2. **Teilen → Zum Home-Bildschirm**, dann öffnen Sie DumbMonit **über das neue Symbol**.
3. Melden Sie sich an, dann **Einstellungen → Push-Benachrichtigungen → Auf diesem Gerät
   aktivieren** und Benachrichtigungen erlauben.
4. **Test senden.**

In Safari ohne Installation zeigt der Abschnitt **Zuerst installieren**.

## Auswählen, welche Alarme ankommen {#choose-which-alerts-arrive}

Fügen Sie unter **Alarme → Benachrichtigungen** einen Kanal vom Typ **Web Push** hinzu. Ohne
Einstellung erhält ihn jedes abonnierte Gerät jedes Kontos; tragen Sie Kontonamen unter
**Konten** ein, um nur bestimmte Personen anzusprechen. Es gilt die übliche Kanalrichtlinie
(Mindestschweregrad, Ruhezeiten, Routing-Filter …), sodass Sie zum Beispiel ein Smartphone nur
für kritische Alarme vorsehen können.

Eine Benachrichtigung lautet `Kritisch · nas01` mit einer kurzen Zusammenfassung; ein Tipp
darauf öffnet die Seite **Alarme**. Die Auflösung eines Alarms ersetzt dessen Benachrichtigung,
statt eine zweite hinzuzufügen.

## Geräte und Konten {#devices-and-accounts}

Jedes Konto sieht und verwaltet seine eigenen Geräte: Auch Betrachter können Push auf ihrem
Smartphone aktivieren, das ändert sonst nichts. Ein Konto hat höchstens 10 Geräte (entfernen Sie
ein altes, um ein weiteres hinzuzufügen), und ein Test kann alle 30 Sekunden gesendet werden.
Die Liste zeigt Browser und System, den Push-Dienst, wann das Gerät hinzugefügt wurde und wann
es zuletzt eine Nachricht erhielt; **Entfernen** vergisst es (und meldet auf dem verwendeten
Gerät auch den Browser ab). Ein Gerät, das der Push-Dienst als verschwunden meldet (App
entfernt, Browserdaten gelöscht), wird beim nächsten Senden automatisch gelöscht.

## Fehlerbehebung {#troubleshooting}

| Was Sie sehen | Was zu tun ist |
|---|---|
| **HTTPS erforderlich** | Öffnen Sie DumbMonit über die HTTPS-Adresse. |
| **Zuerst installieren** (iPhone, iPad) | Teilen → Zum Home-Bildschirm, über das Symbol öffnen. |
| **Blockiert** | Benachrichtigungen wurden für diese Website abgelehnt: Erlauben Sie sie in den Website-Einstellungen des Browsers (Schloss in der Adressleiste, oder iOS-Einstellungen → Mitteilungen → DumbMonit), dann neu laden. |
| **Nicht unterstützt** | Der Browser kennt kein Web Push (alte Version, manche Datenschutz-Browser): Verwenden Sie ein aktuelles Chrome, Edge, Firefox oder Safari. |
| Test meldet *Push-Dienst nicht erreichbar* | Der Server erreicht den Push-Dienst nicht: Prüfen Sie ausgehendes HTTPS und DNS. |
| Test meldet *VAPID-Signatur abgelehnt* | Meist eine falsche Serveruhr; prüfen Sie NTP auf dem Host. |
| Auf Android kommt nichts an, wenn das Smartphone schläft | Die Akkuoptimierung kann den Browser verzögern: Nehmen Sie ihn aus, oder setzen Sie die **Dringlichkeit** des Kanals auf `high`. |

## Datenschutz und Sicherheit {#privacy-and-security}

- Nachrichten werden für jedes Gerät verschlüsselt (RFC 8291), bevor sie den Server verlassen:
  Der Push-Dienst sieht nur einen undurchsichtigen Blob, das Gerät entschlüsselt ihn.
- Der Server signiert jede Nachricht mit seinem eigenen VAPID-Schlüsselpaar (RFC 8292), das
  beim ersten Start erzeugt und mit dem Instanzgeheimnis verschlüsselt gespeichert wird; der
  private Schlüssel wird von der API nie zurückgegeben. Eine [Wiederherstellung](../install/backup.md)
  auf einem neuen Rechner erzeugt ein neues Schlüsselpaar: Aktivieren Sie Push auf jedem Gerät
  erneut.
- Die Abonnementadresse stammt vom Browser, daher akzeptiert der Server nur HTTPS-Adressen auf
  öffentlichen IPs: Private, Loopback- und Link-Local-Adressen werden abgelehnt, bei der
  Anmeldung des Abonnements und bei jeder Verbindung erneut.
- Der Service Worker (`/sw.js`) zeigt nur Benachrichtigungen an. Er cacht nichts und fängt
  keine Anfrage ab: Oberfläche und API kommen immer vom Server, nie aus einer veralteten Kopie.
