# Journaux : agents, syslog et traps SNMP

DumbMonit peut garder les lignes de journal de vos machines et équipements à
côté de leurs métriques, les chercher et alerter sur des motifs. Tout arrive
dans le VictoriaLogs embarqué et se cherche sur la page **Journaux**, réservée aux
administrateurs, avec [LogsQL](https://docs.victoriametrics.com/victorialogs/logsql/).

!!! note "Activez d'abord le stockage des journaux"
    **Réglages → Système → Stockage des journaux**. Tant qu'il est éteint, les
    lignes reçues sont perdues (et comptées), et la page Journaux le dit.

## Ce qui peut l'alimenter

| Source | Flux (`service`) | Comment |
|---|---|---|
| DumbMonit lui-même | `dumbmonit` | Toujours, dès que le stockage est actif. |
| Agents | `agent` | Par machine, depuis sa page d'équipement. |
| Syslog | `syslog` | Le serveur intégré (UDP et TCP). |
| Traps SNMP | `trap` | Le récepteur intégré (v1, v2c). |

Chaque ligne porte `level` (`error`, `warn`, `info`, `debug`), `host`, `origin`
(unité journald, fichier, canal du Journal d'événements ou adresse de
l'émetteur) et, quand l'émetteur est l'un de vos équipements, `target` (son
identifiant) et `target_name`.

## Collecte de journaux par l'agent

Ouvrez un équipement agent et utilisez **Collecte de journaux**. Elle est
éteinte tant que vous ne l'activez pas ; l'agent applique le réglage à son
prochain envoi, sans redémarrage.

- **Journal systemd** (Linux) : les nouvelles entrées à partir de maintenant,
  filtrées par priorité.
- **Fichiers suivis** : chemins absolus avec `*` et `?` dans chaque composant,
  par exemple `/var/log/nginx/*.log`. La rotation et la troncature sont suivies ;
  les lignes déjà présentes au démarrage de la collecte ne sont pas rejouées. Le
  niveau est deviné d'après les mots `error`, `warn`, `debug`.
- **Canaux du Journal d'événements Windows** : `System`, `Application`… lus par
  PowerShell toutes les 15 secondes.
- **Niveau minimal** : les lignes en dessous ne sont jamais envoyées.

Les lignes passent par le canal habituel de l'agent (même serveur, même port,
même jeton ou certificat), par lots. Si le serveur est injoignable, l'agent
garde jusqu'à 8 Mio sur disque (à côté de son secret de liaison) et les envoie
au retour ; au-delà, les plus anciennes sont abandonnées. Posez
`DUMBMONIT_AGENT_LOGS=off` dans l'environnement de l'agent pour interdire la
collecte sur une machine, quoi que demande le serveur.

!!! warning
    Les chemins viennent du serveur et l'agent tourne le plus souvent en root.
    Seuls les administrateurs peuvent les régler, mais traitez ce réglage comme
    le canal de commandes de l'agent.

## Serveur syslog

Désactivé par défaut. Posez `DUMBMONIT_SYSLOG_ENABLED=true`, publiez le port
dans votre fichier Compose (`5514/udp` et `5514/tcp`, non privilégiés car le
conteneur ne tourne pas en root) et pointez vos équipements dessus. Les RFC 3164
et 5424 sont comprises, avec un cadrage TCP par lignes ou par comptage
d'octets. L'émetteur est rattaché à l'équipement dont l'adresse est l'IP source
(ou, à défaut, dont le nom est le nom d'hôte du message). Le port 514 peut être
redirigé vers 5514 sur l'hôte ou le routeur. Variables : voir
[Configuration](../reference/configuration.md#journaux-centralises-serveur-syslog-et-traps-snmp).

## Traps SNMP

Désactivés par défaut (`DUMBMONIT_TRAPS_ENABLED=true`, port `1162/udp`). Les
traps SNMP v1 et v2c et les informs v2c (acquittés) sont reçus ; **la v3 n'est
pas prise en charge** et ces messages sont comptés comme refusés. Posez
`DUMBMONIT_TRAPS_COMMUNITIES` pour n'accepter que vos communautés. Chaque trap
devient une ligne nommée d'après la trap (`linkDown`, `coldStart`… ou l'OID)
suivie de ses variables, nommées d'après les profils SNMP intégrés quand ils
connaissent l'OID.

## Recherche

La page **Journaux** ajoute une **Source** (serveur, agents, syslog, traps SNMP,
toutes) et un filtre **Équipement** à la période, au niveau minimal et à la
requête LogsQL. La page d'un équipement a un bouton **Journaux** qui ouvre la
page déjà filtrée. Le bloc **Récepteurs** montre ce qui écoute et ce qui est
arrivé.

## Alerter sur un motif

Ouvrez **Alerter sur cette recherche** sous le formulaire : il crée une règle
*motif de journal* à partir de la source, du niveau et de la requête courants.
La règle compte les lignes qui correspondent sur une fenêtre glissante (1 minute
à 24 heures) et se déclenche quand le compte atteint votre seuil, **par
équipement** (une alerte par `target`) ou globalement. C'est une règle d'alerte
ordinaire : sévérité, durée de maintien, canaux, silences, suppression par
dépendance, historique et sauvegardes s'appliquent, et elle se modifie sur la
page **Alertes**.

Exemples (la partie requête) :

- `service:=syslog "Failed password"` : connexions échouées remontées par syslog ;
- `service:=trap level:in("error", "warn")` : traps de lien coupé et
  d'authentification, c'est ainsi qu'une trap devient une alerte ;
- `service:=agent origin:"file:/var/log/nginx/error.log"` : erreurs nginx.

Les règles n'acceptent que des filtres (pas de `|`). Zéro ligne signifie « pas
de problème » : utilisez `>=` avec un seuil d'au moins 1.
