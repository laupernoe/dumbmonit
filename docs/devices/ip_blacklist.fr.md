# Liste noire d'une plage d'IP

Chaque adresse d'une plage IPv4 publique, jusqu'à un /24 (256 adresses),
vérifiée auprès des listes noires DNS de la [réputation du serveur mail](mail_blacklist.md#the-blocklists) :
Spamhaus ZEN, Barracuda, SpamCop, Mailspike, UCEPROTECT niveau 1, PSBL et
SpamRATS. La page du moniteur indique combien d'adresses sont inscrites, sur
quelles listes, et donne un lien direct vers la page de consultation et de
retrait de chaque inscription.

Utile pour la plage publique d'un bureau, le bloc qu'un hébergeur vous a
attribué, ou les adresses depuis lesquelles vos serveurs envoient : la
machine compromise d'un voisin vaut une inscription qu'un destinataire plus
strict peut étendre à toute la plage.

## Limites {#limits}

- **Un /24 au plus par moniteur.** Une plage plus grande est refusée avec un
  message qui dit combien d'adresses elle compte : découpez-la en plusieurs
  moniteurs /24. Chaque adresse coûte une question à chaque liste, et un
  balayage de milliers d'adresses ressemble pour elles à un abus.
- **IPv4 seulement.** De ces listes, seule Spamhaus ZEN répond pour l'IPv6,
  et une plage IPv6 ne se parcourt pas adresse par adresse. Une adresse IPv6
  isolée se vérifie avec la [réputation du serveur mail](mail_blacklist.md).
- **Adresses publiques seulement.** Une plage qui contient une adresse
  privée, partagée (CGNAT) ou de documentation est refusée : les listes ne
  connaissent que l'Internet public.
- **Les listes de domaines** (Spamhaus DBL, SURBL, URIBL) ne s'appliquent pas
  à une plage.

## Résolveurs et cadence {#resolvers-and-pace}

Les règles de la [réputation du serveur mail](mail_blacklist.md#resolvers-refusals-and-timeouts)
s'appliquent. **Aucune clé d'API n'est nécessaire.** Seul Spamhaus refuse les
résolveurs publics : son refus (`127.255.255.254`) n'est jamais affiché comme une
inscription, la page du moniteur indique « Spamhaus refuses this resolver » et les
autres listes gardent leurs résultats. Une liste qui ne répond pas est inconnue,
jamais propre. Pour Spamhaus aussi, utilisez un résolveur récursif à vous, ou la
clé DQS gratuite et facultative de Spamhaus comme identifiant.

Les listes sont celles cochées sous **Blocklists** (par défaut les principales
listes d'adresses) plus vos propres zones sous **Custom lists** ; voir
[Vos propres listes](mail_blacklist.md#your-own-lists). Les listes de domaines ne
s'appliquent pas à une plage.

Un /24 demande près de deux mille questions. Elles partent à **Queries per
second** (10 par défaut), huit au plus en vol, et toute la plage est vérifiée
une fois par **intervalle de rafraîchissement** (6 heures par défaut) ; le
dernier résultat est réaffiché à chaque sonde entre deux. La première
vérification d'une plage prend quelques minutes et tourne en tâche de fond.

## Vérifier une plage d'IP publiques auprès des listes noires {#check-a-public-ip-range-against-the-blocklists}

1. Saisissez une adresse IPv4 ou une plage CIDR jusqu'à /24, par exemple « 198.51.100.0/24 » (256 adresses). Une plage plus grande est refusée : découpez-la en plusieurs moniteurs /24. Les plages IPv6 sont refusées, seul Spamhaus répondant pour l'IPv6 : vérifiez une adresse IPv6 isolée avec la réputation du serveur mail.

2. Aucune clé d'API n'est nécessaire : laissez l'identifiant sur No key. Les listes noires sont interrogées par le résolveur DNS de ce serveur, ou par celui que vous indiquez dans Resolver. Seul Spamhaus peut refuser, et seulement quand ce résolveur est public (Google, Cloudflare, Quad9) : le moniteur affiche alors « Spamhaus refuses this resolver » et les autres listes continuent de fonctionner. Pour Spamhaus aussi, utilisez un résolveur récursif à vous, ou la clé DQS gratuite et facultative de Spamhaus.

3. Cochez les listes noires d'adresses à interroger sous Blocklists (les majeures sont cochées par défaut) et ajoutez sous Custom lists les zones DNSBL de toute autre liste. Un /24 demande près de deux mille questions avec toutes les listes d'adresses cochées, envoyées à 10 par seconde et répétées toutes les 6 heures par défaut : gardez cette cadence, les listes bannissent les résolveurs qui les balaient.

4. La première vérification d'une plage prend quelques minutes : la page du moniteur indique qu'elle tourne, puis montre combien d'adresses sont inscrites, sur quelles listes, et un lien de retrait pour chacune.

!!! warning
    La PBL de Spamhaus inscrit les plages d'abonnés qui ne devraient pas envoyer de courrier directement (connexions résidentielles, pools dynamiques) : y figurer relève d'une politique, pas d'un signalement d'abus, et ne compte que pour une adresse qui fait tourner un serveur mail.

## Ce qu'il surveille {#what-it-watches}

Les mêmes métriques que la [réputation du serveur mail](mail_blacklist.md#what-it-watches),
préfixées `dumbmonit_blacklist_`, sans celles du DNS inverse, de SPF et de
DMARC. Pour limiter le nombre de séries, `check` n'est écrit que pour une
adresse inscrite ou qu'une liste n'a pas su juger ; `list_checked`,
`list_listed` et `list_unknown` comptent toutes les adresses, liste par liste.

Les [règles livrées](../alerting/rules.md#dns-blocklists) qui s'appliquent :
Listé sur une liste noire majeure et Listé sur une liste noire.

La page du moniteur a un lien **Also check on MXToolbox** : pour une plage, à côté
de chaque adresse inscrite, il ouvre dans un nouvel onglet la recherche publique de
listes noires de mxtoolbox.com. DumbMonit n'appelle pas leur API.

## Identifiants {#credentials}

Aucun n'est nécessaire (par défaut), ou facultativement une clé DQS de Spamhaus.

Adresse : une adresse IPv4 ou une plage CIDR jusqu'à /24. Options : Resolver,
Blocklists (les majeures cochées sur un nouveau moniteur), Custom lists, intervalle
de rafraîchissement (6 heures), Queries per second (10) et délai par question
(5 s).
