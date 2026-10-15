# Surveillance de démonstration

Une surveillance fictive produisant des mesures, pour découvrir l'outil sans
matériel. Aucun matériel requis.

## Configuration

1. Donnez-lui n'importe quel nom et confirmez.
2. De fausses mesures apparaissent tout de suite, assez pour parcourir l'interface.

L'adresse est libre (le placeholder indique `demo`), il n'y a ni identifiant ni
option. La surveillance de démonstration produit une série `cpu_usage_percent`, si bien
que les règles « CPU élevé » et « CPU inhabituel » s'appliquent à lui, et il
peut servir de parent ou d'enfant dans une
[chaîne de dépendance](../alerting/index.md#dependency-suppression) pour voir
la suppression à l'œuvre.

Pour de vraies mesures sans matériel dédié, pointez un appareil SNMP sur
n'importe quelle machine faisant tourner `snmpd` : voir
[Tester sans matériel](../install/docker.md#testing-without-hardware).
