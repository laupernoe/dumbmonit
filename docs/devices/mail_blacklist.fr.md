# Réputation du serveur mail

Un domaine de messagerie, ou l'adresse publique d'un serveur mail, vérifié
auprès des listes noires DNS qui décident encore de la remise du courrier :
Spamhaus, Barracuda, SpamCop et quelques listes plus modestes. Pour chaque
liste qui inscrit une adresse, la page du moniteur donne le code de réponse
de la liste, le motif qu'elle publie et un lien direct vers sa page de
consultation et de demande de retrait.

Saisissez un domaine (`example.com`) et DumbMonit vérifie les adresses de ses
hôtes MX (enregistrements A et AAAA), celles du domaine lui-même, et les
**adresses supplémentaires** que vous ajoutez (un relais d'envoi qui n'est pas
un MX). Le domaine passe aussi devant les listes de domaines. Saisissez une
adresse IP et seule cette adresse est vérifiée. Pour surveiller toute une
plage d'adresses, utilisez la [liste noire d'une plage d'IP](ip_blacklist.md).

## Gmail, Outlook.com et Yahoo {#gmail-outlookcom-and-yahoo}

Les grandes messageries ne publient **aucune liste interrogeable** : ce
qu'elles pensent d'un expéditeur leur reste interne. DumbMonit ne prétend pas
les interroger. Il vérifie ce qui pèse sur leur décision, et donne leurs
outils :

- **DNS inverse** : chaque adresse d'envoi doit avoir un enregistrement PTR
  dont le nom se résout vers la même adresse (DNS inverse confirmé, FCrDNS).
  Gmail, Outlook.com et Yahoo refusent ou classent en indésirable le courrier
  des adresses qui n'en ont pas.
- **SPF et DMARC** : un seul enregistrement SPF sur le domaine, et un
  enregistrement DMARC sous `_dmarc.<domaine>`. Gmail et Yahoo exigent les
  deux des envois en nombre.
- **Les listes noires ci-dessous**, que leurs filtres lisent aussi.

DKIM n'est pas vérifié : son sélecteur est choisi par chaque expéditeur et ne
se devine pas. La page du moniteur renvoie vers :

