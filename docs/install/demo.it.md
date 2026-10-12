# Eseguire un'istanza demo

La demo pubblica su **[demo.dumbmonit.app](https://demo.dumbmonit.app)** è
una normale immagine DumbMonit avviata con `DUMBMONIT_DEMO=1`. Puoi eseguire
la stessa cosa tu stesso — per mostrare DumbMonit a un team prima di
installarlo per davvero, o per mettere una vetrina sul tuo stesso sito.

!!! warning "Mai su un volume di dati reale"
    La modalità demo **ricrea il database a ogni avvio**. Dalle un volume
    tutto suo, oppure nessuno: puntata sul `/data` di un'istanza reale,
    sostituirebbe i tuoi dispositivi, regole e cronologia con il patrimonio
    fittizio.

## Cosa fa la modalità demo

- **Accedi con `demo` / `demo`.** La pagina di login lo indica e compila i
  campi con un clic. L'account vede ogni schermata che vede un
  amministratore.
- **Solo lettura.** Ogni modifica — dispositivi, regole, silenzi, conferme,
  canali, token, utenti, impostazioni, backup e ripristini, scansioni di
  scoperta, iscrizione di agent, strumenti di scrittura dell'endpoint
  assistente — viene rifiutata dal server con `403` e
  `{"error": "This is a read-only demo…", "demo": true}`. Accedere e uscire
  sono le uniche scritture consentite. L'interfaccia mostra il messaggio nel
  punto in cui è avvenuto il clic.
- **Un patrimonio fittizio e fisso.** All'avvio il database viene creato da
  zero e popolato con un piccolo homelab sotto `*.home.arpa`: un cluster
  Proxmox VE, un NAS Synology, un Proxmox Backup Server, un TrueNAS, un
  firewall OPNsense, un server Redfish con un componente guasto, alcuni
  controlli HTTP, TLS e ping, un host agent e una pagina di stato. I
  dispositivi vengono risposti da responder finti dentro il server, associati
  a `127.0.0.1` e che riproducono risposte API registrate, così i collector e
  i pannelli dei dispositivi reali funzionano. Riavviando si ottiene lo
  stesso patrimonio.
- **Sette giorni di cronologia.** Metriche sintetiche vengono scritte in
  VictoriaMetrics all'avvio, così i grafici, *Last 7 days*, le previsioni e la
  cronologia degli avvisi sono popolati, e alcuni avvisi sono attivi affinché
  *Needs you* contenga qualcosa.
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
