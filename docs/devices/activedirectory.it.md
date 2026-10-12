# Active Directory

Un dominio Windows via LDAP, di sola lettura: i controller di dominio e
chi detiene ogni ruolo FSMO, i membri dei gruppi privilegiati, la policy
delle password, i conteggi di account, e i fatti grezzi di sicurezza che
un audit cerca per primi — account esposti a Kerberoasting o AS-REP
roasting, delegazione non vincolata, una vecchia password krbtgt,
computer senza LAPS, release Windows fuori supporto.

DumbMonit si collega con un account di servizio dedicato che è un
**utente di dominio semplice, membro di nessun gruppo**. In Active
Directory ogni utente autenticato può leggere ciò che DumbMonit legge;
nessun diritto amministrativo è necessario, e nessuno dovrebbe essere
dato. DumbMonit non scrive mai nella directory, non legge mai un
attributo password (di LAPS legge la data di scadenza, mai la password)
e non chiede mai i diritti di replicazione che esporrebbero segreti.

Un dispositivo copre l'intero dominio: puntalo su un controller di
dominio e DumbMonit trova gli altri nella partizione di configurazione.

## Cosa LDAP può e non può dire

LDAP mostra la directory com'è, non i log di ciò che è accaduto in essa.
In particolare:

- **Replicazione.** Il controller di dominio interrogato pubblica lo
  stato dei suoi partner di replicazione in entrata (ultimo successo,
  fallimenti consecutivi, ultimo errore) solo a un account che possiede
  il diritto *Monitor active directory replication*. Senza di esso, il
  pannello dice *replication status not readable* invece di indovinare.
  Viene mostrata solo la vista del controller di dominio interrogato:
  per vedere i partner di un altro, aggiungilo come un secondo
  dispositivo.
- **Non visibile tramite LDAP**: log degli eventi (accessi, blocchi
  mentre avvengono, cambi di gruppo come eventi), contenuti delle Group
  Policy, SYSVOL, zone DNS, e gli hash delle password stessi.
- **Le date di ultimo accesso** provengono da `lastLogonTimestamp`, che
  i controller di dominio replicano solo ogni 9-14 giorni: un account
  contato come inattivo è inattivo entro due settimane.
- **L'appartenenza ai gruppi** viene letta con nidificazione (la regola
  `LDAP_MATCHING_RULE_IN_CHAIN`), tramite `member`/`memberOf`. Un gruppo
  primario impostato da `primaryGroupID` non è un link `member` e non
  viene visto.

## Come legge

Due ritmi:

- **A ogni controllo** (ogni minuto per impostazione predefinita):
  connessione e bind, la RootDSE (livelli funzionali, orologio,
  sincronizzazione), l'oggetto dominio (policy delle password, quota
  account macchina), i cinque ruoli FSMO, i controller di dominio dalla
  partizione di configurazione, i gruppi privilegiati e i loro membri
  effettivi, la funzionalità Recycle Bin, e una connessione TCP alla
  porta LDAP di ogni altro controller di dominio.
- **In background** ogni `inventory_minutes` (15 per impostazione
  predefinita): ogni account utente e computer, con solo gli attributi
  di cui i risultati hanno bisogno, in pagine di 500. Enumerare
  l'intera directory ogni minuto caricherebbe i controller di dominio
  per nulla.

