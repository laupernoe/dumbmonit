# Proxmox Mail Gateway

Passerelle mail : files Postfix et l'ancienneté du plus vieux message en
attente, mail compté et filtré aujourd'hui, tailles de quarantaine,
l'ancienneté des bases de signatures antivirus et antispam, services,
certificats et état du cluster.

!!! warning "Testé contre une instance de test seulement"

    Cette intégration a été construite et vérifiée contre un Proxmox Mail
    Gateway 9 installé pour l'occasion, pas contre un en production.
    Chaque appel est une lecture, et les formes viennent d'une vraie
    passerelle mail, mais personne n'a encore fait tourner cela pendant
    des semaines sur une passerelle mail en fonctionnement. Dites-nous ce
    qui casse.

## La page de l'appareil

Au-delà des graphiques génériques, un appareil Proxmox Mail Gateway montre
trois panneaux, lus depuis ce que la sonde a stocké — ouvrir la page
n'interroge jamais la passerelle elle-même :

* **Files de mail** — les quatre files Postfix dans l'ordre où un
  administrateur mail les lit : *incoming* (juste acceptée), *active* (en
  cours de livraison), *deferred* (livraison échouée, sera retentée) et
  *hold* (retenue par une règle). Chacune montre combien de messages elle
  contient, combien de domaines de destination, le plus occupé de ces
  domaines, et depuis combien de temps le plus vieux message attend. Une
  file qui a simplement du mail dedans est un trafic normal ; une file qui
  garde un message depuis plus de quatre heures est signalée *Bloquée*.
* **Mail filtré aujourd'hui** — messages entrants et sortants, octets,
  spam, virus, rebonds, temps de traitement moyen, et ce qui a été rejeté
  avant livraison (greylisting, listes noires, SPF, pregreeting). Une
  barre par tranche de la fenêtre récente dessine la courbe de trafic sans
  interroger la base de séries temporelles. Puis les trois quarantaines
  avec leur nombre de messages et leur usage disque, et les virus attrapés
  aujourd'hui par nom.
* **Passerelle** — les services arrêtés d'abord, puis les membres du
  cluster et leur état de synchro, puis chaque nœud : uptime, CPU, mémoire,
  disque, décalage d'horloge, les bases de signatures avec leur âge et leur
  verdict, les certificats expirant dans les deux semaines, les mises à
  jour en attente et le statut d'abonnement.

Compteurs seulement, jamais de contenu. DumbMonit lit combien de messages
se trouvent dans chaque quarantaine, jamais un sujet, un expéditeur, un
destinataire ou un corps. Les points de terminaison de statistiques par
adresse (`/statistics/sender`, `/statistics/receiver`,
`/statistics/contact`) ne sont pas appelés du tout.

## Aujourd'hui, pas les dernières vingt-quatre heures

