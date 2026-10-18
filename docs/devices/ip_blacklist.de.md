# Blockliste für IP-Bereiche {#ip-range-blocklist}

Jede Adresse eines öffentlichen IPv4-Bereichs, bis zu einem /24 (256 Adressen),
geprüft gegen die DNS-Blocklisten aus [Reputation des Mailservers](mail_blacklist.md#the-blocklists):
Spamhaus ZEN, Barracuda, SpamCop, Mailspike, UCEPROTECT Level 1, PSBL und
SpamRATS. Die Monitorseite sagt, wie viele Adressen gelistet sind und auf welchen
Listen, und gibt einen direkten Link zur Abfrage- und Entfernungsseite jedes
Eintrags.

Nützlich für den öffentlichen Bereich eines Büros, den Block, den Ihnen ein
Hosting-Anbieter gegeben hat, oder die Adressen, von denen Ihre Server senden: Die
kompromittierte Maschine eines Nachbarn bekommt einen Eintrag, den ein strengerer
Empfänger auf den ganzen Bereich ausdehnen kann.

## Grenzen {#limits}

- **Höchstens ein /24 pro Monitor.** Ein größerer Bereich wird mit einer Meldung
  abgelehnt, die sagt, wie viele Adressen er enthält: Teilen Sie ihn in mehrere
  /24-Monitore. Jede Adresse kostet eine Anfrage an jede Liste, und ein Durchlauf
  von Tausenden Adressen wirkt auf sie wie Missbrauch.
- **Nur IPv4.** Von diesen Listen antwortet nur Spamhaus ZEN für IPv6, und ein
  IPv6-Bereich lässt sich nicht Adresse für Adresse ablaufen. Eine einzelne
  IPv6-Adresse kann mit [Reputation des Mailservers](mail_blacklist.md) geprüft
  werden.
- **Nur öffentliche Adressen.** Ein Bereich mit einer privaten, geteilten (CGNAT)
  oder Dokumentationsadresse wird abgelehnt: Blocklisten kennen nur das öffentliche
  Internet.
- **Die Domain-Listen** (Spamhaus DBL, SURBL, URIBL) gelten nicht für einen
  Bereich.

## Resolver und Tempo {#resolvers-and-pace}

Es gelten dieselben Regeln wie bei [Reputation des Mailservers](mail_blacklist.md#resolvers-refusals-and-timeouts).
Spamhaus lehnt öffentliche Resolver ab: Seine Ablehnung (`127.255.255.254`) erscheint
als Konfigurationsfehler, nie als Eintrag. Eine Liste, die nicht antwortet, ist
unbekannt, nie sauber. Nutzen Sie einen eigenen rekursiven Resolver oder einen
kostenlosen Spamhaus-DQS-Schlüssel als Zugangsdaten.

Ein /24 braucht fast zweitausend Anfragen. Sie werden mit **Queries per second**
(standardmäßig 10) gesendet, höchstens acht gleichzeitig, und der ganze Bereich wird
einmal pro **Refresh interval** (standardmäßig 6 Stunden) geprüft; dazwischen wird
bei jeder Prüfung das letzte Ergebnis erneut gezeigt. Die erste Prüfung eines
Bereichs dauert einige Minuten und läuft im Hintergrund.

## Einen öffentlichen IP-Bereich gegen die Blocklisten prüfen {#check-a-public-ip-range-against-the-blocklists}

1. Geben Sie eine IPv4-Adresse oder einen CIDR-Bereich bis /24 ein, zum Beispiel „198.51.100.0/24“ (256 Adressen). Ein größerer Bereich wird abgelehnt: Teilen Sie ihn in mehrere /24-Monitore. IPv6-Bereiche werden abgelehnt, da nur Spamhaus für IPv6 antwortet: Prüfen Sie eine einzelne IPv6-Adresse mit Reputation des Mailservers.

2. Nutzen Sie einen eigenen rekursiven Resolver, wie bei Reputation des Mailservers: Spamhaus lehnt öffentliche Resolver ab, oder fügen Sie einen kostenlosen Spamhaus-DQS-Schlüssel als Zugangsdaten ein. Ein /24 braucht fast zweitausend Anfragen, gesendet mit 10 pro Sekunde und standardmäßig alle 6 Stunden wiederholt: Halten Sie dieses Tempo, Blocklisten sperren Resolver, die sie durchkämmen.

3. Die erste Prüfung eines Bereichs dauert einige Minuten: Die Monitorseite sagt, dass sie läuft, und zeigt dann, wie viele Adressen gelistet sind, auf welchen Listen, und für jede einen Link zur Entfernung.

!!! warning
    Die Spamhaus PBL führt Endnutzer-Bereiche, die nicht direkt Mail senden sollten (Heimanschlüsse, dynamische Pools): Ein Eintrag dort ist eine Richtlinie, keine Missbrauchsmeldung, und zählt nur für eine Adresse, die einen Mailserver betreibt.

## Was beobachtet wird {#what-it-watches}

Dieselben Metriken wie bei [Reputation des Mailservers](mail_blacklist.md#what-it-watches),
mit dem Präfix `dumbmonit_blacklist_`, ohne die zu Reverse-DNS, SPF und DMARC. Um die
Zahl der Reihen klein zu halten, wird `check` nur für eine Adresse geschrieben, die
gelistet ist oder auf die eine Liste nicht geantwortet hat; `list_checked`,
`list_listed` und `list_unknown` zählen jede Adresse pro Liste.

Die [integrierten Regeln](../alerting/rules.md#dns-blocklists), die greifen: Auf
einer großen Blockliste geführt und Auf einer Blockliste geführt.

## Zugangsdaten {#credentials}

Keine, oder ein Spamhaus-DQS-Schlüssel.

Adresse: eine IPv4-Adresse oder ein CIDR-Bereich bis /24. Optionen: Resolver, Lists
(standardmäßig alle Adresslisten), Refresh interval (6 Stunden), Queries per second
(10) und Timeout per query (5 s).
