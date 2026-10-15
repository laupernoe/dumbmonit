# Punteggio di sicurezza

Il punteggio di sicurezza è un voto **facoltativo** da **A** a **F** (e un
punteggio da 0 a 100) mostrato accanto ai dispositivi per i quali pochi
controlli dicono qualcosa di utile: host Linux e Windows con l'agente, Proxmox
VE e Proxmox Backup Server, Synology, OPNsense, siti web e Active Directory. Gli
altri dispositivi non mostrano nulla.

!!! warning "Indicativo, non un audit"
    Il punteggio è **approssimativo e non esaustivo**. Legge soltanto ciò che
    DumbMonit raccoglie già da un dispositivo e lo confronta con una manciata
    di buone pratiche del produttore. Un voto A non significa che un
    dispositivo sia sicuro, e un controllo mancante non significa che
    l'impostazione sia a posto. Usalo come invito a guardare, poi, quando conta,
    usa un vero strumento di valutazione (PingCastle, Mozilla HTTP
    Observatory, Lynis, il security advisor del produttore stesso…).

## Attivarlo

Il punteggio è **disattivato per impostazione predefinita**. Un amministratore
lo attiva in **Impostazioni → Account e sicurezza → Punteggio di sicurezza**.
Una volta attivo:

- ogni dispositivo valutato riceve un piccolo **badge a forma di scudo** con la
  sua lettera nell'elenco dei dispositivi, e un badge **Security** con il
  punteggio nella sua pagina;
- facendo clic sul badge si apre il **report** del dispositivo: cosa fallisce,
  con l'evidenza e la correzione, cosa passa, i passaggi per migliorare il
  punteggio su questo tipo di dispositivo e un link alla sezione corrispondente
  qui sotto.

Disattivandolo si nascondono i badge, si ferma il calcolo periodico e le serie
`dumbmonit_security_score` non vengono più scritte.

!!! note "Aggiornamento da 0.1.0-alpha.8 o precedente"
    Il punteggio era prima sempre attivo, con una propria pagina **Security**.
    Un'installazione che monitorava già un dispositivo valutabile mantiene il
    punteggio attivo dopo l'aggiornamento; una nuova installazione parte con il
    punteggio disattivato. Il vecchio indirizzo `/security` ora apre
    l'impostazione.

## Come viene calcolato il punteggio

DumbMonit non chiede mai nulla di extra al dispositivo per calcolare il
punteggio. I controlli leggono gli ultimi valori che i collector hanno già
registrato (guardando indietro di 6 ore), quindi un punteggio non costa nulla
al dispositivo.

Ogni controllo ha una categoria (esposizione, patching, autenticazione,
cifratura, backup, configurazione), una **gravità** che è il suo peso
(critical 10, high 6, medium 3, low 1), un risultato (**superato**,
**fallito** o **sconosciuto** quando il dato non viene raccolto), l'evidenza,
la correzione e un link alla pagina del produttore su cui si basa.

**Punteggio** = 100 × (peso dei controlli superati) ÷ (peso dei superati +
falliti). **I controlli sconosciuti sono mostrati ma mai contati**: un
punteggio non deve scendere perché a un account di monitoraggio manca un
privilegio, né salire perché non si sa nulla. Un dispositivo i cui controlli
sono tutti sconosciuti *non è valutato*.

| Voto | Punteggio | Parola |
|---|---|---|
| A | 90–100 | Solido |
| B | 75–89 | Buono |
| C | 60–74 | Discreto |
| D | 40–59 | Debole |
| F | 0–39 | Scarso |

**Un controllo critico fallito limita il voto a C**, qualunque sia il
punteggio.

## Storico e avvisi