| Messagerie | Outils |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/) (réputation d'un domaine vérifié), [règles pour les expéditeurs](https://support.google.com/a/answer/81126), [formulaire de problème de remise](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/) (données de réputation par adresse), [règles pour les expéditeurs](https://sendersupport.olc.protection.outlook.com/pm/), [formulaire du support expéditeurs](https://olcsupport.office.com/) (retrait d'Outlook.com), [portail de retrait Microsoft 365](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/), [bonnes pratiques](https://senders.yahooinc.com/best-practices/), [formulaire du support expéditeurs](https://senders.yahooinc.com/contact/) |

## Les listes noires {#the-blocklists}

Vérifiées le 16 octobre 2026. SORBS a fermé en juin 2024 et n'est pas
interrogée. Chaque liste reçoit une question `A` sous sa zone ; elle est
désignée par l'identifiant de la première colonne, que l'option **Lists**
accepte pour restreindre la vérification.

| Id | Liste | Zone | Vérifie | IPv6 | Majeure | Usage et retrait |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | les adresses | oui | oui | Miroirs publics gratuits pour un usage non commercial et à faible volume ; refuse les résolveurs publics et ouverts. Retrait depuis le [vérificateur Spamhaus](https://check.spamhaus.org/). La PBL inscrit par principe les plages d'abonnés : n'en retirez une adresse que si elle fait vraiment tourner un serveur mail. |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | les adresses | non | oui | Gratuite ; Barracuda demande que les serveurs DNS qui l'interrogent soient déclarés avec un compte gratuit sur barracudacentral.org. [Demande de retrait](https://www.barracudacentral.org/rbl/removal-request). |
| `spamcop` | SpamCop | `bl.spamcop.net` | les adresses | non | oui | Gratuite. Les inscriptions expirent d'elles-mêmes environ un jour après la fin des signalements ; la [page de consultation](https://www.spamcop.net/bl.shtml) en donne la raison et propose le retrait. |
| `mailspike` | Mailspike | `bl.mailspike.net` | les adresses | non | non | Gratuite à faible volume. [Consultation et retrait](https://mailspike.io/ip_verify). |
| `uceprotect_1` | UCEPROTECT niveau 1 | `dnsbl-1.uceprotect.net` | les adresses | non | non | Interrogation gratuite. Les inscriptions expirent après sept jours sans nouveau spam ; la [page de vérification](https://www.uceprotect.net/en/rblcheck.php) vend aussi un retrait express, jamais nécessaire. Peu de destinataires s'en servent. |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | les adresses | non | non | Gratuite. Retrait en libre-service sur la [page de l'inscription](https://psbl.org/). |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | les adresses | non | non | Gratuite à faible volume. RATS-NoPtr inscrit les adresses sans DNS inverse. [Consultation](https://www.spamrats.com/lookup.php) et [retrait](https://www.spamrats.com/removal.php). |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | le domaine | — | oui | Mêmes conditions que ZEN. Retrait depuis le [vérificateur Spamhaus](https://check.spamhaus.org/). |
| `surbl` | SURBL (multi) | `multi.surbl.org` | le domaine | — | non | Gratuite à faible volume ; refuse les résolveurs publics et chargés. [Consultation et retrait](https://www.surbl.org/surbl-analysis). |
| `uribl` | URIBL (multi) | `multi.uribl.com` | le domaine | — | non | Gratuite à faible volume ; refuse les résolveurs publics et chargés. [Consultation](https://lookup.uribl.com/), [retrait](https://admin.uribl.com/). |

Une inscription sur une liste **majeure** déclenche une alerte critique ; sur
les autres, un avis. Les adresses IPv6 des MX ne sont vérifiées que par
Spamhaus ZEN, la seule liste ici qui réponde pour l'IPv6 ; les autres les
indiquent comme non couvertes.

## Résolveurs, refus et délais {#resolvers-refusals-and-timeouts}

Les listes s'interrogent par le DNS, et leur réponse dépend **du résolveur
qui pose la question** :

- **Spamhaus refuse les questions qui passent par des résolveurs publics**
  (Google `8.8.8.8`, Cloudflare `1.1.1.1`, Quad9), par des résolveurs qui leur
  transmettent, ou par des résolveurs sans DNS inverse attribuable. Il répond
  alors `127.255.255.254` au lieu de `NXDOMAIN` (`127.255.255.255` quand un
  résolveur pose trop de questions, `127.255.255.252` pour une question mal
  formée ou une clé DQS erronée). Ces réponses ressemblent à des inscriptions
  et n'en sont pas : DumbMonit affiche la vérification en **erreur de
  configuration**, jamais en « listé », et n'envoie aucune alerte de panne.
- **URIBL et SURBL répondent `127.0.0.1`** aux résolveurs qu'elles refusent.
  La liste est alors affichée « Refusé » (inconnue), pas comme une
  inscription.
- **Une liste qui ne répond pas à temps est inconnue, jamais propre.** Seul
  `NXDOMAIN` veut dire « non listé ». Une réponse hors de `127.0.0.0/8` (un
  résolveur qui remplace `NXDOMAIN` par l'adresse d'une page de recherche)
  est inconnue elle aussi.

Utilisez donc **un résolveur récursif à vous** : Unbound, Knot Resolver, BIND
ou PowerDNS Recursor qui résout depuis les serveurs racine (sans transmettre à
un résolveur public), et indiquez son adresse dans **Resolver** si ce n'est
pas le résolveur du système. Pi-hole ou AdGuard Home devant Unbound convient ;
devant `1.1.1.1`, non.

Ou créez un compte gratuit au [Data Query Service de Spamhaus](https://www.spamhaus.com/data-access/free-data-query-service/)
(usage non commercial, faible volume) et saisissez sa clé comme identifiant :
Spamhaus est alors interrogé sous `<clé>.zen.dq.spamhaus.net` et
`<clé>.dbl.dq.spamhaus.net`, qui répondent quel que soit le résolveur. La clé
est stockée chiffrée et n'est plus jamais affichée.

## Cadence {#pace}

Les listes limitent la fréquence des questions d'un résolveur et bannissent
ceux qui les balaient. DumbMonit interroge chaque liste une fois par
**intervalle de rafraîchissement** (6 heures par défaut), au plus **Queries
per second** (10 par défaut) avec huit questions en vol, et réaffiche le
dernier résultat à chaque sonde entre deux, pour que les règles d'alerte
voient une série continue. La vérification tourne en tâche de fond : le
premier résultat paraît quelques secondes après l'ajout du moniteur, et
changer l'adresse, une option ou la clé relance aussitôt une vérification.
Une vérification en erreur est retentée au bout de dix minutes.

## Vérifier un serveur mail auprès des listes noires {#check-a-mail-server-against-the-blocklists}

1. Saisissez le domaine pour lequel vous envoyez du courrier, par exemple « example.com » : DumbMonit vérifie les adresses de ses MX et du domaine lui-même. Ou saisissez l'adresse IP publique depuis laquelle le serveur envoie. Si le courrier sort par un relais qui n'est pas un MX, ajoutez son adresse dans Extra addresses.

2. Utilisez un résolveur récursif à vous. Spamhaus refuse les questions qui arrivent par des résolveurs publics (Google, Cloudflare, Quad9) ou par un résolveur qui leur transmet, et d'autres listes les limitent : un Unbound qui résout depuis les serveurs racine suffit. Indiquez son adresse dans Resolver si ce n'est pas le résolveur du système.

3. Ou créez un compte gratuit au Data Query Service de Spamhaus (usage non commercial, faible volume) et collez sa clé comme identifiant : Spamhaus répond alors quel que soit le résolveur.

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

4. Pour vérifier la configuration, ajoutez un moniteur pour 127.0.0.2, l'adresse d'essai que toutes les listes inscrivent : chacune doit l'afficher comme listée.

5. Gmail, Outlook.com et Yahoo ne publient aucune liste interrogeable. DumbMonit vérifie à la place ce qui pèse pour eux (un DNS inverse qui revient vers l'adresse, les enregistrements SPF et DMARC) et renvoie vers leurs outils pour expéditeurs sur la page du moniteur.

!!! warning
    Une réponse refusée par Spamhaus (127.255.255.254, un résolveur public) est affichée en erreur de configuration, jamais comme une inscription, et une liste qui ne répond pas est affichée inconnue, jamais propre. Les listes sont interrogées toutes les 6 heures par défaut : plus souvent n'apporte rien et fait bannir les résolveurs.

## Ce qu'il surveille {#what-it-watches}

Toutes les métriques sont préfixées `dumbmonit_blacklist_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `listed_major_lists` | listes majeures (Spamhaus ZEN et DBL, Barracuda, SpamCop) qui inscrivent une adresse ou le domaine | |
| `listed_lists` | toutes les listes qui inscrivent une adresse ou le domaine | |
| `addresses_checked` | adresses vérifiées | |
| `addresses_listed` | adresses inscrites sur au moins une liste | |
| `domain_listed` | 1 quand le domaine figure sur une liste de domaines | |
| `unknown_checks` | vérifications sans réponse (délai, refus) | |
| `check` | une série par adresse (ou domaine) et par liste : 1 listé, 0 propre, −1 sans réponse, −2 refusé, −3 non couvert | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | par liste : sujets interrogés, inscrits, sans réponse | `list` |
| `address_info` | valeur 1, d'où vient chaque adresse | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 1 quand le DNS inverse revient vers l'adresse | `address`, `ptr` |
| `mx_hosts` | hôtes MX du domaine | |
| `spf_records` | enregistrements `v=spf1` du domaine (1 est la bonne valeur) | |
| `dmarc_records` | enregistrements `v=DMARC1` sous `_dmarc.<domaine>` | |
| `dmarc_policy_info` | valeur 1 | `policy` |
| `mail_auth_missing` | 0 à 2 : SPF absent ou en double, DMARC absent | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | la vérification affichée : son âge, sa durée, et si une nouvelle tourne | |

Les [règles livrées](../alerting/rules.md#dns-blocklists) qui s'appliquent :
Listé sur une liste noire majeure, Listé sur une liste noire, DNS inverse du
serveur mail incohérent, SPF ou DMARC absent, plus Surveillance inaccessible
quand le résolveur ne répond à aucune question.

## La page du moniteur {#the-monitor-page}

Le panneau lit ce que la sonde a enregistré ; ouvrir la page n'interroge
aucune liste. Il donne les compteurs, puis chaque liste avec le nombre de
sujets qu'elle inscrit, puis chaque résultat avec le code de la liste, son
motif publié et les liens vers ses pages de consultation et de retrait, puis
le DNS inverse de chaque adresse, SPF et DMARC, et les outils pour
expéditeurs de Gmail, Outlook.com et Yahoo.

## Identifiants {#credentials}

Aucun, ou une clé DQS de Spamhaus (voir plus haut).

Adresse : un domaine (`example.com`) ou une adresse IP publique. Options :
Resolver, Lists (toutes par défaut), Extra addresses, intervalle de
rafraîchissement (6 heures, de 1 à 168), Queries per second (10, de 1 à 50)
et délai par question (5 s, de 1 à 30). Les adresses privées ne sont jamais
vérifiées : les listes ne connaissent que l'Internet public. `127.0.0.2` est
accepté comme adresse d'essai.