Il primo controllo attende qualche secondo per l'inventario; su un
grande dominio i risultati sugli account appaiono al controllo
successivo.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_ad_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `bind_ok` | 1 quando l'account di servizio è accettato, 0 quando il controller di dominio lo rifiuta | |
| `connect_seconds`, `bind_seconds` | tempo per aprire la connessione (TLS incluso) e per il bind | |
| `tls_verified` | 1 quando il certificato è verificato | |
| `domain_info` | valore 1 | `domain`, `forest`, `dc` |
| `domain_functional_level`, `forest_functional_level` | da 0 (2000) a 7 (2016), 10 (2025) | |
| `dc_synchronized`, `dc_global_catalog_ready` | il controller di dominio interrogato ha terminato la sua replicazione iniziale, il suo global catalog è pronto | |
| `clock_skew_seconds` | orologio del controller di dominio meno orologio dell'host DumbMonit | |
| `recycle_bin_enabled` | 1 quando l'AD Recycle Bin è abilitato | |
| `dc_count` | controller di dominio del dominio | |
| `dc_info` | valore 1 | `dc`, `site`, `gc`, `rodc`, `os` |
| `dc_reachable` | 1 o 0 per controller di dominio; nessuna serie quando il suo nome non risolve da DumbMonit | `dc` |
| `fsmo_role` | valore 1 | `role` (`schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`), `holder` |
| `replication_readable` | 1 quando lo stato di replicazione in entrata può essere letto | |
| `replication_consecutive_failures`, `replication_last_success_age_seconds` | per partner in entrata e partizione | `source`, `naming_context` |
| `privileged_group_members`, `privileged_group_enabled_members` | membri effettivi, nidificazione inclusa, gruppi esclusi | `group` |
| `privileged_group_fingerprint` | un hash dei membri: cambia quando qualcuno si unisce o lascia | `group` |
| `users_total`, `users_enabled`, `users_disabled`, `users_locked_out`, `users_password_never_expires`, `users_stale` | account utente | |
| `computers_total`, `computers_enabled`, `computers_stale`, `groups_total` | computer e gruppi | |
| `admincount_accounts` | utenti protetti da AdminSDHolder (`adminCount=1`) | |
| `krbtgt_password_age_seconds` | età della password krbtgt | |
| `laps_schema`, `laps_computers`, `laps_eligible_computers` | LAPS nello schema; computer non-DC abilitati con una scadenza LAPS, su tutti quanti | |
| `password_min_length`, `password_history_length`, `password_max_age_seconds` (0 = mai), `password_complexity`, `lockout_threshold`, `machine_account_quota` | policy di dominio predefinita | |
| `finding_count` | oggetti interessati da un risultato, 0 quando esaminato e pulito | `finding`, `severity`, `title` |
| `inventory_age_seconds`, `scrape_errors` | età dell'ultimo inventario completo; parti della directory che non è stato possibile leggere | |

I gruppi privilegiati vengono trovati dal loro identificatore conosciuto,
non dal loro nome, così un dominio installato in un'altra lingua
funziona lo stesso: Domain Admins (RID 512), Enterprise Admins (519) e
Schema Admins (518) — gli ultimi due solo nel dominio radice della
foresta — e gli Administrators, Account Operators, Server Operators,
Print Operators e Backup Operators integrati.

## Risultati (Findings)

I risultati sono fatti grezzi — quanti oggetti, e quali — non un
punteggio; il [punteggio di sicurezza](../using/security-score.md) del
dispositivo li pesa. Ciascuno ha un `id` stabile, una severità sulla
scala del punteggio (low, medium, high, critical, mostrata sulla pagina
come Info, Advisory e Warning), una categoria, un conteggio e fino a
venti nomi di account di esempio. Un risultato che è stato esaminato e
non ha trovato nulla viene mantenuto con un conteggio di 0, così un
problema risolto diventa verde invece di scomparire; i risultati sugli
account appaiono una volta completato il primo inventario in
background. Vengono serviti così come sono da
`GET /api/targets/{id}/ad/findings` e pubblicati come
`dumbmonit_ad_finding_count{finding, severity, title}`.

