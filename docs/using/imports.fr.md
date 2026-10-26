# Importer depuis d'autres outils

**Ajouter un moniteur → Import** reprend vos moniteurs d'un autre outil. Tous
les importeurs fonctionnent pareil : vous donnez la source, DumbMonit montre un
**aperçu** (ce qui sera créé, ce qui est ignoré et pourquoi), vous cochez ce que
vous voulez, puis vous confirmez. Un moniteur déjà présent (même type et même
adresse) n'est jamais dupliqué. Réservé aux administrateurs.

Ce qui n'est **jamais** importé : les canaux de notification, les mots de passe
et jetons, les en-têtes personnalisés et les corps de requête. Reconfigurez-les
dans DumbMonit.

| Depuis | Source | Devient |
|---|---|---|
| Uptime Kuma | `kuma.db`, ou la sauvegarde JSON de la 1.x | voir [page des moniteurs](devices.md) |
| UptimeRobot | clé d'API en lecture seule, ou export CSV | `http`, `ping`, `tcp`, heartbeat |
| Gatus | `config.yaml` | `http`, `tcp`, `ping`, `dns`, heartbeat |
| Healthchecks.io | clé d'API (healthchecks.io ou auto-hébergé) | heartbeat (push) |

## UptimeRobot

1. Dans UptimeRobot : **Integrations → API → Read-only API key**.
2. Dans DumbMonit : **Importer depuis UptimeRobot**, collez la clé, **Aperçu**.
   La clé sert une fois à lister vos moniteurs et n'est jamais conservée.

Variante : chargez le CSV exporté du tableau de bord (colonnes *Friendly Name*,
*URL / IP*, *Type*, *Port*, *Interval*, *Keyword* ; l'ordre importe peu).

| UptimeRobot | DumbMonit |
|---|---|
| HTTP(s) | `http` (méthode conservée) |
| Keyword | `http` avec le mot-clé (et « ne doit pas apparaître » si demandé) |
| Ping | `ping` |
| Port | `tcp` |
| Heartbeat | [heartbeat](../devices/push.md) (nouvelle URL) |

Les moniteurs en pause le restent.

## Gatus

Collez ou chargez votre `config.yaml`.

- Les `endpoints` à URL `http(s)://` deviennent des moniteurs `http`. Les
  conditions `[STATUS] == 200` et `[BODY].chemin == valeur` sont conservées ; les
  autres (`[RESPONSE_TIME]`, …) sont listées dans le message de la ligne.
- `tcp://hôte:port` devient `tcp`, `icmp://hôte` devient `ping`, un endpoint
  avec un bloc `dns:` devient `dns`.
- Les `external-endpoints` deviennent des heartbeats (nouvelle URL).
- `group` devient le dossier ; `interval` et `enabled` sont conservés. Les autres
  schémas (`starttls://`, `ssh://`, …) sont ignorés avec leur raison.

## Healthchecks.io

1. Dans Healthchecks : **Project Settings → API Access**. Une clé en
   *lecture-écriture* rappelle aussi l'URL de ping actuelle de chaque check ; une
   clé en lecture seule fonctionne, mais les anciennes URL ne sont pas affichées.
2. Dans DumbMonit : **Importer depuis Healthchecks.io**, collez la clé (et
   l'adresse de votre serveur s'il est auto-hébergé), **Aperçu**.

Chaque check devient un [moniteur heartbeat](../devices/push.md) : la période
attendue est celle du check (ou une approximation de son horaire cron, signalée
sur la ligne), la *grace time* est conservée, les tags deviennent des tags, un
check en pause le reste.

### Rediriger les anciennes URL de ping

Un moniteur heartbeat a une **nouvelle** URL secrète. Le travail qui appelait
`https://hc-ping.com/<uuid>` doit maintenant appeler l'URL DumbMonit (affichée
sur la page du moniteur). Deux façons :

- modifier le travail (crontab, script) — la façon propre ;
- ou, si vous hébergez Healthchecks sur un domaine à vous, laisser l'ancienne
  adresse répondre par une redirection vers la nouvelle, par exemple avec nginx :

  ```nginx
  location = /ping/<ancien-uuid> {
      return 307 https://monit.example.org/api/push/<nouveau-jeton>;
  }
  ```

  `307` conserve la méthode et le corps. `hc-ping.com` n'est pas à vous : là, il
  faut modifier les travaux.

Le message de la ligne rappelle l'ancienne URL de ping de chaque check, pour la
retrouver dans vos scripts.
