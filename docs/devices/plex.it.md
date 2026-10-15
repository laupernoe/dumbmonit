# Plex Media Server

Cosa sta facendo Plex e se è aggiornato: stream in riproduzione ora, quanti
sono transcodificati e se in hardware, la banda che riservano, stream
provenienti dall'esterno della casa, librerie in fase di scansione, e
aggiornamenti.

DumbMonit legge l'API propria del server sulla porta 32400, mai plex.tv. Non
interrompe mai uno stream, non scansiona mai una libreria e non modifica mai
un'impostazione.

Plex non pubblica l'esito dei propri task di manutenzione (il "butler" ne dà
solo la pianificazione) né lo stato dei backup del proprio database, quindi
non c'è nulla da dire su quelli.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_plex_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `update_available` | 1 quando il controllo aggiornamenti proprio di Plex ha trovato una release più recente; assente finché Plex non ha controllato almeno una volta | `latest_version` |
| `claimed` | 1 quando il server è collegato a un account Plex | |
| `streams`, `streams_paused`, `streams_remote` | riproduzioni in corso; in pausa; dall'esterno della rete locale | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | riproduzioni ri-codificate (video o audio); con il video ri-codificato; con accelerazione hardware | |
| `transcoder_active_video_sessions` | transcodifiche video che il server stesso conta | |
| `stream_bandwidth_bits_per_second` | banda riservata da tutte le riproduzioni | |
| `libraries`, `libraries_scanning` | librerie; quelle in fase di scansione ora | |
| `activities` | attività in background in esecuzione (scansioni, analisi, aggiornamenti) | |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

La [regola integrata](../alerting/rules.md#self-hosted-applications) che si
applica: Plex update available, più Monitor unreachable.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non chiama mai Plex. Indica lo stato dell'aggiornamento e se il server
è collegato a un account, poi stream, transcodifiche, transcodifiche
hardware, stream remoti, banda e librerie.

## Dare a DumbMonit l'accesso al server Plex

1. DumbMonit legge l'API propria del server sulla porta 32400, mai plex.tv. Un server collegato a un account Plex richiede un token (X-Plex-Token), e Plex non può emetterne uno limitato: è il token del proprietario.

2. Oppure evita il token: in Plex, Settings → Network → List of IP addresses and networks that are allowed without auth, aggiungi l'indirizzo del server DumbMonit, per esempio come segue, e scegli No token più sotto.

    ```
    10.0.0.20/32
    ```

3. Oppure usa il token: in Plex Web, apri qualsiasi film → ⋯ → Get Info → View XML. L'indirizzo della pagina che si apre termina con X-Plex-Token= seguito dal token; copia quel valore.

4. In DumbMonit, inserisci l'indirizzo del server, per esempio "plex.lan" (porta 32400). DumbMonit legge solo le sessioni, le librerie, l'updater e l'elenco delle attività; non interrompe mai uno stream e non modifica mai un'impostazione.

!!! warning
    Il token è quello del tuo account Plex: chi lo possiede può gestire il server e ogni libreria. Preferisci la rete consentita sopra quando DumbMonit si trova sulla tua stessa rete.

La rete consentita è anche ciò che fa rispondere Plex senza token da Docker:
il container di DumbMonit raggiunge Plex dal suo indirizzo di rete Docker,
quindi elenca quella rete, oppure l'indirizzo dell'host quando Plex pubblica
la sua porta sull'host.

## Credenziali

| Credenziale | Campi |
|---|---|
| X-Plex-Token | Inviato nell'header `X-Plex-Token`. |
| No token (allowed network) | Niente viene inviato; Plex deve elencare l'indirizzo di DumbMonit tra le reti consentite senza autenticazione. |

Indirizzo: un nome host (`plex.lan`), `host:porta`, oppure un URL completo;
un `/web` finale viene accettato. Opzioni: protocollo (HTTP per impostazione
predefinita; i certificati propri di Plex sono emessi per nomi
`*.plex.direct`, quindi HTTPS per indirizzo IP richiede la verifica del
certificato disattivata), porta (32400), verifica del certificato e timeout
della richiesta (15 s).

Validato su Plex Media Server 1.43.4 (immagine ufficiale), un server non
collegato che risponde senza token, con una riproduzione transcodificata
reale. Il percorso con token segue l'header `X-Plex-Token` documentato da
Plex e non è stato testato su un server collegato.
