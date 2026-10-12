# Security Score

Jedes unterstützte Gerät erhält einen **Security Score** von 0 bis
100 und eine Note von **A** bis **F**, im Geiste von PingCastle oder
Microsoft Secure Score: eine Liste von Prüfungen, aus der eigenen
Best-Practice-Dokumentation des Herstellers gezogen, jede bestehend,
fehlschlagend oder nicht bewertet.

Der Score erscheint als **Security**-Karte auf der Geräteseite, und
auf der **Security**-Seite, die jedes bewertete Gerät listet, am
schlechtesten zuerst.

## Wie der Score berechnet wird

DumbMonit fragt das Gerät nie etwas Zusätzliches, um den Score zu
berechnen. Die Prüfungen lesen die letzten von den Collectors schon
aufgezeichneten Werte (bis zu 6 Stunden zurückblickend), daher kostet
ein Score das Gerät nichts und funktioniert offline, auf Basis der
letzten Abfrage.

Jede Prüfung hat:

| Feld | Bedeutung |
|---|---|
| Kategorie | exposure, patching, authentication, encryption, backup oder configuration |
| Schweregrad | critical, high, medium oder low — ihr Gewicht im Score: 10, 6, 3, 1 |
| Ergebnis | **pass**, **fail**, oder **unknown**, wenn die Daten nicht erfasst werden |
| Nachweis | was beobachtet wurde: `pve1: 3 security updates`, `Expires in 9 days`… |
| Behebung | was zu tun ist, in den Worten des Herstellers |
| Herstellerhinweis | die Hersteller- (oder Standard-)Seite, auf der die Prüfung beruht |

**Score** = 100 × (Gewicht bestehender Prüfungen) ÷ (Gewicht
bestehender + fehlschlagender Prüfungen). **Unbekannte Prüfungen
werden angezeigt, aber nie gezählt**, weder dafür noch dagegen: Ein
Score darf nicht fallen, weil einem Monitoring-Token ein Recht fehlt,
noch steigen, weil nichts bekannt ist. Ein Gerät, dessen Prüfungen
alle unbekannt sind, ist *nicht bewertet*.

| Note | Score | Wort |
|---|---|---|
| A | 90–100 | Strong |
| B | 75–89 | Good |
| C | 60–74 | Fair |
| D | 40–59 | Weak |
| F | 0–39 | Poor |

**Eine fehlschlagende kritische Prüfung begrenzt die Note auf C**,
egal was der Score sagt: Eine Firewall, die nicht mehr filtert, oder
eine aktiv ausgenutzte, ungepatchte Firmware, wird nicht durch zehn
kleine gute Einstellungen ausgeglichen. Die Karte sagt das, wenn es
passiert.

## Historie und Alarme

Der Server berechnet jeden Score alle 15 Minuten neu und speichert
ihn als `dumbmonit_security_score{target="<id>"}` (und die Anzahl
fehlschlagender Prüfungen als
`dumbmonit_security_checks_failed`), sodass Sie ihn wie jede andere
Metrik plotten und darauf alarmieren können. Zwei eingebaute Regeln
nutzen ihn:

- **Security score dropped** — der Score ist um 10 Punkte oder mehr
  gegenüber seinem besten Wert der letzten 24 Stunden gefallen: Etwas
  Neues schlägt fehl.
- **Security score low** — der Score liegt unter 40 (Note F).
  Bearbeiten Sie die Regel, um den Schwellenwert an Ihre eigene
  Erwartung anzupassen.

Siehe [Alarmregeln](../alerting/rules.md#security-score).

## Prüfungen je Art

### Proxmox VE

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Security updates installed | high | ausstehende Security-Updates pro Knoten (braucht `Sys.Modify` auf dem Token, sonst unknown) |
| Proxmox VE packages up to date | medium | aktualisierbare Proxmox-Pakete pro Knoten |
| Running the installed kernel | medium | neuerer Kernel installiert als laufend |
| No test repository in use | medium | aktivierte Standard-Repositories |
| Package repositories correctly configured | medium | von Proxmox VE auf den Repositories gehobene Fehler und Warnungen (z. B. Enterprise-Repository ohne Abonnement) |
| Enterprise repository subscription | low | Abonnementstatus pro Knoten |
| Web interface certificate valid for 14+ days | medium | Zertifikatsablauf des Knotens |
| Every guest is in a backup job | high | von keinem Backup-Job erfasste Gäste |
| Backup jobs succeed | high | letztes Ergebnis jedes Backup-Jobs |

Quellen: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server

| Prüfung | Schweregrad | Daten |
|---|---|---|
| System updates installed | high | ausstehende Updates (braucht `Sys.Modify` auf `/system`) |
| Running the installed Backup Server version | medium | Daemons, die noch die Vor-Upgrade-Version laufen |
| Verify job scheduled | high | Anzahl der Verify-Jobs |
| Prune job scheduled | medium | Anzahl der Prune-Jobs |
| Backups copied elsewhere (sync or tape) | medium | Sync- und Tape-Backup-Jobs (3-2-1-Regel) |
| Garbage collection clean, no bad chunks | high | letztes GC-Ergebnis und fehlerhafte Chunks pro Datastore |
| Certificate valid for 14+ days | medium | Zertifikatsablauf |

Quellen: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html),
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html),
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html).

