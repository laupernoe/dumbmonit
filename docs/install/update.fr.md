# Mettre à jour DumbMonit

DumbMonit vous prévient quand une version plus récente existe et vous guide
dans la mise à jour. Il n'installe jamais rien de lui-même.

## Repérer une nouvelle version

Une fois par jour au plus, le serveur interroge l'API publique des releases de
GitHub (`api.github.com/repos/laupernoe/dumbmonit/releases`) pour connaître la
dernière release et la compare à la version que vous utilisez. Les
pré-versions comme `0.1.0-alpha.7` sont comparées correctement :
`alpha.10` est plus récent que `alpha.9`, et `0.1.0` est plus récent que
n'importe quel `0.1.0-alpha.N`. Les brouillons sont ignorés, et une instance
sur une version stable ne se voit jamais proposer de pré-version.

Quand une version plus récente existe, **Mise à jour disponible : vX** apparaît
en bas à droite de chaque page et dans **Réglages → À propos**, où **Comment
mettre à jour** ouvre les étapes ci-dessous et **Voir les nouveautés** rouvre
la fenêtre *Nouveautés*. **Vérifier maintenant** relance la vérification
immédiatement (au plus une fois par minute).

!!! note "Ce qui est envoyé"
    Une seule requête `GET` anonyme avec un user agent `DumbMonit/<version>`.
    Rien sur votre instance, vos appareils ou votre réseau n'est envoyé, et
    l'adresse est fixée dans le code.

### Désactiver la vérification

Pour les réseaux isolés (air-gapped), désactivez **Vérifier les nouvelles
versions** dans **Réglages → À propos** (administrateurs), ou définissez la
variable d'environnement, qui l'emporte sur l'interrupteur :

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

La même information est disponible pour les scripts et les jetons d'API sur
`GET /api/update`.

## Mettre à jour

1. **Sauvegardez.** Téléchargez une sauvegarde depuis **Réglages → Sauvegarde**,
   et conservez une copie de `/data/secret.key` (voir
   [Sauvegarde et restauration](backup.md)). Sans cette clé, les mots de passe
   et jetons stockés ne peuvent pas être déchiffrés.
2. **Récupérez l'image et redémarrez**, depuis le dossier qui contient votre
   `docker-compose.yml` :

    ```bash
    docker compose pull && docker compose up -d
    ```

    Vos données vivent dans le volume `/data` et sont conservées ; les
    migrations de base de données s'exécutent au démarrage.
3. **Rechargez la page.** **Réglages → À propos** affiche la nouvelle version
   et *Vous êtes à jour*.

Pour figer une version plutôt que de suivre `:latest`, changez le tag d'image
dans le fichier Compose (par exemple
`ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`) et relancez la même commande.

Pour revenir en arrière, restaurez la sauvegarde prise à l'étape 1 sur
l'image précédente.
