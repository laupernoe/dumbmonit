# Aggiungi il tuo primo dispositivo

Aggiungere qualsiasi cosa funziona sempre nello stesso modo: scegli un
tipo dall'elenco, una nota a destra spiega cosa preparare sul
dispositivo, e il modulo mostra solo i campi che contano per quel tipo.

## I tre primi passaggi

Su un'istanza completamente nuova la panoramica non mostra un bollettino
vuoto: mostra una breve guida — **add your first device**, **connect a
way to be told**, **check a message arrives**. Ogni passaggio è un clic,
e ognuno diventa verde da solo. Il terzo conta solo un messaggio che è
davvero uscito da un canale, quindi creare un canale e non testarlo mai
non lo spunta.

La guida viene mantenuta sull'istanza, non nel browser: saltare un
passaggio, o l'intera guida, lo salta in ogni browser e per ogni utente.
Scompare per sempre una volta risolti i tre passaggi, e non appare mai su
un'istanza che aveva già dispositivi e un canale — aggiornare non
consegna a un homelab consolidato un tutorial.

## Aggiungere un dispositivo SNMP

SNMP è il tipo più universale: quasi ogni apparecchiatura di rete lo
parla. Switch, router, NAS, UPS e stampanti sono i soliti sospetti.

1. Sul dispositivo, abilita SNMP v2c e annota la community di sola lettura (`public` per impostazione predefinita su molti dispositivi). Se il dispositivo filtra per indirizzo, consenti l'indirizzo del server DumbMonit.
2. In DumbMonit, apri **Devices** e clicca **Add a device** (oppure premi ++ctrl+k++ e digita "add").
3. Scegli **SNMP device**. Il selettore si riduce a una riga e il modulo mostra i campi per SNMP.
4. Inserisci un **name**, l'**address** (IP o nome host) e la **community**. Per SNMP v3, cambia la credenziale in *SNMP v3* e compila il nome utente e le passphrase di autenticazione e privacy.
5. Clicca **Add device**.

![Aggiungere un dispositivo SNMP: il modulo a sinistra, la nota di configurazione a destra](../assets/screenshots/add-device-light.png){ loading=lazy }

Il profilo di raccolta viene rilevato automaticamente dal `sysObjectID`
del dispositivo, in background, così il modulo non attende mai un
dispositivo irraggiungibile. Le interfacce, e su server e NAS CPU,
memoria e storage, appaiono entro un minuto.

!!! warning "Le community viaggiano in chiaro"
    Una community SNMP v1/v2c non è cifrata sulla rete. Su una rete
    condivisa, preferisci SNMP v3, che autentica e cifra.

## Scansionare una rete

Invece di digitare i dispositivi uno per uno, scansiona un intervallo:

1. Su **Add a device**, apri il pannello **Scan my network**.
2. Inserisci una rete in notazione CIDR, per esempio `192.168.1.0/24`
   (fino a 4096 indirizzi per scansione), e la community da provare su
   ogni indirizzo (`public` per impostazione predefinita).
3. Ogni indirizzo che risponde viene elencato con il suo `sysName`, la
   descrizione e il profilo che verrebbe applicato. I dispositivi già in
   DumbMonit sono contrassegnati *Already added*, e tutto ciò che è
   nuovo è spuntato per te.
4. Deseleziona ciò che non vuoi, poi clicca **Add *n* devices**. Vengono
   creati uno per uno, ogni riga mostra il proprio esito, così un
   fallimento non ferma mai il resto.

## Aggiungere un server Proxmox VE

Lo stesso flusso, con una credenziale in due campi invece di uno:

