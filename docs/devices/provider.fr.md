# État d'un fournisseur tiers

La page de statut publique d'un fournisseur dont vous dépendez (Cloudflare,
GitHub, Atlassian, AWS…), lue comme un moniteur. Il passe en échec quand le
fournisseur déclare un incident : vous le savez avant vos utilisateurs, et une
[page de statut](../using/status-pages.md) peut afficher le fournisseur à côté
de vos propres services, comme composant externe.

Deux formats sont lus :

- le JSON d'une Atlassian Statuspage, `/api/v2/summary.json` : la plupart des
  pages de statut publiques (Cloudflare, GitHub, Atlassian, Discord,
  DigitalOcean…) ;
- un flux RSS ou Atom d'incidents : le flux AWS Health
  (`https://status.aws.amazon.com/rss/all.rss`, ou un flux par service et par
  région) et le `history.rss` d'une Statuspage.

Microsoft 365 n'est pas couvert : l'état de ses services n'est pas public, il
n'est disponible que par l'API Microsoft Graph (`serviceAnnouncement`) avec une
application déclarée dans votre tenant. Tout flux RSS ou Atom public
d'incidents peut en revanche être donné comme adresse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | Page de statut du fournisseur (HTTPS) |

## Ce qui est surveillé

En plus des [métriques communes de disponibilité](services.md)
(`probe_success`, `probe_duration_seconds`, `probe_failure_info`), toutes
préfixées `dumbmonit_` :

| Métrique | Sens |
|---|---|
| `probe_provider_impact` | 0 opérationnel, 1 mineur, 2 majeur, 3 critique |
| `probe_provider_incidents` | incidents ouverts (non résolus) |
| `probe_provider_components_affected` | composants non opérationnels (Statuspage seulement) |

Le moniteur échoue (`probe_success` à 0, motif `status`) quand l'impact atteint
**Échec à partir de** : toute dégradation par défaut. Une maintenance planifiée
n'est pas une panne. La règle d'alerte intégrée « service en panne »
s'applique donc à un fournisseur comme à n'importe quel service.

Avec un filtre **Composant**, seuls comptent les composants dont le nom contient
le texte (sans tenir compte de la casse) : « Workers », « Git Operations »,
« EC2 ». Un filtre qui ne retient rien est signalé comme une erreur de
configuration, pas comme une panne.

Dans un flux, seule l'entrée la plus récente de la **Fenêtre du flux** (24 heures
par défaut) compte : tant qu'elle n'est pas marquée « resolved » ou « operating
normally », un incident est ouvert, et « disruption » ou « outage » dans son
texte compte comme majeur. Restreignez un flux qui mêle plusieurs services avec
Composant.

Si la page de statut elle-même est illisible, la dernière réponse sert encore
15 minutes ; ensuite le moniteur échoue avec le motif `connect`, `timeout`,
`status` ou `body`.

La réponse est partagée : plusieurs moniteurs, ou plusieurs pages de statut,
sur le même fournisseur coûtent une requête par **Intervalle de rafraîchissement**
(60 secondes par défaut). L'adresse ne peut pas viser l'hôte DumbMonit lui-même
sans cocher « Autoriser les cibles locales ».

## Sur une page de statut

Un moniteur `provider` ajouté à une [page de statut](../using/status-pages.md)
apparaît comme un service ordinaire marqué « Externe », avec son propre
historique de disponibilité.

## Surveiller la page de statut d'un fournisseur

1. Saisissez un nom de fournisseur (cloudflare, github, atlassian, aws, discord, npm… ; la liste complète est dans le guide), ou l'adresse d'une page Statuspage ou d'un flux RSS/Atom.

2. Pour ne surveiller qu'une partie du fournisseur, saisissez son nom sous Composant, par exemple « Workers » ou « EC2 ».

3. Choisissez ce qui fait échouer le moniteur : toute dégradation, une panne partielle ou pire, ou une panne majeure seulement.

!!! warning
    Le moniteur lit ce que le fournisseur choisit de publier, souvent après plusieurs minutes : il complète vos propres vérifications et ne les remplace jamais.

Aucun identifiant n'est nécessaire : ces pages sont publiques.

## Identifiants

Aucun.

Noms : cloudflare, github, atlassian, discord, dropbox, npm, digitalocean, twilio, zoom, vercel, netlify, datadog et aws (flux AWS Health).

Adresse : un nom de la liste, l'adresse d'une page Statuspage
(`https://www.githubstatus.com`) ou d'un flux. Options : Composant, Échec à
partir de (minor, major, critical), Intervalle de rafraîchissement (60 s, de 30
à 3600), Fenêtre du flux (24 h, de 1 à 168) et le délai habituel (5 s).
