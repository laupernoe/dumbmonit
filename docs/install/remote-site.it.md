# Monitorare una sede remota

Un solo DumbMonit, diverse reti. Un **agent relay** installato nella sede
remota esegue, per conto del server, le sonde dei dispositivi che gli
assegni, e riporta i risultati tramite la stessa connessione in uscita
che usa già per inviare le proprie metriche. Nulla da aprire nella sede
remota, nessuna VPN, nessuna seconda istanza da controllare.

```
 sede principale                          sede remota (NAT, firewall)
 ┌──────────────────┐   HTTPS, outbound   ┌──────────────────────────────┐
 │ DumbMonit server │◀────────────────────│ relay agent                  │
 │  scheduler       │  probes to run  ──▶ │  runs SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alerting, UI    │◀── samples, verdict │  pushes its own metrics too  │    hypervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Pubblicare il server su HTTPS

Il relay recupera i suoi job dal server: il server deve essere
raggiungibile dalla sede remota, tipicamente
`https://monitor.example.org` dietro un reverse proxy. Due cose
viaggiano su quel collegamento:

- il **token di iscrizione** (`dmon_…`), nell'header `Authorization` di
  ogni richiesta — come per qualsiasi agent;
- le **credenziali dei dispositivi** dei dispositivi che il relay sonda
  (community SNMP, token API Proxmox…), decifrate, dentro ogni job di
  sonda.

Su HTTP semplice attraverso internet entrambe sarebbero leggibili lungo
il percorso: metti il server dietro HTTPS prima di fare relay di
qualsiasi cosa fuori dalla tua stessa rete. Su una LAN o una VPN, HTTP è
accettabile e il token è il solo segreto in transito finché non assegni
dispositivi al relay. Le credenziali non vengono mai scritte su disco su
nessuno dei due lati: un job di sonda vive nella memoria del server finché
il relay non lo riporta, e l'agent non mantiene nulla tra due sonde.

Il reverse proxy deve inoltrare `/api/agent/relay` come `/api/ingest`,
consentire corpi di richiesta fino a 16 MB (un hypervisor occupato
riporta migliaia di campioni), e mantenere le connessioni inattive aperte
per almeno 60 secondi: il server mantiene una richiesta relay in attesa
fino a 25 secondi mentre aspetta lavoro (long polling).

## 2. Creare un token di iscrizione

**Settings → Agents → New token**, oppure aggiungi un dispositivo di tipo
*agent*. Copia il valore `dmon_…`: viene mostrato una volta.

## 3. Avviare il relay nella sede remota

Qualsiasi macchina nella sede remota con Docker, usando
`docker-compose.agent.yml` dal repository:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

Oppure l'agent normale installato con l'installer a una riga, con due
chiavi in più in `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

La macchina appare in DumbMonit entro secondi, come un dispositivo agent
ordinario, e la sua pagina dice *Relay · site Lyon office · relays 0
devices*. Il log dell'agent dice `relay mode enabled: waiting for probes
from the server` una volta accettato il suo primo batch.

!!! note "Cosa il relay ha bisogno di raggiungere"
    Il relay sonda i dispositivi dalla sua propria rete: ha bisogno dello
    stesso accesso che il server avrebbe bisogno se fosse lì — UDP 161
    per SNMP, 8006 per Proxmox, 5001 per DSM, e così via. Per i monitor
    **ping**, il container ha bisogno di `cap_add: [NET_RAW]` (vedi il
    file compose). Per dispositivi su una VLAN o rete link-local, oppure
    per target `localhost`, usa `network_mode: host`.

## 4. Assegnare dispositivi al relay

Aggiungi i dispositivi remoti come al solito — switch SNMP, nodo Proxmox,
controllo HTTP — con il loro indirizzo **come visto dalla sede remota**
(`192.168.10.1`, `https://pve.lan:8006`). Nel modulo del dispositivo,
sotto *More options*, imposta **Reached through** sull'agent relay
invece di *Direct*. I dispositivi esistenti possono essere cambiati allo
stesso modo; tornare a *Direct* fa sondare di nuovo dal server.

Da quel momento:

- il server accoda una sonda per il dispositivo a ogni intervallo; il
  relay la raccoglie entro millisecondi (attende sul server del lavoro),
  esegue il collector e riporta;
- **Probe now** e il rilevamento del profilo sulla pagina del dispositivo
  passano anch'essi attraverso il relay;
- l'*ultima sonda* e l'errore del dispositivo seguono il verdetto del
  relay. Quando il relay non raccoglie una sonda in tempo, il dispositivo
  mostra *Timed out: relay agent … did not pick up the probe* — verifica
  che l'agent funzioni con `relay: true`;
- la pagina dell'agent mostra *Relay for N devices*, e `GET /api/relays`
  elenca i relay con il loro sito e conteggio.

## Avvisi quando il relay è inattivo

Un relay che diventa silenzioso porta con sé i suoi dispositivi: le loro
sonde si fermano, e diventerebbero tutti *unreachable* in una volta. Il
relay viene trattato come un **genitore** di ogni dispositivo che sonda
(in aggiunta al genitore che potresti avere impostato): quando il relay
stesso è *unreachable*, gli avvisi dei suoi dispositivi vengono mostrati
come `suppressed` con `suppressed_by` = il relay, e solo una notifica
viene inviata — per il relay. Il genitore che hai impostato esplicitamente
mantiene il suo ruolo e vince quando entrambi sono inattivi.

## Limiti

- Un relay per dispositivo. Un dispositivo sondato dal relay non viene
  anche sondato dal server.
- I dispositivi agent stessi non possono essere fatti relayare: inviano
  le proprie metriche.
- Il relay esegue al massimo 8 sonde alla volta; una sonda è limitata dal
  timeout di sonda del server (`DUMBMONIT_PROBE_TIMEOUT`), limitato a
  120 s sull'agent.
- Un agent relay ha bisogno dei collector compilati dentro: agent più
  vecchi di questa funzionalità ignorano `relay: true`, e il server
  segnala le loro sonde come scadute. Reinstalla con il binario o
  l'immagine attuale.
