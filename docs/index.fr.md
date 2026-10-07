# DumbMonit {#dumbmonit}

Une supervision simple, du homelab à la petite entreprise. Un conteneur, une adresse IP
à saisir, des graphiques et des alertes utiles en moins d'une minute.

!!! warning "Travaux en cours"
    DumbMonit est en développement actif ; l'image actuelle
    (`ghcr.io/laupernoe/dumbmonit:latest`) est une alpha destinée aux premiers
    testeurs. Attendez-vous à des aspérités et à des changements incompatibles
    jusqu'à la première version.

DumbMonit lit le réseau comme un **bulletin météo** : la page d'accueil énonce
le ciel en une phrase (« Ciel dégagé. » ou « 2 avis, 1 injoignable. ») et
liste ce qui a besoin de vous, avant toute autre chose. Les niveaux de gravité suivent
l'échelle météorologique (info → avis → alerte), les prédictions sont des prévisions, et les fenêtres
de maintenance sont planifiées.

![La page de vue d'ensemble : la phrase du bulletin, la liste « Needs you » et les prévisions](assets/screenshots/overview-light.png){ loading=lazy }

<div class="dm-links" markdown>
<a href="install/docker/">Installer<small>Docker Compose, premier démarrage, sauvegardes</small></a>
<a href="install/first-device/">Ajouter votre premier appareil<small>SNMP, scan réseau, la suite</small></a>
<a href="alerting/">Alertes<small>Règles intégrées, silencieuses par construction</small></a>
</div>

## Ce qu'il surveille {#what-it-watches}

| Source | Ce que vous obtenez |
|---|---|
| [SNMP v1 / v2c / v3](devices/snmp.md) | Commutateurs, routeurs, NAS, onduleurs, imprimantes. Cinq profils sont fournis avec le produit et appliqués automatiquement d'après le `sysObjectID` de l'appareil. Un scan réseau ajoute d'un coup tout ce qui répond. |
| [Proxmox VE](devices/proxmox.md) | Nœuds, machines virtuelles et conteneurs, stockages, quorum du cluster, et l'ancienneté de la dernière sauvegarde réussie de chaque machine. |
| [Proxmox Backup Server](devices/pbs.md) | Occupation des datastores et prévision de saturation, déduplication, ancienneté et vérification du dernier snapshot de chaque machine, tâches en échec, ramasse-miettes. |
| [Synology DSM](devices/synology.md) | Volumes, disques et leur état SMART, température, charge, via l'API web du NAS. |
| [Agent Linux, macOS, FreeBSD et Windows](devices/agent.md) | CPU, mémoire, disques, réseau, services, conteneurs et uptime des machines qui ne parlent pas SNMP, plus les températures, l'état des disques (SMART) et les pools ZFS lorsque la machine les expose. Une seule commande pour l'installer, ou une image Docker ; en [mode relais](install/remote-site.md), il sonde les appareils d'un site distant uniquement par des connexions sortantes. |
| [Services](devices/services.md) | HTTP(S), port TCP, DNS, ping et expiration des certificats TLS, façon Uptime Kuma, avec une barre d'historique et un pourcentage de disponibilité. |
| [Packs d'intégration](packs/index.md) | Un type d'appareil déclaré dans un seul fichier YAML — une API HTTP ou une page Prometheus `/metrics` de l'appareil — avec ses propres règles d'alerte, installé sans nouvelle version du serveur. |

## Lui parler {#talk-to-it}

- Un [assistant via MCP](using/assistant.md) — Claude Code, Claude Desktop,
  ChatGPT, VS Code, Cursor — avec 27 outils : un jeton `read` ne fait que regarder (état,
  appareils, alertes, métriques, agents, conteneurs…), un jeton `write` peut aussi agir
  (mettre en sourdine, acquitter, ajouter un appareil, redémarrer un conteneur, publier un
  incident sur une page d'état). Les secrets ne sont jamais renvoyés.
- L'[API HTTP](reference/api.md) derrière tout ce que fait l'interface, décrite à
  `/api/openapi.json` (OpenAPI 3.1). Les jetons (`dmt_…`) sont limités en lecture ou en
  écriture, peuvent expirer, être restreints à une liste de réseaux, et sont
  soumis à une limite de débit.

## Ce qui tourne {#what-runs}

| Conteneur | Rôle | Empreinte |
|---|---|---|
| `dumbmonit` | Collecte, API, alertes, interface web, et VictoriaMetrics embarqué pour les séries temporelles | ~40 Mo de RAM + le budget de VictoriaMetrics (256 Mo par défaut) |

Un conteneur, un volume : le serveur démarre VictoriaMetrics depuis la même
image, et la configuration et l'état vivent dans une base SQLite embarquée. Il
n'y a pas de conteneur de base de données. Un VictoriaMetrics externe peut être utilisé à la place
(`DUMBMONIT_VM_URL`).

!!! note "À propos du nom"
    DumbMonit s'appelait EzyMonit jusqu'en septembre 2026. Les commandes, variables
    d'environnement, noms d'image et chemins ont été renommés avec lui ; les anciennes
    variables `EZYMONIT_*` et les jetons d'agent `ezym_` sont toujours acceptés. Voir
    [Mise à jour](install/docker.md#upgrading).

DumbMonit est 100 % open source sous licence Apache 2.0, dépendances
comprises : aucune fonctionnalité n'est réservée à une édition payante.
