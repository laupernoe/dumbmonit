# Tipi di dispositivo

DumbMonit chiama *dispositivo* tutto ciò che osserva (l'API dice *target*),
anche quando si tratta di una pagina web o di un nome DNS. Ogni tipo è un
collector sul server; il server descrive ogni tipo all'interfaccia
(`GET /api/collectors`), così il modulo corrisponde sempre a ciò che questa
versione può monitorare.

| Tipo | `kind` | Credenziale | Uso tipico |
|---|---|---|---|
| [Dispositivo SNMP](snmp.md) | `snmp` | community o SNMP v3 | Switch, router, NAS, UPS, stampante |
| [Proxmox VE](proxmox.md) | `proxmox` | token API o username/password | Hypervisor, cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | token API o username/password | Server di backup |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | token API | Console che federa diversi cluster PVE e istanze PBS |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | username/password | Gateway di posta che filtra spam e virus |
| [VMware vSphere](vsphere.md) | `vsphere` | username/password | vCenter o ESXi standalone: host, VM, datastore, allarmi |
| [Synology DSM](synology.md) | `synology` | username/password | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | chiave API e secret | Firewall, router, edge multi-WAN |
| [pfSense](pfsense.md) | `pfsense` | chiave API REST (utente di sola lettura) | Firewall: gateway inattivi o con perdite, interfacce senza link, servizi fermi |
| [FortiGate](fortigate.md) | `fortigate` | token API REST (profilo di sola lettura, host fidato) | Firewall: tunnel IPsec, link, sincronizzazione HA, modalità conserve, licenze FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | username/password (profilo di sola lettura) | Firewall: solo link delle interfacce (l'API XML non espone stato di tunnel, HA o licenze) |
| [UniFi Network](unifi.md) | `unifi` | chiave API o username/password | Console UniFi o server self-hosted: dispositivi, WAN, client |
| [Home Assistant](homeassistant.md) | `homeassistant` | token di accesso di lunga durata | Casa intelligente: entità non disponibili, batterie scariche, aggiornamenti, riparazioni |
| [TrueNAS](truenas.md) | `truenas` | chiave API | Server di storage ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | chiave API (ruolo Viewer) | NAS: array, dischi, controlli di parità, cache pool, container, VM |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | username/password (gruppo di sola lettura) | Router o switch: versioni e firmware in ritardo, CPU, memoria, sensori, errori di interfaccia |
| [UPS con NUT](nut.md) | `nut` | nessuna, oppure username/password | UPS su un NAS, Raspberry Pi o server che esegue Network UPS Tools |
| [Hardware server (Redfish)](redfish.md) | `redfish` | username/password | Ventole, temperature, alimentatori e dischi del server, letti dal suo BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | nessuna, username/password o token | Database di serie temporali: ingestione, campioni rifiutati, margine di disco |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | nessuna, username/password o token | Database di log: ingestione, righe rifiutate, margine di disco |
| [Grafana Loki](loki.md) | `loki` | nessuna, username/password o token | Server di log: readiness, righe rifiutate, fallimenti di flush |
| [Graylog](graylog.md) | `graylog` | token di accesso o username/password | Server di log e il suo cluster OpenSearch: journal, buffer, throughput, input |
| [Pi-hole](pihole.md) | `pihole` | app password, oppure nessuna | Blocca-pubblicità DNS: stato del blocco, percentuale bloccata, età della blocklist, aggiornamenti |
| [AdGuard Home](adguard.md) | `adguard` | username/password | Filtro DNS: protezione, blocchi, liste di filtri, server upstream, aggiornamenti |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | nessuna, oppure indirizzo email/password | Server di posta: SMTP, IMAP, POP3, webmail, versione |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | nessuna, oppure chiave API | Gateway di posta: servizi, coda di consegna e altri contatori |
| [Redis / Valkey](redis.md) | `redis` | utente ACL, solo password, oppure nessuna | Store in memoria: memoria rispetto a maxmemory, replicazione, persistenza, chiavi |
| [MongoDB](mongodb.md) | `mongodb` | username/password (clusterMonitor) oppure nessuna | Database documentale: replica set, ritardo, connessioni, cache WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | username/password (tag monitoring) | Message broker: allarmi di risorse, nodi, partizioni, code senza consumer |
| [CrowdSec](crowdsec.md) | `crowdsec` | nessuna, oppure chiave API bouncer | Motore di sicurezza: decisioni, avvisi, bouncer che hanno smesso di scaricare, lettura dei log |
| [Traefik](traefik.md) | `traefik` | nessuna, oppure utente basicAuth | Reverse proxy: router disattivati, server backend inattivi, certificati non ottenuti o non rinnovati, percentuale 5xx |
| [Caddy](caddy.md) | `caddy` | utente di sola visualizzazione, oppure nessuna | Server web e reverse proxy: stato di salute upstream, reload falliti, errori dei handler, percentuale 5xx |
| [Nginx](nginx.md) | `nginx` | nessuna, oppure basic auth | Server web: connessioni attive/in attesa, tassi accept/handled/request, stato di salute upstream NGINX Plus |
| [Apache httpd](apache.md) | `apache` | nessuna, oppure basic auth | Server web: worker occupati/inattivi, scoreboard per stato, tassi di richieste e byte |
| [Nginx Proxy Manager](npm.md) | `npm` | email e password utente (solo visualizzazione) | Gestore di proxy: host rifiutati da nginx, host disattivati, certificati serviti |
| [Scadenza del dominio](domain.md) | `domain` | nessuna | Dominio registrato via RDAP: giorni prima della scadenza, blocco, periodo di redenzione, registrar |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | token di service account (ClusterRole di sola lettura) | Cluster: nodi non pronti, pod in crash loop o in attesa, workload senza repliche, volume claim bloccati |
| [Active Directory](activedirectory.md) | `activedirectory` | username/password (utente di dominio semplice) | Dominio Windows via LDAPS: controller di dominio e ruoli FSMO, membri di gruppi privilegiati, policy delle password, risultati di sicurezza |
| [Veeam Backup & Replication](veeam.md) | `veeam` | username/password (Veeam Backup Viewer) | Server di backup: job falliti, sessioni fallite, repository che si riempiono, licenza |
| [Tailscale](tailscale.md) | `tailscale` | client OAuth (devices:core:read) o token di accesso API | Tailnet: server offline, chiavi di nodo in scadenza, dispositivi in attesa di approvazione |
| [Nextcloud](nextcloud.md) | `nextcloud` | token di monitoraggio (NC-Token) | Sincronizzazione file: modalità di manutenzione, aggiornamento in attesa, aggiornamenti, utenti attivi, OPcache |
| [Immich](immich.md) | `immich` | chiave API | Libreria fotografica: code di job, aggiornamenti, disco, conteggi foto |
| [Paperless-ngx](paperless.md) | `paperless` | token API o username/password | Documenti: Redis, Celery, indice, importazioni fallite, aggiornamenti |
| [Jellyfin](jellyfin.md) | `jellyfin` | chiave API | Server multimediale: task pianificati falliti, plugin, stream e transcodifiche |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, oppure nessuna da una rete consentita | Server multimediale: stream, transcodifiche, librerie, aggiornamenti |
| [GitLab (self-managed)](gitlab.md) | `gitlab` | token di accesso personale | Forge: arretrato Sidekiq, runner CI, migrazioni in attesa, impostazioni di iscrizione/2FA, licenza |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | token di accesso | Forge: controllo di salute, task pianificati, runner Actions, conteggi di repository/account |
| [Server con agent](agent.md) | `agent` | nessuna (token di iscrizione) | Linux, Windows, Raspberry Pi, [host Hyper-V](agent.md#hyper-v-hosts) |
| [Sito web o API web](services.md#http) | `http` | nessuna, username/password o token | Pagina di salute, API REST |
| [Porta di rete](services.md#tcp) | `tcp` | nessuna | SSH, SMB, database |
| [Nome di dominio](services.md#dns) | `dns` | nessuna | Il tuo dominio, un nome interno |
| [Host raggiungibile](services.md#ping) | `ping` | nessuna | Gateway, access point, stampante |
| [Certificato TLS](services.md#tls) | `tls` | nessuna | IMAPS, LDAPS, reverse proxy |
| [Server NTP](services.md#ntp) | `ntp` | nessuna | Router, NAS, server orario interno, server di pool pubblico |
| [Heartbeat](push.md) | `push` | nessuna (URL segreto) | Job cron, script di backup, automazione che deve chiamare |
| [Modifiche a un sito web](webchange.md) | `webchange` | nessuna | Una pagina o un intero sito: cambi di testo, pagine nuove e rimosse, prima/dopo con screenshot |
| [Dispositivo demo](demo.md) | `dummy` | nessuna | Esplora l'interfaccia senza hardware |

## Come viene letto un dispositivo

Lo scheduler esegue ogni dispositivo abilitato sul proprio intervallo (60 s
per impostazione predefinita, minimo 10 s) attraverso il collector del suo
tipo, con al massimo `DUMBMONIT_MAX_CONCURRENT_PROBES` sonde in corso e un
timeout rigido di `DUMBMONIT_PROBE_TIMEOUT_SECS` per sonda. Ogni sonda
riuscita scrive i suoi campioni più `dumbmonit_up = 1`; una sonda fallita
non scrive nulla, ed è il silenzio della serie che la regola "Device
unreachable" rileva.

I fallimenti sono classificati. Un dispositivo che non risponde è
*unreachable* e genera un avviso. Una community errata, un certificato
rifiutato o una funzionalità mancante è un *errore di configurazione*:
viene mostrato sulla pagina del dispositivo, in rosso, con il motivo, e non
viene notificato. La distinzione conta: un dispositivo mal configurato non
dovrebbe svegliare nessuno di notte.

## Opzioni

Alcuni tipi leggono opzioni dai tag del dispositivo: la porta API, se
accettare un certificato autofirmato, quali nodi monitorare. Il modulo
mostra un campo tipizzato per opzione sotto **More options**, con il suo
valore predefinito e un testo di aiuto. Le pagine per tipo le elencano alla
lettera. Le opzioni vengono copiate su ogni serie come etichette
`tag_<key>`, quindi non metterci mai un segreto: usa il campo credenziale,
che è cifrato.

## Dispositivi genitori

Qualsiasi dispositivo può dichiarare un altro come suo genitore. Quando il
genitore è irraggiungibile, gli avvisi dei suoi discendenti vengono
contrassegnati *Suppressed by parent* invece di essere inviati: uno switch
che si spegne produce una notifica, non trenta. Nella pagina Devices, i
figli si impilano sotto il loro genitore e si attenuano quando è
irraggiungibile.
