# Nginx Proxy Manager

Il proxy manager: host la cui configurazione nginx ha rifiutato, host disabilitati,
e i certificati che gli host presentano, prima che il fallimento del rinnovo di
Let's Encrypt si trasformi in un sito scaduto.

DumbMonit accede all'API REST di Nginx Proxy Manager (`/api`, sulla porta della
sua interfaccia web, 81) con un utente suo, quindi legge gli host proxy, gli host
di reindirizzamento, gli host 404 (chiamati host morti nell'API) e i flussi. Ogni
host dice se è abilitato e se nginx ha accettato la sua configurazione.
Un host che nginx ha rifiutato non viene servito, e l'interfaccia web lo mostra
solo come un punto rosso.

I certificati non vengono letti dall'API. `/api/nginx/certificates` passa la chiave
privata dei certificati importati e le credenziali della sfida DNS, a qualsiasi
utente autorizzato a visualizzare i certificati. DumbMonit legge ogni certificato
dove viene servito: un handshake TLS sulla porta 443 di Nginx Proxy Manager, con
il nome di un host che lo usa. Questo misura anche il certificato che i visitatori
effettivamente ricevono.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_npm_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | host di ogni tipo | `type`: `proxy`, `redirection`, `dead` (host 404), `stream` |
| `hosts_offline` | host abilitati la cui configurazione nginx ha rifiutato | `type` |
| `host_online` | per ogni host abilitato: 0 quando nginx ha rifiutato la sua configurazione | `type`, `proxy_host` |
| `host_error_info` | valore 1, per un host che nginx ha rifiutato: la prima riga dell'errore nginx | `type`, `proxy_host`, `error` |
| `certificates_checked`, `certificates_read` | certificati collegati a un host abilitato, e quelli letti sulla porta HTTPS | |
| `cert_expiry_days` | giorni rimasti prima che il certificato che un host presenta scada | `proxy_host` |

`proxy_host` è il primo nome di dominio dell'host, o `port 9000` per un flusso.
Gli host disabilitati vengono contati, non descritti: disabilitare un host è
deliberato, e il suo avviso, se presente, si risolve. Gli host sono descritti uno
per uno fino a 500, e al massimo 100 certificati vengono letti per controllo.

Le [regole incorporate](../alerting/rules.md#reverse-proxies-and-domains) che
si applicano: Nginx Proxy Manager host offline, Certificato proxy non rinnovato, più
Device irraggiungibile.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la pagina
non interroga mai Nginx Proxy Manager. Dice cosa c'è che non va in una frase (host
che nginx ha rifiutato, con il suo motivo, certificati prossimi alla scadenza,
certificati che non potevano essere letti), poi gli host proxy, i reindirizzamenti,
gli host 404, i flussi e gli host disabilitati, poi i certificati più prossimi alla
scadenza per primi.

## Crea un utente Nginx Proxy Manager di sola lettura

1. In Nginx Proxy Manager, apri Users e aggiungi un utente per DumbMonit, con un indirizzo email suo come dumbmonit@example.com. Lascia ogni ruolo non spuntato.

2. Nel menu dell'utente, apri Permissions: Item Visibility All Items; Proxy Hosts, Redirection Hosts, 404 Hosts e Streams View Only; Access Lists e SSL Certificates Hidden. Poi imposta la sua password con Change Password nello stesso menu.

3. SSL Certificates rimangono Hidden di proposito: Nginx Proxy Manager passa la chiave privata dei certificati importati e le credenziali della sfida DNS, a qualsiasi utente autorizzato a visualizzarli. DumbMonit legge ogni certificato dove viene servito, con un handshake TLS sulla porta 443 per ogni host che ne ha uno.

4. In DumbMonit, inserisci l'host Nginx Proxy Manager con la porta della sua interfaccia web, ad esempio "npm.lan:81", e l'email e la password dell'utente. Se gli host vengono serviti su HTTPS su una porta diversa da 443, imposta la porta HTTPS qui sotto.

!!! warning
    Un host la cui configurazione nginx rifiuta (un errore di battitura in Advanced, un file certificato mancante) non viene servito, e Nginx Proxy Manager lo mostra solo come un punto rosso nell'elenco. I certificati Let's Encrypt vengono rinnovati trenta giorni prima: uno che scade in meno di quattordici giorni è uno il cui rinnovo ha fallito, e DumbMonit avvisa allora.

Item Visibility All Items conta: con Created Items, l'utente vede solo gli host che ha creato, che non sono nessuno. Un utente senza Proxy Hosts View Only viene rifiutato; uno senza Streams, Redirection Hosts o 404 Hosts semplicemente non li segnala.

DumbMonit mantiene il token che Nginx Proxy Manager dà al login (valido un giorno) e
accede di nuovo quando scade, piuttosto che ad ogni controllo.

## Credenziali

| Credenziale | Campi |
|---|---|
| Email e password dell'utente | L'indirizzo email e la password dell'utente creato sopra. |

Indirizzo: un nome host o IP con la porta dell'interfaccia web (`npm.lan:81`),
o un URL completo. Opzioni: protocollo (HTTP per impostazione predefinita), porta dell'interfaccia
web (81), Controlla i certificati (acceso), porta HTTPS (443), verifica del certificato
dell'API e timeout della richiesta (10 s).

La verifica del certificato si connette alla parte host dell'indirizzo: quando
l'interfaccia web viene raggiunta attraverso un altro proxy, punta l'indirizzo a Nginx Proxy
Manager stesso, o deseleziona Controlla i certificati e osserva ogni sito con un
[controllo del certificato TLS](services.md) invece. I certificati wildcard vengono letti
attraverso il primo host che li usa.
