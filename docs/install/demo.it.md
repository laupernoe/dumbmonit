# Eseguire un'istanza demo

La demo pubblica su **[demo.dumbmonit.app](https://demo.dumbmonit.app)** è
una normale immagine DumbMonit avviata con `DUMBMONIT_DEMO=1`. Puoi eseguire
la stessa cosa tu stesso — per mostrare DumbMonit a un team prima di
installarlo per davvero, o per mettere una vetrina sul tuo stesso sito.

!!! warning "Mai su un volume di dati reale"
    La modalità demo **ricrea il database a ogni avvio**. Dalle un volume
    tutto suo, oppure nessuno: puntata sul `/data` di un'istanza reale,
    sostituirebbe i tuoi monitor, regole e cronologia con il patrimonio
    fittizio.

## Cosa fa la modalità demo

- **Accedi con `demo` / `demo`.** La pagina di login lo indica e compila i
  campi con un clic. L'account vede ogni schermata che vede un
  amministratore.
- **Solo lettura.** Ogni modifica — monitor, regole, silenzi, conferme,
  canali, token, utenti, impostazioni, backup e ripristini, scansioni di
  scoperta, iscrizione di agent, strumenti di scrittura dell'endpoint
  assistente — viene rifiutata dal server con `403` e
  `{"error": "This is a read-only demo…", "demo": true}`. Accedere e uscire
  sono le uniche scritture consentite. L'interfaccia mostra il messaggio nel
  punto in cui è avvenuto il clic.
- **Due clienti fittizi.** All'avvio il database viene ricreato e popolato con
  un homelab sotto `*.home.arpa` — Proxmox VE, un NAS Synology con un LUN
  iSCSI, Proxmox Backup Server, TrueNAS, OPNsense, UniFi, Kopia, un server
  Redfish con un componente guasto, uno switch SNMP, controlli HTTP, TLS, DNS e
  ping, un agente Linux con Docker — e, in modalità MSP, con un piccolo studio
  di architettura a Ginevra: un controller di dominio Windows con il suo
  agente, Veeam Backup & Replication, una rete Omada, Microsoft 365 ed
  Entra ID, interrogati tramite un agente relay installato in ufficio. I
  monitor sono serviti da dispositivi simulati interni al server, legati a
  `127.0.0.1`, che riproducono risposte API registrate: i veri collettori e i
  veri pannelli funzionano. Un riavvio ridà lo stesso parco.
- **Trenta giorni di storico, e il resto del prodotto.** All'avvio vengono
  scritte in VictoriaMetrics metriche sintetiche, a blocchi limitati (l'ultima
  settimana con risoluzione di cinque minuti): grafici, *Ultimi 7 giorni*,
  disponibilità a 30 giorni, previsioni, la matrice dei backup e lo storico
  degli allarmi sono popolati, e alcuni allarmi sono attivi. Il seed riempie
  anche ciò che di solito si imposta a mano: team e cartelle, account e turni
  di reperibilità con politiche di escalation, silenzi, runbook, due
  dashboard, report pianificati, canali di notifica (mai usati), sedi sulla
  mappa del mondo, vulnerabilità note, quattro pagine di stato (una per scena
  notturna) con un incidente in corso e una scansione di rete con un nuovo
  dispositivo. Tutto è calcolato da un seed fisso: nulla viene scaricato dalla
  rete, nemmeno le fonti delle vulnerabilità. La ricerca nei log resta spenta:
  la demo non ha un archivio dei log.
- **Niente lascia il server.** Nessuna notifica viene mai inviata, e nessuna
  sonda raggiunge un indirizzo digitato da un visitatore.
- **Un tour guidato** si apre dopo il primo accesso; il banner lo riapre.

Il rate limiting dell'accesso funziona come al solito. Metti la demo dietro
il tuo solito reverse proxy con HTTPS e imposta `DUMBMONIT_COOKIE_SECURE=1` e
`DUMBMONIT_TRUSTED_PROXIES` come per qualsiasi istanza pubblica.

## Avviarla

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

Oppure con Compose, senza un volume così ogni riavvio parte da zero:

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/laupernoe/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

Scarica regolarmente l'immagine (o lascia che uno strumento come Watchtower
lo faccia): la demo mostrerà allora sempre l'ultima release con gli stessi
dati.
