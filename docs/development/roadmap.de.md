# Roadmap {#roadmap}

Was ausgeliefert ist, in Arbeit ist und geplant wird, steht auf dem öffentlichen
Projektboard: [DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2).
Prioritäten und Zeitplan können sich ändern; das Board ist ein lebendiger Plan,
kein Versprechen.

- **Ausgelieferte Änderungen** stehen im
  [Changelog](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md) und in
  den Notizen jedes
  [GitHub-Releases](https://github.com/laupernoe/dumbmonit/releases).
- **Ideen und Fehler** gehören in den
  [Issue-Tracker](https://github.com/laupernoe/dumbmonit/issues). Eröffnen Sie ein
  Issue, bevor Sie Code für etwas Größeres schreiben.
- **Sicherheitskorrekturen** werden in den Release-Notes und als
  GitHub-Security-Advisories mit ihrer CVE- oder GHSA-Kennung angekündigt.

DumbMonit befindet sich vor 1.0: Rechnen Sie damit, dass sich API und Datenmodell
weiterentwickeln, bis eine stabile Linie angekündigt wird. Zum Mitmachen siehe den
[Contributing-Leitfaden](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
und die
[Governance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md).

## Richtung für die nächsten zwölf Monate {#direction-for-the-next-twelve-months}

Das sind Absichten, keine Zusagen, in grober Reihenfolge.

- **Auf dem Weg zu 1.0.** HTTP-API und Datenbankschema festziehen, damit eine
  stabile Linie angekündigt werden kann; bis dahin können Minor-Versionen sie noch
  ändern, mit Upgrade-Hinweisen im Changelog.
- **Mehr Menschen mit Zugriff.** Ein zweiter Administrator für Repository,
  Registry, Dokumentations-Hosting und Domains (siehe die
  [Governance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)).
- **Lieferkette.** Reproduzierbare Release-Builds
  ([aktueller Stand](reproducible-builds.md)) und signierte Versions-Tags.
- **arm64-Images.** Server-Images gibt es vorerst nur für amd64; arm64 kommt mit
  nativen Build-Runnern zurück.
- **Integrationen und Packs.** Mehr Geräte- und Dienstintegrationen und
  SNMP-Profile, getragen von Beiträgen und Wünschen.
- **Dokumentation.** Die neun Sprachen der Oberfläche und der Dokumentation im
  Gleichschritt mit der englischen Quelle halten.

## Was DumbMonit nicht vorhat {#what-dumbmonit-does-not-intend-to-do}

- Zu einer Log-, Trace- oder APM-Plattform werden: Es überwacht Geräte und Dienste.
- Eine kostenpflichtige Edition oder einen gehosteten Dienst anbieten: Es bleibt zu
  100 % Open Source.
- Telemetrie über Ihre Instanz irgendwohin senden.
- Flotten von Tausenden Geräten ansteuern: Es ist für Homelabs und kleine Teams
  gebaut, auf einem Container.
