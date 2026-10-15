# Réglages

![Réglages : sept onglets, ici Mon compte avec apparence, compte et sécurité et notifications push](../assets/screenshots/settings-light.png){ loading=lazy }

Réglages contient ce qui est administratif : votre compte, qui peut se connecter et comment, les jetons qui laissent entrer agents et assistants, la sauvegarde de l'instance, le thème, et la santé du serveur. Il est découpé en sept onglets, chacun avec sa propre adresse. Un rail sur grand écran, une bande de puces sur téléphone et une recherche sur les noms de sections aident à trouver une section.

| Onglet | Adresse | Contenu |
|---|---|---|
| Mon compte | `/settings/account` | Apparence, Compte et sécurité (mot de passe, double authentification, journal de sécurité), Notifications push |
| Équipe et accès | `/settings/team` | Utilisateurs, Authentification unique |
| Supervision | `/settings/monitoring` | Agents, Packs d'intégration, [Score de sécurité](security-score.md) |
| Notifications et rapports | `/settings/notifications` | Canaux, Politique de notification, Heures de silence ([Canaux de notification](../notifications.md)), Rapports |
| Pages de statut | `/settings/status` | Pages d'état et annonces ([Pages d'état](status-pages.md)) |
| Mur | `/settings/wall` | Mode mural, Musique murale, Actualités murales ([Mode mural](overview.md#wall-mode)) |
| Système | `/settings/system` | Sauvegarde, DumbMonit lui-même, API et assistants, À propos |

Chaque section garde son ancre : `/settings#agents` redirige vers `/settings/monitoring#agents`. Les anciens liens (`/alerts#notifications`, `/status`) redirigent aussi vers le nouvel emplacement.

## Apparence

Trois tuiles, chacune prévisualisant son propre thème : **Système** (suit
votre appareil et change avec lui), **Jour** (le thème « papier
graphique ») et **Nuit** (le thème « radar »). Le choix est stocké dans
le navigateur. L'interrupteur de l'en-tête et le *Basculer le thème* de
la palette de commandes alternent entre jour et nuit.

**Ouvrir le mode mur** se trouve en bas de la section : le bulletin
seul, plein écran, pour un moniteur dans la pièce. ++esc++ le quitte, et
la palette de commandes l'ouvre depuis n'importe où — voir
[Mode mur](overview.md#wall-mode).

## Compte et sécurité

C'est votre propre compte. Le premier est créé au premier démarrage, et
d'autres peuvent être ajoutés ci-dessous. Ici vous changez votre mot de
passe : mot de passe actuel, nouveau mot de passe (12 caractères au
moins ; toute une phrase est plus sûre qu'un mot compliqué),
confirmation. Le changer déconnecte chaque autre session. Les comptes qui
se connectent via le fournisseur d'identité n'ont pas de mot de passe
ici. **Se déconnecter** termine cette session.

La connexion est limitée en débit par adresse client et par compte :
après cinq tentatives échouées, chaque tentative suivante attend plus
longtemps (30 s, en doublant, jusqu'à 5 minutes). Derrière un reverse
proxy, réglez `DUMBMONIT_TRUSTED_PROXIES` pour que le limiteur voie la
vraie adresse client plutôt que celle du proxy (voir
[Configuration](../reference/configuration.md)). Les sessions durent 30
jours. Un mot de passe oublié se réinitialise avec
`DUMBMONIT_RESET_PASSWORD=1` : voir la
[FAQ](../faq.md#i-lost-the-password).

### Double authentification

Les comptes qui se connectent avec un mot de passe peuvent ajouter un
second facteur : un code à six chiffres basé sur le temps (TOTP) depuis
une application d'authentification — Aegis, FreeOTP, Google
Authenticator, 1Password, Bitwarden… **Configurer** demande votre mot de
passe, montre un QR code à scanner (ou la clé à saisir), et active le
second facteur une fois que vous saisissez un premier code depuis
l'application. Huit **codes de récupération** sont ensuite montrés une
fois : sauvegardez-les, chacun vous connecte une seule fois si le
téléphone est perdu. L'écran de connexion demande le code (ou un code de
récupération) après le mot de passe ; cinq codes incorrects annulent la
tentative et vous repartez du mot de passe, et la limite de débit de
connexion habituelle continue de compter à travers les tentatives. Un
code est accepté une fois : celui que vous venez d'utiliser (y compris
celui qui a activé le second facteur) ne vous connecte pas à nouveau —
attendez le suivant.

**Désactiver** redemande le mot de passe. Les codes de récupération ne
peuvent pas être régénérés seuls : désactivez et reconfigurez pour en
obtenir un nouveau jeu. Un administrateur peut réinitialiser le second
facteur d'un autre utilisateur depuis **Utilisateurs** (*Réinitialiser
2FA*) quand le téléphone et les codes sont tous les deux perdus — ça
déconnecte l'utilisateur partout et laisse le mot de passe intact. Le
secret est stocké chiffré avec le secret de l'instance ; les codes sont
hashés. Les comptes à connexion unique obtiennent leur second facteur du
fournisseur d'identité, pas ici.

Les administrateurs trouvent aussi le **journal de sécurité** dans cette
section : connexions et échecs, changements de mot de passe et de double
authentification, changements de jeton et de compte, avec l'adresse
client. Il garde les 5 000 dernières entrées.

## Notifications push

Notifications natives sur ce téléphone ou cet ordinateur, même avec
DumbMonit fermé : **Activer sur cet appareil**, puis **Envoyer un test**.
Chaque compte gère ses propres appareils ici (les lecteurs aussi) ;
quelles alertes les atteignent est réglé par un canal **Web Push** dans
Réglages → Notifications et rapports. Le bouton **i** résume les prérequis : HTTPS (ou
localhost), installation sur l'écran d'accueil sur iPhone et iPad, et les
domaines de service de push que le serveur doit atteindre. Guide
complet : [Notifications push (PWA)](push-notifications.md).

## Utilisateurs

Administrateurs seulement. Les comptes qui peuvent se connecter, et leur
rôle :

| Rôle | Lit tout | Traite les alertes (acquitter, repousser, ignorer, fenêtres de maintenance, effacer l'historique) | Change la configuration (surveillances, règles, canaux, pages d'état, réglages, sauvegardes, jetons, comptes) |
|---|---|---|---|
| **Lecteur** | oui | non | non |
| **Opérateur** | oui | oui | non |
| **Admin** | oui | oui | oui |

Un lecteur voit chaque page avec chaque contrôle qui changerait quelque
chose caché. Un opérateur est le rôle d'astreinte : les contrôles
d'alerte sont là, les écrans de configuration lisent *Opérateur — la
configuration est réservée aux administrateurs*. Seul un administrateur
crée des comptes ou change un rôle — un opérateur ne peut promouvoir
personne, lui-même inclus. Les sections *Utilisateurs*, *Connexion
unique* et le journal de sécurité restent réservés aux administrateurs.
**Ajouter un utilisateur** demande un nom d'utilisateur, un nom
d'affichage optionnel, un rôle et un mot de passe ; le bouton à côté du
champ de mot de passe en invente un aléatoire. Le mot de passe est montré
une fois, pour être transmis.

Chaque ligne porte le rôle, si le compte se connecte avec un mot de
passe ou via le fournisseur d'identité, si la double authentification est
active, et la dernière connexion. **Modifier** change le nom
d'affichage, le rôle et le mot de passe (une réinitialisation déconnecte
cet utilisateur partout et montre le nouveau mot de passe une fois).
L'interrupteur **Activé** suspend un compte sans le supprimer ;
**Réinitialiser 2FA** apparaît quand le compte a un second facteur ;
**Supprimer** le retire pour de bon. Le serveur garde au moins un
administrateur actif, et les contrôles reflètent cette règle plutôt que
de la rapporter comme une erreur : le dernier administrateur ne peut pas
être rétrogradé, désactivé ou supprimé, et vous ne pouvez pas désactiver
ou supprimer votre propre compte.

## Connexion unique

Administrateurs seulement. Connexion via un fournisseur OpenID Connect
(Authelia, Authentik, Keycloak, Pocket ID, Google Workspace…). Remplissez
le *nom du fournisseur* (il étiquette le bouton « Continuer avec … » sur
l'écran de connexion), l'*URL de l'émetteur* — la découverte est lue
depuis `{issuer}/.well-known/openid-configuration` — l'*id client*, le
*secret client* (stocké chiffré, jamais remontré) et l'*URL publique* par
laquelle les navigateurs atteignent DumbMonit. Le formulaire construit
l'URI de redirection depuis cette URL publique : copiez-la et
enregistrez-la chez votre fournisseur.

**Tester la découverte** lit la configuration du fournisseur sans
connecter personne, liste les points de terminaison trouvés, et avertit
quand le fournisseur ne signe ni en RS256 ni en ES256 — les deux seules
signatures que DumbMonit accepte. Les réglages enregistrés ici ont
priorité sur les variables d'environnement `DUMBMONIT_OIDC_*`, qui ne
s'appliquent que tant que rien n'est enregistré ; **Oublier les réglages
enregistrés** les efface et revient aux variables quand elles existent.

**Avancé.** Le reste du formulaire se trouve sous *Avancé* : *Scopes*
nomme ce qu'il faut demander au fournisseur, *Claim des groupes* le claim
du jeton qui liste les groupes de l'utilisateur, et *Créer des comptes à
la première connexion* décide si une personne inconnue obtient un
compte. C'est **désactivé par défaut** : seuls les comptes qui existent
déjà peuvent se connecter jusqu'à ce que vous l'activiez.

**Rôles.** *Groupes administrateurs* liste les groupes dont les membres
deviennent administrateurs ; *Groupes opérateurs* les groupes dont les
autres membres deviennent opérateurs ; tout le reste est lecteur.
Laissez les deux vides pour continuer à gérer les rôles sous
**Utilisateurs**. Avec seulement *Groupes opérateurs* rempli, les
groupes ne touchent jamais au rôle administrateur : les administrateurs
restent administrateurs, les autres deviennent opérateurs ou lecteurs.
Les rôles sont réévalués à chaque connexion, sauf que le dernier
administrateur actif n'est jamais rétrogradé. Les variables
d'environnement correspondantes sont `DUMBMONIT_OIDC_ADMIN_GROUPS` et
`DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Sur quel compte une connexion arrive.** L'identité est mémorisée par
son sujet de fournisseur après la première connexion. À une première
connexion :

- Un compte existant n'est lié que quand le fournisseur affirme
  `email_verified: true` et que cet email est le nom d'utilisateur du
  compte. Un `preferred_username` ou un email non vérifié ne lie jamais :
  sur la plupart des fournisseurs l'utilisateur les choisit, donc `admin`
  venant du fournisseur ne doit pas devenir *votre* `admin`.
- Un **administrateur local qui a un mot de passe n'est jamais lié
  automatiquement**, même avec un email vérifié : la connexion crée un
  compte distinct à la place. Continuez à gérer cet administrateur avec
  son mot de passe, ou créez un administrateur SSO séparé via les groupes
  administrateurs.
- Sinon, avec *Créer des comptes à la première connexion* activé, un
  nouveau compte est créé (nom d'utilisateur du fournisseur, suffixé
  `-1`, `-2`… si pris). Désactivé, seule la liaison par email vérifié
  ci-dessus peut se connecter.

**La création de compte est désactivée sauf si vous la choisissez.** Les
nouvelles installations démarrent avec *Créer des comptes à la première
connexion* désactivé, et toute configuration où ça n'a jamais été réglé
aussi — réglages enregistrés avant que l'option n'existe, ou
configurations uniquement par environnement sans
`DUMBMONIT_OIDC_AUTO_CREATE`. Les releases antérieures traitaient « jamais
réglé » comme activé ; le panneau Connexion unique dit maintenant quand
ce défaut s'applique. Une valeur que vous avez enregistrée explicitement
est gardée telle quelle.

## Agents

Jetons d'enrôlement pour l'[agent Linux, macOS, FreeBSD et
Windows](../devices/agent.md). Créez-en un avec un nom (« Serveur de
fichiers », « Parc maison ») : le jeton est montré une fois, avec les
commandes d'installation Linux et Windows prêtes à copier. Chaque
commande porte les sommes de contrôle SHA-256 des binaires d'agent que ce
serveur fournit, et l'installateur refuse un téléchargement qui ne
correspond pas ; les sommes de contrôle sont aussi listées sous les
commandes, pour comparer à la main. La liste montre le préfixe de chaque
jeton, sa date de création, sa dernière utilisation et s'il a été révoqué.
**Révoquer** arrête chaque agent utilisant ce jeton à son prochain push.

Un jeton peut enrôler plusieurs machines. Le révoquer ne supprime pas les
surveillances. Les lecteurs voient la liste et la section lit *Lecteur —
lecture seule* ; créer et révoquer sont réservés aux administrateurs.

## Score de sécurité {#security-score}

Dans **Réglages → Supervision**, sous les agents : une note qui dit à quel point l'instance et les hôtes surveillés sont durcis. Voir [Score de sécurité](security-score.md).

## Rapports

Dans **Réglages → Notifications et rapports**, sous la politique : emails de
disponibilité périodiques pour l'équipe. Choisissez
hebdomadaire, quotidien ou mensuel, le jour, l'heure et le fuseau
horaire, les destinataires et le canal email, puis **Envoyer un aperçu**
pour voir le résultat. Réservé aux administrateurs. Voir
[Rapports par email](reports.md).

## Sauvegarde

Trois choses, dans l'ordre où vous les rencontrez.

Un **avertissement que vous ne pouvez pas manquer** : `/data/secret.key`
est ce qui déchiffre les identifiants de surveillance, et une copie de la base
de données sans lui restaure une instance qui ne peut parler à rien. Le
texte change quand le secret vient de `DUMBMONIT_SECRET` à la place — il
n'y a alors pas de fichier à copier, et c'est votre gestionnaire de mots
de passe qui doit le détenir.

**Exporter la configuration** liste ce que le paquet contiendrait, avec
un compteur par section, puis demande une phrase de passe deux fois. Le
fichier qui en résulte porte chaque identifiant de cette instance,
chiffré avec cette phrase de passe seule : gardez-le où vous gardez vos
mots de passe. Les mots de passe de compte et les secrets 2FA sont
laissés de côté sauf si vous activez l'interrupteur.

**Restaurer un paquet** prend le fichier et sa phrase de passe et montre
d'abord un **essai à sec** : un tableau de ce qui serait créé, mis à jour
et laissé tel quel, section par section, avec une note pour tout ce qu'il
ne peut pas placer. Rien n'est écrit jusqu'à ce que vous appuyiez sur
*Restaurer pour de vrai*.

**Sauvegardes locales planifiées** montre si elles sont actives, où elles
vont, la dernière exécution et si elle a réussi, et les fichiers gardés
avec leur taille. **Sauvegarder maintenant** en écrit une immédiatement —
la chose à faire avant une mise à jour.

Tout ici est réservé aux administrateurs. Voir
[Sauvegarde et restauration](../install/backup.md).

## DumbMonit lui-même {#self-monitoring}

L'état du moteur à l'instant — planificateur, base SQLite, base de métriques —
avec l'espace libre du volume de données, la taille de la base et, pour le
VictoriaMetrics embarqué, le nombre de fois où il a été relancé. Des règles
intégrées vous préviennent déjà quand l'un d'eux va mal.

En dessous, le **battement de cœur sortant** : une URL à vous (Healthchecks.io,
un moniteur push d'Uptime Kuma, Better Stack…) que DumbMonit appelle à
intervalle régulier, seulement tant qu'il va bien, pour que quelqu'un dehors
remarque quand cet hôte se tait. **Tester** envoie un appel tout de suite ;
**Signaler les pannes** appelle l'URL d'échec plutôt que de se taire. L'URL est
rangée chiffrée et n'est plus jamais affichée, seul son hôte. La modifier est
réservé aux administrateurs. Voir
[Surveiller DumbMonit lui-même](../install/self-monitoring.md).

## API & assistants

Un seul type de jeton pour les deux : clients MCP et
[API REST](../reference/api.md). Nommez un jeton, choisissez sa portée et
créez-le — il est montré une fois. Un jeton **read** ne peut que
regarder ; un jeton **read and write** peut aussi changer des choses —
mettre en silence une surveillance, lancer une sonde, ajouter ou modifier des
surveillances, règles et canaux — mais jamais les comptes, les réglages de
connexion ni d'autres jetons.

Sous **Connecter**, trois onglets contiennent les extraits, remplis avec
le jeton tant qu'il est encore à l'écran : *Claude* (la commande
`claude mcp add` et le bloc `claude_desktop_config.json`), *ChatGPT*
(l'URL du serveur MCP et l'en-tête d'autorisation, avec un avertissement
quand cette page n'est pas servie en HTTPS — ChatGPT se connecte depuis
les serveurs d'OpenAI, l'adresse doit donc être accessible depuis
l'internet) et *Cursor / autre*, la même forme pour tout client qui parle
Streamable HTTP avec un en-tête bearer. **Accès API** montre le même
jeton sur l'API REST comme un exemple `curl`, et pointe vers les routes
qu'un Prometheus ou un Grafana sonde avec lui. Voir
[Connecter un assistant](assistant.md) et
[Sonder DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## À propos

Version du serveur et santé de ses deux magasins, la base de données
(configuration des surveillances et historique d'alertes) et VictoriaMetrics
(chaque mesure dans les graphiques), comme rapporté par `GET /api/health`
et rafraîchi toutes les 15 secondes. Un magasin qui arrête de répondre
montre un avertissement avec l'erreur et quoi vérifier. La section pointe
aussi vers cette documentation et nomme la licence, Apache 2.0.
