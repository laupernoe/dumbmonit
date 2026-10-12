# Domain-Ablauf

Eine registrierte Domain, über RDAP von ihrer Registry gelesen: die
verbleibenden Tage bis zum Ablauf, eine Sperre, die sie aus dem DNS nimmt,
eine Erlösungsfrist (Redemption Period), ihr Registrar und ob ihre
Delegation mit DNSSEC signiert ist.

RDAP ist der Nachfolger von WHOIS: dieselben Registrierungsdaten, als JSON,
mit einem einheitlichen Format über alle Registries hinweg. DumbMonit
findet die Registry der Endung in der IANA-RDAP-Bootstrap-Datei
(`https://data.iana.org/rdap/dns.json`, höchstens einmal täglich gelesen)
und fragt sie dann unter `/domain/example.com`. Es wird kein Credential
benötigt, und nichts außer der Registrierung der Domain wird abgefragt.

Zwei Fehlerarten sind neben dem Ablaufdatum selbst beobachtenswert. Eine
Domain in `clientHold` oder `serverHold` wurde vom Registrar oder der
Registry aus dem DNS genommen, meist wegen einer unbezahlten Rechnung oder
einer nie verifizierten Kontaktadresse: Jede Site und jedes Postfach
darunter stoppt sofort. Eine Domain in `redemptionPeriod` oder
`pendingDelete` ist bereits abgelaufen und wird freigegeben, sofern sie
nicht gegen Gebühr wiederhergestellt wird.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_domain_`.

| Metrik | Was | Labels |
|---|---|---|
| `expiry_days` | verbleibende Tage bis zum Ablauf, mit Dezimalstellen; negativ nach Ablauf | |
| `expiry_timestamp_seconds` | das Ablaufdatum, in Unix-Sekunden | |
| `expiry_published` | 0, wenn die Registry das Ablaufdatum nicht veröffentlicht | |
| `on_hold` | 1 bei `clientHold` oder `serverHold` | |
| `redemption` | 1 bei `redemptionPeriod`, `pendingDelete` oder `pendingRestore` | |
| `status` | Wert 1, eine Zeitreihe pro Statuscode, in EPP-Form (`clientTransferProhibited`) | `status` |
| `registrar_info` | Wert 1 | `registrar` |
| `dnssec` | 1, wenn die Delegation signiert ist | |
| `rdap_ok` | 0, wenn die letzte Anfrage an die Registry fehlschlug und eine frühere Antwort gezeigt wird | |
| `rdap_age_seconds` | Alter der gezeigten Antwort | |

Die Registry wird einmal pro Aktualisierungsintervall abgefragt
(standardmäßig 12 Stunden); dazwischen werden die verbleibenden Tage bei
jeder Prüfung neu berechnet. Antwortet die Registry nicht, wird die letzte
Antwort bis zu sieben Tage weiterverwendet, mit `rdap_ok` auf 0, sodass ein
Registry-Ausfall nicht wie ein Domain-Problem aussieht.

Die geltenden [eingebauten Regeln](../alerting/rules.md#reverse-proxies-and-domains):
Domain expiring (unter 30 Tagen), Domain about to expire (unter 7 Tagen
oder abgelaufen), Domain on hold, Domain in redemption, plus Device
unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt nie die Registry ab. Es zeigt das Ablaufdatum und
die verbleibenden Tage, die Statuscodes (oder die Sperre und die
Erlösungsfrist als Satz), den Registrar und DNSSEC, sowie ob die letzte
Anfrage an die Registry fehlschlug.

## Die Registrierung einer Domain beobachten

1. Geben Sie die registrierte Domain ein, zum Beispiel "example.com", nicht
   einen Namen darunter wie www.example.com, den die Registry nicht kennt.
   Nichts einzurichten und kein Credential: RDAP, der Nachfolger von WHOIS,
   ist öffentlich.

2. DumbMonit findet die Registry der Endung in der IANA-RDAP-Bootstrap-
   Datei und fragt sie dann standardmäßig zweimal täglich nach dem
   Ablaufdatum, den Statuscodes und dem Registrar. Registries begrenzen
   RDAP-Anfragen, und das Ablaufdatum ändert sich einmal pro Jahr: Ein
   kürzeres Aktualisierungsintervall bringt nichts.

3. Einige Registries betreiben RDAP, ohne von der IANA gelistet zu sein,
   etwa die von .ch und .li: Setzen Sie den RDAP-Server auf ihre Adresse.
   Eine Endung, deren Registry keinen öffentlichen RDAP-Dienst betreibt,
   kann nicht beobachtet werden.

    ```
    https://rdap.nic.ch/
    ```

4. Manche Registries veröffentlichen das Ablaufdatum nicht (.ch ist eine
   davon): DumbMonit sagt das dann und beobachtet trotzdem die
   Statuscodes.

!!! warning
    Eine Domain in clientHold oder serverHold wird aus dem DNS genommen:
    Jede Site und jedes Postfach darunter stoppt, meist wegen einer
    unbezahlten Rechnung oder einer nicht verifizierten Kontaktadresse.
    DumbMonit warnt 30 Tage vor dem Ablauf, hebt den Schweregrad 7 Tage
    davor an und alarmiert sofort bei einer Sperre oder Erlösungsfrist.

Eine der Registry unbekannte Domain (eine Subdomain, ein Tippfehler, oder
eine gelöschte Domain) wird als Konfigurationsfehler gemeldet, nicht als
Ausfall. Namen mit Akzenten werden angenommen und in ihrer ASCII-Form
(`xn--…`) abgefragt.

## Credentials

Keine: RDAP ist öffentlich.

Adresse: die registrierte Domain (`example.com`); eine eingefügte URL wird
auf ihren Hostnamen reduziert. Optionen: RDAP-Server (leer: über IANA
gefunden), Aktualisierungsintervall (12 Stunden, von 1 bis 168) und
Anfrage-Timeout (10 s). Das Prüfintervall des Geräts selbst kann auf seinem
Standard bleiben: Es berechnet nur die verbleibenden Tage zwischen zwei
Anfragen an die Registry neu.