1. In Proxmox, crea un utente riservato al monitoraggio, un ruolo di sola lettura e un token API — i quattro comandi `pveum` sono nella nota accanto al modulo, ciascuno con un pulsante copia, e sulla [pagina Proxmox VE](../devices/proxmox.md). Non riutilizzare mai l'account con cui accedi.
2. In DumbMonit, **Add a device** → **Proxmox VE**.
3. Inserisci un **name** e l'**address** di qualsiasi nodo (porta 8006 per impostazione predefinita).
4. Copia il **full-tokenid** del token (`dumbmonit@pve!monitor`) in **Token ID** e il suo **value** (l'UUID) in **Secret**. Hai incollato per errore l'intera stringa `user@pve!name=secret` in Token ID? Il modulo la separa e lo dice.
5. Proxmox fornisce un certificato autofirmato: spunta **Accept an unverifiable certificate** sotto *More options* se la prima sonda si lamenta di esso.
6. Clicca **Add device**.

[Proxmox Backup Server](../devices/pbs.md) funziona nello stesso modo con
`proxmox-backup-manager`; un [NAS Synology](../devices/synology.md)
richiede un account DSM dedicato (nome utente e password); un
[server con l'agent](../devices/agent.md) non richiede alcuna
credenziale affatto, solo il token di iscrizione mostrato una volta
salvato il dispositivo.

## La nota di configurazione

Ogni tipo porta la propria nota, scritta dal server e mostrata accanto al
modulo: passaggi numerati da fare sul dispositivo (crea un utente di
sola lettura, crea un token API, consenti l'indirizzo…), con il percorso
di menu esatto nel prodotto e un pulsante copia su ogni comando o
valore, poi un errore comune e un link alla documentazione del fornitore
quando ce n'è uno. Le pagine per tipo in questa documentazione portano
gli stessi passaggi parola per parola — un test nel server li mantiene
identici: [SNMP](../devices/snmp.md), [Proxmox VE](../devices/proxmox.md),
[Proxmox Backup Server](../devices/pbs.md),
[Synology DSM](../devices/synology.md), [agent](../devices/agent.md),
[servizi](../devices/services.md).

Il principio dietro ogni nota: un account dedicato di sola lettura, mai
quello con cui accedi. Un segreto di monitoraggio compromesso non deve
poter modificare nulla.

## Campi comuni

Qualunque sia il tipo, il modulo ha:

| Campo | Significato |
|---|---|
| Name | Come il dispositivo appare negli elenchi e negli avvisi. |
| Address | IP, nome host, `host:porta` oppure URL a seconda del tipo; il segnaposto mostra la forma attesa. |
| Credential | Le famiglie che il tipo accetta, ciascuna con i propri campi: community, utente e passphrase SNMP v3, token Proxmox (Token ID + Secret), nome utente e password. I segreti hanno un occhio mostra/nascondi e i valori incollati vengono puliti da spazi, virgolette e interruzioni di riga indesiderati. Memorizzato cifrato, mai restituito dall'API: in modifica, i campi vuoti mantengono ciò che è salvato. |
| Check interval | Ogni quanto DumbMonit legge il dispositivo. 60 s per impostazione predefinita, minimo 10 s. |
| Parent device | Se il genitore si spegne, gli avvisi di questo dispositivo vengono soppressi invece che inviati. Vedi [soppressione per dipendenza](../alerting/index.md#dependency-suppression). |
| Enabled | Un dispositivo in pausa non viene controllato e non genera avvisi. |
| Tags | Etichette libere chiave/valore, copiate su ogni serie come `tag_<key>`. Le opzioni specifiche del tipo vivono anche qui, sotto **More options**. |

## Cosa succede dopo

- Il dispositivo appare sulla pagina **Devices** come una targhetta da
  rack: LED di stato, nome, tipo, indirizzo, ultima volta visto, e uno
  sparkline.
- Le misurazioni vengono scritte in VictoriaMetrics ogni 5 secondi
  (`DUMBMONIT_FLUSH_INTERVAL_SECS`), così il primo grafico appare dopo la
  prima sonda più qualche secondo. **Probe now** sulla pagina del
  dispositivo forza una sonda e riporta quanti campioni e serie ha
  prodotto: è il principale strumento diagnostico.
- Le [regole di avviso integrate](../alerting/rules.md) si applicano
  immediatamente: dispositivo irraggiungibile, CPU saturata, disco quasi
  pieno, e così via. Nulla da configurare.
- Per essere avvisato, aggiungi un [canale di notifica](../notifications.md)
  in **Alerts → Notifications**. Le regole integrate notificano ogni
  canale abilitato.

!!! tip "Nessun dato dopo un minuto?"
    Apri la pagina del dispositivo: un errore di configurazione (community
    errata, certificato rifiutato, funzionalità mancante) viene mostrato
    in rosso sotto l'indirizzo, con il motivo. Vedi la
    [FAQ](../faq.md#no-data-after-adding-a-device).
