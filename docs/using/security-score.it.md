# Punteggio di sicurezza

Ogni dispositivo supportato riceve un **punteggio di sicurezza** da 0 a
100 e un voto da **A** a **F**, nello spirito di PingCastle o Microsoft
Secure Score: un elenco di controlli tratti dalle linee guida di best
practice proprie del fornitore, ognuno che passa, fallisce o non viene
valutato.

Il punteggio appare come una scheda **Security** sulla pagina del
dispositivo, e sulla pagina **Security**, che elenca ogni dispositivo
valutato, il peggiore prima.

## Come viene calcolato il punteggio

DumbMonit non chiede mai nulla di extra al dispositivo per calcolare il
punteggio. I controlli leggono gli ultimi valori che i collector hanno
già registrato (guardando indietro di 6 ore), così un punteggio non
costa nulla al dispositivo e funziona offline dall'ultima sonda.

Ogni controllo ha:

| Campo | Significato |
|---|---|
| Categoria | exposure, patching, authentication, encryption, backup o configuration |
| Severità | critical, high, medium o low — il suo peso nel punteggio: 10, 6, 3, 1 |
| Risultato | **pass**, **fail**, oppure **unknown** quando il dato non è raccolto |
| Evidenza | cosa è stato osservato: `pve1: 3 security updates`, `Expires in 9 days`… |
| Rimedio | cosa fare, nei termini del fornitore |
| Guida del fornitore | la pagina del fornitore (o dello standard) su cui si basa il controllo |

**Punteggio** = 100 × (peso dei controlli passati) ÷ (peso dei
controlli passati + falliti). **I controlli unknown sono mostrati ma
mai contati**, né a favore né contro: un punteggio non deve scendere
perché a un token di monitoraggio manca un privilegio, né salire
perché nulla è conosciuto. Un dispositivo i cui controlli sono tutti
unknown *non è valutato*.

| Voto | Punteggio | Parola |
|---|---|---|
| A | 90-100 | Strong |
| B | 75-89 | Good |
| C | 60-74 | Fair |
| D | 40-59 | Weak |
| F | 0-39 | Poor |

**Un controllo critico fallito limita il voto a C**, qualunque sia il
punteggio: un firewall che non filtra più, o un firmware attivamente
sfruttato e lasciato senza patch, non viene compensato da dieci
impostazioni minori buone. La scheda lo dice quando accade.

## Cronologia e avvisi

Il server ricalcola ogni punteggio ogni 15 minuti e lo memorizza come
`dumbmonit_security_score{target="<id>"}` (e il numero di controlli
falliti come `dumbmonit_security_checks_failed`), così puoi grafarlo e
avvisare su di esso come qualsiasi altra metrica. Due regole integrate
lo usano:

- **Security score dropped** — il punteggio è sceso di 10 punti o più
  rispetto al suo valore migliore delle ultime 24 ore: qualcosa di
  nuovo sta fallendo.
- **Security score low** — il punteggio è sotto 40 (voto F). Modifica
  la regola per spostare la soglia alla tua barra.

Vedi [Regole di avviso](../alerting/rules.md#security-score).

## Controlli per tipo

### Proxmox VE

| Controllo | Severità | Dato |
|---|---|---|
| Security updates installed | high | aggiornamenti di sicurezza in attesa per nodo (richiede `Sys.Modify` sul token, altrimenti unknown) |
| Proxmox VE packages up to date | medium | pacchetti Proxmox aggiornabili per nodo |
| Running the installed kernel | medium | kernel più recente installato di quello in esecuzione |
| No test repository in use | medium | repository standard abilitati |
| Package repositories correctly configured | medium | errori e avvisi che Proxmox VE genera sui repository (es. repository enterprise senza abbonamento) |
| Enterprise repository subscription | low | stato dell'abbonamento per nodo |
| Web interface certificate valid for 14+ days | medium | scadenza del certificato del nodo |
| Every guest is in a backup job | high | guest non selezionati da alcun job di backup |
| Backup jobs succeed | high | ultimo risultato di ogni job di backup |

Fonti: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server

| Controllo | Severità | Dato |
|---|---|---|
| System updates installed | high | aggiornamenti in attesa (richiede `Sys.Modify` su `/system`) |
| Running the installed Backup Server version | medium | demoni ancora in esecuzione sulla versione pre-aggiornamento |
| Verify job scheduled | high | numero di job di verifica |
| Prune job scheduled | medium | numero di job di prune |
| Backups copied elsewhere (sync or tape) | medium | job di backup sync e tape (regola 3-2-1) |
| Garbage collection clean, no bad chunks | high | ultimo risultato GC e chunk danneggiati per datastore |
| Certificate valid for 14+ days | medium | scadenza del certificato |

Fonti: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html),
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html),
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html).

