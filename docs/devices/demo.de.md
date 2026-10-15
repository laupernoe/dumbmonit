# Demo-Monitor

Ein fiktiver Monitor, das Messwerte erzeugt, um das Tool ohne Hardware zu
erkunden. Keine Hardware erforderlich.

## Einrichtung

1. Geben Sie ihm irgendeinen Namen und bestätigen Sie.
2. Gefälschte Messwerte erscheinen sofort, genug, um die Oberfläche zu
   erkunden.

Die Adresse ist frei wählbar (der Platzhalter sagt `demo`), es gibt kein
Credential und keine Option. Der Demo-Monitor erzeugt eine
`cpu_usage_percent`-Zeitreihe, daher greifen die Regeln "High CPU" und
"Unusual CPU" darauf, und er kann als Eltern- oder Kind-Monitor in einer
[Abhängigkeitskette](../alerting/index.md#dependency-suppression) genutzt
werden, um die Unterdrückung in Aktion zu sehen.

Für echte Messwerte ohne dedizierte Hardware richten Sie ein SNMP-Monitor auf
eine beliebige Maschine mit laufendem `snmpd`: siehe
[Testen ohne Hardware](../install/docker.md#testing-without-hardware).
