# Active Directory

Eine Windows-Domäne über LDAP, nur-lesend: die Domänencontroller und
wer jede FSMO-Rolle hält, die Mitglieder der privilegierten Gruppen,
die Passwortrichtlinie, Kontenzahlen, und die rohen
Sicherheitsfakten, nach denen ein Audit zuerst sucht — für
Kerberoasting oder AS-REP-Roasting offene Konten, uneingeschränkte
Delegation, ein altes krbtgt-Passwort, Computer ohne LAPS, nicht mehr
unterstützte Windows-Versionen.

DumbMonit bindet sich mit einem dedizierten Dienstkonto, das ein
**einfacher Domänenbenutzer ohne Gruppenmitgliedschaft** ist. In
Active Directory kann jeder authentifizierte Benutzer lesen, was
DumbMonit liest; kein administratives Recht wird benötigt, und keines
sollte gewährt werden. DumbMonit schreibt nie in das Verzeichnis,
liest nie ein Passwort-Attribut (von LAPS liest es das Ablaufdatum,
nie das Passwort) und fragt nie nach den Replikationsrechten, die
Secrets exponieren würden.

Ein Monitor deckt die gesamte Domäne ab: Richten Sie ihn auf einen
Domänencontroller, und DumbMonit findet die anderen in der
Konfigurationspartition.

## Was LDAP sagen kann und nicht

LDAP zeigt das Verzeichnis, wie es ist, nicht die Logs, was darin
geschah. Insbesondere:

- **Replikation.** Der abgefragte Domänencontroller veröffentlicht
  den Zustand seiner eingehenden Replikationspartner (letzter
  Erfolg, aufeinanderfolgende Fehlschläge, letzter Fehler) nur an ein
  Konto mit dem Recht *Monitor active directory replication*. Ohne
  es sagt das Panel *replication status not readable*, statt zu
  raten. Nur die Sicht des abgefragten Domänencontrollers wird
  gezeigt: Um die Partner eines anderen zu sehen, fügen Sie ihn als
  zweiten Monitor hinzu.
- **Über LDAP nicht sichtbar**: Ereignisprotokolle (Logons,
  Sperrungen, wenn sie passieren, Gruppenänderungen als Ereignisse),
  Group-Policy-Inhalte, SYSVOL, DNS-Zonen, und die Passwort-Hashes
  selbst.
- **Letzte Login-Daten** kommen von `lastLogonTimestamp`, das
  Domänencontroller nur alle 9 bis 14 Tage replizieren: Ein als
  inaktiv gezähltes Konto ist auf zwei Wochen genau inaktiv.
- **Gruppenmitgliedschaft** wird mit Verschachtelung gelesen (die
  Regel `LDAP_MATCHING_RULE_IN_CHAIN`), über `member`/`memberOf`.
  Eine durch `primaryGroupID` gesetzte primäre Gruppe ist kein
  `member`-Link und wird nicht gesehen.

## Wie es liest

Zwei Geschwindigkeiten:

- **Bei jeder Prüfung** (standardmäßig jede Minute): Verbindung und
  Bind, die RootDSE (Funktionsebenen, Uhr, Synchronisation), das
  Domänenobjekt (Passwortrichtlinie, Maschinenkonto-Kontingent), die
  fünf FSMO-Rollen, die Domänencontroller aus der
  Konfigurationspartition, die privilegierten Gruppen und ihre
  effektiven Mitglieder, das Recycle-Bin-Feature, und eine
  TCP-Verbindung zum LDAP-Port jedes anderen Domänencontrollers.
- **Im Hintergrund** alle `inventory_minutes` (standardmäßig 15):
  jedes Benutzer- und Computerkonto, mit nur den von den Befunden
  benötigten Attributen, in Seiten zu 500. Das gesamte Verzeichnis
  jede Minute zu enumerieren würde die Domänencontroller umsonst
  belasten.

