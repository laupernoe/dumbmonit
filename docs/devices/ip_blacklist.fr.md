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
s'appliquent. Spamhaus refuse les résolveurs publics : son refus
(`127.255.255.254`) est affiché en erreur de configuration, jamais comme une
inscription. Une liste qui ne répond pas est inconnue, jamais propre. Utilisez
un résolveur récursif à vous, ou une clé DQS gratuite de Spamhaus comme
identifiant.

Un /24 demande près de deux mille questions. Elles partent à **Queries per
second** (10 par défaut), huit au plus en vol, et toute la plage est vérifiée
une fois par **intervalle de rafraîchissement** (6 heures par défaut) ; le
dernier résultat est réaffiché à chaque sonde entre deux. La première
vérification d'une plage prend quelques minutes et tourne en tâche de fond.

## Vérifier une plage d'IP publiques auprès des listes noires {#check-a-public-ip-range-against-the-blocklists}

1. Saisissez une adresse IPv4 ou une plage CIDR jusqu'à /24, par exemple « 198.51.100.0/24 » (256 adresses). Une plage plus grande est refusée : découpez-la en plusieurs moniteurs /24. Les plages IPv6 sont refusées, seul Spamhaus répondant pour l'IPv6 : vérifiez une adresse IPv6 isolée avec la réputation du serveur mail.

2. Utilisez un résolveur récursif à vous, comme pour la réputation du serveur mail : Spamhaus refuse les résolveurs publics ; ou collez une clé DQS gratuite de Spamhaus comme identifiant. Un /24 demande près de deux mille questions, envoyées à 10 par seconde et répétées toutes les 6 heures par défaut : gardez cette cadence, les listes bannissent les résolveurs qui les balaient.

3. La première vérification d'une plage prend quelques minutes : la page du moniteur indique qu'elle tourne, puis montre combien d'adresses sont inscrites, sur quelles listes, et un lien de retrait pour chacune.

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

## Identifiants {#credentials}

Aucun, ou une clé DQS de Spamhaus.

Adresse : une adresse IPv4 ou une plage CIDR jusqu'à /24. Options : Resolver,
Lists (toutes les listes d'adresses par défaut), intervalle de
rafraîchissement (6 heures), Queries per second (10) et délai par question
(5 s).