### Synology DSM

| Controllo | Severità | Dato |
|---|---|---|
| DSM 7 or later | high | versione DSM |
| Hyper Backup task configured | medium | numero di task Hyper Backup |
| Hyper Backup tasks succeed | high | ultimo risultato di ogni task |

Fonti: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense e pfSense

| Controllo | Severità | Dato |
|---|---|---|
| OPNsense: firmware updates installed | high | aggiornamenti pacchetto in attesa (unknown finché OPNsense non ha controllato gli aggiornamenti) |
| OPNsense: on the current major release series | medium | nuova serie major disponibile |
| OPNsense: no reboot pending after an update | medium | riavvio richiesto |
| OPNsense: packet filter enabled | **critical** | stato di `pf` |
| pfSense: REST API package up to date | low | aggiornamento disponibile del pacchetto API REST |

Fonti: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html),
[pfSense REST API](https://pfrest.org/).

### UniFi

| Controllo | Severità | Dato |
|---|---|---|
| Device firmware up to date | high | flag upgradable di ogni dispositivo adottato |

Fonte: [UniFi releases](https://community.ui.com/releases).

### Monitor TLS e HTTPS

Per monitor `tls`, `http`, `smtp`, `mqtt` e `websocket` che negoziano
TLS. Un monitor in chiaro non ha una scheda Security.

| Controllo | Severità | Dato |
|---|---|---|
| Certificate valid for 14+ days | high | giorni alla scadenza |
| Certificate chain trusted | high | verifica della catena |
| TLS 1.3 negotiated | low | versione del protocollo negoziata |

Fonti: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final),
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/).

### Server NTP

| Controllo | Severità | Dato |
|---|---|---|
| Server reports a synchronized clock | medium | stratum e indicatore leap |
| Clock offset within 100 ms | low | offset misurato |

Uno stratum di 16 (o 0, un "kiss-o'-death") e un indicatore leap
"unsynchronized" fanno entrambi fallire il primo controllo
indipendentemente dall'offset: un server che non si è mai sincronizzato
non ha nulla di affidabile da offrire. La soglia di 100 ms è fissa qui
e non segue l'opzione "Tolerated offset" per dispositivo, nello stesso
modo in cui il controllo del certificato avvisa sempre a quattordici
giorni indipendentemente dalle impostazioni proprie di un monitor.

Fonte: [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905).

### Nginx e Apache httpd

| Controllo | Severità | Dato |
|---|---|---|
| Server header does not disclose the version | low | l'header di risposta `Server` catturato insieme alla pagina di stato |

Un header che nomina ancora il prodotto ("nginx", "Apache") senza un
numero di versione passa: è ciò che producono `server_tokens off` e
`ServerTokens Prod`, e un header assente passa anch'esso. Un header
come "nginx/1.25.3" o "Apache/2.4.58 (Unix)" fallisce: dice a uno
scanner esattamente quali CVE provare.

