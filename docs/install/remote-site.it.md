# Monitora un sito remoto

Un DumbMonit, diverse reti. Un **agente relè** installato al sito remoto
esegue, per conto del server, i probe dei dispositivi che assegni ad esso, e
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
- le **credenziali del dispositivo** dei dispositivi che l'agente relè esegue il probe (comunità SNMP,
  token API Proxmox…), decifrate, all'interno di ogni lavoro di probe.

Su plain HTTP attraverso Internet entrambi sarebbero leggibili per strada: metti il
server dietro HTTPS prima di inoltrare nulla al di fuori della tua stessa rete. Su
una LAN o una VPN, HTTP è accettabile e il token è l'unico segreto in volo
fino a quando non assegni dispositivi all'agente relè. Le credenziali non vengono mai scritte su disco
da nessuno: un lavoro di probe vive nella memoria del server fino a quando l'agente relè
lo segnala, e l'agente non mantiene nulla tra due probe.

Il reverse proxy deve inoltrare `/api/agent/relay` come `/api/ingest`, consentire
corpi di richiesta fino a 16 MB (un hypervisor occupato segnala migliaia di campioni),
e mantenere aperte le connessioni inattive per almeno 60 secondi: il server mantiene una
richiesta dell'agente relè fino a 25 secondi mentre aspetta il lavoro (long polling).

## 2. Crea un token di iscrizione

**Impostazioni → Agenti → Nuovo token**, o aggiungi un dispositivo di tipo *agente*. Copia il
valore `dmon_…`: viene mostrato una sola volta.

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

La macchina appare in DumbMonit entro secondi, come un dispositivo agente ordinario,
e la sua pagina dice *Relay · site Lyon office · relays 0 devices*. Il log dell'agente dice `relay mode enabled: waiting for probes from the server` una volta che il primo batch
è accettato.

!!! note "Cosa l'agente relè deve raggiungere"
    L'agente relè esegue il probe dei dispositivi dalla sua stessa rete: ha bisogno dello stesso accesso
    che il server avrebbe se fosse lì — UDP 161 per SNMP, 8006 per
    Proxmox, 5001 per DSM, e così via. Per i monitor di **ping**, il contenitore
    ha bisogno di `cap_add: [NET_RAW]` (vedi il file compose). Per dispositivi su una VLAN
    o rete link-local, o per target `localhost`, usa
    `network_mode: host`.

## 4. Assegna i dispositivi all'agente relè

Aggiungi i dispositivi remoti come al solito — switch SNMP, nodo Proxmox, controllo HTTP —
con il loro indirizzo **come visto dal sito remoto** (`192.168.10.1`,
`https://pve.lan:8006`). Nel modulo del dispositivo, sotto *Più opzioni*, imposta
**Raggiungibile attraverso** all'agente relè invece di *Diretto*. I dispositivi esistenti
possono essere cambiati allo stesso modo; il passaggio indietro a *Diretto* fa sì che il server
li esamini di nuovo.

Da allora:

- il server accoda un probe per il dispositivo ad ogni intervallo; l'agente relè
  lo raccoglie entro millisecondi (aspetta il server per il lavoro), esegue il
  collettore e segnala;
- **Esegui probe ora** e il rilevamento del profilo sulla pagina del dispositivo passano attraverso
  l'agente relè anche;
- l'*ultimo probe* del dispositivo e l'errore seguono il verdetto dell'agente relè. Quando l'agente relè
  non raccoglie un probe in tempo, il dispositivo mostra *Timed out: relay
  agent … did not pick up the probe* — controlla che l'agente viene eseguito con
  `relay: true`;
- la pagina dell'agente mostra *Relay for N devices*, e `GET /api/relays` elenca
  gli agenti relè con il loro sito e conteggio.

## Avviso quando l'agente relè è inattivo

Un agente relè che rimane silenzioso porta i suoi dispositivi con sé: i loro probe si fermano, e
sarebbero tutti *irraggiungibili* una volta. L'agente relè è trattato come un
**genitore** di ogni dispositivo che esamina (in aggiunta al genitore che potresti avere
impostato): quando l'agente relè stesso è *irraggiungibile*, gli avvisi dei suoi dispositivi vengono
mostrati come `suppressed` con `suppressed_by` = l'agente relè, e solo una
notifica esce — per l'agente relè. Il genitore che hai impostato esplicitamente mantiene il suo ruolo e vince quando
entrambi sono inattivi.

## Limiti

- Un agente relè per dispositivo. Un dispositivo sottoposto a probe dall'agente relè non viene
  anche sottoposto a probe dal server.
- I dispositivi agente stessi non possono essere inoltrati: inoltrano le loro stesse metriche.
- L'agente relè esegue al massimo 8 probe per volta; un probe è limitato dal
  timeout del probe del server (`DUMBMONIT_PROBE_TIMEOUT_SECS`), limitato a 120 s su
  l'agente.
- Un agente relè ha bisogno dei collettori compilati in: gli agenti più vecchi di questa
  caratteristica ignorano `relay: true`, e il server segnala i loro probe come
  scaduti. Reinstalla con il binario o l'immagine corrente.
