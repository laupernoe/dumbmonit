# Sophos Firewall

Un Sophos Firewall (SFOS) tramite la sua API XML: il collegamento di ogni
interfaccia associata a una zona, e quali connessioni IPsec sono attivate.

Questo è deliberatamente tutto. L'API XML è un'API di configurazione: a
parte lo stato di connettività in sola lettura di ciascuna interfaccia, non
espone alcuno stato live. `<Get><IPSecConnection>` restituisce se un
amministratore ha attivato una connessione, non se il tunnel è attivo; alta
disponibilità, licenze e abbonamenti e la versione del firmware non vi sono
affatto. DumbMonit pubblica l'attivazione come tale e non la presenta mai
come uno stato del tunnel.

Ogni minuto DumbMonit invia due richieste POST a
`https://<firewall>:4444/webconsole/APIController` (campo del form `reqxml`,
mai nell'URL, dove la password finirebbe nei log): `<Get><Interface>` e
`<Get><IPSecConnection>`. Ognuna porta il nome utente e la password di un
account il cui profilo di accesso al dispositivo è di sola lettura
ovunque: il firewall applica quel profilo anche all'API.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dalla documentazione dell'API XML
    di Sophos (le entità `Interface` e `IPSecConnection`, i messaggi di
    login e stato), non su un firewall in funzione. La dicitura esatta dello
    stato dell'interfaccia (`Connected, 1000 Mbps…` oppure `Disconnected`)
    proviene da quella documentazione e da segnalazioni della comunità.
    Facci sapere cosa non funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_sophos_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `api_version_info` | `api_version` | Valore 1: la versione dell'API annunciata dal firewall (`2000.2`), che segue la release SFOS. |
| `interface_link_up` | `interface`, `zone` | 1 quando lo stato indica Connected, 0 quando Disconnected; assente quando lo stato è qualcos'altro. |
| `interface_enabled` | `interface`, `zone` | 0 quando l'interfaccia è spenta. |
| `interface_down` | `interface`, `zone` | 1 quando l'interfaccia è accesa, associata a una zona, e non ha link. Le porte libere (zona None) sono lasciate stare. |
| `interfaces`, `interfaces_down` | | Conteggi. |
| `ipsec_connection_activated` | `connection` | 1 quando la connessione è attivata nella configurazione. Non lo stato del tunnel. |
| `ipsec_connections` | | Conteggio. |

La [regola integrata](../alerting/rules.md#sophos-firewall) che si applica:

- **Sophos interface without link**: un'interfaccia accesa e associata a una
  zona non ha avuto link per cinque minuti (Advisory).

Per i contatori di traffico, aggiungi anche il firewall come
[dispositivo SNMP](snmp.md): SFOS risponde alla MIB standard delle
interfacce.

## La pagina del monitor

Il pannello Firewall indica se ogni interfaccia in una zona ha il suo link,
poi elenca quelle interfacce e le connessioni IPsec con la loro attivazione
configurata, etichettata come tale.

## Consentire a DumbMonit di leggere l'API XML di Sophos Firewall

1. Sotto Profiles > Device access, aggiungi un profilo chiamato DumbMonit read-only e imposta ogni permesso su Read-only.

2. Sotto Authentication > Users, aggiungi un utente chiamato dumbmonit, di tipo Administrator, con quel profilo e una password lunga e casuale.

3. Sotto Backup & firmware > API (Administration > API access da SFOS 22), attiva la configurazione dell'API e aggiungi l'indirizzo del server DumbMonit agli indirizzi IP consentiti.

4. In DumbMonit, inserisci l'indirizzo del firewall, per esempio "sophos.lan", con il nome utente e la password dumbmonit. L'API ascolta sulla porta di amministrazione, 4444 per impostazione predefinita, con un certificato autofirmato: spunta Accept an unverifiable certificate a meno che tu non ne abbia installato uno proprio.

!!! warning
    L'API XML è un'API di configurazione: riporta il link di ogni interfaccia e se ogni connessione IPsec è attivata, non se il tunnel è attivo. Alta disponibilità, licenze e la versione del firmware non vi sono.

## Credenziali

| Credenziale | Campi |
|---|---|
| User name and password | L'utente `dumbmonit` con il profilo di accesso al dispositivo di sola lettura. |

Indirizzo: il nome host o IP del firewall, `host:porta`, oppure un URL.
Opzioni:

- **Port** (4444): la porta di amministrazione.
- **Accept an unverifiable certificate**: il certificato proprio del
  firewall è autofirmato.
- **Timeout per request** (15 s).

## Risoluzione dei problemi

"Sophos Firewall refused the request (534 …)": l'indirizzo DumbMonit non è
tra gli indirizzi IP consentiti della configurazione dell'API.

"Sophos Firewall refused the login": nome utente o password errati, oppure
una password con caratteri che il firewall rifiuta nelle chiamate API; una
password lunga di lettere e cifre evita il problema.

"Sophos Firewall did not answer in XML": porta errata, oppure l'API è
disattivata.
