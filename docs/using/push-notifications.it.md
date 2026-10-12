# Notifiche push (PWA)

DumbMonit può far suonare il tuo telefono o computer da solo, come fa
un'app nativa, senza installare un'app o creare un account da nessuna
parte: il servizio push proprio del browser (Google, Mozilla, Apple,
Microsoft) porta un messaggio cifrato end-to-end dal tuo server al
dispositivo, e la notifica appare anche con ogni scheda DumbMonit chiusa.
Questo è il **Web Push** standard; il sito può anche essere installato
come un'app (una *progressive web app*, PWA) così si apre da un'icona
come qualsiasi altra.

Due passaggi: abilita il push su ogni dispositivo (questa pagina), poi
decidi quali avvisi arrivano lì con un canale **Web Push**
([Canali di notifica](../notifications.md#web-push)).

## Prima di iniziare

- **HTTPS è richiesto.** I browser offrono il push solo a un'origine
  sicura. Metti DumbMonit dietro un reverse proxy con un certificato
  (Caddy, Traefik, Nginx Proxy Manager…) e aprilo tramite quell'indirizzo.
  `http://localhost` viene accettato anch'esso, utile solo per testare
  sul server stesso. Su un semplice `http://192.168.x.x`, la sezione
  mostra **Needs HTTPS**.
- **Il server ha bisogno di HTTPS in uscita** verso i servizi push:
  `fcm.googleapis.com` (Chrome, ed Edge su Android), `*.notify.windows.com`
  (Edge su Windows), `updates.push.services.mozilla.com` (Firefox) e
  `*.push.apple.com` (Safari, iPhone, iPad). Nulla entra: il tuo DumbMonit
  non deve essere raggiungibile da internet, il telefono deve solo
  raggiungerlo quando tocchi una notifica.
- **Imposta `DUMBMONIT_PUBLIC_URL`** sull'indirizzo HTTPS che usi: viene
  dato ai servizi push come contatto del mittente, ed è l'indirizzo verso
  cui puntano i link negli altri canali.

## Abilitare su un telefono o computer

### Android, Windows, macOS, Linux

1. Apri DumbMonit in Chrome, Edge, Firefox o Safari e accedi.
2. Opzionale: installalo come un'app (barra degli indirizzi → **Install**,
   oppure il menu del browser → **Add to Home screen**). Le notifiche
   funzionano in entrambi i modi.
3. **Settings → Push notifications → Enable on this device**, e consenti
   le notifiche quando il browser le chiede.
4. **Send a test.** Un *DumbMonit — test notification* dovrebbe apparire
   entro pochi secondi.

### iPhone e iPad

Apple offre il push solo ai siti aggiunti alla schermata Home (iOS e
iPadOS 16.4 o successivi):

1. Apri DumbMonit in **Safari**.
2. **Share → Add to Home Screen**, poi apri DumbMonit **dalla nuova
   icona**.
3. Accedi, poi **Settings → Push notifications → Enable on this device**,
   e consenti le notifiche.
4. **Send a test.**

In Safari senza installazione, la sezione mostra **Install first**.

## Scegliere quali avvisi arrivano

In **Alerts → Notifications**, aggiungi un canale di tipo **Web Push**.
Senza alcuna impostazione, ogni dispositivo iscritto di ogni account lo
riceve; elenca i nomi degli account sotto **Accounts** per mirare solo
alcune persone. Si applica la solita politica di canale (severità minima,
ore silenziose, filtro di instradamento…), così puoi per esempio tenere
un telefono solo per gli avvisi critici.

Una notifica legge `Critical · nas01` con un breve riassunto; toccarla
apre la pagina **Alerts**. La risoluzione di un avviso sostituisce la sua
notifica invece di aggiungerne una seconda.

## Dispositivi e account

Ogni account vede e gestisce i propri dispositivi: anche i visualizzatori
possono abilitare il push sul loro telefono, non cambia nulla altro.
L'elenco mostra il browser e il sistema, il servizio push, quando il
dispositivo è stato aggiunto e quando ha ricevuto l'ultimo messaggio;
**Remove** lo scorda (e, sul dispositivo che stai usando, disiscrive
anche il browser). Un dispositivo che il servizio push segnala come
scomparso (l'app è stata rimossa, i dati del browser svuotati) viene
eliminato automaticamente al prossimo invio.

## Risoluzione dei problemi

| Cosa vedi | Cosa fare |
|---|---|
| **Needs HTTPS** | Apri DumbMonit tramite il suo indirizzo HTTPS. |
| **Install first** (iPhone, iPad) | Share → Add to Home Screen, apri dall'icona. |
| **Blocked** | Le notifiche sono state rifiutate per questo sito: consentile nelle impostazioni del sito del browser (lucchetto nella barra degli indirizzi, oppure iOS Settings → Notifications → DumbMonit), poi ricarica. |
| **Not supported** | Il browser non ha Web Push (versione vecchia, alcuni browser orientati alla privacy): usa un Chrome, Edge, Firefox o Safari recente. |
| Il test dice *push service unreachable* | Il server non può raggiungere il servizio push: verifica il suo HTTPS in uscita e il DNS. |
| Il test dice *refused the VAPID signature* | Di solito un orologio del server errato; verifica NTP sull'host. |
| Nulla arriva su Android quando il telefono dorme | L'ottimizzazione della batteria può ritardare il browser: esentalo, oppure imposta l'**Urgency** del canale su `high`. |

## Privacy e sicurezza

- I messaggi sono cifrati per ogni dispositivo (RFC 8291) prima di
  lasciare il server: il servizio push vede solo un blob opaco, il
  dispositivo lo decifra.
- Il server firma ogni messaggio con la propria coppia di chiavi VAPID
  (RFC 8292), generata al primo avvio e memorizzata cifrata con il
  segreto dell'istanza; la chiave privata non viene mai restituita
  dall'API. Un [ripristino](../install/backup.md) su una nuova macchina
  genera una nuova coppia di chiavi: abilita di nuovo il push su ogni
  dispositivo.
- L'indirizzo di iscrizione proviene dal browser, quindi il server accetta
  solo indirizzi HTTPS su IP pubblici: gli indirizzi privati, loopback e
  link-local vengono rifiutati, sia al momento dell'iscrizione che a ogni
  connessione successiva.
- Il service worker (`/sw.js`) mostra solo le notifiche. Non memorizza
  nulla in cache e non intercetta alcuna richiesta: l'interfaccia e l'API
  provengono sempre dal server, mai da una copia obsoleta.
