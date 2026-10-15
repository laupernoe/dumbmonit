# Notifiche push (PWA)

DumbMonit può far suonare da solo il tuo telefono o computer, come farebbe
un'app nativa, senza installare un'app né creare un account da nessuna parte: il
servizio push del browser (Google, Mozilla, Apple, Microsoft) trasporta un
messaggio cifrato end-to-end dal tuo server al dispositivo, e la notifica appare
anche con tutte le schede di DumbMonit chiuse. È lo standard **Web Push**; il
sito può anche essere installato come app (una *progressive web app*, PWA) in
modo da aprirsi da un'icona come qualsiasi altra.

Due passaggi: abilitare il push su ogni dispositivo (questa pagina), poi
decidere quali avvisi vi arrivano con un canale **Web Push**
([Canali di notifica](../notifications.md#web-push)).

## Prima di iniziare

- **HTTPS è obbligatorio.** I browser offrono il push solo a un'origine sicura.
  Metti DumbMonit dietro un reverse proxy con un certificato (Caddy, Traefik,
  Nginx Proxy Manager…) e aprilo tramite quell'indirizzo. Anche
  `http://localhost` è accettato, il che serve solo per i test sul server
  stesso. Su `http://192.168.x.x` semplice, la sezione indica **Richiede HTTPS**.
- **Il server ha bisogno di HTTPS in uscita** verso i servizi push:
  `fcm.googleapis.com` (Chrome, ed Edge su Android), `*.notify.windows.com`
  (Edge su Windows), `updates.push.services.mozilla.com` (Firefox) e
  `*.push.apple.com` (Safari, iPhone, iPad). Nulla entra: il tuo DumbMonit non
  deve essere raggiungibile da internet, il telefono deve solo raggiungerlo
  quando tocchi una notifica.
- **Imposta `DUMBMONIT_PUBLIC_URL`** sull'indirizzo HTTPS che usi: viene
  comunicato ai servizi push come contatto del mittente, ed è l'indirizzo a cui
  puntano i link negli altri canali.

## Abilitare su un telefono o computer

### Android, Windows, macOS, Linux

1. Apri DumbMonit in Chrome, Edge, Firefox o Safari e accedi.
2. Facoltativo: installalo come app (barra degli indirizzi → **Installa**, oppure
   menu del browser → **Aggiungi alla schermata Home**). Le notifiche
   funzionano in entrambi i casi.
3. **Impostazioni → Notifiche push → Abilita su questo dispositivo**, e consenti
   le notifiche quando il browser lo chiede.
4. **Invia un test.** Una *DumbMonit — notifica di test* dovrebbe comparire
   entro pochi secondi.

### iPhone e iPad

Apple fornisce il push solo ai siti aggiunti alla schermata Home (iOS e iPadOS
16.4 o successivi):

1. Apri DumbMonit in **Safari**.
2. **Condividi → Aggiungi alla schermata Home**, poi apri DumbMonit **dalla
   nuova icona**.
3. Accedi, poi **Impostazioni → Notifiche push → Abilita su questo
   dispositivo**, e consenti le notifiche.
4. **Invia un test.**

In Safari senza installazione, la sezione indica **Installa prima**.

## Scegliere quali avvisi arrivano

In **Impostazioni → Notifiche e report**, aggiungi un canale di tipo **Web Push**. Senza
impostazioni, lo ricevono tutti i dispositivi iscritti di ogni account; elenca i
nomi degli account in **Account** per rivolgerti solo ad alcune persone. Si
applica la consueta politica dei canali (gravità minima, ore di silenzio, filtro
di instradamento…), quindi puoi per esempio riservare un telefono ai soli avvisi
critici.

Una notifica ha la forma `Critico · nas01` con un breve riepilogo; toccandola si
apre la pagina **Avvisi**. La risoluzione di un avviso sostituisce la sua
notifica invece di aggiungerne una seconda.

## Dispositivi e account

Ogni account vede e gestisce i propri dispositivi: anche gli osservatori possono
abilitare il push sul loro telefono, il resto non cambia. Un account può avere
al massimo 10 dispositivi (rimuovine uno vecchio per aggiungerne un altro), e un
test può essere inviato una volta ogni 30 secondi. L'elenco mostra il browser e
il sistema, il servizio push, quando il dispositivo è stato aggiunto e quando ha
ricevuto l'ultimo messaggio; **Rimuovi** lo dimentica (e, sul dispositivo che
stai usando, annulla anche l'iscrizione del browser). Un dispositivo che il
servizio push segnala come scomparso (l'app è stata rimossa, i dati del browser
cancellati) viene eliminato automaticamente al successivo invio.

## Risoluzione dei problemi

| Cosa vedi | Cosa fare |
|---|---|
| **Richiede HTTPS** | Apri DumbMonit tramite il suo indirizzo HTTPS. |
| **Installa prima** (iPhone, iPad) | Condividi → Aggiungi alla schermata Home, apri dall'icona. |
| **Bloccato** | Le notifiche sono state rifiutate per questo sito: consentile nelle impostazioni del sito del browser (lucchetto nella barra degli indirizzi, oppure Impostazioni iOS → Notifiche → DumbMonit), poi ricarica. |
| **Non supportato** | Il browser non ha Web Push (versione vecchia, alcuni browser orientati alla privacy): usa un Chrome, Edge, Firefox o Safari recente. |
| Il test dice *servizio push non raggiungibile* | Il server non riesce a raggiungere il servizio push: controlla il suo HTTPS in uscita e il DNS. |
| Il test dice *firma VAPID rifiutata* | Di solito l'orologio del server è sbagliato; controlla NTP sull'host. |
| Su Android non arriva nulla quando il telefono è in standby | L'ottimizzazione della batteria può ritardare il browser: escludilo, oppure imposta l'**Urgenza** del canale su `high`. |

## Privacy e sicurezza

- I messaggi sono cifrati per ogni dispositivo (RFC 8291) prima di lasciare il
  server: il servizio push vede solo un blob opaco, il dispositivo lo decifra.
- Il server firma ogni messaggio con la propria coppia di chiavi VAPID
  (RFC 8292), generata al primo avvio e memorizzata cifrata con il segreto
  dell'istanza; la chiave privata non viene mai restituita dall'API. Un
  [ripristino](../install/backup.md) su una nuova macchina genera una nuova
  coppia di chiavi: abilita di nuovo il push su ogni dispositivo.
- L'indirizzo di iscrizione proviene dal browser, quindi il server accetta solo
  indirizzi HTTPS su IP pubblici: gli indirizzi privati, loopback e link-local
  sono rifiutati, al momento dell'iscrizione e di nuovo a ogni connessione.
- Il service worker (`/sw.js`) mostra soltanto le notifiche. Non mette nulla in
  cache e non intercetta alcuna richiesta: l'interfaccia e l'API provengono
  sempre dal server, mai da una copia obsoleta.
