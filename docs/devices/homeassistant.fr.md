# Home Assistant

La maison connectée derrière le tableau de bord : si Home Assistant tourne
ou a démarré en mode récupération, entités indisponibles ou inconnues par
domaine, batteries faibles, mises à jour en attente (core, système
d'exploitation, extensions, firmware d'appareil) et les réparations
ouvertes.

DumbMonit lit l'API REST de Home Assistant avec le jeton d'accès longue
durée d'un utilisateur dédié **sans droits d'administrateur**, et la liste
des réparations via l'API WebSocket. Il n'appelle jamais un service, ne
déclenche jamais un événement et ne change jamais un état.

La panne à surveiller n'est pas « Home Assistant ne répond pas » (une
[vérification de site web](services.md#http) le voit déjà) mais
**Home Assistant répond et quelque chose derrière lui a cessé de
fonctionner** : une intégration dont toutes les entités sont devenues
indisponibles après une mise à jour, une clé Zigbee tombée du bus USB, un
capteur de porte dont la batterie a atteint 5 %, une réparation en attente
depuis des semaines.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_homeassistant_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `info` | valeur 1 | `version` |
| `running` | 1 quand l'état du core est `RUNNING`, 0 pendant qu'il démarre ou s'arrête | |
| `recovery_mode` | 1 en mode récupération (ou mode sans échec sur les versions plus anciennes) : la configuration n'a pas pu être chargée | |
| `entities` | entités par domaine | `domain` |
| `entities_unavailable`, `entities_unknown` | entités dans cet état, par domaine | `domain` |
| `entity_unavailable` | valeur 1 par entité indisponible, 50 au plus | `entity`, `name`, `domain` |
| `batteries` | capteurs de batterie et capteurs binaires de batterie | |
| `batteries_low` | batteries sous le seuil, et capteurs binaires de batterie qui sont activés | |
| `battery_low` | une série par batterie faible, valeur = son niveau en pourcentage (0 pour un capteur binaire), 50 au plus | `entity`, `name` |
| `updates_available` | entités `update.*` qui sont activées | |
| `update_available` | valeur 1 par mise à jour en attente, 50 au plus | `entity`, `name`, `installed`, `latest` |
| `repairs` | réparations ouvertes (ni ignorées ni rejetées) par gravité : `critical`, `error`, `warning` | `severity` |
| `repair` | valeur 1 par réparation ouverte, 50 au plus | `issue`, `domain`, `severity` |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Certains domaines restent à l'état `unknown` par nature : un bouton jamais
pressé, un événement jamais reçu, un service de notification ou de voix. Ils
ne sont pas comptés comme inconnus : `button`, `input_button`, `event`,
`scene`, `notify`, `tts`, `stt`, `conversation`, `wake_word`, `ai_task`,
`person`. L'option **Domaines laissés de côté** retire d'autres domaines
(par exemple `device_tracker` ou `media_player` pour des appareils souvent
éteints) des compteurs d'indisponibles et d'inconnus.

Les réparations ne sont pas dans l'API REST : DumbMonit ouvre l'API
WebSocket, envoie `repairs/list_issues` et la ferme. Un utilisateur sans
droits d'administrateur peut les lister. L'option **Lire les réparations**
le désactive.

Les [règles intégrées](../alerting/rules.md#home-assistant) qui
s'appliquent : Home Assistant en mode récupération, Home Assistant ne
tourne pas, entités Home Assistant devenues indisponibles, batterie Home
Assistant faible, mise à jour Home Assistant disponible, réparation Home
Assistant à traiter, plus Surveillance inaccessible.

## Appareils

Chaque téléphone et tablette connecté à l'application compagnon
(l'intégration `mobile_app`) est publié sous la famille générique partagée
par chaque intégration DumbMonit qui suit les appareils clients —
`dumbmonit_client_device_*`, préfixée sans `homeassistant`, documentée en
entier sous [Appareils clients](../alerting/rules.md#client-devices) :

| Métrique | Quoi | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Dernière mise à jour de l'entité `device_tracker` de l'appareil | `device`, `type` (`Mobile`), `os` (vide), `user` (la personne à qui il est lié, s'il y en a), `kind` |
| `stale_seconds` | Positif une fois qu'un appareil a dépassé `device_stale_days` (3 par défaut) sans mise à jour ; alimente la règle [Appareil client périmé](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (toujours `connection` : Home Assistant n'a pas de sauvegarde à rapporter) |

**Comment un appareil est reconnu** : `/api/states` ne dit pas quelle
intégration a créé une entité — cela demande le registre d'entités, que
l'API REST ne sert pas. Le `device_tracker` de l'application compagnon est
le seul qui règle un attribut `battery_level` ; les intégrations de
localisation tierces (Life360, OwnTracks, l'intégration de démonstration
utilisée en test) ne règlent que `battery` au plus. Une intégration tierce
qui aurait copié ce nom d'attribut exact serait prise pour l'application
compagnon ; inversement, un appareil de l'application compagnon avec le
rapport de batterie désactivé manque dans le tableau plutôt que d'être
montré à tort. Le compte de l'appareil, quand il est connu, vient d'une
entité `person.*` dont l'attribut `device_trackers` le nomme.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Home Assistant. Il dit d'abord ce qui ne va pas
(mode récupération, réparations de gravité erreur, entités indisponibles),
puis les batteries faibles par niveau, les mises à jour en attente avec
leurs versions, les entités par domaine, et le tableau des appareils de
l'application compagnon avec leur dernière connexion.

## Comment cela a été validé

Contre Home Assistant 2026.9.4, avec le jeton longue durée d'un utilisateur
sans droits d'administrateur : configuration, états d'entité de
l'intégration de démonstration et de capteurs de template (niveaux de
batterie, un capteur binaire de batterie, un capteur indisponible), entités
de mise à jour et liste des réparations via l'API WebSocket. Les attributs
du `device_tracker` de l'application compagnon (`battery_level`,
`source_type`) ont été vérifiés contre le code source de l'intégration
`mobile_app`, pas contre un vrai téléphone.

## Créer un utilisateur dédié et un jeton longue durée dans Home Assistant

1. Dans Home Assistant : Settings → People → Users → Add user (activez le mode Avancé dans votre profil si l'onglet Users manque). Nommez l'utilisateur comme suit, donnez-lui un mot de passe long et aléatoire et laissez l'interrupteur Administrator désactivé. Un utilisateur ordinaire peut lire chaque entité et la liste des réparations, et ne peut pas changer la configuration.

    ```
    dumbmonit
    ```

2. Connectez-vous une fois à Home Assistant comme cet utilisateur, ouvrez son profil (le nom en bas de la barre latérale) → Security → Long-lived access tokens → Create token. Nommez-le d'après DumbMonit et copiez-le maintenant : il n'est montré qu'une fois.

3. Dans DumbMonit, saisissez l'adresse de Home Assistant, par exemple « homeassistant.local » (port 8123) ou « https://ha.example.net » derrière un reverse proxy, et collez le jeton. Les batteries sous 20 % comptent comme faibles ; l'option Seuil de batterie faible change ça.

4. DumbMonit ne lit que /api/config, /api/states et, via l'API WebSocket, la liste des réparations. Il n'appelle jamais un service, ne déclenche jamais un événement et ne change jamais un état.

!!! warning
    Un jeton longue durée porte tous les droits de son utilisateur et reste valide dix ans : créez-le pour l'utilisateur dédié, jamais pour le vôtre. Home Assistant sert du HTTP simple sauf si TLS est configuré, et le jeton voyage avec chaque requête : sur un réseau non sûr, utilisez HTTPS. Chaque téléphone et tablette connecté à l'application compagnon (l'intégration mobile_app) apparaît dans le tableau Appareils ci-dessous avec sa dernière activité ; c'est optionnel et silencieux sans cette application.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'accès longue durée | Un jeton de l'utilisateur dédié, envoyé comme `Authorization: Bearer`. |

Adresse : un nom d'hôte ou une IP (`homeassistant.local`), `host:port`, ou
une URL complète (`https://ha.example.net`, avec un préfixe de chemin
derrière un reverse proxy) ; un `/api` final est accepté. Options :
protocole (HTTP par défaut), port (8123), vérification du certificat, délai
de requête (10 s), seuil de batterie faible (20 %), domaines laissés de
côté (aucun), réparations (lues) et péremption d'appareil
(`device_stale_days`, 3 jours par défaut).