Die erste Prüfung wartet ein paar Sekunden auf das Inventar; auf
einer großen Domäne erscheinen die Konto-Befunde bei der nächsten
Prüfung.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_ad_`.

| Metrik | Was | Labels |
|---|---|---|
| `bind_ok` | 1, wenn das Dienstkonto akzeptiert wird, 0, wenn der Domänencontroller es ablehnt | |
| `connect_seconds`, `bind_seconds` | Zeit zum Öffnen der Verbindung (TLS eingeschlossen) und zum Binden | |
| `tls_verified` | 1, wenn das Zertifikat geprüft wird | |
| `domain_info` | Wert 1 | `domain`, `forest`, `dc` |
| `domain_functional_level`, `forest_functional_level` | 0 (2000) bis 7 (2016), 10 (2025) | |
| `dc_synchronized`, `dc_global_catalog_ready` | der abgefragte Domänencontroller hat seine erste Replikation abgeschlossen, sein Global Catalog ist bereit | |
| `clock_skew_seconds` | Uhr des Domänencontrollers minus Uhr des DumbMonit-Hosts | |
| `recycle_bin_enabled` | 1, wenn das AD Recycle Bin aktiviert ist | |
| `dc_count` | Domänencontroller der Domäne | |
| `dc_info` | Wert 1 | `dc`, `site`, `gc`, `rodc`, `os` |
| `dc_reachable` | 1 oder 0 pro Domänencontroller; keine Zeitreihe, wenn sein Name von DumbMonit aus nicht aufgelöst wird | `dc` |
| `fsmo_role` | Wert 1 | `role` (`schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`), `holder` |
| `replication_readable` | 1, wenn der eingehende Replikationsstatus gelesen werden kann | |
| `replication_consecutive_failures`, `replication_last_success_age_seconds` | pro eingehendem Partner und Partition | `source`, `naming_context` |
| `privileged_group_members`, `privileged_group_enabled_members` | effektive Mitglieder, Verschachtelung eingeschlossen, Gruppen ausgelassen | `group` |
| `privileged_group_fingerprint` | ein Hash der Mitglieder: ändert sich, wenn jemand beitritt oder geht | `group` |
| `users_total`, `users_enabled`, `users_disabled`, `users_locked_out`, `users_password_never_expires`, `users_stale` | Benutzerkonten | |
| `computers_total`, `computers_enabled`, `computers_stale`, `groups_total` | Computer und Gruppen | |
| `admincount_accounts` | durch AdminSDHolder geschützte Benutzer (`adminCount=1`) | |
| `krbtgt_password_age_seconds` | Alter des krbtgt-Passworts | |
| `laps_schema`, `laps_computers`, `laps_eligible_computers` | LAPS im Schema; aktivierte Nicht-DC-Computer mit LAPS-Ablauf, von allen | |
| `password_min_length`, `password_history_length`, `password_max_age_seconds` (0 = nie), `password_complexity`, `lockout_threshold`, `machine_account_quota` | Standard-Domänenrichtlinie | |
| `finding_count` | von einem Befund betroffene Objekte, 0, wenn geprüft und sauber | `finding`, `severity`, `title` |
| `inventory_age_seconds`, `scrape_errors` | Alter des letzten vollständigen Inventars; nicht lesbare Teile des Verzeichnisses | |

Die privilegierten Gruppen werden über ihre bekannte Kennung
gefunden, nicht ihren Namen, daher funktioniert eine in einer anderen
Sprache installierte Domäne genauso: Domain Admins (RID 512),
Enterprise Admins (519) und Schema Admins (518) — die letzten zwei
nur in der Forest-Root-Domäne — und die eingebauten Administrators,
Account Operators, Server Operators, Print Operators und Backup
Operators.

## Befunde

Befunde sind rohe Fakten — wie viele Objekte, und welche — kein
Score; der [Security Score](../using/security-score.md) des Monitors
gewichtet sie. Jeder hat eine stabile `id`, einen Schweregrad auf der
Skala des Scores (low, medium, high, critical, auf der Seite als
Info, Advisory und Warning angezeigt), eine Kategorie, eine Zählung
und bis zu zwanzig Beispiel-Kontonamen. Ein geprüfter und als
unauffällig befundener Befund wird mit einer Zählung von 0 behalten,
sodass ein behobenes Problem grün wird statt zu verschwinden;
Befunde über Konten erscheinen, sobald das erste Hintergrund-Inventar
abgeschlossen ist. Sie werden unverändert von
`GET /api/targets/{id}/ad/findings` ausgeliefert und als
`dumbmonit_ad_finding_count{finding, severity, title}` veröffentlicht.

| `id` | Schweregrad | Was |
|---|---|---|
| `unconstrained_delegation` | critical | aktivierte Konten, denen uneingeschränkte Delegation vertraut wird, Domänencontroller ausgenommen |
| `kerberoastable_privileged` | critical | aktivierte Mitglieder privilegierter Gruppen mit einem Service Principal Name |
| `asrep_roastable_users` | high | aktivierte Konten mit deaktivierter Kerberos-Präauthentifizierung (`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | high | aktivierte Benutzer mit einem Service Principal Name |
| `krbtgt_password_age` | high | krbtgt-Passwort älter als 180 Tage |
| `laps_coverage` | high, wenn LAPS fehlt, medium, wenn teilweise | aktivierte Computer (Domänencontroller ausgenommen) ohne LAPS-Passwort |
| `reversible_encryption` | high | Benutzer, deren Passwort reversibel verschlüsselt gespeichert wird |
| `guest_enabled` | high | das eingebaute Guest-Konto (RID 501) ist aktiviert |
| `obsolete_os` | high | aktivierte Computer auf Windows XP, Vista, 7, 8, 2000, Server 2003, 2008 oder 2012 |
| `dc_unreachable` | high | Domänencontroller, deren LDAP-Port nicht antwortet |
| `replication_failing` | high | eingehende Replikationspartner mit aufeinanderfolgenden Fehlschlägen |
| `dc_not_synchronized` | high | der abgefragte Domänencontroller hat seine erste Replikation nicht abgeschlossen |
| `fsmo_role_orphaned` | high | eine FSMO-Rolle, die von einem gelöschten Domänencontroller gehalten wird |
| `password_policy` | high unter 8 Zeichen oder ohne Lockout, sonst medium | Lücken in der Standard-Domänenrichtlinie gegen die Baseline: 14 Zeichen, ein Lockout-Schwellenwert, Komplexität |
| `privileged_group_size` | medium | mehr als 5 aktivierte Konten in Domain Admins, Enterprise Admins und Administrators zusammen |
| `stale_accounts` | medium | aktivierte Benutzer und Computer, inaktiv über die Schwellenwerte hinaus |
| `protocol_transition_delegation` | medium | eingeschränkte Delegation mit Protokollübergang |
| `des_only` | medium | auf DES-Kerberos-Schlüssel beschränkte Konten |
| `password_not_required` | medium | Benutzer, denen ein leeres Passwort erlaubt ist (`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | medium | privilegierte Benutzer, deren Passwort nie abläuft |
| `privileged_stale` | medium | privilegierte Benutzer, inaktiv über `stale_days_users` hinaus |
| `schema_admins_not_empty` | medium | Schema Admins hat Mitglieder |
| `functional_level_old` | medium | Domänenfunktionsebene unter 2012 R2 |
| `clock_skew` | medium | mehr als fünf Minuten zwischen dem Domänencontroller und DumbMonit |
| `windows10_end_of_support` | medium | aktivierte Computer auf Windows 10 |
| `admincount_orphans` | low | `adminCount=1`-Benutzer, nicht mehr in einer privilegierten Gruppe |
| `password_never_expires` | low | aktivierte Benutzer, deren Passwort nie abläuft |
| `machine_account_quota` | low | jeder Benutzer kann Computer der Domäne beitreten lassen |
| `recycle_bin_disabled` | low | das AD Recycle Bin ist nicht aktiviert |

Die geltenden [eingebauten Regeln](../alerting/rules.md#active-directory):

- **Domain controller unreachable**: ein anderer Domänencontroller
  der Domäne hört seit fünf Minuten auf, auf seinem LDAP-Port zu
  antworten (Warning). Der abgefragte ist von *Monitor unreachable*
  abgedeckt.
- **Active Directory bind failed**: der Domänencontroller lehnt das
  Dienstkonto ab — abgelaufenes Passwort, deaktiviert oder gesperrt
  (Advisory).
- **Privileged group membership changed**: jemand ist einer
  privilegierten Gruppe beigetreten oder hat sie verlassen,
  Verschachtelung eingeschlossen (Advisory, löst sich eine Stunde
  später).
- **Active Directory replication failing**: ein eingehender Partner
  schlägt eine halbe Stunde lang fehl (Advisory), wenn der
  Replikationsstatus lesbar ist.
- **krbtgt password older than 180 days** (Info).

## Die Monitorseite

Das Active-Directory-Panel zeigt die Domäne (Funktionsebenen, der
abgefragte Domänencontroller, Uhrenabweichung), die
Domänencontroller mit ihrer Site, Rollen und ob sie antworten, die
privilegierten Gruppen mit ihren Mitgliedern, die Passwortrichtlinie
und Kontenzahlen, und die Befundtabelle mit Schweregrad-Worten.

## Ein nur-lesendes Dienstkonto für DumbMonit erstellen

1. Erstellen Sie einen dedizierten Benutzer für DumbMonit, zum
   Beispiel svc-dumbmonit, und fügen Sie ihn keiner Gruppe hinzu. Ein
   einfacher Domänenbenutzer reicht: jeder authentifizierte Benutzer
   kann lesen, was DumbMonit liest. In PowerShell, mit dem
   Active-Directory-Modul:

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. Geben Sie ihm ein langes Zufallspasswort. Wenn Passwörter in Ihrer
   Domäne ablaufen, hebt DumbMonit Active Directory bind failed am
   Tag, an dem dieses abläuft.

3. Optional: Um den Status der eingehenden Replikation zu sehen,
   gewähren Sie dem Konto das Recht Monitor active directory
   replication am Domänenobjekt selbst. Es enthüllt kein Passwort
   und kein Secret.

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. LDAPS braucht ein Zertifikat auf jedem Domänencontroller; eine
   Enterprise-Zertifizierungsstelle (AD CS) stellt automatisch eines
   aus. Exportieren Sie das Zertifikat dieser Autorität im
   Base-64-codierten X.509-Format und fügen Sie es unten als CA
   certificate ein.

5. Geben Sie in DumbMonit den Namen eines Domänencontrollers ein, wie
   sein Zertifikat ihn benennt, zum Beispiel "dc1.corp.example.com",
   und loggen Sie sich als svc-dumbmonit@corp.example.com ein.
   DumbMonit findet die anderen Domänencontroller selbst und prüft,
   dass jeder antwortet.

6. DumbMonit liest nur: Es schreibt nie in das Verzeichnis, und von
   LAPS liest es das Ablaufdatum, nie das Passwort.

!!! warning
    Einfaches LDAP auf Port 389 sendet das Passwort des Dienstkontos
    im Klartext: DumbMonit lehnt es ab, sofern Sie nicht Allow plain
    LDAP ankreuzen, und ein Domänencontroller, der LDAP-Signierung
    verlangt, lehnt es ebenfalls ab. Nutzen Sie LDAPS, oder StartTLS
    auf Port 389.

Um das Konto davon abzuhalten, irgendwo eine Sitzung zu öffnen,
verweigern Sie ihm per Group Policy *Log on locally* und *Log on
through Remote Desktop Services*: Ein LDAP-Bind ist davon nicht
betroffen. Beschränken Sie es nicht mit *Log On To* oder *Deny access
to this computer from the network*: Die Domänencontroller würden den
Bind dann ablehnen.

### Geringste Rechte, im Detail

- Das Konto braucht keine Gruppe außer Domain Users, die jedes Konto
  hat.
- Nutzen Sie kein Managed Service Account (gMSA): Es kann keinen
  einfachen LDAP-Bind ausführen.
- Gewähren Sie nicht *Replicating Directory Changes* (oder *All*):
  Das ist das DCSync-Recht, das jeden Passwort-Hash exponiert.
  DumbMonit braucht es nie.
- *Monitor active directory replication* erlaubt nur das Lesen von
  Replikations-Metadaten (Partner, letzter Erfolg, Fehlercodes). Es
  ist optional.

## LDAPS und Zertifikate

- **LDAPS (Port 636)** ist der Standard. Jeder Domänencontroller
  einer Domäne mit einer Enterprise-Zertifizierungsstelle erhält
  automatisch ein Zertifikat; ohne eine hört der Domänencontroller
  nicht auf 636. Das Zertifikat muss den Namen tragen, den DumbMonit
  nutzt: Geben Sie den Domänencontroller mit seinem DNS-Namen ein,
  nicht seiner IP-Adresse, sofern das Zertifikat die Adresse nicht
  listet.
- **CA certificate**: Fügen Sie das PEM der Root- (oder
  ausstellenden) Autorität ein. Nur dieser Autorität wird dann
  vertraut. Ohne sie vertraut DumbMonit den öffentlichen Roots und
  dem System-Store seines Containers, die eine private
  Enterprise-Autorität nicht kennen.
- **StartTLS (Port 389)** verschlüsselt genauso, nach einer
  Klartext-Begrüßung. Nutzen Sie es, wenn Port 636 gefiltert ist.
- **Accept an unverifiable certificate** hält die Verbindung
  verschlüsselt, akzeptiert aber jedes Zertifikat: Jeder auf dem Weg
  könnte sich als der Domänencontroller ausgeben und das Passwort des
  Dienstkontos erhalten.
- **LDAP channel binding** (`LdapEnforceChannelBinding`) betrifft
  NTLM- und Kerberos-Binds über TLS; der von DumbMonit genutzte
  einfache Bind ist nicht betroffen.

## Credentials

| Feld | Wert |
|---|---|
| User name | `svc-dumbmonit@corp.example.com`, `CORP\svc-dumbmonit`, oder der vollständige DN des Kontos |
| Password | das Passwort des Kontos, verschlüsselt gespeichert und nie wieder angezeigt |

Ein leeres Passwort wird abgelehnt: Active Directory würde es als
nicht authentifizierten Bind akzeptieren, ohne irgendetwas zu prüfen.

## Optionen

| Option | Standard | Was |
|---|---|---|
| Connection security (`security`) | `ldaps` | `ldaps`, `starttls` oder `plain` |
| Port (`port`) | 636, oder 389 | genutzt, falls die Adresse keinen angibt |
| CA certificate (`ca_cert`) | | PEM der Autorität, die die Zertifikate der Domänencontroller ausgestellt hat |
| Accept an unverifiable certificate (`insecure_tls`) | aus | |
| Allow plain LDAP (`allow_plaintext`) | aus | für `plain` erforderlich |
| Check every domain controller (`check_all_dcs`) | ein | TCP-Verbindung zum LDAP-Port jedes Domänencontrollers |
| Inactive user after (`stale_days_users`) | 90 | Tage |
| Inactive computer after (`stale_days_computers`) | 90 | Tage |
| Full inventory every (`inventory_minutes`) | 15 | Minuten, 1 bis 1440 |
| Timeout (`request_timeout_seconds`) | 8 | pro LDAP-Operation, 1 bis 60 |

## Fehlerbehebung

- **TLS certificate rejected**: Fügen Sie das Zertifikat der Autorität
  als CA certificate ein, und geben Sie den Domänencontroller mit
  einem Namen ein, den sein Zertifikat listet.
- **Bind refused: wrong user name or password (LDAP result 49)**: Der
  Diagnosecode des Domänencontrollers wird übersetzt — Passwort
  abgelaufen, Konto deaktiviert, gesperrt, oder muss beim nächsten
  Login das Passwort ändern.
- **The domain controller requires a protected connection**:
  LDAP-Signierung wird erzwungen; wechseln Sie zu LDAPS oder StartTLS.
- **Ein Domänencontroller zeigt sich als unresolved**: Sein Name wird
  vom DumbMonit-Host aus nicht aufgelöst. Richten Sie DumbMonits DNS
  auf die DNS-Server der Domäne, oder entfernen Sie das Häkchen bei
  *Check every domain controller*. Nicht aufgelöste Namen lösen nie
  einen Alarm aus.
- **Replication status not readable**: Gewähren Sie *Monitor active
  directory replication* wie in Schritt 3, oder akzeptieren Sie, dass
  Replikation nicht angezeigt wird.