Fonti: [Nginx core module reference](https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens),
[Apache core module reference](https://httpd.apache.org/docs/2.4/mod/core.html#servertokens).

### Macchine con l'agent (e i loro container Docker)

| Controllo | Severità | Dato |
|---|---|---|
| Security updates installed | high | aggiornamenti di sicurezza in attesa (dove il package manager li distingue) |
| No pending updates | low | aggiornamenti in attesa |
| No reboot pending | medium | riavvio richiesto |
| SELinux enforcing | medium | modalità SELinux (unknown dove SELinux è assente) |
| Container images up to date | medium | immagine più recente nel registry (solo su macchine con container) |
| No container image older than 180 days | low | età dell'immagine (solo su macchine con container) |

Fonti: [CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index),
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often).

### Aggiornamenti dei fornitori per altre integrazioni

Un controllo di patching per prodotto, dall'informazione di
aggiornamento che ogni collector già legge:

| Tipo | Controllo | Severità |
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

Un dispositivo Active Directory viene valutato dai risultati in stile
PingCastle del suo collector: account Kerberoastable, account senza
pre-autenticazione Kerberos, delegazione non vincolata, età della
password `krbtgt`, dimensione dei gruppi privilegiati, copertura LAPS,
policy delle password, account inattivi. Ogni risultato è un controllo
che passa quando riguarda zero oggetti; la severità che il collector
riporta è quella usata. I risultati non ancora riportati sono elencati
come unknown.

Fonti: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview),
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password).

### GitLab

Letto dall'API di amministrazione, con un token di accesso personale
di un amministratore; unknown con uno ordinario.

| Controllo | Severità | Dato |
|---|---|---|
| Two-factor authentication required | high | impostazione `require_two_factor_authentication` |
| Public sign-up disabled | medium | impostazione `signup_enabled` |
| Database migrations applied | medium | migrazioni in attesa dopo un aggiornamento |

Fonte: [GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html).

### Forgejo e Gitea

| Controllo | Severità | Dato |
|---|---|---|
| Scheduled tasks run on time | medium | task la cui prossima esecuzione è già scaduta |

Unknown senza un token di accesso di amministratore del sito: Forgejo e
Gitea riservano l'elenco dei task pianificati all'amministrazione.

Fonte: [Forgejo API](https://forgejo.org/docs/latest/user/api/).

## Limiti

Il punteggio riflette solo ciò che DumbMonit può vedere. È un invito a
guardare, non un audit, e un voto A non significa che un dispositivo sia
sicuro.

Best practice che **non** sono ancora controllate perché i collector
non leggono il dato oggi:

| Tipo | Non controllato (richiede nuovi dati) |
|---|---|
| Proxmox VE | firewall datacenter/nodo abilitato; autenticazione a due fattori su `root@pam` e altri account amministratore |
| Proxmox Backup Server | cifratura lato client dei backup (richiede la lettura dei manifest di backup) |
| Synology DSM | verifica in 2 passaggi per gli amministratori; account `admin` predefinito disattivato; auto block; firewall DSM; redirezione HTTP-to-HTTPS; esposizione QuickConnect; se un aggiornamento DSM più recente è disponibile |
| OPNsense / pfSense | GUI web o SSH raggiungibile dalla WAN; credenziali predefinite; aggiornamenti di sistema pfSense (non API REST) |
| UniFi | SSH del dispositivo con credenziali predefinite; impostazioni del sito |
| Monitor TLS | TLS 1.0/1.1 o cipher suite debole ancora offerti — la sonda negozia solo TLS 1.2 e 1.3, quindi un server limitato a versioni più vecchie appare come una sonda fallita invece |
| Host agent | stato del firewall dell'host; configurazione SSH; stato di Windows Defender |
| Docker | container in esecuzione privilegiati, come root, o con il socket Docker montato |

Altri limiti:

- Un controllo è fresco solo quanto il suo dato: alcuni collector
  rileggono gli aggiornamenti o gli abbonamenti ogni poche ore.
- I pesi sono una scelta di prodotto, la stessa per ogni installazione.
- I tipi senza alcun controllo non hanno una scheda Security e non
  appaiono sulla pagina Security.
