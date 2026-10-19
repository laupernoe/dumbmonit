# Reputazione del server di posta

Un dominio di posta, o l'indirizzo pubblico di un server di posta, controllato
rispetto alle blocklist DNS che ancora decidono se la posta viene consegnata:
Spamhaus, Barracuda, SpamCop e alcune più piccole. Per ogni lista che include
un indirizzo, la pagina del monitor mostra il codice di risposta della lista,
il motivo pubblicato e un link diretto alla sua pagina di consultazione e
rimozione.

**Non serve alcuna chiave API.** Lascia la credenziale su "No key (recommended)":
le liste vengono interrogate tramite il resolver DNS del server, o tramite il
resolver che imposti in **Resolver**. Solo Spamhaus può rifiutare, e solo quando
quel resolver è pubblico; vedi [Resolver, rifiuti e timeout](#resolvers-refusals-and-timeouts).

Inserisci un dominio (`example.com`) e DumbMonit controlla gli indirizzi dei
suoi host MX (record A e AAAA), gli indirizzi del dominio stesso e qualsiasi
**Extra addresses** tu aggiunga (un relay in uscita che non è un MX). Il
dominio viene controllato anche rispetto alle blocklist di domini. Inserisci
un indirizzo IP e viene controllato solo quell'indirizzo. Per sorvegliare un
intero intervallo di indirizzi, usa
[Blacklist di un intervallo IP](ip_blacklist.md).

## Gmail, Outlook.com e Yahoo

I grandi fornitori di caselle di posta **non pubblicano alcuna blocklist
interrogabile**: ciò che pensano di un mittente è interno a loro. DumbMonit
non finge di interrogarli. Controlla ciò che pesa sulla loro decisione e
rimanda ai loro strumenti:

- **DNS inverso**: ogni indirizzo di invio dovrebbe avere un record PTR il cui
  nome risolve di nuovo verso lo stesso indirizzo (DNS inverso confermato in
  avanti, FCrDNS). Gmail, Outlook.com e Yahoo rifiutano o declassano la posta
  da indirizzi che non lo hanno.
- **SPF e DMARC**: esattamente un record SPF sul dominio e un record DMARC in
  `_dmarc.<dominio>`. Gmail e Yahoo richiedono entrambi ai mittenti in massa.
- **Le blocklist qui sotto**, che anche i loro filtri leggono.

DKIM non viene controllato: il suo selettore è scelto da ciascun mittente e
non può essere indovinato. La pagina del monitor rimanda a:

| Fornitore | Strumenti |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/) (reputazione di un dominio verificato), [linee guida per i mittenti](https://support.google.com/a/answer/81126), [modulo per problemi di consegna](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/) (dati di reputazione per indirizzo), [politiche per i mittenti](https://sendersupport.olc.protection.outlook.com/pm/), [modulo di supporto per i mittenti](https://olcsupport.office.com/) (rimozione da Outlook.com), [portale di rimozione Microsoft 365](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/), [buone pratiche](https://senders.yahooinc.com/best-practices/), [modulo di supporto per i mittenti](https://senders.yahooinc.com/contact/) |

## Le blocklist {#the-blocklists}

Verificate il 16 ottobre 2026. SORBS ha chiuso a giugno 2024 e non viene
interrogata. A ogni lista viene chiesto un record `A` sotto la sua zona. Il modulo
le mostra come una checklist chiamata **Blocklists** (nome, se controlla gli
indirizzi o il dominio, principale o no, e una breve descrizione); le liste
**principali** sono selezionate su un nuovo monitor. L'id nella prima colonna è ciò
che l'opzione `lists` memorizza, e il nome con cui l'API e le metriche chiamano la
lista.

| Id | Lista | Zona | Controlla | IPv6 | Principale | Uso e rimozione |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | indirizzi | sì | sì | Mirror pubblici gratuiti per uso a basso volume e non commerciale; rifiuta i resolver pubblici e aperti. Rimozione dallo [Spamhaus checker](https://check.spamhaus.org/). La PBL elenca per policy gli intervalli di utenti finali: rimuovi un indirizzo da essa solo se esegue davvero un server di posta. |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | indirizzi | no | sì | Gratuita; Barracuda chiede che i server DNS che la interrogano siano registrati con un account gratuito su barracudacentral.org. [Richiesta di rimozione](https://www.barracudacentral.org/rbl/removal-request). |
| `spamcop` | SpamCop | `bl.spamcop.net` | indirizzi | no | sì | Gratuita. Gli inserimenti scadono da soli circa un giorno dopo che le segnalazioni cessano; la [pagina di consultazione](https://www.spamcop.net/bl.shtml) mostra il perché e offre la rimozione. |
| `mailspike` | Mailspike | `bl.mailspike.net` | indirizzi | no | no | Gratuita a basso volume. [Consultazione e rimozione](https://mailspike.io/ip_verify). |
| `uceprotect_1` | UCEPROTECT Level 1 | `dnsbl-1.uceprotect.net` | indirizzi | no | no | Interrogazione gratuita. Gli inserimenti scadono dopo sette giorni senza nuovo spam; la [pagina di controllo](https://www.uceprotect.net/en/rblcheck.php) vende anche la rimozione express, che non è mai necessaria. Pochi destinatari la usano. |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | indirizzi | no | no | Gratuita. Rimozione self-service dalla [pagina dell'inserimento](https://psbl.org/). |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | indirizzi | no | no | Gratuita a basso volume. RATS-NoPtr elenca gli indirizzi senza DNS inverso. [Consultazione](https://www.spamrats.com/lookup.php) e [rimozione](https://www.spamrats.com/removal.php). |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | il dominio | — | sì | Stesse condizioni di ZEN. Rimozione dallo [Spamhaus checker](https://check.spamhaus.org/). |
| `surbl` | SURBL (multi) | `multi.surbl.org` | il dominio | — | no | Gratuita a basso volume; rifiuta i resolver pubblici e molto carichi. [Consultazione e rimozione](https://www.surbl.org/surbl-analysis). |
| `uribl` | URIBL (multi) | `multi.uribl.com` | il dominio | — | no | Gratuita a basso volume; rifiuta i resolver pubblici e molto carichi. [Consultazione](https://lookup.uribl.com/), [rimozione](https://admin.uribl.com/). |

Un inserimento in una lista **principale** fa scattare un avviso critico; uno
nelle altre liste un avviso di livello avviso. Gli indirizzi IPv6 degli MX
vengono controllati solo rispetto a Spamhaus ZEN, l'unica lista qui che
risponde per IPv6; le altre li mostrano come non coperti.

### Le tue liste {#your-own-lists}

In **Custom lists**, in fondo alla checklist, aggiungi le zone DNSBL di qualsiasi
altra lista che usi o gestisci tu (per esempio `dnsbl.example.org`). Ognuna ha un
**Type**: `ip` (la zona viene interrogata per ogni indirizzo, come
`1.2.0.192.dnsbl.example.org`) o `domain` (per il dominio di posta), e un
interruttore **Major** per una lista il cui inserimento deve far scattare l'avviso
critico. La zona deve essere un nome di dominio valido, non un indirizzo IP, e non
può ripetere un'altra zona né una delle liste qui sopra; al massimo 20 liste
personalizzate. Qualsiasi risposta `127.x.x.x` conta come in lista; `NXDOMAIN` come
pulita.

Il monitor interroga esattamente le liste selezionate più le tue. Un monitor creato
prima di queste opzioni non ha alcuna impostazione e continua a interrogare tutte
le liste qui sopra. L'opzione `lists` contiene gli id selezionati separati da
virgole (`-` quando non ne è selezionato nessuno) e `custom_lists` un array JSON
come `[{"zone": "dnsbl.example.org", "subject": "ip", "major": false}]`.

## Resolver, rifiuti e timeout {#resolvers-refusals-and-timeouts}

Le liste vengono interrogate tramite DNS, e il modo in cui rispondono dipende
da **quale resolver chiede**:

- **Spamhaus rifiuta le query che passano da resolver pubblici** (Google
  `8.8.8.8`, Cloudflare `1.1.1.1`, Quad9), da resolver che inoltrano a essi,
  o da resolver senza un DNS inverso attribuibile. Risponde allora
  `127.255.255.254` invece di `NXDOMAIN` (`127.255.255.255` quando un
  resolver invia troppe query, `127.255.255.252` per una query malformata o
  una chiave DQS errata). Sembrano inserimenti e non lo sono: DumbMonit segna la
  lista Spamhaus come **rifiutata** ("Spamhaus refuses this resolver" nella pagina
  del monitor), mai come "in lista", e le altre liste mantengono i loro risultati.
  Solo quando nessun'altra lista selezionata ha risposto, il monitor mostra un
  **errore di configurazione** (che non invia alcun avviso di interruzione) invece
  di un risultato.
- **URIBL e SURBL rispondono `127.0.0.1`** ai resolver che rifiutano. Quella
  lista viene allora mostrata come "Rifiutato" (sconosciuto), non come un
  inserimento.
- **Una lista che non risponde in tempo è sconosciuta, mai pulita.** Solo
  `NXDOMAIN` significa "non in lista". Anche una risposta fuori da
  `127.0.0.0/8` (un resolver che riscrive `NXDOMAIN` nell'indirizzo di una
  pagina di ricerca) è sconosciuta.

Quindi usa **un resolver ricorsivo tuo**: Unbound, Knot Resolver, BIND o
PowerDNS Recursor che risolvono dai root server (senza inoltrare a un
resolver pubblico), e imposta il suo indirizzo in **Resolver** se non è il
resolver di sistema. Pi-hole o AdGuard Home davanti a Unbound funziona;
davanti a `1.1.1.1` no.

Oppure, solo se vuoi anche Spamhaus e non puoi cambiare resolver, crea un account
gratuito [Spamhaus Data Query Service](https://www.spamhaus.com/data-access/free-data-query-service/)
(uso non commerciale, basso volume) e inserisci la sua chiave come credenziale
("Spamhaus DQS key (optional)"): Spamhaus viene allora interrogata sotto
`<chiave>.zen.dq.spamhaus.net` e `<chiave>.dbl.dq.spamhaus.net`, che rispondono
tramite qualsiasi resolver. La chiave è memorizzata cifrata e non viene più
mostrata.

## Ritmo

Le blocklist limitano la frequenza con cui un resolver può chiedere, e
bannano i resolver che le scandiscono. DumbMonit chiede a ogni lista una volta
per **Refresh interval** (6 ore per impostazione predefinita), al massimo
**Queries per second** (10 per impostazione predefinita) con otto query in
volo, e mostra di nuovo l'ultimo risultato a ogni controllo intermedio, così
le regole di avviso vedono una serie continua. Un controllo gira in
background: il primo risultato compare qualche secondo dopo l'aggiunta del
monitor, e cambiare l'indirizzo, un'opzione o la chiave avvia subito un nuovo
controllo. Un controllo il cui risultato è un errore viene ritentato dopo
dieci minuti.

## Controllare un server di posta rispetto alle blocklist

1. Inserisci il dominio per cui invii posta, per esempio "example.com": DumbMonit controlla gli indirizzi dei suoi MX e del dominio stesso. Oppure inserisci l'indirizzo IP pubblico da cui il server invia. Se la posta esce da un relay che non è un MX, aggiungi il suo indirizzo in Extra addresses.

2. Non serve alcuna chiave API: lascia la credenziale su No key. Senza chiave, le blocklist vengono interrogate tramite il resolver DNS di questo server, o tramite quello che imposti in Resolver.

3. Seleziona in Blocklists le blocklist da interrogare (quelle principali sono selezionate per impostazione predefinita) e aggiungi in Custom lists le zone DNSBL di qualsiasi altra lista che usi.

4. Solo Spamhaus può rifiutare una query, e solo quando il resolver è pubblico (Google, Cloudflare, Quad9) o inoltra a uno di essi. Il monitor mostra allora "Spamhaus refuses this resolver" e tutte le altre liste continuano a funzionare. Per avere anche Spamhaus, imposta Resolver su un resolver ricorsivo tuo (basta un Unbound che risolve dai root server) oppure scegli come credenziale la chiave Spamhaus DQS facoltativa: un account gratuito (uso non commerciale, basso volume) la cui chiave fa rispondere Spamhaus tramite qualsiasi resolver.

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

5. Per verificare la configurazione, aggiungi un monitor per 127.0.0.2, l'indirizzo di test che ogni blocklist elenca: ognuna dovrebbe mostrarlo come in lista.

6. Gmail, Outlook.com e Yahoo non pubblicano alcuna blocklist interrogabile. DumbMonit controlla invece ciò che essi valutano (un DNS inverso che punta all'indirizzo, record SPF e DMARC) e rimanda ai loro strumenti per i mittenti nella pagina del monitor.

!!! warning
    Una risposta rifiutata da Spamhaus (127.255.255.254, un resolver pubblico) non viene mai mostrata come inserimento: quella lista è segnata come rifiutata e le altre mantengono il loro risultato. Solo quando nessun'altra lista risponde, il monitor segnala un errore di configurazione. Una blocklist che non risponde viene mostrata come sconosciuta, mai come pulita. Le liste vengono interrogate ogni 6 ore per impostazione predefinita: chiedere più spesso non serve a nulla e fa bannare i resolver.

## Cosa sorveglia {#what-it-watches}

Tutte le metriche hanno il prefisso `dumbmonit_blacklist_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `listed_major_lists` | liste principali (Spamhaus ZEN e DBL, Barracuda, SpamCop) che includono un indirizzo o il dominio | |
| `listed_lists` | tutte le liste che includono un indirizzo o il dominio | |
| `addresses_checked` | indirizzi controllati | |
| `addresses_listed` | indirizzi presenti in almeno una lista | |
| `domain_listed` | 1 quando il dominio è in una blocklist di domini | |
| `unknown_checks` | controlli senza risposta (timeout, rifiuto) | |
| `check` | una serie per indirizzo (o dominio) e lista: 1 in lista, 0 pulito, −1 nessuna risposta, −2 rifiutato, −3 non coperto | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | per lista: soggetti interrogati, in lista, senza risposta | `list` |
| `list_info` | valore 1, una per ogni lista effettivamente interrogata (anche le tue): ciò che mostra la pagina del monitor | `list`, `name`, `zone`, `subject`, `major` |
| `address_info` | valore 1, da dove viene ogni indirizzo | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 1 quando il DNS inverso risolve di nuovo verso l'indirizzo | `address`, `ptr` |
| `mx_hosts` | host MX del dominio | |
| `spf_records` | record `v=spf1` sul dominio (1 è corretto) | |
| `dmarc_records` | record `v=DMARC1` in `_dmarc.<dominio>` | |
| `dmarc_policy_info` | valore 1 | `policy` |
| `mail_auth_missing` | da 0 a 2: SPF mancante o duplicato, DMARC mancante | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | il controllo mostrato: la sua età, quanto è durato, se ne sta girando uno nuovo | |

Le [regole integrate](../alerting/rules.md#dns-blocklists) che si applicano:
Presente in una blocklist principale, Presente in una blocklist, DNS inverso
del server di posta non coerente, SPF o DMARC mancante, più Monitor
irraggiungibile quando il resolver non risponde a nessuna query.

## La pagina del monitor

Il pannello legge ciò che la sonda ha memorizzato; aprire la pagina non
interroga mai una blocklist. Mostra i conteggi, poi le liste effettivamente
interrogate (quelle selezionate e le tue, ognuna con quanti soggetti include e
"Spamhaus refuses this resolver" quando è il caso), poi ogni risultato con il codice
della lista, il motivo pubblicato e i link alle sue pagine di consultazione e
rimozione, poi il DNS inverso di ogni indirizzo, SPF e DMARC, e gli strumenti per i
mittenti di Gmail, Outlook.com e Yahoo.

La pagina ha anche un link **Also check on MXToolbox**. Apre in una nuova scheda la
ricerca pubblica nelle blocklist di mxtoolbox.com per l'indirizzo o il dominio
monitorato; DumbMonit non chiama la loro API.

## Credenziali

Nessuna necessaria ("No key (recommended)", il valore predefinito), oppure
facoltativamente una chiave Spamhaus DQS (vedi sopra), che conta solo per Spamhaus.

Indirizzo: un dominio (`example.com`) o un indirizzo IP pubblico. Opzioni:
Resolver, Blocklists (quelle principali selezionate su un nuovo monitor; tutte su
uno creato prima), Custom lists, Extra addresses, Refresh interval (6 ore, da 1 a
168), Queries per second (10, da 1 a 50) e Timeout per query (5 s, da 1 a 30). Gli
indirizzi privati non vengono mai controllati: le blocklist conoscono solo
l'Internet pubblica. `127.0.0.2` è accettato come indirizzo di test.
