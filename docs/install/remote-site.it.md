# Monitora un sito remoto

Un DumbMonit, diverse reti. Un **agente relè** installato al sito remoto
esegue, per conto del server, i probe dei monitor che assegni ad esso, e
segnala i risultati sulla stessa connessione in uscita che usa già per inviare
le sue stesse metriche. Niente da aprire al sito remoto, nessuna VPN, nessuna seconda
istanza da guardare.

```
 ufficio principale                          sito remoto (NAT, firewall)
 ┌──────────────────┐   HTTPS, in uscita    ┌──────────────────────────────┐
 │ Server DumbMonit │◀────────────────────│ agente relè                  │
 │  scheduler       │  probe da eseguire ──▶ │  esegue SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alerting, UI    │◀── campioni, verdetto │  invia anche le sue metriche   │    hypervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Pubblica il server su HTTPS

L'agente relè recupera i suoi lavori dal server: il server deve essere raggiungibile dal
sito remoto, tipicamente `https://monitor.example.org` dietro un reverse proxy. Due cose viaggiano su
quel collegamento:

- il **token di iscrizione** (`dmon_…`), nell'intestazione `Authorization` di ogni
  richiesta — come per qualsiasi agente;
- le **credenziali del monitor** dei monitor che l'agente relè esegue il probe (comunità SNMP,
  token API Proxmox…), decifrate, all'interno di ogni lavoro di probe.

Su plain HTTP attraverso Internet entrambi sarebbero leggibili per strada: metti il
server dietro HTTPS prima di inoltrare nulla al di fuori della tua stessa rete. Su
una LAN o una VPN, HTTP è accettabile e il token è l'unico segreto in volo
fino a quando non assegni monitor all'agente relè. Le credenziali non vengono mai scritte su disco
da nessuno: un lavoro di probe vive nella memoria del server fino a quando l'agente relè
lo segnala, e l'agente non mantiene nulla tra due probe.

Il reverse proxy deve inoltrare `/api/agent/relay` come `/api/ingest`, consentire
corpi di richiesta fino a 16 MB (un hypervisor occupato segnala migliaia di campioni),
e mantenere aperte le connessioni inattive per almeno 60 secondi: il server mantiene una
richiesta dell'agente relè fino a 25 secondi mentre aspetta il lavoro (long polling).

## 2. Crea un token di iscrizione

Aggiungi un monitor, scegli **Server con l'agente** e spunta *Questo agente è un relay per un sito
remoto* (oppure apri l'ingresso *Relay* della pagina di aggiunta). Dai un nome al sito e crea il
token: porta con sé la modalità relay e il sito, quindi non c'è nulla da modificare sulla macchina.
Copia il valore `dmon_…`: viene mostrato una sola volta; la pagina attende poi il primo invio.

## 3. Avvia l'agente relè al sito remoto

Qualsiasi macchina al sito remoto con Docker, usando `docker-compose.agent.yml`
dal repository:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

O l'agente normale installato con il programma di installazione a una riga, con due
chiavi aggiuntive in `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

La macchina appare in DumbMonit entro secondi, come un monitor agente ordinario,
e la sua pagina dice *Relay · site Lyon office · relays 0 devices*. Il log dell'agente dice `relay mode enabled: waiting for probes from the server` una volta che il primo batch
è accettato.

!!! note "Cosa l'agente relè deve raggiungere"
    L'agente relè esegue il probe dei monitor dalla sua stessa rete: ha bisogno dello stesso accesso
    che il server avrebbe se fosse lì — UDP 161 per SNMP, 8006 per
    Proxmox, 5001 per DSM, e così via. Per i monitor di **ping**, il contenitore
    ha bisogno del sysctl `net.ipv4.ping_group_range`, già impostato
    nel file compose (vedi [Ping ICMP senza capability](docker.md#icmp-ping-without-a-capability)). Per monitor su una VLAN
    o rete link-local, o per target `localhost`, usa
    `network_mode: host`.

## 4. Assegna i monitor all'agente relè

Aggiungi i monitor remoti come al solito — switch SNMP, nodo Proxmox, controllo HTTP —
con il loro indirizzo **come visto dal sito remoto** (`192.168.10.1`,
`https://pve.lan:8006`). Nel modulo del monitor, sotto *Più opzioni*, imposta
**Raggiungibile attraverso** all'agente relè invece di *Diretto*. I monitor esistenti
possono essere cambiati allo stesso modo; il passaggio indietro a *Diretto* fa sì che il server
li esamini di nuovo.

Da allora:

- il server accoda un probe per il monitor ad ogni intervallo; l'agente relè
  lo raccoglie entro millisecondi (aspetta il server per il lavoro), esegue il
  collettore e segnala;
- **Esegui probe ora** e il rilevamento del profilo sulla pagina del monitor passano attraverso
  l'agente relè anche;
- l'*ultimo probe* del monitor e l'errore seguono il verdetto dell'agente relè. Quando l'agente relè
  non raccoglie un probe in tempo, il monitor mostra *Timed out: relay
  agent … did not pick up the probe* — controlla che l'agente viene eseguito con
  `relay: true`;
- la pagina dell'agente mostra *Relay for N devices*, e `GET /api/relays` elenca
  gli agenti relè con il loro sito e conteggio.

## 5. Installare agent dietro il relay {#5-install-agents-behind-the-relay}

Un sito dietro un firewall ha spesso server che non riescono affatto a raggiungere DumbMonit:
esce solo il relay. Quei server possono comunque eseguire l'agent completo (sistema, dischi,
servizi, container, backup): si collegano al **gateway** del relay, e il relay porta la loro
connessione fino al server.

