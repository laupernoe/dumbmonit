# Moniteurs depuis les étiquettes Docker

Si l'[agent](../devices/agent.md) tourne sur un hôte Docker, un conteneur peut
déclarer ses propres moniteurs avec des étiquettes (labels) `dumbmonit.*`.
DumbMonit les crée, les tient à jour et les archive quand le conteneur
disparaît. Rien à cliquer : le moniteur vit et meurt avec le
`docker-compose.yml` qui décrit le service.

!!! note
    L'agent doit lire le socket Docker (module `docker` de l'agent, actif par
    défaut) et le module **Docker** ne doit pas être décoché pour la machine
    dans DumbMonit.

## Exemple

```yaml
services:
  gitea:
    image: gitea/gitea:latest
    ports: ["3000:3000", "2222:22"]
    labels:
      dumbmonit.enable: "true"
      dumbmonit.name: "Gitea"
      dumbmonit.folder: "Apps/Dev"
      dumbmonit.tags: "env=prod,team=infra"
      dumbmonit.interval: "30s"
      dumbmonit.http.url: "http://{host}:3000/api/healthz"
      dumbmonit.tcp.port: "2222"
```

Cela crée deux moniteurs dans le dossier `Apps/Dev` : un contrôle HTTP nommé
`Gitea (HTTP)` et un contrôle TCP nommé `Gitea (TCP)`. `{host}` désigne l'adresse
de la machine où tourne l'agent.

## Étiquettes

| Étiquette | Sens |
|---|---|
| `dumbmonit.enable` | `true` pour inscrire le conteneur. Sans elle, toutes les autres étiquettes sont ignorées. |
| `dumbmonit.name` | Nom du moniteur. Par défaut : le nom du conteneur. Avec plusieurs contrôles, le type est ajouté (`Gitea (HTTP)`). |
| `dumbmonit.folder` | Dossier, `/` pour imbriquer (`Apps/Dev`). |
| `dumbmonit.tags` | Séparées par des virgules : `env=prod,critique` (une étiquette sans valeur vaut `true`). |
| `dumbmonit.interval` | Période de contrôle : `30`, `30s`, `5m`, `1h`. 10 s au minimum. Défaut 60 s. |
| `dumbmonit.http.url` | Crée un moniteur HTTP. `{host}` est remplacé par l'adresse de la machine. |
| `dumbmonit.http.method`, `.keyword`, `.keyword_absent`, `.accepted_status`, `.insecure_tls`, `.timeout` | Options du moniteur HTTP, même sens que dans le formulaire du moniteur. |
| `dumbmonit.tcp.port` | Crée un moniteur TCP sur ce port. |
| `dumbmonit.tcp.host` | Hôte du contrôle TCP. Défaut `{host}`. |
| `dumbmonit.ping.host` | Crée un moniteur ping sur cet hôte. |
| `dumbmonit.adopt` | `true` pour reprendre un moniteur existant de même type et adresse (voir plus bas). |

Au plus un moniteur de chaque type par conteneur. Chaque moniteur a une clé
stable (`projet/service/http` pour un service Compose, sinon `conteneur/http`) :
recréer le conteneur, ou le répliquer, ne crée donc pas de doublon.

## Ce que fait DumbMonit

- **Création.** Une nouvelle étiquette crée un moniteur, rangé sous l'agent de la
  machine comme *parent* : si l'hôte tombe, ses moniteurs ne lèvent pas chacun
  leur alerte.
- **Mise à jour.** Changez une étiquette (URL, période, dossier, tags…) et le
  moniteur suit en moins d'une minute.
- **Archivage.** Quand le conteneur est supprimé ou l'étiquette retirée, le
  moniteur est **mis en pause et daté**, pas supprimé : son historique reste. Il
  se réveille tout seul si les étiquettes reviennent. Rien n'est archivé tant
  que Docker n'est pas lisible.
- **Lecture seule.** Un moniteur géré par étiquettes porte le badge *Géré par les
  étiquettes Docker*. Ses champs ne se modifient pas et il ne se supprime pas
  tant que vous ne l'avez pas **libéré** (bouton sur la page du moniteur, ou
  `POST /api/targets/{id}/release`). Vous pouvez toujours le mettre en pause.
  Un moniteur archivé peut être supprimé directement.

## Adoption

Si un moniteur de même type et de même adresse existe déjà (créé à la main ou
importé), DumbMonit ne crée **pas** de doublon et ne touche pas au vôtre :
l'étiquette est signalée dans le journal du serveur. Ajoutez
`dumbmonit.adopt: "true"` pour laisser les étiquettes le prendre en charge ;
il est dès lors géré comme les autres.

## Sécurité

Quiconque peut démarrer un conteneur sur un hôte équipé de l'agent peut créer des
moniteurs pour lui. C'est la même confiance que celle déjà donnée par l'accès au
socket Docker. Chaque déclaration est validée comme un moniteur créé à la main,
et 200 au plus sont acceptées par machine.
