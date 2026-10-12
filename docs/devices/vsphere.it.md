# VMware vSphere

Un vCenter Server e gli host che gestisce, oppure un host ESXi standalone:
host che smettono di rispondere, restano in modalità manutenzione o
segnalano uno stato di salute rosso, macchine virtuali e i loro VMware
Tools, datastore che si riempiono o diventano inaccessibili, e gli allarmi
generati da vSphere stesso.

DumbMonit legge vSphere con un utente dedicato che possiede il ruolo
integrato **Read-only**. Non accende o spegne mai una VM, non entra mai in
modalità manutenzione e non conferma mai un allarme.

Il guasto da tenere d'occhio: **un datastore che si riempie**. Quando un
datastore thin provisioned raggiunge il 100%, ogni VM che ha bisogno di
scriverci viene immediatamente messa in pausa. Gli allarmi che vCenter
genera lo dicono anch'essi, ma solo a chi ha il client aperto.

## Perché la Web Services API

DumbMonit parla con la vSphere Web Services API (SOAP, `/sdk`), non con
l'API REST (`/api/vcenter/*`): un host ESXi standalone non ha API REST, e
su un vCenter l'API REST non comunica la modalità manutenzione, lo stato
di salute generale (`overallStatus`), lo stato dei VMware Tools o gli
allarmi attivati. Una vista dell'intero inventario viene letta in un
singolo passaggio, pagina per pagina, e rilasciata alla fine.

La sessione viene mantenuta da una sonda alla successiva e riaperta solo
quando scade: vCenter non registra un nuovo login ogni minuto.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_vsphere_`. I valori di
stato di salute seguono l'`overallStatus` di vSphere: 0 green, 1 yellow, 2
red, 3 gray (sconosciuto).

| Metrica | Cosa | Etichette |
|---|---|---|
| `info` | valore 1 | `product` (`vcenter` o `esxi`), `version`, `build` |
| `hosts` | host per stato di connessione: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 connesso, 1 disconnesso da un amministratore, 2 non risponde | `host` |
| `host_power_state` | 0 acceso, 1 standby, 2 spento, 3 sconosciuto | `host` |
| `host_maintenance` | 1 in modalità manutenzione | `host` |
| `host_status` | stato di salute generale | `host` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | solo host connessi | `host` |
| `vms` | VM per stato di alimentazione: `poweredOn`, `poweredOff`, `suspended` (i template sono esclusi) | `state` |
| `templates` | template di VM | |
| `vm_power_state` | 0 acceso, 1 spento, 2 sospeso; al massimo 1000 VM | `vm` |
| `vm_status` | stato di salute generale | `vm` |
| `vm_tools_status` | solo VM accese: 0 aggiornato, 1 versione vecchia, 2 installato ma non in esecuzione, 3 non installato | `vm` |
| `datastores` | datastore | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 accessibile, 0 inaccessibile | `datastore` |
| `datastore_status` | stato di salute generale | `datastore` |
| `alarms` | allarmi attivati non confermati, per colore: `red`, `yellow` | `status` |
| `alarms_acknowledged` | allarmi attivati già confermati | |
| `alarm` | una serie per allarme non confermato, valore = il suo codice di stato di salute, al massimo 64 | `alarm` (il suo nome), `entity`, `entity_type`, `status` |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

Le [regole integrate](../alerting/rules.md#vmware-vsphere) che si
applicano: vSphere host not responding, vSphere host health red, vSphere
host left in maintenance, vSphere datastore almost full, vSphere
datastore inaccessible, vSphere VM tools not running, vSphere red alarm,
più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire
la pagina non interroga mai vSphere. Indica prima cosa non va (host che
non rispondono, allarmi rossi, datastore quasi pieni o inaccessibili), poi
gli host con il loro carico, i datastore con il loro livello di
riempimento, e le VM per stato di alimentazione.

## Come è stato validato

Contro il simulatore govmomi `vcsim` 0.56.0, in modalità vCenter e in
modalità ESXi: login e password rifiutata, riutilizzo e scadenza della
sessione, scorrimento dell'inventario per pagine, un host che non
risponde, un host in modalità manutenzione, una VM spenta e un template.
Il simulatore non genera alcun allarme: la lettura degli allarmi attivati
è validata contro la struttura documentata `AlarmState` della Web Services
API.

## Creare un utente di sola lettura in vSphere

1. Su un vCenter: Administration → Single Sign On → Users and Groups → Users, domain vsphere.local → Add. Dai all'utente il nome seguente e dagli una password lunga e casuale.

    ```
    dumbmonit
    ```

2. Ancora sul vCenter: Administration → Access Control → Global Permissions → Add. Scegli quell'utente, il ruolo Read-only, e spunta Propagate to children. Read-only vede ogni host, VM, datastore e allarme, e non può modificare nulla.

3. Su un host ESXi standalone invece: nel client dell'host, Manage → Security & users → Users → Add user, con lo stesso nome. Poi Host → Actions → Permissions → Add user: scegli l'utente, il ruolo Read-only, e spunta Propagate to all children.

4. In DumbMonit, inserisci l'indirizzo del vCenter o dell'host, per esempio "vcenter.lan" oppure "esxi1.lan", con il nome utente come creato: dumbmonit@vsphere.local su un vCenter, dumbmonit su un host. Quando un vCenter gestisce gli host, aggiungi solo il vCenter: riporta ogni host, VM e datastore.

5. DumbMonit legge soltanto, tramite la vSphere Web Services API (/sdk). Non accende o spegne mai una VM, non entra mai in modalità manutenzione e non conferma mai un allarme.

!!! warning
    Non riutilizzare l'account di dominio single sign-on o l'account integrato dell'host: il ruolo Read-only sopra è tutto ciò di cui DumbMonit ha bisogno. vCenter ed ESXi usano un certificato autofirmato per impostazione predefinita: abilita "Accept an unverifiable certificate" a meno che l'host DumbMonit non si fidi della loro autorità di certificazione.

## Credenziali

| Credenziale | Campi |
|---|---|
| User name / password | L'utente dedicato con il ruolo Read-only: `dumbmonit@vsphere.local` su un vCenter, `dumbmonit` su un host. |

Indirizzo: un nome host o IP (`vcenter.lan`), `host:porta`, oppure un URL
completo; un `/sdk` o `/ui` finale viene accettato. Opzioni: porta (443),
verifica del certificato, timeout della richiesta (20 s) e allarmi
attivati (lettura).
