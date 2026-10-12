# Push-Benachrichtigungen (PWA)

DumbMonit kann Ihr Telefon oder Ihren Computer selbst läuten lassen,
genau wie eine native App es tut, ohne eine App zu installieren oder
irgendwo ein Konto anzulegen: der eigene Push-Dienst des Browsers
(Google, Mozilla, Apple, Microsoft) trägt eine Ende-zu-Ende-
verschlüsselte Nachricht von Ihrem Server zum Gerät, und die
Benachrichtigung zeigt sich selbst bei geschlossenen DumbMonit-Tabs.
Das ist der Standard **Web Push**; die Site kann auch als App
installiert werden (eine *Progressive Web App*, PWA), sodass sie sich
über ein Icon öffnet wie jede andere.

Zwei Schritte: Push auf jedem Gerät aktivieren (diese Seite), dann mit
einem **Web-Push**-Kanal entscheiden, welche Alarme dort landen
([Benachrichtigungskanäle](../notifications.md#web-push)).

## Bevor Sie beginnen

- **HTTPS wird benötigt.** Browser bieten Push nur einer sicheren
  Origin an. Stellen Sie DumbMonit hinter einen Reverse Proxy mit
  Zertifikat (Caddy, Traefik, Nginx Proxy Manager…) und öffnen Sie es
  über diese Adresse. `http://localhost` wird ebenfalls akzeptiert, was
  nur zum Testen auf dem Server selbst nützlich ist. Bei einfachem
  `http://192.168.x.x` sagt der Abschnitt **Needs HTTPS**.
- **Der Server braucht ausgehendes HTTPS** zu den Push-Diensten:
  `fcm.googleapis.com` (Chrome, und Edge auf Android),
  `*.notify.windows.com` (Edge auf Windows),
  `updates.push.services.mozilla.com` (Firefox) und
  `*.push.apple.com` (Safari, iPhone, iPad). Nichts kommt herein: Ihr
  DumbMonit muss nicht aus dem Internet erreichbar sein, das Telefon
  muss es nur erreichen, wenn Sie auf eine Benachrichtigung tippen.
- **Setzen Sie `DUMBMONIT_PUBLIC_URL`** auf die genutzte HTTPS-Adresse:
  Sie wird den Push-Diensten als Kontakt des Absenders angegeben, und
  es ist die Adresse, auf die die Links in anderen Kanälen zeigen.

## Auf einem Telefon oder Computer aktivieren

### Android, Windows, macOS, Linux

1. Öffnen Sie DumbMonit in Chrome, Edge, Firefox oder Safari und loggen
   Sie sich ein.
2. Optional: als App installieren (Adressleiste → **Install**, oder
   Browser-Menü → **Add to Home screen**). Benachrichtigungen
   funktionieren beides.
3. **Settings → Push notifications → Enable on this device**, und
   Benachrichtigungen erlauben, wenn der Browser fragt.
4. **Send a test.** Eine *DumbMonit — test notification* sollte
   innerhalb weniger Sekunden erscheinen.

### iPhone und iPad

Apple gewährt Push nur Sites, die dem Homescreen hinzugefügt wurden
(iOS und iPadOS 16.4 oder später):

1. Öffnen Sie DumbMonit in **Safari**.
2. **Share → Add to Home Screen**, öffnen Sie DumbMonit dann **über
   das neue Icon**.
3. Loggen Sie sich ein, dann **Settings → Push notifications → Enable
   on this device**, und Benachrichtigungen erlauben.
4. **Send a test.**

In Safari ohne Installation sagt der Abschnitt **Install first**.

## Wählen, welche Alarme ankommen

Fügen Sie unter **Alerts → Notifications** einen Kanal vom Typ
**Web Push** hinzu. Ohne Einstellung erhält jedes abonnierte Gerät
jedes Kontos ihn; listen Sie Kontonamen unter **Accounts**, um nur
bestimmte Personen anzusprechen. Die übliche Kanalrichtlinie gilt
(Mindest-Schweregrad, Ruhezeiten, Routing-Filter…), sodass Sie zum
Beispiel ein Telefon nur für kritische Alarme behalten können.

Eine Benachrichtigung liest sich `Critical · nas01` mit einer kurzen
Zusammenfassung; das Antippen öffnet die **Alerts**-Seite. Die
Behebung eines Alarms ersetzt seine Benachrichtigung statt eine zweite
hinzuzufügen.

## Geräte und Konten

Jedes Konto sieht und verwaltet seine eigenen Geräte: Betrachter können
Push auch auf ihrem Telefon aktivieren, das ändert sonst nichts. Die
Liste zeigt Browser und System, den Push-Dienst, wann das Gerät
hinzugefügt wurde und wann es zuletzt eine Nachricht erhielt;
**Remove** vergisst es (und, auf dem gerade genutzten Gerät, meldet
auch den Browser ab). Ein Gerät, das der Push-Dienst als verschwunden
meldet (die App wurde entfernt, Browser-Daten gelöscht), wird beim
nächsten Versand automatisch gelöscht.

## Fehlerbehebung

| Was Sie sehen | Was zu tun ist |
|---|---|
| **Needs HTTPS** | Öffnen Sie DumbMonit über seine HTTPS-Adresse. |
| **Install first** (iPhone, iPad) | Share → Add to Home Screen, über das Icon öffnen. |
| **Blocked** | Benachrichtigungen wurden für diese Site abgelehnt: erlauben Sie sie in den Site-Einstellungen des Browsers (Schloss in der Adressleiste, oder iOS Settings → Notifications → DumbMonit), dann neu laden. |
| **Not supported** | Der Browser hat kein Web Push (alte Version, manche Privacy-Browser): nutzen Sie ein aktuelles Chrome, Edge, Firefox oder Safari. |
| Test sagt *push service unreachable* | Der Server kann den Push-Dienst nicht erreichen: prüfen Sie dessen ausgehendes HTTPS und DNS. |
| Test sagt *refused the VAPID signature* | Meist eine falsche Serveruhr; prüfen Sie NTP auf dem Host. |
| Nichts kommt auf Android an, wenn das Telefon schläft | Akku-Optimierung kann den Browser verzögern: davon ausnehmen, oder die **Urgency** des Kanals auf `high` setzen. |

## Datenschutz und Sicherheit

- Nachrichten werden für jedes Gerät verschlüsselt (RFC 8291), bevor sie
  den Server verlassen: Der Push-Dienst sieht nur einen undurchsichtigen
  Blob, das Gerät entschlüsselt ihn.
- Der Server signiert jede Nachricht mit seinem eigenen
  VAPID-Schlüsselpaar (RFC 8292), beim ersten Start erzeugt und mit
  dem Instanz-Secret verschlüsselt gespeichert; der private Schlüssel
  wird von der API nie zurückgegeben. Eine
  [Wiederherstellung](../install/backup.md) auf einer neuen Maschine
  erzeugt ein neues Schlüsselpaar: Aktivieren Sie Push auf jedem Gerät
  erneut.
- Die Abonnement-Adresse stammt vom Browser, daher akzeptiert der
  Server nur HTTPS-Adressen auf öffentlichen IPs: private,
  Loopback- und Link-Local-Adressen werden abgelehnt, zum Zeitpunkt
  des Abonnements und erneut bei jeder Verbindung.
- Der Service Worker (`/sw.js`) zeigt nur Benachrichtigungen. Er cacht
  nichts und fängt keinen Request ab: Die Oberfläche und die API
  kommen immer vom Server, nie von einer veralteten Kopie.
