# Sicherheitsbewertung

Die Sicherheitsbewertung ist eine **optionale** Note von **A** bis **F** (und
ein Score von 0 bis 100), die neben den Geräten angezeigt wird, bei denen einige
Prüfungen etwas Nützliches aussagen: Linux- und Windows-Hosts mit dem Agent,
Proxmox VE und Proxmox Backup Server, Synology, OPNsense, Websites und Active
Directory. Andere Geräte zeigen nichts an.

!!! warning "Orientierungshilfe, kein Audit"
    Der Score ist **ungefähr und nicht erschöpfend**. Er liest nur, was
    DumbMonit ohnehin von einem Gerät erfasst, und vergleicht es mit einer
    Handvoll Best Practices des Herstellers. Eine Note A bedeutet nicht, dass
    ein Gerät sicher ist, und eine fehlende Prüfung bedeutet nicht, dass die
    Einstellung in Ordnung ist. Nutzen Sie ihn als Anstoß, genauer
    hinzusehen, und setzen Sie dann ein echtes Bewertungswerkzeug ein
    (PingCastle, Mozilla HTTP Observatory, Lynis, den Security Advisor des
    Herstellers …), wenn es darauf ankommt.

## Einschalten

Die Bewertung ist **standardmäßig ausgeschaltet**. Ein Administrator schaltet
sie unter **Einstellungen → Konto und Sicherheit → Sicherheitsbewertung** ein.
Danach gilt:

- jedes bewertete Gerät erhält in der Geräteliste ein kleines
  **Schild-Abzeichen** mit seinem Buchstaben und auf seiner Seite ein
  **Security**-Abzeichen mit dem Score;
- ein Klick auf das Abzeichen öffnet den **Bericht** des Geräts: was
  fehlschlägt, mit Nachweis und Abhilfe, was besteht, die Schritte zur
  Verbesserung des Scores für diese Geräteart und ein Link zum passenden
  Abschnitt unten.

Beim Ausschalten verschwinden die Abzeichen, die periodische Berechnung
stoppt, und die Serien `dumbmonit_security_score` werden nicht mehr
geschrieben.

!!! note "Update von 0.1.0-alpha.8 oder früher"
    Der Score war früher immer aktiv und hatte eine eigene Seite
    **Security**. Eine Installation, die bereits ein bewertbares Gerät
    überwachte, behält den Score nach dem Update eingeschaltet; eine neue
    Installation startet mit ausgeschaltetem Score. Die alte Adresse
    `/security` öffnet jetzt die Einstellung.

## Wie der Score berechnet wird

DumbMonit fragt das Gerät für die Berechnung nie etwas Zusätzliches ab. Die
Prüfungen lesen die letzten Werte, die die Collectors bereits aufgezeichnet
haben (Rückblick 6 Stunden), ein Score kostet das Gerät also nichts.

Jede Prüfung hat eine Kategorie (Exposition, Patching, Authentifizierung,
Verschlüsselung, Backup, Konfiguration), einen **Schweregrad**, der ihr
Gewicht ist (critical 10, high 6, medium 3, low 1), ein Ergebnis
(**bestanden**, **fehlgeschlagen** oder **unbekannt**, wenn die Daten nicht
erfasst werden), den Nachweis, die Abhilfe und einen Link zur Herstellerseite,
auf der sie beruht.

**Score** = 100 × (Gewicht der bestandenen Prüfungen) ÷ (Gewicht der
bestandenen + fehlgeschlagenen Prüfungen). **Unbekannte Prüfungen werden
angezeigt, aber nie mitgezählt**: Ein Score darf nicht sinken, weil einem
Überwachungskonto ein Recht fehlt, und nicht steigen, weil nichts bekannt ist.
Ein Gerät, dessen Prüfungen alle unbekannt sind, ist *nicht bewertet*.

| Note | Score | Wort |
|---|---|---|
| A | 90–100 | Stark |
| B | 75–89 | Gut |
| C | 60–74 | Ausreichend |
| D | 40–59 | Schwach |
| F | 0–39 | Mangelhaft |

**Eine fehlgeschlagene kritische Prüfung begrenzt die Note auf C**, egal wie
hoch der Score ist.

## Historie und Alarme

