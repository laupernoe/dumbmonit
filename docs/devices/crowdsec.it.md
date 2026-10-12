# CrowdSec

Il motore di sicurezza: decisioni attive e da dove provengono, avvisi,
bouncer che hanno smesso di scaricare, righe di log lette e quelle che
nessun parser ha riconosciuto, e se la Local API risponde.

DumbMonit legge le metriche Prometheus di CrowdSec (porta 6060) e controlla
`/health` della Local API (porta 8080). Con una chiave bouncer opzionale,
chiede anche alla Local API le decisioni su 192.0.2.1, un indirizzo di
documentazione: la risposta è sempre vuota, e prova che l'autenticazione e
il database rispondono. Non crea né elimina mai una decisione e non scarica
mai la lista dei ban.

Il guasto da tenere d'occhio è un bouncer che ha smesso di scaricare. Il
firewall o il reverse proxy continua a funzionare con la lista che aveva, i
nuovi attaccanti passano, e nulla si lamenta. CrowdSec non pubblica quando
un bouncer ha scaricato l'ultima volta; DumbMonit lo legge dal contatore di
richieste del bouncer, che smette di muoversi. Il secondo è un'acquisizione
che si è rotta (un file di log rinominato, un container ricreato): CrowdSec
non legge nulla, non vede alcun attacco, e resta verde.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_crowdsec_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `lapi_up` | 1 quando la Local API risponde `/health` | |
| `lapi_decisions_ok` | con una chiave bouncer: 1 quando la Local API ha risposto alla query delle decisioni | |
| `decisions` | decisioni attive, blocklist della community incluse | |
| `decisions_by_origin` | | `origin`: `crowdsec`, `cscli`, `CAPI`, `lists`… |
| `alerts` | avvisi conservati dalla Local API, quelli della community esclusi | |
| `bouncers_seen` | bouncer che hanno chiamato la Local API da quando CrowdSec è stato avviato | |
| `bouncer_requests_total` | richieste di ogni bouncer; quando smette di muoversi, il bouncer ha smesso di scaricare | `bouncer` |
| `machine_requests_total` | richieste di ogni agent (machine) | `machine` |
| `lapi_requests_total` | tutte le richieste alla Local API | |
| `scenario_overflows_total` | scenari attivati dall'agent locale: ognuno è un avviso | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | righe di log lette dall'agent, e quelle analizzate o no | |
| `source_lines_total`, `source_lines_unparsed_total` | lo stesso per fonte di log | `source` |
| `buckets` | bucket di scenario attualmente aperti | |

Le famiglie della Local API appaiono dove gira la Local API, le famiglie
dei log dove un agent legge i log; l'immagine ufficiale e un'installazione
standard eseguono entrambi.

Le [regole integrate](../alerting/rules.md#databases-message-broker-and-security-engine)
che si applicano: CrowdSec Local API down, CrowdSec bouncer stopped
pulling, CrowdSec reads no logs, più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai CrowdSec. Indica cosa non va in una frase (Local
API, bouncer, lettura dei log e la quota di righe che nessun parser ha
riconosciuto), poi le decisioni attive, gli avvisi conservati, gli scenari
attivati nelle ultime 24 ore e le righe lette al secondo, poi il tempo
dall'ultimo scaricamento di ogni bouncer, le decisioni per origine e la
quota non analizzata di ogni fonte di log.

## Lasciare che DumbMonit legga le metriche di CrowdSec

1. CrowdSec pubblica le sue metriche su 127.0.0.1:6060 per impostazione predefinita. Imposta listen_addr nella sezione prometheus di /etc/crowdsec/config.yaml su un indirizzo che DumbMonit possa raggiungere, poi riavvia CrowdSec. L'immagine Docker ufficiale ascolta già su ogni interfaccia.

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. Le metriche non richiedono alcuna credenziale: contengono conteggi, nomi di bouncer e machine e percorsi di file di log, mai un indirizzo IP. Filtra la porta 6060 così che solo l'host DumbMonit la raggiunga, e verifica che risponda.

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. DumbMonit controlla anche /health della Local API, sulla porta 8080 dello stesso host: listen_uri nella sezione api.server deve allora essere anch'esso raggiungibile. Deseleziona Check the Local API per un host che esegue solo un agent.

4. Opzionale: crea una chiave bouncer per DumbMonit e incollala come Bouncer API key più sotto. Una chiave bouncer può solo leggere le decisioni. DumbMonit chiede le decisioni su 192.0.2.1, un indirizzo di documentazione, così la risposta è sempre vuota e la lista dei ban non viene mai scaricata.

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. In DumbMonit, inserisci l'host CrowdSec, per esempio "crowdsec.lan". Dove la Local API e gli agent girano su host diversi, aggiungi ogni host: decisioni, avvisi e bouncer vengono contati dove gira la Local API, le righe di log dove ogni agent le legge.

!!! warning
    Un bouncer che smette di scaricare continua ad applicare una lista congelata: nuovi attaccanti passano e nulla si lamenta. DumbMonit avvisa quando le richieste di un bouncer si fermano per 30 minuti. Conosce solo i bouncer che hanno scaricato almeno una volta da quando CrowdSec è stato avviato l'ultima volta.

Un bouncer eliminato con `cscli bouncers delete` mantiene il suo contatore
finché CrowdSec non si riavvia, e viene segnalato come fermo finché non
avviene. Un bouncer che chiama la Local API solo quando serve una richiesta
(un bouncer reverse proxy in modalità live) può restare silenzioso per
mezz'ora in una notte tranquilla; passalo in modalità stream, oppure alza
la finestra della regola.

## Credenziali

| Credenziale | Campi |
|---|---|
| Metrics only | Nessuna: legge `/metrics`, e `/health` della Local API. |
| Bouncer API key | La chiave stampata da `cscli bouncers add dumbmonit`, inviata come `X-Api-Key` alla Local API. |

Indirizzo: un nome host o IP (`crowdsec.lan`), `host:porta` per le
metriche, oppure un URL completo. Opzioni: protocollo (HTTP per
impostazione predefinita), porta delle metriche (6060), Check the Local
API (attivo), porta Local API (8080), verifica del certificato e timeout
della richiesta (10 s). Gli elementi del Hub (parser, scenari, collezioni)
contaminati o non aggiornati non sono visibili tramite le metriche, solo
con `cscli hub list` sull'host.