```
 remote site (only the relay goes out)                     head office
 ┌────────────┐  TLS 1.3, mTLS   ┌──────────────┐  tunnel inside the  ┌──────────────────┐
 │ agent      │═════════════════▶│ relay agent  │═ relay's own mTLS ═▶│ DumbMonit server │
 │ (server A) │  to the relay's  │  gateway     │  connection, TCP    │  agent port 8443 │
 └────────────┘  port 8443       └──────────────┘  8443 outbound      └──────────────────┘
        └──────────── one TLS session, end to end, agent ⇄ server ─────────────┘
```

**In DumbMonit**, aggiungi un monitor, scegli **Server con l'agente** e, sotto *Come raggiunge la
macchina questo server?*, scegli **Tramite &lt;relay&gt; (sito)**. Indica l'indirizzo con cui il
sito raggiunge il relay (un IP o un nome locale). I comandi mostrati — Linux/macOS, Windows,
Docker — puntano allora al gateway del relay
(`--url=http://192.168.10.5:8443 --tls-url=https://192.168.10.5:8443`) e portano ancora
l'impronta dell'autorità di certificazione del **server**. Il percorso attende poi la prima
connessione e ti lascia dare un nome alla macchina, metterla in una cartella e collocarla in una
sede senza lasciare la pagina. Da un sito in *Aggiungi per sito*, **Server con l'agente**
preseleziona il relay di quel sito; la pagina del relay stesso ha **Installa un agente attraverso
di esso**.

Scegliere un relay per un'installazione gli chiede di aprire il suo gateway (sulla porta 8443 per
impostazione predefinita) al prossimo invio. Il pannello *Relay* della pagina del relay mostra il
gateway — *In ascolto sulla porta 8443 — 3 agent collegati* — e permette di cambiare porta e
indirizzo, o di chiuderlo. Apri quella porta sul firewall del relay stesso, **solo verso la rete
del sito**. Su un relay Docker, pubblica la porta (`-p 8443:8443`, o i `ports:` commentati di
`docker-compose.agent.yml`). `gateway_listen: 0.0.0.0:9443` nell'`agent.yaml` del relay
(`DUMBMONIT_AGENT_GATEWAY_LISTEN`) prevale sull'impostazione dell'interfaccia.

Cosa passa attraverso il relay:

- **Il canale cifrato dell'agent**, intatto. Il relay riconosce un ClientHello TLS e copia i
  byte, nei due sensi, in un tunnel che apre verso la porta agent del server con il *proprio*
  certificato client (un upgrade HTTP su `/api/agent/relay/tunnel`). Il server esegue allora
  l'handshake TLS 1.3 con l'agent del sito come se si fosse collegato direttamente: certificato
  client dell'agent, autorità fissata, token e segreto di associazione sono verificati dal server,
  da un capo all'altro. La registrazione segue la stessa strada.
- **L'installer e il binario**, che il relay recupera dal server sul proprio canale e serve sulla
  stessa porta (solo `/install.sh`, `/install.ps1`, `/download/<file>`). Il comando di
  installazione porta lo SHA-256 di ogni binario: lo script rifiuta qualsiasi altro.

Il server registra, per ogni agent, il relay attraverso cui si collega. La pagina dell'agent dice
*Raggiunge questo server tramite il relay …*; la mappa della topologia disegna il relay come suo
genitore; e quando il relay va giù, gli agent dietro di esso appaiono come `suppressed` dal relay,
non come tanti guasti separati. Un agent che in seguito si collega direttamente (o tramite un
altro relay) viene aggiornato al suo invio successivo.

Il kit Active Directory offre la stessa scelta: sotto **Route**, scegli il relay, e i server del
dominio scaricano l'agent da esso e inviano i dati attraverso di esso.

## Avviso quando l'agente relè è inattivo

Un agente relè che rimane silenzioso porta i suoi monitor con sé: i loro probe si fermano, e
sarebbero tutti *irraggiungibili* una volta. L'agente relè è trattato come un
**genitore** di ogni monitor che esamina (in aggiunta al genitore che potresti avere
impostato): quando l'agente relè stesso è *irraggiungibile*, gli avvisi dei suoi monitor vengono
mostrati come `suppressed` con `suppressed_by` = l'agente relè, e solo una
notifica esce — per l'agente relè. Il genitore che hai impostato esplicitamente mantiene il suo ruolo e vince quando
entrambi sono inattivi.

## Limiti

- Un agente relè per monitor. Un monitor sottoposto a probe dall'agente relè non viene
  anche sottoposto a probe dal server.
- I monitor agent non vengono mai sondati tramite un relay (inviano le proprie metriche); un
  agent può invece *collegarsi* tramite il gateway di un relay (vedi sopra). Un relay si collega
  sempre direttamente: non esiste un relay dietro un relay.
- Il gateway di un relay accetta per impostazione predefinita 64 connessioni alla volta e 4 MiB/s
  per tunnel e direzione (`gateway_max_connections`, `gateway_rate_kib`); il server accetta 256
  tunnel per relay.
- L'agente relè esegue al massimo 8 probe per volta; un probe è limitato dal
  timeout del probe del server (`DUMBMONIT_PROBE_TIMEOUT_SECS`), limitato a 120 s su
  l'agente.
- Un agente relè ha bisogno dei collettori compilati in: gli agenti più vecchi di questa
  caratteristica ignorano `relay: true`, e il server segnala i loro probe come
  scaduti. Reinstalla con il binario o l'immagine corrente.