### Synology DSM

| Prüfung | Schweregrad | Daten |
|---|---|---|
| DSM 7 or later | high | DSM-Version |
| Hyper Backup task configured | medium | Anzahl der Hyper-Backup-Tasks |
| Hyper Backup tasks succeed | high | letztes Ergebnis jedes Tasks |

Quellen: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense und pfSense

| Prüfung | Schweregrad | Daten |
|---|---|---|
| OPNsense: firmware updates installed | high | ausstehende Paket-Updates (unknown, bis OPNsense nach Updates gesucht hat) |
| OPNsense: on the current major release series | medium | neue Hauptversion verfügbar |
| OPNsense: no reboot pending after an update | medium | Reboot erforderlich |
| OPNsense: packet filter enabled | **critical** | `pf`-Status |
| pfSense: REST API package up to date | low | Update des REST-API-Pakets verfügbar |

Quellen: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html),
[pfSense REST API](https://pfrest.org/).

### UniFi

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Device firmware up to date | high | Upgradable-Flag jedes adoptierten Geräts |

Quelle: [UniFi releases](https://community.ui.com/releases).

### TLS- und HTTPS-Monitore

Für `tls`-, `http`-, `smtp`-, `mqtt`- und `websocket`-Monitore, die
TLS aushandeln. Ein Klartext-Monitor hat keine Security-Karte.

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Certificate valid for 14+ days | high | Tage bis zum Ablauf |
| Certificate chain trusted | high | Verifizierung der Kette |
| TLS 1.3 negotiated | low | ausgehandelte Protokollversion |

Quellen: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final),
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/).

### NTP-Zeitserver

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Server reports a synchronized clock | medium | Stratum und Leap-Indikator |
| Clock offset within 100 ms | low | gemessene Abweichung |

Ein Stratum von 16 (oder 0, ein "kiss-o'-death") und ein
"unsynchronized"-Leap-Indikator lassen beide die erste Prüfung
fehlschlagen, unabhängig von der Abweichung: Ein nie synchronisierter
Server hat nichts Vertrauenswürdiges anzubieten. Der 100-ms-
Schwellenwert ist hier fest und folgt nicht der Pro-Gerät-Option
"Tolerated offset", genauso wie die Zertifikatsprüfung immer bei
vierzehn Tagen warnt, unabhängig von den eigenen Einstellungen eines
Monitors.

Quelle: [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905).

### Nginx und Apache httpd

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Server header does not disclose the version | low | der neben der Statusseite erfasste `Server`-Antwort-Header |

Ein Header, der noch das Produkt nennt ("nginx", "Apache") ohne
Versionsnummer, besteht: Das erzeugen `server_tokens off` und
`ServerTokens Prod`, und ein fehlender Header besteht auch. Ein
Header wie "nginx/1.25.3" oder "Apache/2.4.58 (Unix)" schlägt fehl:
Er sagt einem Scanner genau, welche CVEs zu versuchen sind.

Quellen: [Nginx core module reference](https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens),
[Apache core module reference](https://httpd.apache.org/docs/2.4/mod/core.html#servertokens).

### Maschinen mit dem Agent (und ihre Docker-Container)

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Security updates installed | high | ausstehende Security-Updates (wo der Paketmanager sie unterscheidet) |
| No pending updates | low | ausstehende Updates |
| No reboot pending | medium | Reboot erforderlich |
| SELinux enforcing | medium | SELinux-Modus (unknown, wo SELinux fehlt) |
| Container images up to date | medium | neueres Image in der Registry (nur auf Maschinen mit Containern) |
| No container image older than 180 days | low | Image-Alter (nur auf Maschinen mit Containern) |

Quellen: [CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index),
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often).

### Hersteller-Updates für andere Integrationen

