# Serveur OpenVPN

Le processus serveur d'OpenVPN expose son propre canal de contrôle, l'**interface
de gestion** : un écouteur TCP séparé du VPN lui-même, fait pour un outil de
supervision et non pour un client VPN. DumbMonit la lit comme n'importe quel
client de gestion : la version du serveur, s'il se déclare connecté, et chaque
client actuellement en session, avec son adresse réelle et virtuelle et son
trafic. Rien n'est jamais envoyé qui changerait l'état du serveur ou
déconnecterait un client.

Les clients sont transitoires par nature : un portable qui se met en veille
disparaît simplement de la lecture suivante. DumbMonit ne cherche pas à se
souvenir de qui était connecté il y a cinq minutes ; il rapporte seulement qui
est connecté maintenant, plus le nombre de clients comme métrique à tracer dans
le temps.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 7505 | TCP | Interface de gestion (par défaut) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_openvpn_`.

| Métrique | Quoi | Étiquettes |
|---|---|---|
| `server_info` | valeur 1 | `version` |
| `server_up` | 1 quand l'interface de gestion rapporte le serveur `CONNECTED`, 0 sinon | `state` |
| `server_state_age_seconds` | temps depuis l'entrée du serveur dans son état actuel | |
| `clients_connected` | nombre de clients actuellement en session | |
| `client_bytes_received`, `client_bytes_sent` | trafic d'un client (compteurs) | `common_name`, `real_address` |
| `client_connected_seconds` | temps depuis le début de la session de ce client | `common_name`, `real_address` |

`real_address` ne porte jamais le port source du client : il change à chaque
reconnexion derrière un NAT et multiplierait les séries pour rien. Au plus 256
clients ont chacun une série détaillée ; `clients_connected` reste exact
au-delà.

Règle intégrée qui s'applique : **Surveillance injoignable**, quand la connexion de
gestion elle-même ne peut pas être établie ou que le mot de passe est refusé.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a enregistré ; ouvrir la
page ne se connecte jamais à l'interface de gestion. Il montre la version du
serveur, s'il se déclare connecté, le nombre de clients, et la liste des
clients en session avec leur adresse et leur trafic.

## Activer l'interface de gestion

1. Ajoutez cette ligne à la configuration du serveur (server.conf, ou as.conf sur OpenVPN Access Server) et redémarrez OpenVPN.

    ```
    management 0.0.0.0 7505
    ```

2. Recommandé : protégez-la par un mot de passe, placé dans un fichier lisible par le seul processus OpenVPN et ajouté en troisième argument.

    ```
    management 0.0.0.0 7505 /etc/openvpn/management-password
    ```

3. Dans DumbMonit, saisissez l'adresse du serveur (vpn.lan, port 7505) et le mot de passe de gestion s'il y en a un.

!!! warning
    Quiconque atteint le port de gestion et connaît le mot de passe peut déconnecter des clients et reconfigurer le serveur en marche : n'autorisez que l'hôte DumbMonit dans le pare-feu et définissez un mot de passe.

Documentation de l'éditeur : https://openvpn.net/community-resources/management-interface/

### Notes

L'interface de gestion est un écouteur séparé, désactivé par défaut, sans menu
OpenVPN : c'est une directive de configuration, d'où la ligne à coller ici.
Liez-la à une adresse que l'hôte DumbMonit peut joindre, ou à 127.0.0.1 et
atteignez-la par un tunnel SSH ou un agent relais sur la même machine. Elle n'a
pas de compte propre, seulement le mot de passe partagé.

DumbMonit ne fait que lire : il envoie version, state et status 3, jamais
signal, kill ni client-kill. Il ne peut pas déconnecter un client, changer un
réglage ni arrêter le serveur.

## Identifiants

| Identifiant | Champs |
|---|---|
| Sans mot de passe | Seulement pour une interface de gestion lancée sans fichier de mot de passe. |
| Mot de passe de gestion | Le mot de passe partagé défini par le troisième argument de la directive « management ». |

Adresse : un nom d'hôte ou une IP, éventuellement avec `:port` ; le port par
défaut est 7505, une convention de la documentation d'OpenVPN, pas un standard
imposé par le serveur. Options : port (7505) et délai par échange (10 s).

Une connexion refusée ou sans réponse compte comme serveur injoignable. Un mauvais
mot de passe s'affiche comme une erreur d'identifiants ; ni l'un ni l'autre ne
réveille personne la nuit. La connexion est du TCP en clair : rien de ce qui est
lu n'est secret, mais le mot de passe, s'il y en a un, traverse le réseau en
clair à chaque vérification — n'atteignez l'interface que depuis un réseau de
confiance ou via un agent relais sur la machine du serveur.
