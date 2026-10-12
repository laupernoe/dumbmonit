# Riferimento del formato del pacchetto

Un pacchetto è un singolo file YAML, `pack.yaml`, nella versione di
formato 1. Le chiavi sconosciute sono errori, non impostazioni ignorate:
un errore di digitazione come `treshold` viene segnalato.

```text
my-pack/
  pack.yaml          il pacchetto stesso: il solo file di cui il server ha bisogno
  fixtures/          una risposta catturata per fonte, nominata secondo il suo id
    status.json
  expected.prom      cosa deve produrre l'estrazione dalle fixture
```

## Intestazione

| Chiave | Richiesta | Significato |
|---|---|---|
| `schema` | sì | Sempre `1`. |
| `id` | sì | Lettere minuscole, cifre e trattini singoli, che inizia con una lettera, fino a 40 caratteri: `shelly-plug`. Il tipo di dispositivo diventa `pack.<id>`, il prefisso di metrica `dumbmonit_<id con underscore>_`. |
| `version` | sì | [Versione semantica](https://semver.org) del pacchetto: `1.2.0`. |
| `requires` | no | Versioni di DumbMonit con cui il pacchetto funziona, come requisito semver: `">=0.2"`. Un server fuori dall'intervallo rifiuta il pacchetto. |
| `label` | sì | Nome mostrato nell'elenco dei tipi di dispositivo. |
| `summary` | no | Una frase: cosa osserva il pacchetto. |
| `examples` | no | Prodotti a cui si applica, mostrati come suggerimenti. |
| `address_hint` | no | Segnaposto del campo indirizzo: `192.168.1.50`. |
| `default_port` | no | Porta usata quando l'indirizzo non ne ha una. |
| `scheme` | no | `http` (predefinito) oppure `https`; il dispositivo può sovrascriverlo. |
| `setup` | no | La nota mostrata accanto al modulo del dispositivo: `title`, `steps` (un elenco; ciò che segue un'interruzione di riga in un passaggio viene mostrato come un comando copiabile), `warning`, `doc_url`. |
| `credentials` | no | Tipi di credenziale accettati, in ordine di preferenza: `none`, `api_token`, `username_password`. Predefinito: `[none]`. Una voce può essere `{kind, label, help}` per riformulare il modulo. |
| `options` | no | Impostazioni mostrate nel modulo del dispositivo (sotto). |

## Opzioni

Le opzioni vengono memorizzate sul dispositivo e lette tramite
`{{option.<key>}}` in percorsi, header e corpi, e dalle soglie delle
regole.

```yaml
options:
  - key: switch_id          # minuscolo, cifre, underscore
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (predefinito), number, boolean o select
    default: 0
    required: false
    placeholder: ""
    choices: []             # i valori di un select
```

Ogni pacchetto che sonda ottiene anche cinque opzioni di connessione, che
non può ridichiarare: `scheme`, `port`, `insecure_tls`,
`request_timeout_seconds` (da 1 a 120, predefinito 10) e
`allow_private_targets`.

## Fonti

Una fonte è una richiesta per sondaggio, sempre inviata all'indirizzo
proprio del dispositivo.

```yaml
sources:
  - id: status              # minuscolo, cifre, underscore
    type: http              # http o prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (predefinito) o POST
    format: json            # solo http: json (predefinito) o text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # solo POST
    auth: auto              # auto (predefinito), none, bearer o basic
```

- `path` deve iniziare con un singolo `/`. L'URL della richiesta è
  l'indirizzo del dispositivo seguito dal percorso; dopo la
  sostituzione deve mantenere lo stesso schema, host e porta, altrimenti
  la richiesta viene rifiutata. I valori delle opzioni sono
  percent-encoded nei percorsi, così un valore non può aggiungere un
  segmento o cambiare l'host.
- `auth: auto` invia una credenziale `api_token` come
  `Authorization: Bearer …` e una credenziale `username_password` come
  autenticazione basic. `none` non invia nulla implicitamente, così un
  header può portare la credenziale invece:
  `X-Api-Key: "{{credential.token}}"`. `{{credential.token}}`,
  `{{credential.username}}` e `{{credential.password}}` sono accettati
  **solo negli header**: mai in un percorso o un corpo, che finiscono
  nei log.
- I redirect vengono seguiti solo verso lo stesso schema, host e porta,
  al massimo tre. `401` e `403` vengono segnalati come errori di
  autenticazione, altri stati non-2xx come errori di protocollo.
- Un sondaggio fa al massimo 16 richieste e legge al massimo 4 MiB per
  risposta.

### Fonti Prometheus

```yaml
sources:
  - id: metrics
    type: prometheus
    path: /metrics
    keep: [node_load1, node_memory_MemAvailable_bytes]
    rename:
      node_memory_MemAvailable_bytes: memory_available_bytes
    drop:
      fstype: ^(tmpfs|overlay)$
```

- `keep` elenca le famiglie importate come sono, con tutte le loro
  etichette. Un nome di famiglia copre anche le sue serie histogram e
  summary (`_bucket`, `_sum`, `_count`). Contatori, histogram e somme
  summary restano contatori; tutto il resto è un gauge.
- `rename` dà a una famiglia conservata il suo nome DumbMonit. Un nome
  che non è lettere minuscole, cifre e underscore deve essere rinominato.
- `drop` scarta ogni campione la cui etichetta corrisponde
  all'espressione regolare.
- Le etichette che DumbMonit imposta da solo (`target`, `host`,
  `instance`, `job`, `tag_*`…) vengono conservate come
  `exported_<nome>`.

## Metriche

Ogni metrica ha un nome *senza* prefisso: `power_watts` nel pacchetto
`shelly-plug` viene scritto `dumbmonit_shelly_plug_power_watts`. I nomi
sono lettere minuscole, cifre e underscore, e unici all'interno del
pacchetto.

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (predefinito) o counter
    json: $.apower          # esattamente uno tra json, regex o prom
    scale: 1                # moltiplicatore, es. 0.001 per ms → s
    map: {on: 1, off: 0}    # valore testuale → numero
    labels: {}
```

| Estrattore | Fonte | Valore | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, formato `json` | Ogni nodo che il percorso corrisponde ([RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath, wildcard e filtri inclusi) crea una serie. | etichetta → JSONPath valutato sull'oggetto che *contiene* il valore: `$.sensors[*].value` con etichette `sensor: $.name` etichetta ogni valore con il suo `name` gemello. |
| `regex: <expression>` | `http`, formato `text` | Ogni corrispondenza crea una serie; il valore è il gruppo chiamato `value`, altrimenti il primo gruppo. | etichetta → nome di un gruppo di capture. |
| `prom: <nome campione>` | `prometheus` | Ogni campione con questo nome esatto. | etichetta → etichetta della fonte. Vuoto: tutte le etichette della fonte vengono conservate. |

Valori: i numeri vengono presi come sono; `true`/`false` diventano 1/0;
il testo viene cercato in `map`, oppure analizzato come un numero quando
non c'è `map`. Un valore mancante, `null`, o fuori da `map` non produce
nulla per questo sondaggio.

## Discover

`discover` trasforma ogni elemento di un elenco in un insieme di serie
che condividono etichette: una riga per disco, sensore o coda.

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath: una riga per corrispondenza
    max_rows: 100           # predefinito 100, al massimo 1.000
    labels:                 # JSONPath valutato su ogni riga
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath valutato su ogni riga
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

Le metriche di riga accettano `name`, `help`, `kind`, `json`, `scale` e
`map`. Le righe oltre `max_rows` vengono ignorate.

## Regole

Le regole vengono installate con il pacchetto, come regole di avviso
ordinarie con l'uid `pack:<id>:<name>`. Allora ti appartengono: un
aggiornamento del pacchetto aggiunge le regole che non ha ancora, e non
riscrive mai una che hai modificato o eliminato.

```yaml
rules:
  - name: overheating       # minuscolo, cifre, underscore
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < o <=
    threshold: "{{option.max_temperature}}"   # oppure un numero
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (predefinito) o critical
    unit: °C
```

- `expr` è [MetricsQL](https://docs.victoriametrics.com/metricsql/) e
  deve usare almeno una metrica del pacchetto, con il suo nome completo.
  Qualsiasi altro nome `dumbmonit_*` che cita deve essere prodotto anche
  dal pacchetto (`dumbmonit_up` eccettuato): un pacchetto non può
  avvisare sulle serie di un altro tipo.
- Una `threshold` di `{{option.x}}` installa il valore predefinito
  dell'opzione, e il valore proprio di ogni dispositivo per quella
  opzione diventa la sua soglia per questa regola (un override per
  dispositivo, riscritto ogni volta che il dispositivo viene salvato).

## Profili SNMP

Un pacchetto può portare profili SNMP, scritti esattamente come quelli
integrati in
[`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles).
Si uniscono al catalogo SNMP e vengono applicati per `sysObjectID` come
gli altri; non creano un proprio tipo di dispositivo.

```yaml
snmp_profiles:
  - id: acme-ups            # l'id del pacchetto, oppure che inizia con "<id pacchetto>-"
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # deve iniziare con il prefisso del pacchetto
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

Il catalogo SNMP viene letto all'avvio: riavvia DumbMonit dopo aver
installato o aggiornato un pacchetto con profili SNMP.

## Limiti

| Limite | Valore |
|---|---|
| Dimensione del pacchetto | 256 KiB |
| Fonti (richieste per sondaggio, redirect inclusi) | 16 |
| Dimensione della risposta | 4 MiB |
| Serie per sondaggio | 5.000 (il resto viene scartato e registrato nei log) |
| Metriche dichiarate | 256 |
| Regole | 64 |
| Righe per `discover` | 1.000 |
| Lunghezza del valore di etichetta | 200 byte |

## Fixture e output atteso

`dumbmonit pack test <dir>` legge `fixtures/<source id>.json`, `.txt` o
`.prom`, esegue la stessa estrazione del server, e confronta il
risultato con `expected.prom`: il formato testo Prometheus, ordinato,
senza timestamp e senza le etichette del dispositivo (`target`, `host`,
`tag_*`) che il server aggiunge. `--update` scrive `expected.prom` dalle
fixture; rivedi il diff prima di fare il commit.
