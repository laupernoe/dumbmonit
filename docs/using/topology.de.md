# Topologiekarte

Die Seite **Karte** (Schaltfläche **Karte** auf der Monitorseite, *Karte* in der
Befehlspalette, `/map`) zeichnet Ihre gesamte Infrastruktur in einem Bild. Nichts
wird von Hand eingegeben: Jede Verbindung stammt aus dem, was DumbMonit bereits weiß.

## Woher die Verbindungen kommen

| Verbindung | Quelle | Darstellung |
|---|---|---|
| Hängt ab von | Der an einem Monitor gesetzte **übergeordnete Monitor**, dem auch die Alarmunterdrückung folgt. | durchgezogen |
| Über ein Relay erreicht | Der **Relay-Agent**, der einen Monitor an einem entfernten Standort abfragt. | lange Striche |
| Auf einem Hypervisor | Die Gäste, die Proxmox VE, XCP-ng, Nutanix, vSphere oder das libvirt-Modul eines Linux-Agenten in den letzten 30 Minuten gemeldet haben. | kurze Striche |
| Verkabelt mit (LLDP) | Ein **LLDP-Nachbar**, den ein SNMP-Switch an einem Port sieht (Profil `ethernet-switch`). | Punkte |

Ein Gast oder LLDP-Nachbar wird nur dann mit einem Monitor verbunden, wenn sein
Name genau einen Monitor bezeichnet (gleicher Name oder gleicher Hostname in der
Adresse, Domain ignoriert). Ein mehrdeutiger Name erzeugt keine Verbindung statt
einer falschen. Ein Gast ohne passenden Monitor erscheint als eigenes Kästchen
neben seinem Hypervisor; ein unbekannter LLDP-Nachbar wird weggelassen. Ein
Monitor ohne bekannte Verbindung steht einfach in seinem Ordner.

## Die Karte lesen

- Vorgelagertes links, Abhängiges rechts; ein Abschnitt pro Ordner.
- Jedes Kästchen zeigt Typsymbol, Namen und Zustand in Worten. Ein
  **ausgefallener** Monitor hat einen dicken Rahmen und ein `!`; was dahinter liegt
  (über übergeordneten Monitor, Relay oder Hypervisor), ist als **Betroffen** mit
  gestricheltem Rahmen markiert: seine Alarme werden unterdrückt.
- Ein Klick öffnet den Monitor; das Kästchen eines Gasts öffnet seinen Hypervisor.
- Ziehen zum Verschieben, Mausrad oder Zwei-Finger-Geste zum Zoomen, Pfeiltasten,
  `+`, `-` und `0` (einpassen).
- **Anzeigen** beschränkt die Karte auf einen Ordner oder einen Relay-Standort.
- Auf dem Telefon öffnet die Seite die **Listen**ansicht: derselbe Baum, eingerückt.

## Exportieren und drucken

**Exportieren** bietet **SVG-Bild**, **PNG-Bild** (Titel, Umfang, Datum und
Legende auf weißem Grund) sowie **Drucken / PDF (A4 quer)** und **(A3 quer)**:
Der Browser druckt nur dieses Blatt, auf die Seite skaliert.

## API

`GET /api/topology` liefert Knoten, Verbindungen und deren Quelle (alle Rollen,
nur lesend, ohne Adressen). Siehe die [API-Referenz](../reference/api.md).
