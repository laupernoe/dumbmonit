# Surveillances

Tout ce que DumbMonit surveille, vu de quatre façons. Dans l'application, c'est **Surveillances** dans le menu de gauche (`g d`), avec quatre onglets. **Ajouter une surveillance** est le bouton principal de l'onglet Liste (`g n`).

| Onglet | Ce qu'on y voit | À lire |
|---|---|---|
| **Liste** | Chaque surveillance et son état, filtres, dossiers et actions groupées ; un clic ouvre la surveillance | [Page Surveillances](../using/devices.md) |
| **Carte** | Qui dépend de quoi (`g m`), et vos sites sur une carte du monde | [Carte de topologie](../using/topology.md), [Carte du monde](../using/world-map.md) |
| **Journaux** | Une seule recherche dans les journaux du serveur, des agents, de syslog et des traps SNMP (`g l`, administrateurs) | [Journaux](../using/logs.md) |
| **Sécurité** | D'où viennent les attaques que DumbMonit a stoppées (carte des menaces) et les vulnérabilités connues (CVE) des logiciels de vos surveillances (`g x`) ; le score de sécurité s'active dans les Réglages | [CVE et vulnérabilités](../using/cves.md), [Score de sécurité](../using/security-score.md) |

L'explorateur de métriques (`Ctrl+K`, puis *Explore*) n'est pas un onglet : il parcourt les métriques `dumbmonit_*` et essaie une requête ([Explorateur de métriques](../using/dashboards.md)).

## Ajouter des surveillances

- Une par une, depuis **Ajouter une surveillance** : [Ajouter votre première surveillance](../install/first-device.md).
- En scannant un réseau : [Découverte du réseau](../using/discovery.md).
- À partir des labels des conteneurs : [Surveillances par labels Docker](../using/docker-labels.md).
- Depuis un autre outil (Uptime Kuma, Healthchecks, CSV) : [Importer depuis d'autres outils](../using/imports.md).
- Depuis des fichiers gardés dans Git : [Fichiers de configuration (GitOps)](../using/config-files.md).

## Intégrations

Chaque type de surveillance a sa page, regroupée par catégorie dans le menu de gauche : serveurs et agents, virtualisation, stockage et NAS, réseau et pare-feu, matériel, bases de données, serveurs web, identité, cloud et SaaS, maison et médias, applications auto-hébergées, sauvegarde, e-mail, observabilité et vérifications Internet. Le tableau de [toutes les intégrations](../devices/index.md) les liste avec ce dont chacune a besoin.