`GET /statistics/mail` agrège par **jour local sur la passerelle** : PMG
ignore la fin de la fenêtre que vous demandez, seul `starttime` choisit le
jour. DumbMonit demande donc « maintenant », ce qui donne les totaux
depuis minuit — exactement ce que montre le propre tableau de bord de PMG.
Ces séries sont des jauges qui retombent à zéro à minuit, pas des
compteurs : ne les enveloppez pas dans `rate()`. La courbe récente
(`/statistics/recent`) lit la table brute à la place et roule vraiment.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_pmg_`.

| Famille | Métriques | Labels |
|---|---|---|
| Nœud | `node_cpu_percent`, `node_iowait_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_used_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_clock_offset_seconds` (positif quand la passerelle est en avance sur DumbMonit), `node_insync`, `node_kernel_info`, `node_version_info` (la version d'API de *ce* nœud — dans un cluster, celui laissé en arrière par la dernière mise à jour se montre ici), `version_info` | `node` |
| Files | `queue_messages`, `queue_domains`, `queue_oldest_age_seconds` | `queue` (`incoming`, `active`, `deferred`, `hold`) |
| Mail aujourd'hui | `mail_count_in/out`, `mail_bytes_in/out`, `mail_spam_in/out`, `mail_virus_in/out`, `mail_bounces_in/out`, `mail_junk_in`, `mail_junk_out` (vos propres utilisateurs qui envoient du junk : le premier signe d'un compte compromis), `mail_junk_percent` (absent un jour sans mail entrant), `mail_greylisted`, `mail_spf_rejects`, `mail_rbl_rejects`, `mail_pregreet_rejects`, `mail_avg_processing_seconds` | |
| Débit | `mail_rate_in_per_minute`, `mail_rate_out_per_minute` (depuis la dernière tranche complète de la courbe récente) | |
| Scores de spam | `spam_score_messages` | `level` (`0` à `10` ; `10` agrège tout au-dessus) |
| Virus | `virus_detections` (top dix du jour) | `virus` |
| Quarantaines | `quarantine_messages`, `quarantine_bytes`, `quarantine_avg_spam_level` | `kind` (`spam`, `virus`, `attachment`) |
| Services | `service_running` (1 ou 0 ; aucune série du tout pour une unité qui n'est pas installée) | `node`, `service` |
| Signatures | `signature_age_seconds`, `signature_count`, `signature_update_available` | `node`, `family` (`virus`, `spam`), `database` (`main`, `daily`, `bytecode`, ou le canal SpamAssassin) |
| Certificats | `certificate_expires_in_seconds` | `node`, `certificate` |
| Cluster | `cluster_nodes`, `cluster_node_insync`, `cluster_node_healthy` (tous absents sur une passerelle autonome) | `node`, `role` |
| Abonnement | `subscription_active` (les labels portent le statut exact) | `node`, `status`, `level` |
| Mises à jour | `node_updates_pending`, `node_updates_security_pending` | `node` |
| Collection | `up`, `scrape_errors`, `scrape_duration_seconds` | |

`queue_oldest_age_seconds` est une **borne inférieure**. `qshape` rapporte
des tranches d'âge (moins de 5 min, 5–10 min, 10–20 min, et ainsi de
suite, en doublant), DumbMonit garde donc le plancher de la tranche
occupée la plus haute : un message rapporté à « au moins 20 minutes »
peut avoir 39 minutes. Se tromper par le bas signifie que l'alerte se
déclenche plus tard plutôt que plus tôt, jamais sur un message qui n'est
en fait pas en retard.

`queue_domains` est exact sur une passerelle unique. Sur un cluster, les
files par nœud sont fusionnées et le compteur est pris sur les domaines
les plus occupés de chaque nœud, plafonné à vingt par nœud : au-delà, il
sous-compte. Dédupliquer gagne sur sommer ici, puisque deux nœuds d'un
même cluster reçoivent du mail pour les mêmes domaines.

Une passerelle autonome répond à `/config/cluster/status` avec une liste
vide. C'est une passerelle sans cluster, pas un cluster dégradé : aucune
série n'est publiée et aucune règle ne peut se déclencher.

Seule la base **daily** de ClamAV est jugée sur son âge, par le panneau
comme par la règle. `main` est reconstruite une ou deux fois par an et
`bytecode` à peine plus souvent : les juger sur deux jours afficherait
« obsolète » toute l'année sur une passerelle parfaitement à jour. Elles
montrent quand même leur âge, version et nombre de signatures — sans
verdict. Pareil, un canal SpamAssassin qui n'a jamais été récupéré n'a
aucune date du tout, et « jamais mis à jour » n'est pas « obsolète » :
aucune série, aucun verdict, aucune alerte.

ClamAV écrit sa date de build dans son propre format à l'intérieur de
l'en-tête `.cvd` (`16 Dec 2025 23-18 +0000`, avec un tiret là où tout le
monde met un deux-points) et PMG le transmet tel quel. Si ce format
change un jour, l'âge est simplement absent plutôt que faux de cinquante
ans. La même règle s'applique partout : une valeur que l'API ne renvoie
pas produit une métrique de moins, jamais un zéro qui prétend être une
mesure.

Règles intégrées qui s'appliquent : Appareil inaccessible, file de mail
qui grandit, mail bloqué dans la file, service de passerelle mail arrêté,
signatures antivirus obsolètes, règles antispam obsolètes, quarantaine qui
se remplit, cluster de passerelle mail dégradé, certificat de passerelle
mail arrivant à expiration, mises à jour de passerelle mail en attente.
Les notifications nomment la file, le service, la base ou le nœud
concerné.

## Ce qu'il faut préparer dans Proxmox Mail Gateway

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.
Le principe : un utilisateur réservé à la surveillance, avec un rôle en
lecture seule — jamais le compte avec lequel vous vous connectez.

1. Dans l'interface web : Configuration → User Management → Users → Add.
   Nommez le compte comme suit, donnez-lui un mot de passe long utilisé
   nulle part ailleurs, et cochez Enabled.

    ```
    dumbmonit@pmg
    ```

2. Réglez son Role sur `Audit`. C'est le minimum exact en lecture seule
   pour tout ce que DumbMonit lit : état du nœud, services, files
   postfix, statistiques mail, compteurs de quarantaine, âge des bases
   ClamAV et SpamAssassin, état du cluster, certificats, abonnement et
   mises à jour en attente. `Audit` ne peut rien changer, rien libérer de
   la quarantaine et ne lit aucun message.

3. Vous préférez un shell ? Le même compte en une commande, puis réglez le
   mot de passe.

    ```
    pmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment "DumbMonit monitoring"
    ```

4. Dans DumbMonit, saisissez l'adresse de la passerelle, par exemple
   « mail.lan » ou « mail.lan:8006 », puis le nom d'utilisateur (domaine
   inclus) et le mot de passe de ce compte.

5. Proxmox Mail Gateway n'émet pas de jetons d'API, à la différence de
   Proxmox VE et Proxmox Backup Server : le nom d'utilisateur et le mot
   de passe sont la voie d'entrée. DumbMonit ouvre une session de deux
   heures et la renouvelle, plutôt que de se connecter à chaque mesure.

!!! warning

    Ne réutilisez pas le compte avec lequel vous vous connectez : le rôle Audit ci-dessus ne peut que lire, et ne peut pas libérer un message en quarantaine ni changer une règle. Proxmox Mail Gateway utilise aussi un certificat autosigné par défaut : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options.

Adresse : `10.0.0.40`, `mail.lan`, `mail.lan:8006`, `[fd00::4]` ou une URL
complète. Le schéma `https` et le port 8006 sont ajoutés s'ils manquent.

## Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port de l'API | `8006` | Utilisé si l'adresse ne donne pas de port. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Proxmox Mail Gateway livre un certificat autosigné par défaut : activez ceci si la connexion est refusée pour cette raison. |
| `request_timeout_seconds` | Délai par requête (secondes) | `15` | Temps accordé à chaque appel d'API, de 1 à 120. Lire une file lance une commande Postfix sur la passerelle et peut prendre quelques secondes. |
| `node` | Nœud surveillé | *(vide)* | Nom du nœud de cluster à surveiller. Vide : chaque nœud que la passerelle liste. |
| `recent_hours` | Fenêtre de trafic (heures) | `12` | Jusqu'où remonte la courbe de trafic, de 1 à 24. Les totaux quotidiens couvrent toujours le jour courant. |
| `queues` | Surveiller les files de mail | `true` | Lit les files incoming, active, deferred et hold et l'ancienneté du plus vieux message dans chacune. |
| `quarantine` | Compter les quarantaines | `true` | Lit combien de messages les quarantaines spam et virus contiennent, et combien de disque elles utilisent. Compteurs seulement. |
| `attachment_quarantine` | Compter aussi la quarantaine de pièces jointes | `false` | Cette quarantaine n'a pas d'appel de comptage : elle doit être listée pour être comptée. Désactivé par défaut ; seul le nombre d'entrées est gardé. |
| `signatures` | Surveiller les bases de signatures | `true` | Lit l'âge des bases de virus ClamAV et des canaux de règles SpamAssassin. |
| `services` | Surveiller les services | `true` | Lit l'état de postfix, pmg-smtp-filter, pmgpolicy et les autres unités de chaque nœud. |
| `certificates` | Surveiller les certificats | `true` | Lit les certificats que l'interface web sert et avertit avant leur expiration. |
| `updates` | Compter les mises à jour en attente | `true` | Liste les paquets en attente d'une mise à jour sur chaque nœud. |
| `subscription` | Lire l'abonnement | `true` | Lit le statut d'abonnement de chaque nœud. |

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| ClamAV `main` montre un âge énorme | Attendu : il est reconstruit une ou deux fois par an. Seul `daily` porte un verdict. |
| Erreur de certificat | Certificat autosigné : installez-en un de confiance (ACME est intégré à PMG) ou activez `insecure_tls`. |
| Erreur d'authentification | Mauvais domaine dans le nom d'utilisateur — il doit lire `dumbmonit@pmg`, pas `dumbmonit` — ou le compte n'est pas Enabled. Seul l'échec de `GET /version` condamne toute la sonde ; tout autre appel en échec est compté dans `scrape_errors`. |
| Permissions insuffisantes | Le rôle du compte est en dessous d'`Audit`. `quser` ne voit que son propre mail ; `helpdesk` ne peut pas lire le nœud ni les statistiques. |
| Chaque métrique `mail_*` est à zéro | La passerelle n'a vraiment rien filtré depuis minuit. Les totaux couvrent le jour local courant sur la passerelle et se réinitialisent à son minuit, pas le vôtre. |
| Aucune série `service_running` du tout | L'option `services` est désactivée, ou le rôle ne peut pas les lister. Une seule unité manquant de la liste n'est simplement pas installée sur ce nœud (`chrony` sur une machine utilisant `systemd-timesyncd`, par exemple) et n'est délibérément pas rapportée comme arrêtée. |
| Pas de `signature_age_seconds` pour un canal SpamAssassin | Ce canal n'a jamais été mis à jour, donc PMG ne renvoie aucune date. Lancez `sa-update` sur la passerelle ; un canal secondaire configuré mais jamais récupéré reste sans date volontairement. |
| Aucune base ClamAV du tout | ClamAV n'est pas installé ou `freshclam` n'a jamais terminé un téléchargement. Le point de terminaison répond avec une liste vide, ce qui n'est pas une erreur. |
| Aucune métrique de cluster | La passerelle est autonome. `/config/cluster/status` répond une liste vide, et aucune règle de cluster ne peut se déclencher. |
| « Mail bloqué dans la file » se déclenche tard | `queue_oldest_age_seconds` est le plancher de la tranche d'âge occupée la plus haute de `qshape`, elle sous-rapporte donc volontairement. |
| `quarantine_messages{kind="attachment"}` manquant | L'option `attachment_quarantine` est désactivée : cette quarantaine n'a pas de point de terminaison de comptage, la compter signifie la lister. |
