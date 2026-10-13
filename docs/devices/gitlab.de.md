# GitLab (selbst gehostet) {#gitlab-self-managed}

Ob eine selbst gehostete GitLab-Instanz ausliefert und ob die Teile, die still alles Übrige
tragen, mithalten: die Warteschlangen von Sidekiq, die CI/CD-Runner, nach einem Upgrade
verbliebene Datenbankmigrationen und — bei GitLab Enterprise Edition — die Lizenz.

DumbMonit liest `/api/v4/version` mit einem **persönlichen Zugriffstoken**; das ist der einzige
Aufruf, der gelingen muss: Er belegt, dass GitLab antwortet und das Token gültig ist. Alles
andere — Sidekiq, Runner, Migrationen, Instanzstatistiken, die Registrierungs- und
Zwei-Faktor-Einstellungen, die Lizenz und der Readiness-Check im Detail — wird von Endpunkten
gelesen, die GitLab Administratorkonten vorbehält, und wird übersprungen, ohne die Prüfung
scheitern zu lassen, wenn das Token zu einem gewöhnlichen Konto gehört.

Der Fehler, auf den es hier ankommt: **Sidekiq, das zurückfällt**. GitLab liefert weiter Seiten
aus, während die Warteschlange seiner Hintergrund-Worker wächst, und nichts auf der Seite sagt
es, bis eine Benachrichtigung, ein Webhook oder die Aktualisierung einer CI-Pipeline um Minuten
oder Stunden verspätet ist.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_gitlab_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version`, `revision` |
| `readiness_check` | 1, wenn eine Readiness-Komponente (Datenbank, Cache, Warteschlangen, Shared State, Gitaly) nicht `ok` ist; fehlt, wenn GitLab sie DumbMonit nicht zugänglich macht (siehe unten) | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | wartende Jobs und deren Alter, pro Sidekiq-Warteschlange | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | der Instanz bekannte CI/CD-Runner | |
| `runner_offline` | Wert 1 pro Offline-Runner (höchstens 32) | `runner` |
| `migrations_pending` | noch anzuwendende Datenbankmigrationen | |
| `migration_pending` | Wert 1 pro ausstehender Migration (höchstens 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | instanzweite Zahlen | |
| `signup_enabled` | 1, solange die öffentliche Registrierung offen ist | |
| `two_factor_required` | 1, solange jeder Benutzer die Zwei-Faktor-Authentifizierung einrichten muss | |
| `license_info` | Wert 1 (nur Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | ob die Lizenz abgelaufen ist, und die verbleibende Zeit | |
| `pipelines_failed_recent`, `pipelines_pending` | fehlgeschlagene Pipelines im Rückblickfenster sowie jetzt wartende oder laufende Pipelines — nur für die in der Option „Für CI-Zustand überwachte Projekte“ aufgeführten Projekte | `project` |
| `scrape_errors` | nur für Administratoren verfügbare Aufrufe, die bei der letzten Prüfung fehlschlugen (0 bei einem gewöhnlichen Token, dessen Aufrufe nie versucht werden) | |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#self-hosted-forges): GitLab-Runner
alle offline, GitLab-Sidekiq-Rückstand hoch, GitLab-Migrationen ausstehend, GitLab-Lizenz läuft
ab, außerdem Gerät nicht erreichbar. Ein Sicherheitswert prüft die Zwei-Faktor- und
Registrierungseinstellungen sowie ausstehende Migrationen: siehe
[Hersteller-Updates und weitere Prüfungen](../using/security-score.md).

## Was nicht überwacht wird {#what-it-does-not-watch}

Die REST-API von GitLab hat keinen Endpunkt, der sagt „eine neuere GitLab-Version ist
verfügbar“: Anders als bei Nextcloud oder Paperless-ngx gibt es keine selbst gemeldete
Update-Markierung zu lesen, daher behauptet DumbMonit keine. Vergleichen Sie die obige Version
von Hand mit der [Release-Seite](https://about.gitlab.com/releases/) von GitLab oder achten Sie
auf den Update-Hinweis von GitLab im Admin-Bereich.

Der CI-Pipeline-Zustand wird nur für die von Ihnen aufgeführten Projekte gelesen: Die API von
GitLab hat keinen instanzweiten Endpunkt „alle fehlschlagenden Pipelines“, nur einen pro
Projekt.

## Die Geräteseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite ruft
GitLab nie auf. Es nennt zuerst, was nicht stimmt (eine Readiness-Komponente ausgefallen, alle
Runner offline, ein großer Sidekiq-Rückstand), dann die Version, dann die Instanzzahlen und die
über die Verwaltungs-API gelesenen Einstellungen.

## Ein persönliches Zugriffstoken auf einem Administratorkonto erstellen {#create-a-personal-access-token-on-an-administrator-account}

1. Erstellen Sie ein persönliches Zugriffstoken auf einem Administratorkonto: Avatar → Profil bearbeiten → Zugriffstoken → Neues Token hinzufügen. Benennen Sie es wie folgt, setzen Sie ein Ablaufdatum, das Sie gern erneuern, und haken Sie nur das Scope read_api an: Es liest die gesamte API und schreibt nichts.

    ```
    dumbmonit
    ```

2. Sidekiq-Warteschlangen, CI-Runner, ausstehende Migrationen, Instanzstatistiken, die Registrierungs- und Zwei-Faktor-Einstellungen und die Lizenz werden jeweils von einem Endpunkt nur für Administratoren gelesen: GitLab hat dafür keine feinere Delegation. Ein Token eines gewöhnlichen Kontos funktioniert trotzdem; dann wird nur die Version gelesen, und der Rest wird übersprungen, ohne die Prüfung scheitern zu lassen.

3. Geben Sie in DumbMonit die Adresse ein, mit der Sie GitLab öffnen, zum Beispiel „gitlab.example.com“ (HTTPS auf 443) oder „https://git.example.com“ hinter einem Reverse-Proxy, und fügen Sie das Token ein.

4. Um auch CI-Pipelines zu überwachen, tragen Sie die numerischen Projekt-IDs (auf der Übersichtsseite jedes Projekts unter seinem Namen angezeigt) kommagetrennt in die Option „Für CI-Zustand überwachte Projekte“ ein.

!!! warning
    Ein persönliches Zugriffstoken auf einem Administratorkonto kann alles lesen, was das Konto lesen kann, selbst mit dem Scope read_api: GitLab hat keine Nur-Lese-Administratorrolle. Richten Sie dafür ein eigenes Bot-Konto ein, statt Ihr eigenes zu verwenden, und widerrufen Sie das Token, falls es je durchsickert.

### Readiness-Check {#readiness-check}

GitLab beantwortet `/-/readiness` mit 404 für jede Client-Adresse, die nicht in seiner
Monitoring-Allowlist steht (standardmäßig nur 127.0.0.1; hinter Docker oder einem Reverse-Proxy
kommt DumbMonit von einer anderen Adresse). DumbMonit behandelt dieses 404 als „nicht
bereitgestellt“: Das Readiness-Panel bleibt leer und wird nicht in `scrape_errors` gezählt. Zum
Aktivieren tragen Sie die Adresse von DumbMonit in `gitlab.rb` ein und führen
`gitlab-ctl reconfigure` aus:

```
gitlab_rails['monitoring_whitelist'] = ['127.0.0.0/8', '::1/128', '192.168.1.10/32']
```

Der Lizenz-Endpunkt antwortet in der Community Edition ebenfalls mit 404, was auch kein Fehler
ist.

Das Token von Hand prüfen, vom DumbMonit-Host aus:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Persönliches Zugriffstoken | Wird als Header `PRIVATE-TOKEN` gesendet. |

Adresse: ein Hostname (`gitlab.example.com`) oder `host:port`. Optionen: Protokoll (standardmäßig
HTTPS), Port (443), Zertifikatsprüfung, Anfrage-Timeout (15 s), die für den CI-Zustand
überwachten Projekte (standardmäßig keine) und das Rückblickfenster für fehlgeschlagene
Pipelines (24 h).

Validiert gegen GitLab Enterprise Edition 17.5 (offizielle API-Dokumentation).
