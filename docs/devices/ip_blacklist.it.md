# Blacklist di un intervallo IP

Ogni indirizzo di un intervallo IPv4 pubblico, fino a un /24 (256 indirizzi),
controllato rispetto alle blocklist DNS di
[Reputazione del server di posta](mail_blacklist.md#the-blocklists):
Spamhaus ZEN, Barracuda, SpamCop, Mailspike, UCEPROTECT Level 1, PSBL e
SpamRATS. La pagina del monitor indica quanti indirizzi sono in lista, in
quali liste, e dà un link diretto alla pagina di consultazione e rimozione di
ogni inserimento.

Utile per l'intervallo pubblico di un ufficio, il blocco che un fornitore di
hosting ti ha dato, o gli indirizzi da cui inviano i tuoi server: la macchina
compromessa di un vicino ottiene un inserimento che un destinatario più
severo può estendere all'intero intervallo.

## Limiti

- **Al massimo un /24 per monitor.** Un intervallo più grande viene rifiutato
  con un messaggio che dice quanti indirizzi contiene: dividilo in più monitor
  /24. Ogni indirizzo costa una query a ogni lista, e una scansione di
  migliaia di indirizzi sembra un abuso.
- **Solo IPv4.** Di queste liste, solo Spamhaus ZEN risponde per IPv6, e un
  intervallo IPv6 non può essere percorso indirizzo per indirizzo. Un singolo
  indirizzo IPv6 può essere controllato con
  [Reputazione del server di posta](mail_blacklist.md).
- **Solo indirizzi pubblici.** Un intervallo che include un indirizzo privato,
  condiviso (CGNAT) o di documentazione viene rifiutato: le blocklist
  conoscono solo l'Internet pubblica.
- **Le liste di domini** (Spamhaus DBL, SURBL, URIBL) non si applicano a un
  intervallo.

## Resolver e ritmo

Si applicano le stesse regole di
[Reputazione del server di posta](mail_blacklist.md#resolvers-refusals-and-timeouts).
**Non serve alcuna chiave API.** Solo Spamhaus rifiuta i resolver pubblici: il suo
rifiuto (`127.255.255.254`) non viene mai mostrato come inserimento, la pagina del
monitor dice "Spamhaus refuses this resolver" e le altre liste mantengono i loro
risultati. Una lista che non risponde è sconosciuta, mai pulita. Anche per Spamhaus
usa un resolver ricorsivo tuo, oppure la chiave Spamhaus DQS gratuita e facoltativa
come credenziale.

Le liste sono quelle selezionate in **Blocklists** (per impostazione predefinita le
principali liste di indirizzi) più le tue zone in **Custom lists**; vedi
[Le tue liste](mail_blacklist.md#your-own-lists). Le liste di domini non si
applicano a un intervallo.

Un /24 richiede quasi duemila query. Vengono inviate a **Queries per second**
(10 per impostazione predefinita), otto al massimo in volo, e l'intero
intervallo viene controllato una volta per **Refresh interval** (6 ore per
impostazione predefinita); l'ultimo risultato viene mostrato di nuovo a ogni
controllo intermedio. Il primo controllo di un intervallo richiede qualche
minuto e gira in background.

## Controllare un intervallo IP pubblico rispetto alle blocklist

1. Inserisci un indirizzo IPv4 o un intervallo CIDR fino a /24, per esempio "198.51.100.0/24" (256 indirizzi). Un intervallo più grande viene rifiutato: dividilo in più monitor /24. Gli intervalli IPv6 vengono rifiutati, dato che solo Spamhaus risponde per IPv6: controlla un singolo indirizzo IPv6 con Reputazione del server di posta.

2. Non serve alcuna chiave API: lascia la credenziale su No key. Le blocklist vengono interrogate tramite il resolver DNS di questo server, o tramite quello che imposti in Resolver. Solo Spamhaus può rifiutare, e solo quando quel resolver è pubblico (Google, Cloudflare, Quad9): il monitor mostra allora "Spamhaus refuses this resolver" e le altre liste continuano a funzionare. Anche per Spamhaus usa un resolver ricorsivo tuo, oppure la chiave Spamhaus DQS gratuita e facoltativa.

3. Seleziona in Blocklists le blocklist di indirizzi da interrogare (quelle principali sono selezionate per impostazione predefinita) e aggiungi in Custom lists le zone DNSBL di qualsiasi altra lista. Un /24 richiede quasi duemila query con tutte le liste di indirizzi selezionate, inviate a 10 al secondo e ripetute ogni 6 ore per impostazione predefinita: mantieni questo ritmo, le blocklist bannano i resolver che le scandiscono.

4. Il primo controllo di un intervallo richiede qualche minuto: la pagina del monitor dice che è in corso, poi mostra quanti indirizzi sono in lista, in quali liste, e un link di rimozione per ciascuno.

!!! warning
    La Spamhaus PBL elenca gli intervalli di utenti finali che non dovrebbero inviare posta direttamente (connessioni domestiche, pool dinamici): un inserimento lì è una policy, non una segnalazione di abuso, e conta solo per un indirizzo che esegue un server di posta.

## Cosa sorveglia

Le stesse metriche di [Reputazione del server di posta](mail_blacklist.md#what-it-watches),
con il prefisso `dumbmonit_blacklist_`, senza quelle di DNS inverso, SPF e
DMARC. Per tenere basso il numero di serie, `check` viene scritta solo per un
indirizzo in lista o a cui una lista non ha risposto; `list_checked`,
`list_listed` e `list_unknown` contano ogni indirizzo per lista.

Le [regole integrate](../alerting/rules.md#dns-blocklists) che si applicano:
Presente in una blocklist principale e Presente in una blocklist.

La pagina del monitor ha un link **Also check on MXToolbox**: per un intervallo,
accanto a ogni indirizzo in lista, apre in una nuova scheda la ricerca pubblica
nelle blocklist di mxtoolbox.com. DumbMonit non chiama la loro API.

## Credenziali

Nessuna necessaria (il valore predefinito), oppure facoltativamente una chiave
Spamhaus DQS.

Indirizzo: un indirizzo IPv4 o un intervallo CIDR fino a /24. Opzioni: Resolver,
Blocklists (quelle principali selezionate su un nuovo monitor), Custom lists,
Refresh interval (6 ore), Queries per second (10) e Timeout per query (5 s).
