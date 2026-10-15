# Expiration de domaine

Un domaine enregistré, lu depuis son registre via RDAP : les jours restants
avant expiration, une suspension qui le sort du DNS, une période de rachat,
son registrar et si sa délégation est signée avec DNSSEC.

RDAP est le successeur de WHOIS : les mêmes données d'enregistrement, en JSON,
avec un format standard entre registres. DumbMonit trouve le registre de
l'extension dans le fichier d'amorçage RDAP de l'IANA
(`https://data.iana.org/rdap/dns.json`, lu au plus une fois par jour), puis lui
demande `/domain/example.com`. Aucun identifiant n'est nécessaire, et rien
d'autre que l'enregistrement du domaine n'est demandé.

Deux pannes valent d'être surveillées en plus de la date d'expiration
elle-même. Un domaine en `clientHold` ou `serverHold` a été sorti du DNS par
le registrar ou le registre, généralement pour une facture impayée ou une
adresse de contact jamais vérifiée : chaque site et boîte mail en dessous
s'arrête net. Un domaine en `redemptionPeriod` ou `pendingDelete` a déjà
expiré et sera libéré à moins d'être restauré, à prix.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_domain_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `expiry_days` | jours restants avant expiration, fractionnaire ; négatif une fois expiré | |
| `expiry_timestamp_seconds` | la date d'expiration, en secondes Unix | |
| `expiry_published` | 0 quand le registre ne publie pas la date d'expiration | |
| `on_hold` | 1 en `clientHold` ou `serverHold` | |
| `redemption` | 1 en `redemptionPeriod`, `pendingDelete` ou `pendingRestore` | |
| `status` | valeur 1, une série par code de statut, au format EPP (`clientTransferProhibited`) | `status` |
| `registrar_info` | valeur 1 | `registrar` |
| `dnssec` | 1 quand la délégation est signée | |
| `rdap_ok` | 0 quand la dernière requête au registre a échoué et qu'une réponse antérieure est affichée | |
| `rdap_age_seconds` | âge de la réponse affichée | |

Le registre est interrogé une fois par intervalle de rafraîchissement (12
heures par défaut) ; entre deux, les jours restants sont recalculés à chaque
vérification. Quand le registre ne répond pas, la dernière réponse est encore
utilisée jusqu'à sept jours, avec `rdap_ok` à 0, pour qu'une panne du registre
ne ressemble pas à un problème de domaine.

Les [règles intégrées](../alerting/rules.md#reverse-proxies-and-domains) qui
s'appliquent : Domaine arrivant à expiration (moins de 30 jours), Domaine
proche de l'expiration (moins de 7 jours ou expiré), Domaine suspendu, Domaine
en rachat, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais le registre. Il indique la date d'expiration et les
jours restants, les codes de statut (ou la suspension et le rachat en une
phrase), le registrar et DNSSEC, et si la dernière requête au registre a
échoué.

## Surveiller l'enregistrement d'un domaine

1. Saisissez le domaine enregistré, par exemple « example.com », pas un nom en dessous comme www.example.com, que le registre ne connaît pas. Rien à configurer et aucun identifiant : RDAP, le successeur de WHOIS, est public.

2. DumbMonit trouve le registre de l'extension dans le fichier d'amorçage RDAP de l'IANA, puis lui demande la date d'expiration, les codes de statut et le registrar, deux fois par jour par défaut. Les registres limitent les requêtes RDAP, et la date d'expiration change une fois par an : un intervalle de rafraîchissement plus court n'apporte rien.

3. Quelques registres font tourner RDAP sans être listés par l'IANA, ceux de .ch et .li par exemple : réglez Serveur RDAP sur leur adresse. Une extension dont le registre ne fait tourner aucun service RDAP public ne peut pas être surveillée.

    ```
    https://rdap.nic.ch/
    ```

4. Certains registres ne publient pas la date d'expiration (.ch en est un) : DumbMonit le signale alors, et surveille quand même les codes de statut.

!!! warning
    Un domaine en clientHold ou serverHold est sorti du DNS : chaque site et boîte mail en dessous s'arrête, généralement pour une facture impayée ou une adresse de contact non vérifiée. DumbMonit avertit 30 jours avant expiration, monte le niveau 7 jours avant, et alerte immédiatement sur une suspension ou une période de rachat.

Un domaine que le registre ne connaît pas (un sous-domaine, une faute de
frappe, ou un domaine qui a été supprimé) est signalé comme une erreur de
configuration, pas comme une panne. Les noms avec accents sont acceptés et
demandés dans leur forme ASCII (`xn--…`).

## Identifiants

Aucun : RDAP est public.

Adresse : le domaine enregistré (`example.com`) ; une URL collée est réduite à
son nom d'hôte. Options : serveur RDAP (vide : trouvé via l'IANA), intervalle
de rafraîchissement (12 heures, de 1 à 168) et délai de requête (10 s).
L'intervalle de vérification de la surveillance elle-même peut rester à sa valeur
par défaut : il ne fait que recalculer les jours restants entre deux requêtes
au registre.