Solange die Bewertung eingeschaltet ist, berechnet der Server sie alle 15
Minuten neu und speichert sie als `dumbmonit_security_score{target="<id>"}`
(und die Anzahl fehlgeschlagener Prüfungen als
`dumbmonit_security_checks_failed`). Zwei eingebaute Regeln nutzen sie:
**Security score dropped** (10 Punkte oder mehr unter dem besten Wert der
letzten 24 Stunden) und **Security score low** (unter 40). Siehe
[Alarmregeln](../alerting/rules.md#security-score).

## Prüfungen und wie man sie verbessert

### Linux-Hosts { #linux-hosts }

Maschinen mit dem Agent. Die SSH- und Firewall-Prüfungen brauchen einen Agent
desselben Releases wie der Server; mit einem älteren Agent sind sie unbekannt.

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Security updates installed | high | ausstehende Sicherheitsupdates (wo der Paketmanager sie unterscheidet) |
| No pending updates | low | ausstehende Updates |
| No reboot pending | medium | Neustart erforderlich |
| SSH root login with a password refused | high | `PermitRootLogin` aus `sshd -T` (`prohibit-password` besteht) |
| SSH accepts keys only | medium | `PasswordAuthentication` und `KbdInteractiveAuthentication` mit `UsePAM` |
| Host firewall filters inbound traffic | high | eine nftables- oder iptables-Input-Kette, die verwirft oder ablehnt |
| SELinux enforcing | medium | SELinux-Modus (unbekannt, wo SELinux fehlt) |
| Container images up to date | medium | neueres Image in der Registry (nur mit Containern) |
| No container image older than 180 days | low | Alter des Images (nur mit Containern) |

So verbessern Sie sie:

1. Installieren Sie ausstehende Sicherheitsupdates und schalten Sie dann
   automatische Sicherheitsupdates ein: `unattended-upgrades` unter Debian und
   Ubuntu, `dnf-automatic` unter Fedora und RHEL.
2. Starten Sie nach Kernel- oder Bibliotheksupdates neu.
3. Prüfen Sie, dass Ihr SSH-Schlüssel funktioniert, setzen Sie dann
   `PermitRootLogin no`, `PasswordAuthentication no` und
   `KbdInteractiveAuthentication no` in `/etc/ssh/sshd_config` und laden Sie
   sshd neu.
4. Schalten Sie eine Host-Firewall ein, die unaufgeforderten eingehenden
   Verkehr verwirft: `ufw enable` unter Ubuntu, firewalld unter Fedora und
   RHEL, anderswo ein nftables-Regelwerk.
5. Lassen Sie SELinux auf enforcing, wo die Distribution es mitliefert; holen
   Sie regelmäßig neuere Container-Images.

Quellen: [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Windows-Hosts { #windows-hosts }

Maschinen mit dem Windows-Agent, aus demselben Release wie der Server (aus der
Registry gelesen; ein älterer Agent meldet keine dieser Prüfungen).

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Windows Firewall on for every profile | high | Domänen-, privates und öffentliches Profil (Gruppenrichtlinie zuerst) |
| No reboot pending | medium | Windows Update oder Komponentenwartung wartet auf einen Neustart |
| Remote Desktop requires Network Level Authentication | high | besteht, wenn Remotedesktop aus ist |
| SMBv1 disabled | high | Einstellung des SMBv1-Servers |

So verbessern Sie sie:

1. Installieren Sie Windows-Updates (Einstellungen → Windows Update) und
   starten Sie neu, wenn dazu aufgefordert wird.
2. Lassen Sie die Windows Defender Firewall für das Domänen-, das private und
   das öffentliche Profil eingeschaltet.
3. Wenn Remotedesktop eingeschaltet ist, verlangen Sie die Authentifizierung
   auf Netzwerkebene (NLA) und erreichen Sie ihn über ein VPN, nie direkt aus
   dem Internet.
4. Entfernen Sie SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Quellen: [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE { #proxmox-ve }

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Security updates installed | high | ausstehende Sicherheitsupdates je Node (braucht `Sys.Modify` am Token, sonst unbekannt) |
| Proxmox VE packages up to date | medium | aktualisierbare Proxmox-Pakete je Node |
| Running the installed kernel | medium | neuerer Kernel installiert als der laufende |
| No test repository in use | medium | aktivierte Standard-Repositories |
| Package repositories correctly configured | medium | Fehler und Warnungen, die Proxmox VE zu den Repositories meldet |
| Enterprise repository subscription | low | Subscription-Status je Node |
| Web interface certificate valid for 14+ days | medium | Ablauf des Node-Zertifikats |
| Every guest is in a backup job | high | Gäste, die von keinem Backup-Job erfasst werden |
| Backup jobs succeed | high | letztes Ergebnis jedes Backup-Jobs |

So verbessern Sie sie:

1. Nutzen Sie ein Repository, auf das Sie Anspruch haben: Enterprise mit einer
   Subscription, sonst `pve-no-subscription`; nie `pvetest` auf einem Node,
   der wichtig ist.
2. Installieren Sie Updates im Bereich **Updates** des Nodes und starten Sie
   dann in den neuen Kernel neu.
3. Nehmen Sie jede VM und jeden Container in einen Backup-Job auf, am besten
   auf einen Proxmox Backup Server, und beheben Sie jeden fehlschlagenden Job.
4. Ersetzen Sie das selbstsignierte Webzertifikat durch ein ACME-Zertifikat.

Quellen: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server { #proxmox-backup-server }

| Prüfung | Schweregrad | Daten |
|---|---|---|
| System updates installed | high | ausstehende Updates (braucht `Sys.Modify` auf `/system`) |
| Running the installed Backup Server version | medium | Daemons, die noch die Version vor dem Upgrade ausführen |
| Verify job scheduled | high | Anzahl der Verify-Jobs |
| Prune job scheduled | medium | Anzahl der Prune-Jobs |
| Backups copied elsewhere (sync or tape) | medium | Sync- und Band-Backup-Jobs (3-2-1-Regel) |
| Garbage collection clean, no bad chunks | high | letztes GC-Ergebnis und defekte Chunks je Datastore |
| Certificate valid for 14+ days | medium | Ablauf des Zertifikats |

So verbessern Sie sie:

1. Installieren Sie Updates und starten Sie neu, wenn sich der Kernel oder
   `proxmox-backup` geändert hat.
2. Planen Sie auf jedem Datastore einen Verify-Job und einen Prune-Job.
3. Halten Sie eine Kopie andernorts: einen Sync-Job auf einen zweiten Backup
   Server oder Band.
4. Verwenden Sie für die Weboberfläche ein ACME-Zertifikat.

Quellen: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html).

### Synology { #synology }

| Prüfung | Schweregrad | Daten |
|---|---|---|
| DSM 7 or later | high | DSM-Version |
| DSM up to date | high | ein DSM-Update ist verfügbar (nur Administratorkonto) |
| Built-in admin account disabled | medium | Zustand des Kontos `admin` (nur Administratorkonto) |
| Auto Block enabled | medium | Auto-Block-Einstellung (nur Administratorkonto) |
| DSM firewall enabled | medium | Firewall-Einstellung (nur Administratorkonto) |
| Hyper Backup task configured | medium | Anzahl der Hyper-Backup-Aufgaben |
| Hyper Backup tasks succeed | high | letztes Ergebnis jeder Aufgabe |

Die vier Einstellungen werden einmal pro Stunde gelesen und nur, wenn sich
DumbMonit mit einem Administratorkonto anmeldet; mit einem normalen Konto
bleiben sie unbekannt und zählen nicht. Die Zwei-Faktor-Bestätigung und die
HTTPS-Weiterleitung werden nicht gelesen.

So verbessern Sie sie:

1. Betreiben Sie das neueste DSM-7-Release für Ihr Modell und lassen Sie DSM
   wichtige Updates automatisch installieren (Systemsteuerung → Update und
   Wiederherstellung).
2. Deaktivieren Sie das eingebaute Konto `admin`; melden Sie sich mit einem
   persönlichen Administrator an, der durch die Zwei-Faktor-Bestätigung
   geschützt ist.
3. Schalten Sie Auto Block und die Firewall ein (Systemsteuerung →
   Sicherheit) und leiten Sie keine DSM-Ports ins Internet weiter: Nutzen Sie
   ein VPN.
4. Sichern Sie mit Hyper Backup auf ein anderes Gerät oder ein Cloud-Ziel und
   beheben Sie jede fehlschlagende Aufgabe: RAID und Snapshots sind keine
   Backups.

Quellen: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense { #opnsense }

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Firmware updates installed | high | ausstehende Paketupdates (unbekannt, bis OPNsense nach Updates gesucht hat) |
| On the current major release series | medium | neue Major-Serie verfügbar |
| No reboot pending after an update | medium | Neustart erforderlich |
| Packet filter enabled | **critical** | Status von `pf` |

So verbessern Sie sie:

1. Suchen Sie unter System → Firmware → Status nach Updates, installieren Sie
   sie und starten Sie neu, wenn dazu aufgefordert wird.
2. Wechseln Sie auf das neue Major-Release, sobald es erscheint.
3. Lassen Sie den Paketfilter eingeschaltet.
4. Erlauben Sie die Weboberfläche und SSH nur aus dem LAN oder über ein VPN,
   nie aus dem WAN.

Quellen: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Websites { #websites }

Für **HTTP(S)**- und **TLS**-Monitore, die TLS aushandeln, und für **Website
health & SEO**-Geräte. Ein reiner HTTP-Monitor hat keinen Score.

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Certificate valid for 14+ days | high | Tage bis zum Ablauf (HTTP(S)- und TLS-Monitore) |
| Certificate chain trusted | high | Prüfung der Kette (HTTP(S)- und TLS-Monitore) |
| TLS 1.3 negotiated | low | ausgehandelte Protokollversion (HTTP(S)- und TLS-Monitore) |
| Served over HTTPS | high | endgültige Adresse nach Weiterleitungen (Website health) |
| Strict-Transport-Security header | medium | Header mit einem `max-age` über 0 |
| X-Content-Type-Options: nosniff | low | Wert des Headers |
| Protected against clickjacking | low | `X-Frame-Options` oder CSP `frame-ancestors` |
| Content-Security-Policy header | low | eine erzwungene Richtlinie (Report-only zählt nicht) |

Header werden nur bei HTTPS-Antworten gelesen.

So verbessern Sie sie:

1. Liefern Sie die Website ausschließlich über HTTPS aus und leiten Sie
   reines HTTP auf HTTPS um.
2. Automatisieren Sie die Zertifikatserneuerung (ACME, z. B. Let's Encrypt)
   und liefern Sie die vollständige Kette aus.
3. Fügen Sie Sicherheits-Header im Webserver oder Reverse Proxy hinzu:
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff` und eine
   `Content-Security-Policy` mit `frame-ancestors`.
4. Gleichen Sie mit dem [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory)
   ab, das weit mehr testet als dieser Score.

Quellen: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory { #active-directory }

Bewertet aus den PingCastle-artigen Befunden des Active-Directory-Collectors:
Kerberoastable-Konten, Konten ohne Kerberos-Vorauthentifizierung,
uneingeschränkte Delegierung, Alter des `krbtgt`-Passworts, Größe privilegierter
Gruppen, LAPS-Abdeckung, Kennwortrichtlinie, verwaiste Konten. Jeder Befund ist
eine Prüfung, die besteht, wenn sie null Objekte betrifft.

So verbessern Sie sie:

1. Beheben Sie zuerst kritische und hohe Befunde: Sie sind die üblichen Wege zu
   den Domain Admins.
2. Wenn das `krbtgt`-Passwort alt ist, setzen Sie es zweimal im Abstand von
   einigen Stunden zurück und entfernen Sie die uneingeschränkte Delegierung
   von Computern, die keine Domänencontroller sind.
3. Stellen Sie Dienste, die unter Benutzerkonten mit SPNs laufen, auf gruppen-
   verwaltete Dienstkonten (gMSA) um.
4. Führen Sie [PingCastle](https://www.pingcastle.com/) für eine vollständige
   Bewertung aus.

Quellen: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Grenzen

- Der Score spiegelt nur wider, was DumbMonit sehen kann; viele Best Practices
  werden nicht geprüft, weil kein Collector die Daten liest: Proxmox-Firewall
  und Zwei-Faktor-Authentifizierung, im WAN erreichbare OPNsense-Weboberfläche,
  Docker-Container, die privilegiert oder mit eingehängtem Docker-Socket
  laufen, weiterhin angebotenes TLS 1.0/1.1 (die Sonde handelt nur TLS 1.2 und
  1.3 aus) und vieles mehr.
- Eine Prüfung ist nur so aktuell wie ihre Daten: Manche Collectors lesen
  Updates nur alle paar Stunden neu.
- Die Gewichte sind eine Produktentscheidung, für jede Installation gleich.
- Nur die oben genannten Arten werden bewertet. Andere (Switches, Drucker,
  UniFi, pfSense …) zeigen absichtlich kein Abzeichen: Ein paar Prüfungen
  ergäben eine Note, die nichts aussagt.
