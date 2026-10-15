# Report via email

Un report è una breve email che racconta al tuo team come si è comportata
l'infrastruttura nell'ultimo giorno, settimana o mese, senza che nessuno apra
DumbMonit. È settimanale per impostazione predefinita; giornaliero e mensile
sono a un clic di distanza. I report riutilizzano un **canale di notifica
email (SMTP)**, quindi configuralo prima (vedi
[Canali di notifica](../notifications.md)).

Apri **Settings > Reports** per configurarli. Tutto qui è riservato agli
amministratori, lettura compresa, perché la pagina elenca gli indirizzi dei
destinatari.

## Cosa contiene un report

| Sezione | Contenuto |
| --- | --- |
| **Titolo** | Disponibilità complessiva per il periodo, numero di incidenti e variazione rispetto al periodo precedente. |
| **Disponibilità per gruppo** | Una riga per gruppo, poi per monitor (al massimo i 40 meno disponibili), calcolata nello stesso modo delle [pagine di stato](status-pages.md): le sonde in base al tasso di successo, host e monitor in base alla frequenza con cui hanno riportato. I monitor senza cronologia sono mostrati come "no data" invece che come 100%. |
| **Incidenti** | Quando ciascuno è iniziato, quanto è durato (o che è ancora aperto) e su quale monitor. Al massimo 25. Gli avvisi informativi sono esclusi. |
| **Monitor meno stabili** | I cinque monitor con la disponibilità più bassa o il maggior numero di incidenti. |
| **Confronto** | Disponibilità e numero di incidenti rispetto al periodo della stessa durata immediatamente precedente. |

L'email è composta da semplici tabelle HTML con stili inline e nessuna
immagine remota, così viene visualizzata allo stesso modo ovunque e nulla
viene recuperato quando viene aperta. Una versione testuale viene inviata
insieme. Ogni nome di monitor e messaggio è sottoposto a escaping.

## Impostazioni

| Impostazione | Cosa fa |
| --- | --- |
| **Name** | Mostrato solo nell'elenco. |
| **Enabled** | Disattivato mantiene le impostazioni senza inviare nulla. |
| **Frequency** | Giornaliera, settimanale (predefinita) o mensile. |
| **Day** | Il giorno della settimana per i report settimanali, il giorno del mese (da 1 a 28) per quelli mensili. |
| **Hour** e **Timezone** | Quando il report viene inviato, in un fuso orario IANA come `Europe/Paris`. L'ora legale viene seguita. |
| **Recipients** | Fino a 20 indirizzi, liberi: non devono essere necessariamente utenti DumbMonit. Ogni destinatario riceve la propria email, così nessuno vede gli altri. |
| **Email channel** | Quale canale SMTP lo invia. Per impostazione predefinita, il primo abilitato. Viene usato solo il server di posta del canale: i suoi destinatari propri vengono ignorati. |

Il periodo è una finestra scorrevole che termina quando il report viene
inviato: le ultime 24 ore, 7 giorni o 30 giorni. Il periodo precedente è la
finestra della stessa durata immediatamente precedente.

Clicca sulla **i** vicino al titolo della sezione per un richiamo del
contenuto.

## Invia un'anteprima

**Send a preview** invia subito via email il report corrente ai destinatari
salvati, con `[Preview]` nell'oggetto. Non cambia nulla nella pianificazione:
il prossimo report viene comunque inviato in orario. Un'anteprima può essere
inviata al massimo una volta ogni 30 secondi per report. Salva prima le tue
modifiche, poiché l'anteprima usa i destinatari salvati.

## Garanzie di consegna

- **Al massimo una volta.** Lo slot viene bloccato nel database prima che
  l'email venga costruita, così un riavvio non invia mai due volte lo stesso
  report. Quando l'invio fallisce, l'errore viene conservato sul report e
  mostrato nell'elenco, e lo slot non viene ritentato; il prossimo viene
  inviato normalmente.
- **Nessun recupero.** Se DumbMonit era inattivo nell'orario pianificato, il
  report viene inviato al ritorno online solo se ciò avviene entro 3 ore
  dallo slot. Più tardi di così, attende il prossimo slot. Un report che crei
  o riattivi non invia mai uno slot già passato.
- Possono esistere al massimo 10 report.
- In modalità demo non viene inviato nulla.

I report sono inclusi nei [backup](../install/backup.md): il canale email
viene associato per nome al ripristino, e un report ripristinato riparte da
zero (la sua data di ultimo invio non viene conservata).

## API

Vedi [API HTTP](../reference/api.md#email-reports).
