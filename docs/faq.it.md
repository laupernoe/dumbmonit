# FAQ

## Ho perso la password {#i-lost-the-password}

Chiedi prima a un altro amministratore: in Settings → **Users**, **Edit**
sul tuo account imposta una nuova password e la mostra una volta, e **Reset
2FA** sulla stessa riga azzera un secondo fattore che ti blocca fuori.
Questa è la risposta quotidiana — un'istanza con due account amministratore
non ha mai bisogno del resto di questa pagina.

Quando nessuno può più accedere, non c'è un'email di recupero: avvia il
server una volta con `DUMBMONIT_RESET_PASSWORD=1`: aggiungi la riga sotto
`environment:` del servizio in `docker-compose.yml` ed esegui
`docker compose up -d`:

```yaml
    environment:
      DUMBMONIT_RESET_PASSWORD: "1"
```

Ogni account e ogni sessione viene rimosso all'avvio e l'interfaccia mostra
di nuovo `/setup`. Crea il primo amministratore, poi riavvia senza la
riga (rimuovila da `docker-compose.yml`, poi `docker compose up -d`). Dispositivi, regole, canali e le impostazioni di
single sign-on restano intatti.

## Nessun dato dopo l'aggiunta di un dispositivo {#no-data-after-adding-a-device}

Dagli un minuto: la prima sonda viene eseguita all'intervallo del
dispositivo (60 s per impostazione predefinita) e le scritture raggiungono
VictoriaMetrics ogni 5 secondi (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Poi:

1. Apri la pagina del dispositivo e clicca **Probe now**. Riporta quanti
   campioni e serie una sonda ha prodotto, oppure l'errore esatto.
2. *Misconfigured* con un errore sotto l'indirizzo è un problema di
   configurazione: community o password errata, certificato rifiutato
   (abilita "Accept an unverifiable certificate"), funzionalità mancante.
   Viene mostrato, non avvisato.
3. *Unreachable* senza errore significa che il dispositivo non ha risposto
   affatto. Per SNMP, una community errata appare esattamente come un
   dispositivo spento, perché il dispositivo non risponde mai a essa.
   Verifica la community, che SNMP sia abilitato, e che il dispositivo
   consenta l'indirizzo del server.
4. Verifica che il container possa raggiungere il dispositivo: le reti
   Docker e i firewall sono i colpevoli abituali.

## Ping dice "configuration error"

Il monitor ping richiede un socket echo ICMP. Il container funziona senza
alcuna capability, quindi il file Compose consente quei socket con il
sysctl `net.ipv4.ping_group_range` sotto il servizio `dumbmonit`:
assicurati che le righe `sysctls:` siano ancora presenti, poi
`docker compose up -d`. Vedi
[Ping ICMP senza una capability](install/docker.md#icmp-ping-without-a-capability).

## L'agent non appare

- L'installer esegue un push di prova prima di avviare il servizio e si
  interrompe con un errore esplicito se l'URL o il token sono errati.
  Correggi `/etc/dumbmonit/agent.yaml` e riavvia `dumbmonit-agent`.
- L'URL nel comando di installazione è quello che il tuo browser ha usato
  per raggiungere l'interfaccia. Dalla macchina monitorata, quell'URL deve
  raggiungere il server: `curl http://server:8080/api/health` da lì te lo
  dice.
- Dietro un reverse proxy, `/api/ingest` deve essere forwardato e i corpi
  fino a 16 MB consentiti (`client_max_body_size 16m` in nginx).
- `journalctl -u dumbmonit-agent -f` mostra ogni push rifiutato. Il token
  stesso non appare mai nel log.
- Un token revocato ferma l'agent al suo prossimo push; esegui di nuovo
  l'installer con un nuovo token.

Vedi [Agent Linux, macOS, FreeBSD e Windows](devices/agent.md).

## Cambiare la porta

Il container ascolta sulla 8080; la porta dell'host è quella che
`docker-compose.yml` pubblica. Modifica il lato sinistro della mappatura,
poi esegui `docker compose up -d`:

```yaml
    ports:
      - "8099:8080"
```

Per cambiare la porta *dentro* il container, imposta `DUMBMONIT_BIND`, per
esempio `0.0.0.0:9000`, e adatta la mappatura della porta.

## Perché l'avviso è "Advisory" quando la regola dice "warning"?

L'API memorizza tre severità: `info`, `warning`, `critical`. L'interfaccia
le mostra con le parole del bollettino meteo: Info, Advisory, Warning.
Vedi [Severità](alerting/index.md#severities).

## Un avviso è "Suppressed by parent"

Il dispositivo ha un dispositivo genitore irraggiungibile, quindi i suoi
propri avvisi vengono trattenuti invece che inviati. Risolvi il genitore;
gli avvisi del figlio si risolvono o si attivano da soli in seguito senza
una seconda notifica. Vedi
[Soppressione per dipendenza](alerting/index.md#dependency-suppression).

## "Unusual CPU" non si attiva mai

È una baseline stagionale che resta silenziosa per i suoi primi 14 giorni
su ogni serie. Durante quel periodo mostra solo cosa si sarebbe attivato.
Vedi [Apprendimento della baseline](alerting/index.md#baseline-learning).

## Posso usare il mio VictoriaMetrics?

Sì: imposta `DUMBMONIT_VM_URL` sul suo indirizzo (`http://host:8428`) e il
VictoriaMetrics integrato non viene avviato. Ogni metrica ha il prefisso
`dumbmonit_`, così può condividere l'istanza con altri strumenti.
`GET /api/health` riporta `victoria.embedded: false` una volta effettuato
il cambio.

## DumbMonit deve essere raggiungibile da internet?

No. Contatta i dispositivi e i servizi di notifica; nulla richiama,
eccetto gli agent, che devono raggiungere l'URL del server. Tienilo sulla
tua LAN o dietro una VPN; se lo esponi, usa un reverse proxy con TLS e
imposta `DUMBMONIT_COOKIE_SECURE=1`.

## Esiste un token API?

Sì. Settings → **API & assistants** crea token `dmt_…` con un ambito
`read` o `write`; invia uno come `Authorization: Bearer dmt_…` su qualsiasi
rotta e nessun cookie o header CSRF è necessario. Lo stesso token collega
un [assistente](using/assistant.md) tramite MCP. I token non gestiscono mai
account, impostazioni di accesso o altri token. Vedi il
[riferimento API HTTP](reference/api.md#authentication).
