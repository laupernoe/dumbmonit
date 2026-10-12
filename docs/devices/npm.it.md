# Nginx Proxy Manager

Il gestore di proxy: host la cui configurazione nginx ha rifiutato, host
disattivati, e i certificati che gli host presentano, prima che un
fallimento del rinnovo Let's Encrypt si trasformi in un sito scaduto.

DumbMonit accede all'API REST di Nginx Proxy Manager (`/api`, sulla porta
della sua interfaccia web, 81) con un utente proprio, poi legge i proxy
host, i redirection host, i 404 host (chiamati dead host nell'API) e gli
stream. Ogni host indica se è attivato e se nginx ha accettato la sua
configurazione. Un host che nginx ha rifiutato non viene servito, e
l'interfaccia web lo mostra solo come un punto rosso.

I certificati non vengono letti dall'API. `/api/nginx/certificates`
restituisce la chiave privata dei certificati importati, e le credenziali
della sfida DNS, a qualsiasi utente autorizzato a vedere i certificati.
DumbMonit legge invece ogni certificato dove viene servito: un handshake TLS
sulla porta 443 di Nginx Proxy Manager, con il nome di un host che lo usa.
Questo misura anche il certificato che i visitatori ricevono effettivamente.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_npm_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | host di ciascun tipo | `type`: `proxy`, `redirection`, `dead` (404 host), `stream` |
| `hosts_offline` | host attivati la cui configurazione nginx ha rifiutato | `type` |
| `host_online` | per host attivato: 0 quando nginx ha rifiutato la sua configurazione | `type`, `host` |
| `host_error_info` | valore 1, per un host che nginx ha rifiutato: la prima riga dell'errore di nginx | `type`, `host`, `error` |
| `certificates_checked`, `certificates_read` | certificati associati a un host attivato, e quelli letti sulla porta HTTPS | |
| `cert_expiry_days` | giorni rimanenti prima che il certificato presentato da un host scada | `host` |

`host` è il primo nome di dominio dell'host, oppure `port 9000` per uno
stream. Gli host disattivati sono contati, non descritti: disattivare un
host è deliberato, e il suo avviso, se presente, si risolve. Gli host sono
descritti uno per uno fino a 500, e al massimo 100 certificati vengono letti
per controllo.

Le [regole integrate](../alerting/rules.md#reverse-proxies-and-domains) che
si applicano: Nginx Proxy Manager host offline, Proxy certificate not
renewed, più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Nginx Proxy Manager. Indica cosa non va in una
frase (host che nginx ha rifiutato, con il motivo, certificati vicini alla
scadenza, certificati che non è stato possibile leggere), poi i proxy host,
i redirect, i 404 host, gli stream e gli host disattivati, poi i certificati
più vicini alla scadenza per primi.

## Creare un utente Nginx Proxy Manager di sola visualizzazione

1. In Nginx Proxy Manager, apri Users e aggiungi un utente per DumbMonit, con un indirizzo email proprio come dumbmonit@example.com. Lascia ogni ruolo non spuntato.

2. Nel menu di quell'utente, apri Permissions: Item Visibility su All Items; Proxy Hosts, Redirection Hosts, 404 Hosts e Streams su View Only; Access Lists e SSL Certificates su Hidden. Poi imposta la sua password con Change Password nello stesso menu.

3. SSL Certificates resta su Hidden di proposito: Nginx Proxy Manager restituisce la chiave privata dei certificati importati, e le credenziali della sfida DNS, a qualsiasi utente autorizzato a vederli. DumbMonit legge invece ogni certificato dove viene servito, con un handshake TLS sulla porta 443 per ogni host che ne ha uno.

4. In DumbMonit, inserisci l'host di Nginx Proxy Manager con la porta della sua interfaccia web, per esempio "npm.lan:81", e l'email e la password di quell'utente. Se gli host sono serviti su HTTPS su una porta diversa da 443, imposta HTTPS port più sotto.

!!! warning
    Un host la cui configurazione nginx rifiuta (un errore di digitazione in Advanced, un file di certificato mancante) non viene servito, e Nginx Proxy Manager lo mostra solo come un punto rosso nell'elenco. I certificati Let's Encrypt vengono rinnovati trenta giorni in anticipo: uno che scade in meno di quattordici giorni è uno il cui rinnovo è fallito, e DumbMonit avvisa allora.

Item Visibility su All Items è importante: con Created Items, l'utente vede
solo gli host che ha creato lui stesso, che sono nessuno. Un utente senza
Proxy Hosts su View Only viene rifiutato; uno senza Streams, Redirection
Hosts o 404 Hosts semplicemente non li riporta.

DumbMonit conserva il token che Nginx Proxy Manager fornisce al login
(valido un giorno) e accede di nuovo quando scade, invece che a ogni
controllo.

## Credenziali

| Credenziale | Campi |
|---|---|
| User email and password | L'indirizzo email e la password dell'utente creato sopra. |

Indirizzo: un nome host o IP con la porta dell'interfaccia web
(`npm.lan:81`), oppure un URL completo. Opzioni: protocollo (HTTP per
impostazione predefinita), porta dell'interfaccia web (81), Check the
certificates (attivo), porta HTTPS (443), verifica del certificato
dell'API e timeout della richiesta (10 s).

Il controllo del certificato si collega alla parte host dell'indirizzo:
quando l'interfaccia web è raggiunta attraverso un altro proxy, punta
l'indirizzo su Nginx Proxy Manager stesso, oppure disattiva Check the
certificates e osserva ogni sito con un
[controllo del certificato TLS](services.md) invece. I certificati
wildcard vengono letti attraverso il primo host che li usa.