| `id` | Severità | Cosa |
|---|---|---|
| `unconstrained_delegation` | critical | account abilitati fidati per delegazione non vincolata, controller di dominio esclusi |
| `kerberoastable_privileged` | critical | membri abilitati di gruppi privilegiati con un service principal name |
| `asrep_roastable_users` | high | account abilitati con la pre-autenticazione Kerberos disattivata (`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | high | utenti abilitati con un service principal name |
| `krbtgt_password_age` | high | password krbtgt più vecchia di 180 giorni |
| `laps_coverage` | high quando LAPS è assente, medium quando parziale | computer abilitati (controller di dominio esclusi) senza una password LAPS |
| `reversible_encryption` | high | utenti la cui password è memorizzata con cifratura reversibile |
| `guest_enabled` | high | l'account Guest integrato (RID 501) è abilitato |
| `obsolete_os` | high | computer abilitati su Windows XP, Vista, 7, 8, 2000, Server 2003, 2008 o 2012 |
| `dc_unreachable` | high | controller di dominio la cui porta LDAP non risponde |
| `replication_failing` | high | partner di replicazione in entrata con fallimenti consecutivi |
| `dc_not_synchronized` | high | il controller di dominio interrogato non ha terminato la sua replicazione iniziale |
| `fsmo_role_orphaned` | high | un ruolo FSMO detenuto da un controller di dominio eliminato |
| `password_policy` | high sotto 8 caratteri o senza lockout, medium altrimenti | lacune nella policy di dominio predefinita rispetto alla baseline: 14 caratteri, una soglia di lockout, complessità |
| `privileged_group_size` | medium | più di 5 account abilitati in Domain Admins, Enterprise Admins e Administrators insieme |
| `stale_accounts` | medium | utenti e computer abilitati inattivi oltre le soglie |
| `protocol_transition_delegation` | medium | delegazione vincolata con transizione di protocollo |
| `des_only` | medium | account limitati alle chiavi Kerberos DES |
| `password_not_required` | medium | utenti a cui è consentita una password vuota (`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | medium | utenti privilegiati la cui password non scade mai |
| `privileged_stale` | medium | utenti privilegiati inattivi oltre `stale_days_users` |
| `schema_admins_not_empty` | medium | Schema Admins ha membri |
| `functional_level_old` | medium | livello funzionale di dominio sotto 2012 R2 |
| `clock_skew` | medium | più di cinque minuti tra il controller di dominio e DumbMonit |
| `windows10_end_of_support` | medium | computer abilitati su Windows 10 |
| `admincount_orphans` | low | utenti `adminCount=1` non più in un gruppo privilegiato |
| `password_never_expires` | low | utenti abilitati la cui password non scade mai |
| `machine_account_quota` | low | qualsiasi utente può unire computer al dominio |
| `recycle_bin_disabled` | low | l'AD Recycle Bin non è abilitato |

Le [regole integrate](../alerting/rules.md#active-directory) che si
applicano:

- **Domain controller unreachable**: un altro controller di dominio del
  dominio smette di rispondere sulla sua porta LDAP per cinque minuti
  (Warning). Quello interrogato è coperto da *Device unreachable*.
- **Active Directory bind failed**: il controller di dominio rifiuta
  l'account di servizio — password scaduta, disattivato o bloccato
  (Advisory).
- **Privileged group membership changed**: qualcuno si è unito o ha
  lasciato un gruppo privilegiato, nidificazione inclusa (Advisory, si
  risolve un'ora dopo).
- **Active Directory replication failing**: un partner in entrata
  fallisce per mezz'ora (Advisory), quando lo stato di replicazione è
  leggibile.
- **krbtgt password older than 180 days** (Info).

## La pagina del dispositivo

Il pannello Active Directory mostra il dominio (livelli funzionali, il
controller di dominio interrogato, lo scarto dell'orologio), i
controller di dominio con il loro sito, ruoli e se rispondono, i gruppi
privilegiati con i loro membri, la policy delle password e i conteggi
degli account, e la tabella dei risultati con le parole di severità.

## Creare un account di servizio di sola lettura per DumbMonit

1. Crea un utente dedicato per DumbMonit, per esempio svc-dumbmonit, e non aggiungerlo a nessun gruppo. Un utente di dominio semplice è sufficiente: ogni utente autenticato può leggere ciò che DumbMonit legge. In PowerShell, con il modulo Active Directory:

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. Dagli una password lunga e casuale. Se le password scadono nel tuo dominio, DumbMonit genera Active Directory bind failed il giorno in cui questa scade.

3. Opzionale: per vedere lo stato di replicazione in entrata, concedi all'account il diritto Monitor active directory replication sull'oggetto dominio stesso. Non rivela alcuna password o segreto.

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. LDAPS richiede un certificato su ogni controller di dominio; un'autorità di certificazione enterprise (AD CS) ne emette uno automaticamente. Esporta il certificato di quell'autorità in formato X.509 codificato Base-64 e incollalo come CA certificate sotto.

5. In DumbMonit, inserisci il nome di un controller di dominio come il suo certificato lo nomina, per esempio "dc1.corp.example.com", e accedi come svc-dumbmonit@corp.example.com. DumbMonit trova gli altri controller di dominio da solo e verifica che ognuno risponda.

6. DumbMonit legge soltanto: non scrive mai nella directory, e di LAPS legge la data di scadenza, mai la password.

!!! warning
    LDAP semplice sulla porta 389 invia la password dell'account di servizio in chiaro: DumbMonit la rifiuta a meno che tu non spunti Allow plain LDAP, e un controller di dominio che richiede la firma LDAP la rifiuta anch'esso. Usa LDAPS, oppure StartTLS sulla porta 389.

Per impedire all'account di aprire una sessione ovunque, negagli *Log
on locally* e *Log on through Remote Desktop Services* tramite Group
Policy: un bind LDAP non ne è influenzato. Non limitarlo con *Log On
To* o *Deny access to this computer from the network*: i controller di
dominio allora rifiuterebbero il bind.

### Minimo privilegio, in dettaglio

- L'account non ha bisogno di alcun gruppo oltre Domain Users, che ogni
  account ha.
- Non usare un account di servizio gestito (gMSA): non può fare un
  semplice bind LDAP.
- Non concedere *Replicating Directory Changes* (o *All*): quello è il
  diritto DCSync, che esporrebbe ogni hash di password. DumbMonit non
  ne ha mai bisogno.
- *Monitor active directory replication* permette solo di leggere i
  metadati di replicazione (partner, ultimo successo, codici di
  errore). È opzionale.

## LDAPS e certificati

- **LDAPS (porta 636)** è il predefinito. Ogni controller di dominio di
  un dominio con un'autorità di certificazione enterprise riceve un
  certificato automaticamente; senza una, il controller di dominio non
  ascolta sulla 636. Il certificato deve portare il nome che DumbMonit
  usa: inserisci il controller di dominio con il suo nome DNS, non il
  suo indirizzo IP, a meno che il certificato non elenchi l'indirizzo.
- **CA certificate**: incolla il PEM dell'autorità root (o emittente).
  Solo quell'autorità viene allora fidata. Senza, DumbMonit fida le
  radici pubbliche e lo store di sistema del suo container, che non
  conoscono un'autorità enterprise privata.
- **StartTLS (porta 389)** cifra allo stesso modo, dopo un saluto in
  chiaro. Usalo quando la porta 636 è filtrata.
- **Accept an unverifiable certificate** mantiene la connessione cifrata
  ma accetta qualsiasi certificato: chiunque sul percorso potrebbe
  fingersi il controller di dominio e ricevere la password dell'account
  di servizio.
- **LDAP channel binding** (`LdapEnforceChannelBinding`) riguarda i
  bind NTLM e Kerberos su TLS; il bind semplice che DumbMonit usa non
  ne è interessato.

## Credenziali

| Campo | Valore |
|---|---|
| User name | `svc-dumbmonit@corp.example.com`, `CORP\svc-dumbmonit`, oppure il DN completo dell'account |
| Password | la password dell'account, memorizzata cifrata e mai mostrata di nuovo |

Una password vuota viene rifiutata: Active Directory l'accetterebbe come
un bind non autenticato senza controllare nulla.

## Opzioni

| Opzione | Predefinito | Cosa |
|---|---|---|
| Connection security (`security`) | `ldaps` | `ldaps`, `starttls` o `plain` |
| Port (`port`) | 636, oppure 389 | usata se l'indirizzo non ne fornisce una |
| CA certificate (`ca_cert`) | | PEM dell'autorità che ha emesso i certificati dei controller di dominio |
| Accept an unverifiable certificate (`insecure_tls`) | disattivato | |
| Allow plain LDAP (`allow_plaintext`) | disattivato | richiesto per `plain` |
| Check every domain controller (`check_all_dcs`) | attivo | connessione TCP alla porta LDAP di ogni controller di dominio |
| Inactive user after (`stale_days_users`) | 90 | giorni |
| Inactive computer after (`stale_days_computers`) | 90 | giorni |
| Full inventory every (`inventory_minutes`) | 15 | minuti, da 1 a 1440 |
| Timeout (`request_timeout_seconds`) | 8 | per operazione LDAP, da 1 a 60 |

## Risoluzione dei problemi

- **TLS certificate rejected**: incolla il certificato dell'autorità
  come CA certificate, e inserisci il controller di dominio con un
  nome che il suo certificato elenca.
- **Bind refused: wrong user name or password (LDAP result 49)**: il
  codice diagnostico dal controller di dominio viene tradotto —
  password scaduta, account disattivato, bloccato, oppure deve
  cambiare la password al prossimo accesso.
- **The domain controller requires a protected connection**: la firma
  LDAP è imposta; passa a LDAPS oppure StartTLS.
- **A domain controller shows as unresolved**: il suo nome non risolve
  dall'host DumbMonit. Punta il DNS di DumbMonit sui server DNS del
  dominio, oppure deseleziona *Check every domain controller*. I nomi
  non risolti non generano mai un avviso.
- **Replication status not readable**: concedi *Monitor active
  directory replication* come nel passaggio 3, oppure accetta che la
  replicazione non venga mostrata.
