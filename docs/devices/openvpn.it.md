# Server OpenVPN

Il processo server di OpenVPN espone un proprio canale di controllo, la
**management interface**: un listener TCP locale, separato dalla VPN vera e
propria, pensato per uno strumento di monitoraggio e non per un client VPN.
DumbMonit la legge come qualsiasi client di gestione: la versione del server,
se si dichiara connesso e ogni client attualmente in sessione, con il suo
indirizzo reale e virtuale e il suo traffico. Non viene mai inviato nulla che
possa cambiare lo stato del server o disconnettere un client.

I client sono transitori per natura — un portatile che va in sospensione
scompare semplicemente dalla lettura successiva. DumbMonit non cerca di
ricordare chi era connesso cinque minuti fa; riporta solo chi è connesso ora,
più il numero di client come metrica da rappresentare nel tempo.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_openvpn_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `server_info` | valore 1 | `version` |
| `server_up` | 1 quando la management interface segnala il server `CONNECTED`, 0 altrimenti | `state` |
| `server_state_age_seconds` | tempo dall'ingresso del server nel suo stato attuale | |
| `clients_connected` | numero di client attualmente in sessione | |
| `client_bytes_received`, `client_bytes_sent` | traffico di un client (contatori) | `common_name`, `real_address` |
| `client_connected_seconds` | tempo dall'inizio della sessione di quel client | `common_name`, `real_address` |

`real_address` non riporta mai la porta di origine del client: cambia a ogni
riconnessione dietro NAT e moltiplicherebbe le serie per nulla. Al massimo 256
client ricevono ciascuno una serie dettagliata; `clients_connected` resta
esatto oltre tale limite.

Regole integrate che si applicano: **Dispositivo irraggiungibile**, quando la
connessione di gestione stessa non può essere stabilita o la password viene
rifiutata.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non si connette mai alla management interface. Mostra la versione del
server e se si dichiara connesso, il numero di client e un elenco dei client
attualmente in sessione con il loro indirizzo e il loro traffico.

## Attivare la management interface

1. La management interface è un listener separato, disattivato per impostazione predefinita: aggiungi questo alla configurazione del server (server.conf, oppure as.conf di OpenVPN Access Server con un altro nome) e riavvialo. Associalo a un indirizzo raggiungibile dall'host DumbMonit, oppure a 127.0.0.1 e raggiungilo tramite un tunnel SSH o un agent relay sulla stessa macchina.

    ```
    management 0.0.0.0 7505
    ```

2. Facoltativo ma consigliato: la management interface non ha un account proprio, solo una password condivisa. Mettila in un file leggibile solo dal processo OpenVPN e aggiungilo come terzo argomento.

    ```
    management 0.0.0.0 7505 /etc/openvpn/management-password
    ```

3. In DumbMonit, inserisci l'indirizzo del server, ad esempio "vpn.lan" (porta 7505), e la password di gestione se ne è impostata una.

4. DumbMonit legge soltanto: invia version, state e status 3, mai signal, kill o client-kill. Non può disconnettere un client, cambiare un'impostazione né fermare il server.

!!! warning
    Chiunque raggiunga la porta di gestione e conosca la password può disconnettere i client, leggere tutti i loro indirizzi IP reali e riconfigurare il server in esecuzione: tienila fuori dalla rete pubblica, dietro una regola di firewall che consenta solo l'host DumbMonit, con una password impostata.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nessuna password | Solo per una management interface avviata senza l'argomento del file di password. |
| Password di gestione | La password condivisa impostata dal terzo argomento della direttiva "management" del server. |

Indirizzo: un nome host o IP, facoltativamente con `:porta`; la porta
predefinita è 7505, una convenzione della documentazione di OpenVPN stessa, non
uno standard imposto dal server. Opzioni: porta (7505) e timeout per scambio
(10 s).

Una connessione rifiutata o l'assenza di risposta conta come server
irraggiungibile. Una password errata viene mostrata come errore di credenziali;
nessuno dei due sveglia qualcuno di notte. La connessione è TCP semplice:
nulla di ciò che viene letto è segreto, ma la password, se impostata,
attraversa la rete in chiaro a ogni controllo — raggiungi l'interfaccia solo
da una rete attendibile o tramite un agent relay sulla stessa macchina del
server.
