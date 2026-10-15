# Forgejo / Gitea {#forgejo-gitea}

Forgejo und Gitea teilen sich dieselbe REST-API, daher liest DumbMonit beide mit einem
Collector: ob die Instanz antwortet, ihren Health-Check, wie viele Repositories, Konten und
Organisationen sie enthält, ob ihre geplanten Aufgaben (Repository-Prüfungen, Bereinigungen, der
Update-Check) noch in ihrem eigenen Intervall laufen und ob ihre Actions-Runner online sind.

Zwei Aufrufe müssen gelingen: `/api/v1/version`, öffentlich, und `/api/v1/user`, der Nachweis,
dass das **Zugriffstoken** gültig ist. Alles andere — der Health-Check, die Zahlen zu
Repositories/Konten/Organisationen, die geplanten Aufgaben und die Actions-Runner — wird von
Endpunkten gelesen, die Forgejo und Gitea Administratorkonten der Website vorbehalten, und wird
übersprungen, ohne die Prüfung scheitern zu lassen, wenn das Token zu einem gewöhnlichen Konto
gehört.

Der Fehler, auf den es hier ankommt: **eine geplante Aufgabe, die nicht mehr läuft**. Forgejo
und Gitea liefern weiter Seiten und Git-Operationen aus, während ihr interner Scheduler
festhängt; niemand merkt es, bis der Update-Check oder eine Repository-Konsistenzprüfung seit
Wochen nicht mehr gelaufen ist.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_forgejo_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `token_is_admin` | 1, wenn das Konto des Tokens Administrator der Website ist | |
| `healthz_ok` | 1, außer der Health-Check meldet `fail` | |
| `healthz_check` | 1 pro ausgefallener Komponente (Datenbank, Cache) | `check` |
| `repos` | für das Konto des Tokens sichtbare Repositories | |
| `users`, `orgs` | Konten und Organisationen der Instanz | |
| `cron_tasks` | auf der Instanz definierte geplante Aufgaben | |
| `cron_task_overdue` | 1 pro Aufgabe, deren nächste Ausführung bereits fällig ist, sonst 0; deaktivierte Aufgaben (Zeitplan `-`) werden übersprungen | `task` |
| `runners_total`, `runners_online`, `runners_offline` | der Instanz bekannte Actions-Runner; `online`, `idle` und `active` zählen als online | |
| `runner_offline` | Wert 1 pro Offline-Runner (höchstens 32) | `runner` |
| `scrape_errors` | nur für Administratoren verfügbare Aufrufe, die bei der letzten Prüfung fehlschlugen (0 bei einem gewöhnlichen Token, dessen Aufrufe nie versucht werden) | |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#self-hosted-forges): Forgejo/Gitea-
Runner alle offline, Forgejo/Gitea geplante Aufgabe überfällig, außerdem Monitor nicht erreichbar.
Ein Sicherheitswert prüft die geplanten Aufgaben: siehe
[Hersteller-Updates und weitere Prüfungen](../using/security-score.md).

## Was nicht überwacht wird {#what-it-does-not-watch}

Forgejo und Gitea haben keine Nur-Lese-Administratorrolle: Jeder Aufruf, der „admin“
vorbehalten ist, braucht ein Konto mit vollen Rechten, keinen auf Scopes beschränkten
Delegierten. Ihre API hat zudem kein Gegenstück zu den Registrierungs- oder
Zwei-Faktor-Einstellungen von GitLab und auch keine selbst gemeldete Markierung „Update
verfügbar“ — vergleichen Sie die obige Version von Hand mit den
[Release-Notes](https://forgejo.org/releases/) des Projekts.

## Die Monitorseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite ruft
Forgejo oder Gitea nie auf. Es nennt zuerst, was nicht stimmt (Health-Check schlägt fehl, alle
Runner offline, eine geplante Aufgabe überfällig), dann die Version, dann die Zahlen zu
Repositories, Konten und Organisationen.

## Ein Zugriffstoken auf einem Administratorkonto der Website erstellen {#create-an-access-token-on-a-site-administrator-account}

1. Erstellen Sie ein Zugriffstoken auf einem Administratorkonto der Website: Avatar → Einstellungen → Anwendungen → Neues Token generieren. Benennen Sie es wie folgt und haken Sie die Scopes read:admin, read:repository und read:user an.

    ```
    dumbmonit
    ```

2. read:admin öffnet die Verwaltungszahlen, die geplanten Aufgaben und die Actions-Runner; ohne dieses Scope werden nur die Version und das eigene Konto des Tokens gelesen, und der Rest wird übersprungen, ohne die Prüfung scheitern zu lassen. Forgejo und Gitea haben auch keine Nur-Lese-Administratorrolle.

3. Geben Sie in DumbMonit die Adresse ein, zum Beispiel „forgejo.lan“ (Port 3000) oder „https://git.example.com“ hinter einem Reverse-Proxy, und fügen Sie das Token ein.

!!! warning
    Ein Zugriffstoken auf einem Administratorkonto der Website kann alles lesen und ändern, was das Konto kann, unabhängig von den angehakten Scopes: Forgejo und Gitea steuern nur, welche API-Routen ein Scope öffnet, nicht wie viel ein Administratorkonto selbst darf. Richten Sie dafür ein eigenes Bot-Konto ein, statt Ihr eigenes zu verwenden.

Das Token von Hand prüfen, vom DumbMonit-Host aus:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Zugriffstoken | Wird als `Authorization: token …` gesendet. |

Adresse: ein Hostname (`forgejo.lan`) oder `host:port`. Optionen: Protokoll (standardmäßig
HTTP — setzen Sie für alles außer einem LAN einen Reverse-Proxy mit TLS davor), Port (3000),
Zertifikatsprüfung und Anfrage-Timeout (15 s).

Validiert gegen Forgejo 9.0 (offizielle API-Quellen und Swagger-Definitionen, mit Gitea geteilt).
