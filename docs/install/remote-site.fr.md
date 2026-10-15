# Surveiller un site distant

Un seul DumbMonit, plusieurs réseaux. Un **agent relais** installé sur le
site distant fait tourner, pour le compte du serveur, les sondes des
surveillances que vous lui assignez, et rapporte les résultats via la même
connexion sortante qu'il utilise déjà pour envoyer ses propres métriques.
Rien à ouvrir sur le site distant, pas de VPN, pas de deuxième instance à
surveiller.

```
 siège                                     site distant (NAT, pare-feu)
 ┌──────────────────┐   HTTPS, sortant    ┌──────────────────────────────┐
 │ serveur DumbMonit│◀────────────────────│ agent relais                 │
 │  planificateur   │  sondes à lancer ─▶ │  fait tourner SNMP/Proxmox/HTTP…│──▶ commutateur, NAS,
 │  alertes, UI     │◀── échantillons,    │  envoie aussi ses propres      │    hyperviseur…
 │                  │    verdict          │  métriques                    │
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Publier le serveur sur HTTPS

Le relais récupère ses tâches depuis le serveur : le serveur doit être
accessible depuis le site distant, typiquement
`https://monitor.example.org` derrière un reverse proxy. Deux choses
voyagent sur ce lien :

- le **jeton d'enrôlement** (`dmon_…`), dans l'en-tête `Authorization` de
  chaque requête — comme pour tout agent ;
- les **identifiants de surveillance** des surveillances que le relais sonde
  (communauté SNMP, jeton d'API Proxmox…), déchiffrés, à l'intérieur de
  chaque tâche de sonde.

En HTTP simple à travers l'internet, les deux seraient lisibles en chemin :
mettez le serveur derrière HTTPS avant de relayer quoi que ce soit en
dehors de votre propre réseau. Sur un LAN ou un VPN, HTTP est acceptable et
le jeton est le seul secret en vol jusqu'à ce que vous assigniez des
surveillances au relais. Les identifiants ne sont jamais écrits sur disque d'un
côté ou de l'autre : une tâche de sonde vit dans la mémoire du serveur
jusqu'à ce que le relais la rapporte, et l'agent ne garde rien entre deux
sondes.

Le reverse proxy doit transférer `/api/agent/relay` comme `/api/ingest`,
autoriser des corps de requête jusqu'à 16 Mo (un hyperviseur chargé rapporte
des milliers d'échantillons), et garder les connexions inactives ouvertes
pendant au moins 60 secondes : le serveur garde une requête de relais
jusqu'à 25 secondes en attendant du travail (long polling).

## 2. Créer un jeton d'enrôlement

**Réglages → Agents → Nouveau jeton**, ou ajoutez une surveillance de type
*agent*. Copiez la valeur `dmon_…` : elle n'est montrée qu'une fois.

## 3. Démarrer le relais sur le site distant

N'importe quelle machine sur le site distant avec Docker, en utilisant
`docker-compose.agent.yml` du dépôt :

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

Ou l'agent ordinaire installé avec l'installateur en une ligne, avec deux
clés de plus dans `/etc/dumbmonit/agent.yaml` :

```yaml
relay: true
site: Lyon office
```

La machine apparaît dans DumbMonit en quelques secondes, comme une surveillance
agent ordinaire, et sa page dit *Relais · site Lyon office · relaie 0
surveillances*. Le journal de l'agent dit `relay mode enabled: waiting for
probes from the server` une fois son premier lot accepté.

!!! note "Ce que le relais doit atteindre"
    Le relais sonde les surveillances depuis son propre réseau : il a besoin du
    même accès que le serveur aurait besoin s'il était là — UDP 161 pour
    SNMP, 8006 pour Proxmox, 5001 pour DSM, et ainsi de suite. Pour les
    surveillances **ping**, le conteneur a besoin du sysctl `net.ipv4.ping_group_range`, déjà défini
    dans le fichier compose (voir [Ping ICMP sans capacité](docker.md#icmp-ping-without-a-capability)). Pour des surveillances sur un VLAN ou un réseau
    link-local, ou pour des cibles `localhost`, utilisez
    `network_mode: host`.

## 4. Assigner des surveillances au relais

Ajoutez les surveillances distantes comme d'habitude — commutateur SNMP, nœud
Proxmox, vérification HTTP — avec leur adresse **telle que vue depuis le
site distant** (`192.168.10.1`, `https://pve.lan:8006`). Dans le formulaire
de surveillance, sous *Plus d'options*, réglez **Atteint via** sur l'agent relais
plutôt que *Direct*. Les surveillances existantes peuvent être basculées de la
même façon ; repasser à *Direct* fait que le serveur les sonde à nouveau.

À partir de là :

- le serveur mettra en file une sonde pour la surveillance à chaque intervalle ;
  le relais la récupère en quelques millisecondes (il attend du travail
  sur le serveur), lance le collecteur et rapporte ;
- **Sonder maintenant** et la détection de profil sur la page de la surveillance
  passent aussi par le relais ;
- la *dernière sonde* et l'erreur de la surveillance suivent le verdict du
  relais. Quand le relais ne récupère pas une sonde à temps, la surveillance
  montre *Délai dépassé : l'agent relais … n'a pas récupéré la sonde* —
  vérifiez que l'agent tourne avec `relay: true` ;
- la page de l'agent montre *Relais pour N surveillances*, et
  `GET /api/relays` liste les relais avec leur site et leur compteur.

## Alerter quand le relais est en panne

Un relais qui devient silencieux emporte ses surveillances avec lui : leurs
sondes s'arrêtent, et ils deviendraient tous *inaccessibles* d'un coup. Le
relais est traité comme un **parent** de chaque surveillance qu'il sonde (en
plus du parent que vous avez pu régler) : quand le relais lui-même est
*inaccessible*, les alertes de ses surveillances sont montrées comme
`suppressed` avec `suppressed_by` = le relais, et une seule notification
sort — pour le relais. Le parent que vous avez réglé explicitement garde
son rôle et gagne quand les deux sont en panne.

## Limites

- Un relais par surveillance. Une surveillance sondée par le relais n'est pas aussi
  sondé par le serveur.
- Les surveillances de type agent ne peuvent pas être relayées : elles envoient
  leurs propres métriques.
- Le relais fait tourner 8 sondes à la fois au plus ; une sonde est bornée
  par le délai de sonde du serveur (`DUMBMONIT_PROBE_TIMEOUT`), plafonné à
  120 s sur l'agent.
- Un agent relais a besoin des collecteurs compilés : les agents plus
  anciens que cette fonctionnalité ignorent `relay: true`, et le serveur
  rapporte leurs sondes comme en délai dépassé. Réinstallez avec le binaire
  ou l'image actuels.
