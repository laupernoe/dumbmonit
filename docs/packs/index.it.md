# Pacchetti di integrazione

Un pacchetto di integrazione aggiunge un tipo di dispositivo a DumbMonit
senza una nuova release. È un singolo file YAML che dice dove leggere
(un'API HTTP o una pagina Prometheus `/metrics` sul dispositivo), cosa
estrarre, e quali regole di avviso lo accompagnano. Una volta installato,
il tipo del pacchetto appare in **Add device** accanto a quelli
integrati, e i suoi dispositivi vengono sondati, graficati e avvisati
come qualsiasi altro.

Un pacchetto è dichiarativo: niente codice, niente script. DumbMonit
esegue ogni richiesta da solo, sotto regole che il pacchetto non può
cambiare.

## Cosa un pacchetto può e non può fare

| Un pacchetto può | Un pacchetto non può |
|---|---|
| Chiamare percorsi `GET` o `POST` sull'indirizzo proprio del dispositivo | Chiamare qualsiasi altro host, seguire un redirect verso un altro host, o usare un URL assoluto |
| Leggere JSON (JSONPath), testo semplice (espressioni regolari) o il formato testo Prometheus | Eseguire codice, comandi shell o template oltre a `{{option.x}}` e `{{credential.x}}` |
| Inviare la credenziale del dispositivo in un header o come autenticazione basic/bearer | Metter una credenziale in un URL o nel corpo di una richiesta |
| Dichiarare opzioni mostrate nel modulo del dispositivo (un canale da switch, una soglia) | Scrivere metriche fuori dal proprio prefisso `dumbmonit_<pack>_` |
| Fornire regole di avviso sulle proprie metriche | Modificare un tipo di dispositivo integrato o una regola integrata |
| Portare profili SNMP, nel formato di quelli integrati | Leggere più di 4 MiB per risposta, fare più di 16 richieste per sondaggio, o produrre più di 5.000 serie per sondaggio |

Gli indirizzi loopback e link-local (`127.0.0.1`, `169.254.x.x`) vengono
rifiutati a meno che l'opzione **Allow loopback and link-local targets**
del dispositivo non sia attiva, esattamente come i controlli HTTP. Gli
indirizzi di LAN privata sono sempre consentiti.

## Pacchetti di riferimento

Il repository fornisce tre pacchetti sotto
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs), ciascuno
con le sue fixture e l'output atteso:

| Pacchetto | Legge | Regole |
|---|---|---|
| `node-exporter` | un sottoinsieme curato di Prometheus node_exporter (CPU, memoria, file system, dischi, rete) | file system quasi pieno, memoria quasi esaurita |
| `shelly-plug` | prese e relè Shelly Gen2+, `/rpc/Switch.GetStatus` | sovrariscaldamento |
| `speedtest-tracker` | l'ultimo risultato dell'API v1 di Speedtest Tracker | internet sotto le aspettative, test di velocità fallito |

## Installare un pacchetto

Nell'interfaccia, come amministratore: **Settings → Integration packs**.
Incolla il YAML del pacchetto o carica il suo `pack.yaml`, poi
**Install the pack**. Un pacchetto che non supera i controlli viene
rifiutato con l'elenco di cosa correggere; uno che passa viene salvato
con le sue note, se presenti. La stessa pagina attiva e disattiva i
pacchetti e li disinstalla. I tipi installati appaiono sotto
**Community packs** quando aggiungi un dispositivo, contrassegnati
*Pack*.

Tramite l'API HTTP, hai bisogno di una sessione amministratore oppure di
un token API con l'ambito `write` (Settings → **API & assistants**).

```bash
# Installa, oppure aggiorna a una versione più recente dello stesso pacchetto
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @packs/shelly-plug/pack.yaml http://localhost:8080/api/packs

# Elenca cosa è installato
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/api/packs
```

La risposta dice cosa è successo (`created`, `updated` o `unchanged`), le
metriche che il pacchetto produce e quante regole di avviso sono state
aggiunte. Un pacchetto che non supera la validazione viene rifiutato con
l'elenco completo dei problemi, uno per riga.

| Chiamata | Fa |
|---|---|
| `GET /api/packs` | pacchetti installati: versione, tipo, metriche, regole, dispositivi che lo usano |
| `GET /api/packs/{id}` | un pacchetto |
| `POST /api/packs` | installa o aggiorna; corpo: il YAML (`application/yaml`), oppure `{"yaml": "…"}` in JSON |
| `PUT /api/packs/{id}/disable` | ferma il sondaggio dei dispositivi del pacchetto, mantiene tutto il resto |
| `PUT /api/packs/{id}/enable` | li sonda di nuovo |
| `DELETE /api/packs/{id}` | disinstalla, con le sue regole; rifiutato (`409`) mentre dispositivi lo usano ancora |

Con un cookie di sessione invece di un token, aggiungi
`X-Requested-With: DumbMonit` a ogni `POST`, `PUT` e `DELETE`, come per il
resto dell'[API](../reference/api.md).

L'installazione ha effetto immediato: il nuovo tipo è disponibile nel
modulo del dispositivo senza riavvio. L'unica eccezione è un pacchetto
che porta profili SNMP; la risposta allora dice
`"restart_required": true`, perché il catalogo SNMP viene letto
all'avvio.

### Aggiornamenti e le tue modifiche

Aggiornare un pacchetto (stesso `id`, nuovo contenuto) sostituisce la sua
estrazione e la sua descrizione. Le sue regole di avviso vengono solo
*aggiunte*: una regola che hai modificato, disattivato o eliminato non
viene mai riscritta da un aggiornamento.

Disattivare un pacchetto ritira il suo tipo di dispositivo e mantiene
tutto il resto: dispositivi, cronologia, regole. I suoi dispositivi non
vengono più sondati (ognuno mostra un errore di configurazione) e, come
qualsiasi dispositivo che smette di riportare, generano *Device
unreachable* dopo tre minuti: metti anche loro in pausa se disattivi il
pacchetto per un po'.

### Backup

I pacchetti installati viaggiano nei [backup](../install/backup.md) con
il loro YAML, e vengono ripristinati prima dei dispositivi che li usano.

## Verificare un pacchetto prima di installarlo

Il binario del server valida e testa i pacchetti offline:

```bash
# Controlli di schema e coerenza, senza un server o un database
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# Riproduce fixtures/ attraverso l'estrazione reale e confronta con expected.prom
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` riscrive `expected.prom` dalle fixture (monta la
directory in lettura-scrittura ed esegui con `--user "$(id -u)"` così il
file resta tuo).

Prossimo passo: [scrivi il tuo primo pacchetto](first-pack.md) in circa
dieci minuti, oppure leggi il
[riferimento del formato](format.md).
