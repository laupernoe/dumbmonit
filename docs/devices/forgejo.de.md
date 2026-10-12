# Forgejo / Gitea

Forgejo und Gitea teilen dieselbe REST-API, daher liest DumbMonit beide
mit einem Collector: ob die Instanz antwortet, ihr Healthcheck, wie
viele Repositories, Konten und Organisationen sie hält, ob ihre
geplanten Tasks (Repository-Checks, Bereinigungen, der Update-Checker)
noch in ihrem eigenen Intervall laufen, und ob ihre Actions-Runner
online sind.

Zwei Aufrufe müssen gelingen: `/api/v1/version`, öffentlich, und
`/api/v1/user`, der Beweis, dass das **Access Token** gültig ist. Alles
andere — der Healthcheck, die Repository-/Konten-/Organisationszahlen,
die geplanten Tasks und die Actions-Runner — wird von Endpunkten
gelesen, die Forgejo und Gitea Site-Administrator-Konten vorbehalten,
und wird ohne Fehlschlag der Abfrage übersprungen, wenn das Token zu
einem gewöhnlichen Konto gehört.

Der hier beobachtenswerte Fehler: **ein geplanter Task, der aufgehört
hat zu laufen**. Forgejo und Gitea liefern weiter Seiten und
Git-Operationen aus, während ihr interner Scheduler hängt; niemand
merkt es, bis der Update-Checker oder ein Repository-Consistency-Check
seit Wochen nicht gelaufen ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_forgejo_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `token_is_admin` | 1, wenn das Konto des Tokens Site-Administrator ist | |
| `healthz_ok` | 1, außer der Healthcheck meldet `fail` | |
| `healthz_check` | 1 pro fehlschlagender Komponente (Datenbank, Cache) | `check` |
| `repos` | für das Konto des Tokens sichtbare Repositories | |
| `users`, `orgs` | Konten und Organisationen der Instanz | |
| `cron_tasks` | auf der Instanz definierte geplante Tasks | |
| `cron_task_overdue` | 1 pro Task, dessen nächster Lauf bereits überfällig ist, sonst 0 | `task` |
| `runners_total`, `runners_online`, `runners_offline` | der Instanz bekannte Actions-Runner | |
| `runner_offline` | Wert 1 pro offline Runner (höchstens 32) | `runner` |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene nur-administrative Aufrufe (0 mit einem gewöhnlichen Token, deren Aufrufe nie versucht werden) | |

Die geltenden [eingebauten Regeln](../alerting/rules.md#self-hosted-forges):
Forgejo/Gitea runners all offline, Forgejo/Gitea scheduled task overdue,
plus Device unreachable. Ein Security Score prüft die geplanten Tasks:
siehe [Vendor-Updates und andere Prüfungen](../using/security-score.md).

## Was nicht beobachtet wird

Forgejo und Gitea haben keine nur-lesende Administrator-Rolle: jeder
Aufruf, der "admin" vorbehalten ist, braucht ein Konto mit vollen
Rechten, keinen scope-begrenzten Delegierten. Ihre API hat auch kein
Äquivalent zu GitLabs Sign-up- oder Zwei-Faktor-Einstellungen, noch ein
selbst gemeldetes "Update verfügbar"-Flag — vergleichen Sie die Version
oben von Hand mit den [Release Notes](https://forgejo.org/releases/)
des Projekts.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite ruft Forgejo oder Gitea nie auf. Es sagt zuerst, was
nicht stimmt (fehlschlagender Healthcheck, alle Runner offline, ein
überfälliger geplanter Task), dann die Version, dann die Repository-,
Konten- und Organisationszahlen.

## Ein Access Token auf einem Site-Administrator-Konto erstellen

1. Erstellen Sie ein Access Token auf einem Site-Administrator-Konto:
   Avatar → Settings → Applications → Generate new token. Benennen Sie
   es wie folgt und kreuzen Sie die Scopes read:admin, read:repository
   und read:user an.

    ```
    dumbmonit
    ```

2. read:admin öffnet die Administrationszahlen, die geplanten Tasks und
   die Actions-Runner; ohne es werden nur die Version und das eigene
   Konto des Tokens gelesen, und der Rest wird ohne Fehlschlag der
   Abfrage übersprungen. Forgejo und Gitea haben ebenfalls keine
   nur-lesende Administrator-Rolle.

3. Geben Sie in DumbMonit die Adresse ein, zum Beispiel "forgejo.lan"
   (Port 3000) oder "https://git.example.com" hinter einem Reverse
   Proxy, und fügen Sie das Token ein.

!!! warning
    Ein Access Token auf einem Site-Administrator-Konto kann alles
    lesen und ändern, was das Konto kann, egal welche Scopes angekreuzt
    sind: Forgejo und Gitea begrenzen nur, welche API-Routen ein Scope
    öffnet, nicht wie viel ein Administrator-Konto selbst tun kann.
    Richten Sie dafür ein eigenes Bot-Konto ein, statt Ihr eigenes
    wiederzuverwenden.

Das Token von Hand prüfen, vom DumbMonit-Host aus:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credentials

| Credential | Felder |
|---|---|
| Access Token | Als `Authorization: token …` gesendet. |

Adresse: ein Hostname (`forgejo.lan`) oder `host:port`. Optionen:
Protokoll (standardmäßig HTTP — stellen Sie für alles außer einem LAN
einen Reverse Proxy mit TLS davor), Port (3000), Zertifikatsprüfung und
Anfrage-Timeout (15 s).

Validiert gegen Forgejo 9.0 (offizielle API-Quellen und
Swagger-Definitionen, mit Gitea geteilt).
