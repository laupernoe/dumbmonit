# GitLab (self-managed)

Ob eine selbst gehostete GitLab-Instanz bedient, und ob die Teile, die
leise alles andere tragen, mithalten: Sidekiqs Warteschlangen, die
CI/CD-Runner, von einem Upgrade übriggebliebene Datenbankmigrationen,
und — für GitLab Enterprise Edition — die Lizenz.

DumbMonit liest `/api/v4/version` mit einem **Personal Access Token**,
der einzige Aufruf, der gelingen muss: Er ist der Beweis, dass GitLab
antwortet und dass das Token gültig ist. Alles andere — Sidekiq,
Runner, Migrationen, Instanz-Statistiken, die Sign-up- und
Zwei-Faktor-Einstellungen, die Lizenz, und der Readiness-Check im
Detail — wird von Endpunkten gelesen, die GitLab Administrator-Konten
vorbehält, und wird ohne Fehlschlag der Abfrage übersprungen, wenn das
Token zu einem gewöhnlichen Konto gehört.

Der hier beobachtenswerte Fehler: **Sidekiq, der zurückfällt**. GitLab
liefert weiter Seiten aus, während seine Hintergrund-Worker-
Warteschlange wächst, und nichts auf der Seite sagt das, bis eine
Benachrichtigung, ein Webhook oder ein CI-Pipeline-Update um Minuten
oder Stunden verspätet ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_gitlab_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version`, `revision` |
| `readiness_check` | 1, wenn eine Readiness-Komponente (Datenbank, Cache, Warteschlangen, Shared State, Gitaly) nicht `ok` ist | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | wartende Jobs und ihr Alter, pro Sidekiq-Warteschlange | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | der Instanz bekannte CI/CD-Runner | |
| `runner_offline` | Wert 1 pro offline Runner (höchstens 32) | `runner` |
| `migrations_pending` | noch anzuwendende Datenbankmigrationen | |
| `migration_pending` | Wert 1 pro ausstehender Migration (höchstens 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | instanzweite Zählungen | |
| `signup_enabled` | 1, während die öffentliche Anmeldung offen ist | |
| `two_factor_required` | 1, während jeder Benutzer Zwei-Faktor-Authentifizierung einrichten muss | |
| `license_info` | Wert 1 (nur Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | ob die Lizenz abgelaufen ist, und die verbleibende Zeit | |
| `pipelines_failed_recent`, `pipelines_pending` | fehlgeschlagene Pipelines im Rückblickfenster, und jetzt ausstehende oder laufende Pipelines — nur für die in der Option "Projects watched for CI health" aufgeführten Projekte | `project` |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene nur-administrative Aufrufe (0 mit einem gewöhnlichen Token, deren Aufrufe nie versucht werden) | |

Die geltenden [eingebauten Regeln](../alerting/rules.md#self-hosted-forges):
GitLab runners all offline, GitLab Sidekiq backlog high, GitLab
migrations pending, GitLab license expiring, plus Device unreachable.
Ein Security Score prüft die Zwei-Faktor- und Sign-up-Einstellungen, und
ausstehende Migrationen: siehe
[Vendor-Updates und andere Prüfungen](../using/security-score.md).

## Was nicht beobachtet wird

GitLabs eigene REST-API hat keinen Endpunkt, der sagt "eine neuere
GitLab-Version ist verfügbar": anders als bei Nextcloud oder
Paperless-ngx gibt es kein selbst gemeldetes Update-Flag zu lesen,
daher behauptet DumbMonit keines. Vergleichen Sie die obige Version von
Hand mit GitLabs [Release-Seite](https://about.gitlab.com/releases/),
oder beobachten Sie GitLabs eigenen Update-Check-Hinweis im
Administrationsbereich.

Die CI-Pipeline-Gesundheit wird nur für die von Ihnen aufgeführten
Projekte gelesen: GitLabs API hat keinen instanzweiten
"alle fehlschlagenden Pipelines"-Endpunkt, nur einen pro Projekt.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite ruft GitLab nie auf. Es sagt zuerst, was nicht stimmt
(eine fehlgeschlagene Readiness-Komponente, alle Runner offline, ein
großer Sidekiq-Rückstand), dann die Version, dann die Instanzzahlen und
die aus der Administrations-API gelesenen Einstellungen.

## Ein Personal Access Token auf einem Administrator-Konto erstellen

1. Erstellen Sie ein Personal Access Token auf einem
   Administrator-Konto: Avatar → Edit profile → Access tokens → Add new
   token. Benennen Sie es wie folgt, setzen Sie ein Ablaufdatum, das Sie
   gerne erneuern, und kreuzen Sie nur den Scope read_api an: Er liest
   die gesamte API und schreibt nichts.

    ```
    dumbmonit
    ```

2. Sidekiq-Warteschlangen, CI-Runner, ausstehende Migrationen,
   Instanz-Statistiken, die Sign-up- und Zwei-Faktor-Einstellungen und
   die Lizenz werden jeweils von einem nur-administrativen Endpunkt
   gelesen: GitLab hat dafür keine feinere Delegation. Ein Token von
   einem gewöhnlichen Konto funktioniert trotzdem; dann wird nur die
   Version gelesen, und der Rest wird ohne Fehlschlag der Abfrage
   übersprungen.

3. Geben Sie in DumbMonit die Adresse ein, mit der Sie GitLab öffnen,
   zum Beispiel "gitlab.example.com" (HTTPS auf 443) oder
   "https://git.example.com" hinter einem Reverse Proxy, und fügen Sie
   das Token ein.

4. Um auch CI-Pipelines zu beobachten, listen Sie die numerischen
   Projekt-IDs (auf der Übersichtsseite jedes Projekts unter seinem
   Namen angezeigt) in der Option "Projects watched for CI health",
   kommagetrennt.

!!! warning
    Ein Personal Access Token auf einem Administrator-Konto kann alles
    lesen, was das Konto lesen kann, selbst mit dem Scope read_api:
    GitLab hat keine nur-lesende Administrator-Rolle. Richten Sie dafür
    ein eigenes Bot-Konto ein, statt Ihr eigenes wiederzuverwenden, und
    widerrufen Sie das Token, wenn es jemals durchsickert.

Das Token von Hand prüfen, vom DumbMonit-Host aus:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credentials

| Credential | Felder |
|---|---|
| Personal Access Token | Als `PRIVATE-TOKEN`-Header gesendet. |

Adresse: ein Hostname (`gitlab.example.com`) oder `host:port`. Optionen:
Protokoll (standardmäßig HTTPS), Port (443), Zertifikatsprüfung,
Anfrage-Timeout (15 s), die für CI-Gesundheit beobachteten Projekte
(standardmäßig keine) und das Rückblickfenster für fehlgeschlagene
Pipelines (24 h).

Validiert gegen GitLab Enterprise Edition 17.5 (offizielle
API-Dokumentation).