Finché il punteggio è attivo, il server lo ricalcola ogni 15 minuti e lo
memorizza come `dumbmonit_security_score{target="<id>"}` (e il numero di
controlli falliti come `dumbmonit_security_checks_failed`). Due regole
integrate lo usano: **Security score dropped** (10 punti o più sotto il
miglior valore delle ultime 24 ore) e **Security score low** (sotto 40). Vedi
[Regole di avviso](../alerting/rules.md#security-score).

## Controlli e come migliorarli

### Host Linux { #linux-hosts }

Macchine con l'agente. I controlli SSH e firewall richiedono un agente della
stessa release del server; con un agente più vecchio sono sconosciuti.

| Controllo | Gravità | Dato |
|---|---|---|
| Security updates installed | high | aggiornamenti di sicurezza in sospeso (dove il gestore di pacchetti li distingue) |
| No pending updates | low | aggiornamenti in sospeso |
| No reboot pending | medium | riavvio richiesto |
| SSH root login with a password refused | high | `PermitRootLogin` da `sshd -T` (`prohibit-password` passa) |
| SSH accepts keys only | medium | `PasswordAuthentication`, e `KbdInteractiveAuthentication` con `UsePAM` |
| Host firewall filters inbound traffic | high | una catena input nftables o iptables che scarta o rifiuta |
| SELinux enforcing | medium | modalità SELinux (sconosciuto dove SELinux è assente) |
| Container images up to date | medium | immagine più recente nel registry (solo con container) |
| No container image older than 180 days | low | età dell'immagine (solo con container) |

Per migliorarlo:

1. Installa gli aggiornamenti di sicurezza in sospeso, poi attiva gli
   aggiornamenti di sicurezza automatici: `unattended-upgrades` su Debian e
   Ubuntu, `dnf-automatic` su Fedora e RHEL.
2. Riavvia dopo gli aggiornamenti del kernel o delle librerie.
3. Verifica che la tua chiave SSH funzioni, poi imposta `PermitRootLogin no`,
   `PasswordAuthentication no` e `KbdInteractiveAuthentication no` in
   `/etc/ssh/sshd_config` e ricarica sshd.
4. Attiva un firewall dell'host che scarti il traffico in ingresso non
   richiesto: `ufw enable` su Ubuntu, firewalld su Fedora e RHEL, un ruleset
   nftables altrove.
5. Mantieni SELinux in enforcing dove la distribuzione lo fornisce; scarica
   regolarmente immagini di container più recenti.

Fonti: [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Host Windows { #windows-hosts }

Macchine con l'agente Windows, della stessa release del server (letti dal
registro; un agente più vecchio non riporta nessuno di questi controlli).

| Controllo | Gravità | Dato |
|---|---|---|
| Windows Firewall on for every profile | high | profili dominio, privato e pubblico (prima i criteri di gruppo) |
| No reboot pending | medium | Windows Update o manutenzione dei componenti in attesa di un riavvio |
| Remote Desktop requires Network Level Authentication | high | passa quando Desktop remoto è disattivato |
| SMBv1 disabled | high | impostazione del server SMBv1 |

Per migliorarlo:

1. Installa gli aggiornamenti di Windows (Impostazioni → Windows Update) e
   riavvia quando richiesto.
2. Mantieni attivo Windows Defender Firewall per i profili dominio, privato e
   pubblico.
3. Se Desktop remoto è attivo, richiedi l'autenticazione a livello di rete
   (NLA) e raggiungilo tramite VPN, mai direttamente da internet.
4. Rimuovi SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Fonti: [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE { #proxmox-ve }

| Controllo | Gravità | Dato |
|---|---|---|
| Security updates installed | high | aggiornamenti di sicurezza in sospeso per nodo (richiede `Sys.Modify` sul token, altrimenti sconosciuto) |
| Proxmox VE packages up to date | medium | pacchetti Proxmox aggiornabili per nodo |
| Running the installed kernel | medium | kernel installato più recente di quello in esecuzione |
| No test repository in use | medium | repository standard abilitati |
| Package repositories correctly configured | medium | errori e avvisi che Proxmox VE segnala sui repository |
| Enterprise repository subscription | low | stato della sottoscrizione per nodo |
| Web interface certificate valid for 14+ days | medium | scadenza del certificato del nodo |
| Every guest is in a backup job | high | guest non selezionati da alcun job di backup |
| Backup jobs succeed | high | ultimo risultato di ogni job di backup |

Per migliorarlo:

1. Usa un repository a cui hai diritto: enterprise con una sottoscrizione,
   altrimenti `pve-no-subscription`; mai `pvetest` su un nodo che conta.
2. Installa gli aggiornamenti dal pannello **Updates** del nodo, poi riavvia
   con il nuovo kernel.
3. Metti ogni VM e container in un job di backup, idealmente verso un Proxmox
   Backup Server, e correggi ogni job che fallisce.
4. Sostituisci il certificato web autofirmato con uno ACME.

Fonti: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server { #proxmox-backup-server }

| Controllo | Gravità | Dato |
|---|---|---|
| System updates installed | high | aggiornamenti in sospeso (richiede `Sys.Modify` su `/system`) |
| Running the installed Backup Server version | medium | daemon che eseguono ancora la versione precedente all'aggiornamento |
| Verify job scheduled | high | numero di job di verifica |
| Prune job scheduled | medium | numero di job di prune |
| Backups copied elsewhere (sync or tape) | medium | job di sincronizzazione e di backup su nastro (regola 3-2-1) |
| Garbage collection clean, no bad chunks | high | ultimo risultato del GC e chunk difettosi per datastore |
| Certificate valid for 14+ days | medium | scadenza del certificato |

Per migliorarlo:

1. Installa gli aggiornamenti e riavvia quando il kernel o `proxmox-backup`
   sono cambiati.
2. Pianifica un job di verifica e un job di prune su ogni datastore.
3. Conserva una copia altrove: un job di sincronizzazione verso un secondo
   Backup Server, o su nastro.
4. Usa un certificato ACME per l'interfaccia web.

Fonti: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html).

### Synology { #synology }

| Controllo | Gravità | Dato |
|---|---|---|
| DSM 7 or later | high | versione di DSM |
| DSM up to date | high | è disponibile un aggiornamento di DSM (solo account amministratore) |
| Built-in admin account disabled | medium | stato dell'account `admin` (solo account amministratore) |
| Auto Block enabled | medium | impostazione Auto Block (solo account amministratore) |
| DSM firewall enabled | medium | impostazione del firewall (solo account amministratore) |
| Hyper Backup task configured | medium | numero di attività Hyper Backup |
| Hyper Backup tasks succeed | high | ultimo risultato di ogni attività |

Le quattro impostazioni vengono lette una volta all'ora e solo quando DumbMonit
accede con un account amministratore; con un account normale restano
sconosciute e non contano. La verifica in due passaggi e il reindirizzamento a
HTTPS non vengono letti.

Per migliorarlo:

1. Esegui l'ultima release di DSM 7 per il tuo modello e lascia che DSM
   installi automaticamente gli aggiornamenti importanti (Pannello di
   controllo → Aggiornamento e ripristino).
2. Disattiva l'account integrato `admin`; accedi con un amministratore
   personale protetto dalla verifica in due passaggi.
3. Attiva Auto Block e il firewall (Pannello di controllo → Sicurezza) e non
   inoltrare le porte di DSM a internet: usa una VPN.
4. Esegui il backup con Hyper Backup su un altro dispositivo o su una
   destinazione cloud e correggi ogni attività che fallisce: RAID e snapshot
   non sono backup.

Fonti: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense { #opnsense }

| Controllo | Gravità | Dato |
|---|---|---|
| Firmware updates installed | high | aggiornamenti di pacchetti in sospeso (sconosciuto finché OPNsense non ha cercato aggiornamenti) |
| On the current major release series | medium | nuova serie maggiore disponibile |
| No reboot pending after an update | medium | riavvio richiesto |
| Packet filter enabled | **critical** | stato di `pf` |

Per migliorarlo:

1. Cerca aggiornamenti in System → Firmware → Status, installali e riavvia
   quando richiesto.
2. Passa alla nuova release maggiore quando esce.
3. Mantieni attivo il packet filter.
4. Consenti la GUI web e SSH solo dalla LAN o da una VPN, mai dalla WAN.

Fonti: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Siti web { #websites }

Per i monitor **HTTP(S)** e **TLS** che negoziano TLS, e per i dispositivi
**Website health & SEO**. Un monitor HTTP in chiaro non ha punteggio.

| Controllo | Gravità | Dato |
|---|---|---|
| Certificate valid for 14+ days | high | giorni alla scadenza (monitor HTTP(S) e TLS) |
| Certificate chain trusted | high | verifica della catena (monitor HTTP(S) e TLS) |
| TLS 1.3 negotiated | low | versione del protocollo negoziata (monitor HTTP(S) e TLS) |
| Served over HTTPS | high | indirizzo finale dopo i reindirizzamenti (Website health) |
| Strict-Transport-Security header | medium | header con un `max-age` superiore a 0 |
| X-Content-Type-Options: nosniff | low | valore dell'header |
| Protected against clickjacking | low | `X-Frame-Options` o `frame-ancestors` della CSP |
| Content-Security-Policy header | low | una policy applicata (report-only non conta) |

Gli header vengono letti solo sulle risposte HTTPS.

Per migliorarlo:

1. Servi il sito solo via HTTPS e reindirizza l'HTTP in chiaro a HTTPS.
2. Automatizza il rinnovo del certificato (ACME, ad es. Let's Encrypt) e servi
   la catena completa.
3. Aggiungi header di sicurezza nel web server o nel reverse proxy:
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff` e una
   `Content-Security-Policy` con `frame-ancestors`.
4. Confronta con [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory),
   che verifica molto più di questo punteggio.

Fonti: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory { #active-directory }

Valutato a partire dai rilievi in stile PingCastle del collector Active
Directory: account kerberoastable, account senza preautenticazione Kerberos,
delega non vincolata, età della password di `krbtgt`, dimensione dei gruppi
privilegiati, copertura LAPS, policy delle password, account obsoleti. Ogni
rilievo è un controllo che passa quando riguarda zero oggetti.

Per migliorarlo:

1. Correggi prima i rilievi critici e alti: sono i percorsi abituali verso i
   Domain Admins.
2. Se la password di `krbtgt` è vecchia, reimpostala due volte, a distanza di
   qualche ora, e rimuovi la delega non vincolata dai computer che non sono
   controller di dominio.
3. Sposta i servizi eseguiti con account utente con SPN su account di servizio
   gestiti di gruppo (gMSA).
4. Esegui [PingCastle](https://www.pingcastle.com/) per una valutazione
   completa.

Fonti: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Limiti

- Il punteggio riflette solo ciò che DumbMonit può vedere; molte buone
  pratiche non sono verificate perché nessun collector legge il dato: firewall
  e autenticazione a due fattori di Proxmox, GUI web di OPNsense esposta sulla
  WAN, container Docker eseguiti privilegiati o con il socket Docker montato,
  TLS 1.0/1.1 ancora offerti (la sonda negozia solo TLS 1.2 e 1.3), e molto
  altro.
- Un controllo è fresco quanto i suoi dati: alcuni collector rileggono gli
  aggiornamenti solo ogni poche ore.
- I pesi sono una scelta di prodotto, uguale per ogni installazione.
- Vengono valutati solo i tipi sopra elencati. Gli altri (switch, stampanti,
  UniFi, pfSense…) non mostrano un badge di proposito: un paio di controlli
  darebbero un voto privo di significato.