Eine Patching-Prüfung pro Produkt, aus den Update-Informationen, die
jeder Collector schon liest:

| Art | Prüfung | Schweregrad |
|---|---|---|
| FortiGate | FortiOS patch installed | **critical** |
| FortiGate | FortiGuard licences valid | high |
| MikroTik | RouterOS up to date | high |
| MikroTik | RouterBOOT firmware matches RouterOS | low |
| Home Assistant | Home Assistant components up to date | medium |
| Nextcloud | Nextcloud server up to date | high |
| Nextcloud | Nextcloud apps up to date | medium |
| AdGuard Home | AdGuard Home up to date | medium |
| Pi-hole | Pi-hole up to date | medium |
| Plex | Plex Media Server up to date | medium |
| Paperless-ngx | Paperless-ngx up to date | medium |
| Tailscale | Tailscale clients up to date | medium |

### Active Directory

Ein Active-Directory-Gerät wird aus den Befunden im PingCastle-Stil
seines Collectors bewertet: Kerberoastbare Konten, Konten ohne
Kerberos-Präauthentifizierung, uneingeschränkte Delegation,
`krbtgt`-Passwortalter, Größe privilegierter Gruppen, LAPS-Abdeckung,
Passwortrichtlinie, veraltete Konten. Jeder Befund ist eine Prüfung,
die besteht, wenn er null Objekte betrifft; der vom Collector
gemeldete Schweregrad ist der genutzte. Noch nicht gemeldete Befunde
werden als unknown gelistet.

Quellen: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview),
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password).

### GitLab

Aus der Administrations-API gelesen, mit dem Personal Access Token
eines Administrators; unknown mit einem gewöhnlichen.

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Two-factor authentication required | high | Einstellung `require_two_factor_authentication` |
| Public sign-up disabled | medium | Einstellung `signup_enabled` |
| Database migrations applied | medium | ausstehende Migrationen nach einem Upgrade |

Quelle: [GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html).

### Forgejo und Gitea

| Prüfung | Schweregrad | Daten |
|---|---|---|
| Scheduled tasks run on time | medium | Tasks, deren nächster Lauf schon überfällig ist |

Unknown ohne das Access-Token eines Site-Administrators: Forgejo und
Gitea behalten die Liste geplanter Tasks der Administration vor.

Quelle: [Forgejo API](https://forgejo.org/docs/latest/user/api/).

## Grenzen

Der Score spiegelt nur, was DumbMonit sehen kann. Er ist ein Anstoß
zum Hinsehen, kein Audit, und eine Note A bedeutet nicht, dass ein
Gerät sicher ist.

Best Practices, die **noch nicht** geprüft werden, weil die
Collectors die Daten heute nicht lesen:

| Art | Nicht geprüft (braucht neue Daten) |
|---|---|
| Proxmox VE | Datacenter-/Knoten-Firewall aktiviert; Zwei-Faktor-Authentifizierung auf `root@pam` und anderen Admin-Konten |
| Proxmox Backup Server | Client-seitige Verschlüsselung von Backups (braucht das Lesen von Backup-Manifesten) |
| Synology DSM | 2-Step-Verification für Administratoren; Standardkonto `admin` deaktiviert; Auto Block; DSM-Firewall; HTTP-zu-HTTPS-Weiterleitung; QuickConnect-Exposition; ob ein neueres DSM-Update verfügbar ist |
| OPNsense / pfSense | Web-GUI oder SSH aus dem WAN erreichbar; Standard-Zugangsdaten; pfSense-System- (nicht REST-API-)Updates |
| UniFi | Geräte-SSH mit Standard-Zugangsdaten; Site-Einstellungen |
| TLS-Monitore | TLS 1.0/1.1 oder schwache Cipher Suites weiterhin angeboten — die Abfrage handelt nur TLS 1.2 und 1.3 aus, daher zeigt sich ein auf ältere Versionen begrenzter Server stattdessen als fehlgeschlagene Abfrage |
| Agent-Hosts | Host-Firewall-Status; SSH-Konfiguration; Windows-Defender-Status |
| Docker | Container, die privilegiert, als root, oder mit eingehängtem Docker-Socket laufen |

Weitere Grenzen:

- Eine Prüfung ist nur so frisch wie ihre Daten: manche Collectors
  lesen Updates oder Abonnements nur alle paar Stunden neu.
- Die Gewichte sind eine Produktentscheidung, dieselbe für jede
  Installation.
- Arten ohne jede Prüfung haben keine Security-Karte und erscheinen
  nicht auf der Security-Seite.
