# Tipi di dispositivo

DumbMonit chiama tutto quello che controlla un *dispositivo* (l'API dice *target*), anche quando è una pagina web o un nome DNS. Ogni tipo è un collector sul server; il server descrive ogni tipo all'interfaccia (`GET /api/collectors`), quindi il modulo corrisponde sempre a quello che questa versione può monitorare.

| Tipo | `kind` | Credenziale | Uso tipico |
|---|---|---|---|
| [Dispositivo SNMP](snmp.md) | `snmp` | comunità o SNMP v3 | Switch, router, NAS, UPS, stampante |
| [Proxmox VE](proxmox.md) | `proxmox` | token API o nome utente/password | Hypervisor, cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | token API o nome utente/password | Server di backup |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | token API | Console che federalizza più cluster PVE e istanze PBS |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | nome utente/password | Gateway di posta che filtra spam e virus |
| [VMware vSphere](vsphere.md) | `vsphere` | nome utente/password | vCenter o ESXi autonomo: host, VM, datastore, allarmi |
| [Synology DSM](synology.md) | `synology` | nome utente/password | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | chiave API e secret | Firewall, router, multi-WAN edge |
| [pfSense](pfsense.md) | `pfsense` | chiave API REST (utente di sola lettura) | Firewall: gateway giù o lossy, interfacce senza link, servizi arrestati |
| [FortiGate](fortigate.md) | `fortigate` | token API REST (profilo di sola lettura, host attendibile) | Firewall: tunnel IPsec, link, sincronizzazione HA, modalità conservare, licenze FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | nome utente/password (profilo di sola lettura) | Firewall: link di interfaccia solo (l'API XML non espone nessun tunnel, HA o stato della licenza) |
| [UniFi Network](unifi.md) | `unifi` | chiave API o nome utente/password | Console UniFi o server auto-ospitato: dispositivi, WAN, client |
| [Home Assistant](homeassistant.md) | `homeassistant` | token di accesso di lunga durata | Casa intelligente: entità non disponibili, batterie basse, aggiornamenti, riparazioni |
| [TrueNAS](truenas.md) | `truenas` | chiave API | Server di archiviazione ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | chiave API (ruolo Visualizzatore) | NAS: array, dischi, controlli di parità, pool di cache, contenitori, VM |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | nome utente/password (gruppo di sola lettura) | Router o switch: versioni e firmware dietro, CPU, memoria, sensori, errori di interfaccia |
| [UPS con NUT](nut.md) | `nut` | nessuno, o nome utente/password | UPS su un NAS, Raspberry Pi o server che esegue Network UPS Tools |
| [Hardware server (Redfish)](redfish.md) | `redfish` | nome utente/password | Ventole server, temperature, alimentatori e unità, letti dal suo BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | nessuno, nome utente/password o token | Database delle serie temporali: ingestione, campioni rifiutati, spazio su disco |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | nessuno, nome utente/password o token | Database di log: ingestione, linee rifiutate, spazio su disco |
| [Grafana Loki](loki.md) | `loki` | nessuno, nome utente/password o token | Server di log: prontezza, linee rifiutate, guasti di scaricamento |
| [Graylog](graylog.md) | `graylog` | token di accesso o nome utente/password | Server di log e il suo cluster OpenSearch: diario, buffer, throughput, input |
| [Pi-hole](pihole.md) | `pihole` | password dell'app, o nessuno | Bloccatore DNS di annunci: stato di blocco, condivisione bloccata, età della lista di blocco, aggiornamenti |
| [AdGuard Home](adguard.md) | `adguard` | nome utente/password | Filtro DNS: protezione, blocchi, elenchi di filtri, server upstream, aggiornamenti |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | nessuno, o indirizzo e-mail/password | Server di posta: SMTP, IMAP, POP3, webmail, versione |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | nessuno, o chiave API | Gateway di posta: servizi, coda di consegna e altri contatori |
| [Redis / Valkey](redis.md) | `redis` | utente ACL, password sola, o nessuno | Archivio in memoria: memoria rispetto a maxmemory, replica, persistenza, chiavi |
| [MongoDB](mongodb.md) | `mongodb` | nome utente/password (clusterMonitor) o nessuno | Database di documenti: set di replica, lag, connessioni, cache WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | nome utente/password (tag di monitoraggio) | Broker di messaggi: allarmi di risorsa, nodi, partizioni, code senza consumer |
| [CrowdSec](crowdsec.md) | `crowdsec` | nessuno, o chiave API del bouncer | Motore di sicurezza: decisioni, allarmi, bouncer che hanno smesso di tirare, lettura di log |
| [Traefik](traefik.md) | `traefik` | nessuno, o utente basicAuth | Reverse proxy: router disabilitati, server di backend giù, certificati non ottenuti o non rinnovati, condivisione 5xx |
| [Caddy](caddy.md) | `caddy` | utente di visualizzazione di sola lettura, o nessuno | Server web e reverse proxy: salute upstream, ricaricamenti falliti, errori di handler, condivisione 5xx |
| [Nginx](nginx.md) | `nginx` | nessuno, o basic auth | Server web: connessioni attive/in attesa, rate di accetta/gestisce/richiesta, salute upstream NGINX Plus |
| [Apache httpd](apache.md) | `apache` | nessuno, o basic auth | Server web: worker occupati/inattivi, scoreboard per stato, tassi di richiesta e byte |
| [Nginx Proxy Manager](npm.md) | `npm` | e-mail utente e password (vista sola) | Gestore proxy: host che nginx ha rifiutato, host disabilitati, certificati serviti |
| [Scadenza del dominio](domain.md) | `domain` | nessuno | Dominio registrato su RDAP: giorni prima della scadenza, sospensione, periodo di redenzione, registrar |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | token dell'account di servizio (ClusterRole di sola lettura) | Cluster: nodi non pronti, pod che crashano in loop o in sospeso, carichi di lavoro con replica mancanti, claim di volume bloccati |
| [Active Directory](activedirectory.md) | `activedirectory` | nome utente/password (utente del dominio semplice) | Dominio Windows su LDAPS: controller di dominio e ruoli FSMO, membri di gruppo privilegiati, politica password, scoperte di sicurezza |
| [Veeam Backup & Replication](veeam.md) | `veeam` | nome utente/password (Visualizzatore di backup Veeam) | Server di backup: lavori falliti, sessioni fallite, repository che si riempiono, licenza |
| [Tailscale](tailscale.md) | `tailscale` | Client OAuth (dispositivi:core:read) o token di accesso API | Tailnet: server offline, chiavi nodo in scadenza, dispositivi in attesa di approvazione |
| [Nextcloud](nextcloud.md) | `nextcloud` | token di monitoraggio (NC-Token) | Sincronizzazione file: modalità di manutenzione, aggiornamento in sospeso, aggiornamenti, utenti attivi, OPcache |
| [Immich](immich.md) | `immich` | chiave API | Libreria fotografica: code di lavoro, aggiornamenti, disco, conteggi di foto |
| [Paperless-ngx](paperless.md) | `paperless` | token API o nome utente/password | Documenti: Redis, Celery, indice, importazioni fallite, aggiornamenti |
| [Jellyfin](jellyfin.md) | `jellyfin` | chiave API | Server multimediale: attività pianificate fallite, plugin, stream e transcodifiche |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, o nessuno da una rete autorizzata | Server multimediale: stream, transcodifiche, librerie, aggiornamenti |
| [GitLab (auto-gestito)](gitlab.md) | `gitlab` | token di accesso personale | Forge: backlog di Sidekiq, runner di CI, migrazioni in sospeso, impostazioni sign-up/2FA, licenza |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | token di accesso | Forge: controllo di salute, compiti pianificati, runner di Actions, conteggi di repository/account |
| [Server con agente](agent.md) | `agent` | nessuno (token di registrazione) | Linux, Windows, Raspberry Pi, [host Hyper-V](agent.md#hyper-v-hosts) |
| [Sito web o API web](services.md#http) | `http` | nessuno, nome utente/password o token | Pagina di salute, API REST |
| [Porta di rete](services.md#tcp) | `tcp` | nessuno | SSH, SMB, database |
| [Nome di dominio](services.md#dns) | `dns` | nessuno | Il tuo dominio, un nome interno |
| [Host raggiungibile](services.md#ping) | `ping` | nessuno | Gateway, punto di accesso, stampante |
| [Certificato TLS](services.md#tls) | `tls` | nessuno | IMAPS, LDAPS, reverse proxy |
| [Server SMTP](services.md#smtp) | `smtp` | nessuno, o nome utente/password | Server di posta: saluto, STARTTLS, certificato, login facoltativo |
| [PostgreSQL](services.md#postgresql) | `postgres` | nome utente/password | Database: connessione e una query che scegli |
| [MySQL o MariaDB](services.md#mysql-and-mariadb) | `mysql` | nome utente/password | Database: connessione e una query che scegli |
| [Broker MQTT](services.md#mqtt) | `mqtt` | nessuno, o nome utente/password | Broker: connessione, iscrizione, messaggio conservato |
| [WebSocket](services.md#websocket) | `websocket` | nessuno, nome utente/password o token | Endpoint WebSocket: handshake, scambio di messaggi facoltativo |
| [Server ora NTP](services.md#ntp) | `ntp` | nessuno | Router, NAS, server ora interno, server pool pubblico |
| [Heartbeat](push.md) | `push` | nessuno (URL segreto) | Lavoro cron, script di backup, automazione che deve richiamare |
| [Modifiche del sito web](webchange.md) | `webchange` | nessuno | Una pagina o un intero sito: modifiche di testo, pagine nuove e rimosse, prima/dopo con screenshot |
| [Salute e SEO del sito web](website.md) | `website` | nessuno | Siti in massa: giù, pagina vuota, reindirizzamenti, noindex, robots.txt, modifiche di titolo/descrizione/canonico, piattaforma (importazione CSV) |
| [Dispositivo demo](demo.md) | `dummy` | nessuno | Esplora l'interfaccia web senza hardware |

## Come viene letto un dispositivo

Lo scheduler esegue ogni dispositivo abilitato al suo intervallo (60 s di default, minimo 10 s) attraverso il collector per il suo tipo, con al massimo `DUMBMONIT_MAX_CONCURRENT_PROBES` sonde in volo e un timeout fisso di `DUMBMONIT_PROBE_TIMEOUT_SECS` per sonda. Ogni sonda riuscita scrive i suoi campioni più `dumbmonit_up = 1`; una sonda fallita non scrive nulla, ed è il silenzio della serie che la regola "Dispositivo irraggiungibile" rileva. La pagina del dispositivo mostra il fallimento alla prossima sonda, ma l'avviso attende di proposito: si attiva circa quattro minuti dopo l'ultima sonda riuscita (tre minuti di silenzio, poi un mantenimento di un minuto), quindi una singola sonda persa non sveglia mai nessuno.

I fallimenti vengono classificati. Un dispositivo che non risponde è *irraggiungibile* e solleva un avviso. Una comunità sbagliata, un certificato rifiutato o una capacità mancante è un *errore di configurazione*: è mostrato sulla pagina del dispositivo, in rosso, con il motivo, ed è non notificato. La distinzione è importante: un dispositivo configurato erroneamente non dovrebbe svegliare nessuno di notte.

## Opzioni

Alcuni tipi leggono opzioni dai tag del dispositivo: la porta API, se accettare un certificato auto-firmato, quali nodi monitorare. Il modulo mostra un campo tipizzato per opzione sotto **Altre opzioni**, con il suo default e il testo di aiuto. Le pagine per tipo le elencano verbatim. Le opzioni sono impostazioni, non etichette: non vengono copiate sulla serie. I tag liberi (qualsiasi altra chiave) vengono copiati su ogni serie come etichette `tag_<key>`, eccetto chiavi che nominano un secret (abbinate per parola: `password`, `token`, `key`, `cert`, `secret`, `pass`, `private`, `pem`…, quindi `primary_key` è saltato ma `keyboard` no) e valori più lunghi di 256 caratteri. Non mettere mai un secret in un tag comunque: usa il campo di credenziale, che è crittografato.

## Dispositivi genitore

Qualsiasi dispositivo può dichiarare un altro come suo genitore. Quando il genitore è irraggiungibile, gli avvisi dai suoi discendenti sono marcati *Soppresso da genitore* invece di essere inviati: uno switch che scende produce una notifica, non trenta. Sulla pagina Dispositivi, i figli si impilano sotto il genitore e si oscurano quando è irraggiungibile.
