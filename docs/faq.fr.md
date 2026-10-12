# FAQ

## J'ai perdu le mot de passe {#i-lost-the-password}

Demandez d'abord à un autre administrateur : dans Réglages → **Utilisateurs**,
**Modifier** sur votre compte définit un nouveau mot de passe et le montre
une fois, et **Réinitialiser 2FA** sur la même ligne efface un second
facteur qui vous bloque. C'est la réponse du quotidien — une instance avec
deux comptes administrateurs n'a jamais besoin du reste de cette page.

Quand plus personne ne peut se connecter, il n'y a pas d'e-mail de
récupération : démarrez le serveur une fois avec
`DUMBMONIT_RESET_PASSWORD=1` :

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
```

Chaque compte et chaque session est supprimé au démarrage et l'interface
montre `/setup` à nouveau. Créez le premier administrateur, puis démarrez à
nouveau sans la variable (`docker compose up -d dumbmonit`, avec la
variable non définie ou vide dans votre environnement). Appareils, règles,
canaux et les réglages de connexion unique restent intacts.

## Aucune donnée après l'ajout d'un appareil {#no-data-after-adding-a-device}

Laissez une minute : la première sonde tourne à l'intervalle de l'appareil
(60 s par défaut) et les écritures atteignent VictoriaMetrics toutes les 5
secondes (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Ensuite :

1. Ouvrez la page de l'appareil et cliquez sur **Sonder maintenant**. Elle
   rapporte combien d'échantillons et de séries une sonde a produits, ou
   l'erreur exacte.
2. *Mal configuré* avec une erreur sous l'adresse est un problème de
   configuration : mauvaise communauté ou mot de passe, certificat refusé
   (activez « Accepter un certificat non vérifiable »), fonctionnalité
   manquante. C'est affiché, pas notifié.
3. *Inaccessible* sans erreur signifie que l'appareil n'a pas répondu du
   tout. Pour SNMP, une mauvaise communauté ressemble exactement à un
   appareil éteint, parce que l'appareil ne y répond jamais. Vérifiez la
   communauté, que SNMP est activé, et que l'appareil autorise l'adresse du
   serveur.
4. Vérifiez que le conteneur peut atteindre l'appareil : les réseaux Docker
   et les pare-feu sont les coupables habituels.

## Ping dit « erreur de configuration »

Le moniteur ping a besoin d'une socket d'écho ICMP. Le conteneur tourne sans
aucune capacité, le fichier Compose autorise donc ces sockets avec le
sysctl `net.ipv4.ping_group_range` sous le service `dumbmonit` : vérifiez
que les lignes `sysctls:` sont toujours là, puis `docker compose up -d`.
Voir [Ping ICMP sans capacité](install/docker.md#icmp-ping-without-a-capability).

## L'agent n'apparaît pas

- L'installateur fait un push de test avant de démarrer le service et
  s'arrête avec une erreur explicite si l'URL ou le jeton est incorrect.
  Corrigez `/etc/dumbmonit/agent.yaml` et redémarrez `dumbmonit-agent`.
- L'URL dans la commande d'installation est celle que votre navigateur a
  utilisée pour atteindre l'interface. Depuis la machine surveillée, cette
  URL doit atteindre le serveur : `curl http://server:8080/api/health`
  depuis là vous le dit.
- Derrière un reverse proxy, `/api/ingest` doit être transféré et les corps
  jusqu'à 16 Mo autorisés (`client_max_body_size 16m` dans nginx).
- `journalctl -u dumbmonit-agent -f` montre chaque push rejeté. Le jeton
  lui-même n'apparaît jamais dans le journal.
- Un jeton révoqué arrête l'agent à son prochain push ; relancez
  l'installateur avec un nouveau jeton.

Voir [Agent Linux, macOS, FreeBSD et Windows](devices/agent.md).

## Changer le port

Le conteneur écoute sur 8080 ; le port de l'hôte est celui que
`docker-compose.yml` publie. Réglez `DUMBMONIT_PORT` (dans un fichier
`.env` à côté du fichier Compose, ou en ligne de commande) :

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Pour changer le port *à l'intérieur* du conteneur, réglez `DUMBMONIT_BIND`,
par exemple `0.0.0.0:9000`, et ajustez le mappage de port.

## Pourquoi l'alerte dit « Avis » quand la règle dit « warning » ?

L'API stocke trois gravités : `info`, `warning`, `critical`. L'interface les
affiche avec les mots du bulletin météo : Info, Avis, Avertissement. Voir
[Gravités](alerting/index.md#severities).

## Une alerte est « Supprimée par le parent »

L'appareil a un appareil parent qui est inaccessible, ses propres alertes
sont donc retenues plutôt qu'envoyées. Corrigez le parent ; les alertes de
l'enfant se résolvent ou se déclenchent ensuite d'elles-mêmes sans une
seconde notification. Voir
[Suppression par dépendance](alerting/index.md#dependency-suppression).

## « CPU inhabituel » ne se déclenche jamais

C'est une référence saisonnière qui reste silencieuse pendant ses 14
premiers jours sur chaque série. Pendant ce temps elle montre seulement ce
qu'elle aurait déclenché. Voir
[Apprentissage de référence](alerting/index.md#baseline-learning).

## Puis-je utiliser mon propre VictoriaMetrics ?

Oui : réglez `DUMBMONIT_VM_URL` sur son adresse (`http://host:8428`) et le
VictoriaMetrics embarqué n'est pas démarré. Chaque métrique est préfixée
`dumbmonit_`, il peut donc partager l'instance avec d'autres outils.
`GET /api/health` rapporte `victoria.embedded: false` une fois le
changement fait.

## DumbMonit a-t-il besoin d'être accessible depuis l'internet ?

Non. Il contacte les appareils et les services de notification ; rien ne
rappelle, sauf les agents, qui ont besoin d'atteindre l'URL du serveur.
Gardez-le sur votre LAN ou derrière un VPN ; si vous l'exposez, utilisez un
reverse proxy avec TLS et réglez `DUMBMONIT_COOKIE_SECURE=1`.

## Y a-t-il un jeton d'API ?

Oui. Réglages → **API & assistants** crée des jetons `dmt_…` avec une
portée `read` ou `write` ; envoyez-en un comme
`Authorization: Bearer dmt_…` sur n'importe quelle route et aucun cookie ni
en-tête CSRF n'est nécessaire. Le même jeton connecte un
[assistant](using/assistant.md) via MCP. Les jetons ne gèrent jamais les
comptes, les réglages de connexion ni d'autres jetons. Voir la
[référence de l'API HTTP](reference/api.md#authentication).
