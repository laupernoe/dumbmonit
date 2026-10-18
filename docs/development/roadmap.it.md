# Roadmap

Ciò che è rilasciato, in corso e pianificato è tracciato sulla bacheca pubblica
del progetto: [DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2).
Priorità e tempistiche possono cambiare; la bacheca è un piano vivo, non una
promessa.

- **Le modifiche rilasciate** sono nel
  [changelog](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md)
  e nelle note di ciascuna
  [release di GitHub](https://github.com/laupernoe/dumbmonit/releases).
- **Idee e bug** vivono nell'
  [issue tracker](https://github.com/laupernoe/dumbmonit/issues). Apri una issue
  prima di scrivere codice per qualcosa di grande.
- **Le correzioni di sicurezza** sono annunciate nelle note di rilascio e come
  advisory di sicurezza di GitHub, con il loro identificativo CVE o GHSA.

DumbMonit è precedente alla 1.0: aspettati che l'API e il modello dei dati
continuino a evolvere finché non sarà annunciata una linea stabile. Consulta la
[guida al contributo](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
e la
[governance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)
per partecipare.

## Direzione per i prossimi dodici mesi

Sono intenzioni, non impegni, in ordine approssimativo.

- **Verso la 1.0.** Stabilizzare l'API HTTP e lo schema del database in modo da
  poter annunciare una linea stabile; fino ad allora le versioni minori possono
  ancora modificarli, con note di aggiornamento nel changelog.
- **Più persone con accesso.** Un secondo amministratore del repository, del
  registry, dell'hosting della documentazione e dei domini (vedi la
  [governance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)).
- **Supply chain.** Build di release riproducibili
  ([stato attuale](reproducible-builds.md)) e tag di versione firmati.
- **Immagini arm64.** Le immagini del server sono per ora solo amd64; arm64
  tornerà con runner di build nativi.
- **Integrazioni e pacchetti.** Più integrazioni di dispositivi e servizi e
  profili SNMP, guidati da contributi e richieste.
- **Documentazione.** Mantenere le nove lingue dell'interfaccia e della
  documentazione allineate con la fonte inglese.

## Cosa DumbMonit non intende fare

- Diventare una piattaforma di log, trace o APM: monitora dispositivi e
  servizi.
- Offrire un'edizione a pagamento o un servizio ospitato: resta open source al
  100 %.
- Inviare telemetria sulla tua istanza da nessuna parte.
- Puntare a flotte di migliaia di dispositivi: è pensato per homelab e piccoli
  team, su un solo container.
