# Aggiornare DumbMonit

DumbMonit ti avvisa quando esiste una release più recente e ti guida
nell'aggiornamento. Non installa mai nulla da solo.

## Individuare una nuova versione

Al massimo una volta al giorno, il server interroga l'API pubblica delle
release di GitHub (`api.github.com/repos/laupernoe/dumbmonit/releases`) per
l'ultima release e la confronta con la versione in uso. Le pre-release come
`0.1.0-alpha.7` vengono confrontate correttamente: `alpha.10` è più recente di
`alpha.9`, e `0.1.0` è più recente di qualsiasi `0.1.0-alpha.N`. Le bozze
(draft) sono ignorate, e a un'istanza su una versione stabile non viene mai
proposta una pre-release.

Quando esiste una versione più recente, **Update available: vX** appare in
basso a destra su ogni pagina e in **Settings → About**, dove **How to
update** apre i passaggi seguenti e **See what's new** riapre la finestra
*What's new*. **Check now** verifica di nuovo immediatamente (al massimo una
volta al minuto).

!!! note "Cosa viene inviato"
    Una sola richiesta `GET` anonima con uno user agent `DumbMonit/<version>`.
    Nulla riguardo la tua istanza, i tuoi dispositivi o la tua rete viene
    inviato, e l'indirizzo è fisso nel codice.

### Disattivare il controllo

Per reti air-gapped, disattiva **Check for new versions** in **Settings →
About** (amministratori), oppure imposta la variabile d'ambiente, che prevale
sull'interruttore:

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

La stessa informazione è disponibile per script e token API su
`GET /api/update`.

## Aggiornamento

1. **Fai un backup.** Scarica un backup da **Settings → Backup**, e conserva
   una copia di `/data/secret.key` (vedi [Backup e ripristino](backup.md)).
   Senza questa chiave, le password e i token memorizzati non possono essere
   decifrati.
2. **Scarica e riavvia**, dalla cartella che contiene il tuo
   `docker-compose.yml`:

    ```bash
    docker compose pull && docker compose up -d
    ```

    I tuoi dati vivono nel volume `/data` e vengono conservati; le
    migrazioni del database vengono eseguite all'avvio.
3. **Ricarica la pagina.** **Settings → About** mostra la nuova versione e
   *You are up to date*.

Per fissare una versione invece di seguire `:latest`, cambia il tag
dell'immagine nel file Compose (per esempio
`ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`) ed esegui lo stesso comando.

Per tornare indietro, ripristina il backup fatto al passaggio 1 sull'immagine
precedente.
