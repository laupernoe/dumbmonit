# Sauvegarde et restauration

Deux choses différentes, pour deux accidents différents.

| | Paquet chiffré | Sauvegarde locale planifiée |
|---|---|---|
| Ce qu'il contient | La configuration : appareils et leurs identifiants, règles, canaux, pages d'état, comptes, jetons. | Toute la base de données, comme un fichier, plus `secret.key`. |
| Où il va | Où vous le mettez — il est chiffré avec une phrase de passe que vous choisissez. | `/data/backups/`, dans le même volume. |
| Ce dont il vous sauve | Un serveur perdu, un déménagement vers un nouveau matériel, une installation neuve. | Le changement que vous regrettez dix minutes plus tard, une mise à jour qui a mal tourné. |
| Ce qu'il ne contient pas | Historique de métriques, état et historique d'alertes, le journal d'audit, les sessions. | Historique de métriques (`vm/`). |

Ni l'un ni l'autre ne remplace une archive de tout le volume si vous
voulez aussi récupérer les graphiques ; c'est en bas de cette page.

!!! danger "`/data/secret.key` est ce qui déchiffre les identifiants de vos appareils"
    Communautés SNMP, jetons Proxmox, mots de passe SMTP et URL de
    battement de cœur sont stockés chiffrés avec une clé dérivée du
    secret de l'instance. Au premier démarrage, DumbMonit l'écrit dans
    `/data/secret.key` (ou le lit depuis `DUMBMONIT_SECRET` si vous en
    réglez un).

    **Une copie de la base de données sans ce fichier restaure une
    instance qui ne peut parler à rien.** Le serveur le remarque et
    refuse de démarrer avec un message explicite plutôt que d'échouer à
    chaque sonde — mais les identifiants ont disparu, et chaque appareil
    doit être ressaisi.

    Le paquet chiffré est l'exception : ses secrets sont rechiffrés
    avec votre phrase de passe, il se restaure donc sur une instance
    neuve qui a son propre secret, tout neuf.

## Le paquet chiffré

**Réglages → Sauvegarde → Télécharger le paquet.** Choisissez une
phrase de passe d'au moins 16 caractères — trois mots suffisent — et
confirmez-la. Le fichier qui en résulte est du JSON simple avec un corps
chiffré :

```json
{
  "format": "dumbmonit-backup",
  "version": 1,
  "created_at": "2026-09-22 19:04:11",
  "source_version": "0.1.0-alpha.3",
  "summary": { "targets": 12, "channels": 2, "rules": 41 },
  "kdf": { "algorithm": "argon2id", "salt": "…", "m_cost": 32768, "t_cost": 3, "p_cost": 1 },
  "cipher": "aes-256-gcm",
  "payload": "…"
}
```

L'en-tête reste lisible pour que vous puissiez savoir ce qu'est un
fichier avant de taper quoi que ce soit. Tout le reste — adresses
incluses — est dans `payload`, chiffré avec AES-256-GCM sous une clé
dérivée de votre phrase de passe par Argon2id. Il n'y a **aucune
récupération** : un paquet dont la phrase de passe est perdue est perdu.

!!! warning "Le paquet porte chaque identifiant de cette instance"
    Quiconque a le fichier et la phrase de passe peut lire le mot de
    passe de chaque appareil que vous surveillez. Gardez-le où vous
    gardez vos mots de passe.

### Ce qu'il contient

| Section | Contenu |
|---|---|
| `targets` | Appareils : nom, type, adresse, intervalle, options, parent, agent relais, et l'identifiant. |
| `rules` | Règles d'alerte, y compris les intégrées que vous avez retouchées ou désactivées. |
| `rule_overrides` | Seuils et exclusions par appareil. |
| `channels` | Canaux de notification, leurs réglages, leur politique par canal et leurs secrets. |
| `notify_policy` | La politique de notification globale (groupement, plafond horaire, flapping, URL publique). |
| `silences` | Fenêtres de maintenance. |
| `status_pages` | Pages d'état publiques et les services que chacune liste. |
| `incidents` | Incidents et annonces de maintenance, avec leurs mises à jour. |
| `users` | Comptes : nom d'utilisateur, nom d'affichage, rôle, lien SSO, activé ou non. |
| `agent_tokens` | Jetons d'enrôlement d'agent, comme des hashs. Les agents installés continuent d'envoyer après une restauration. |
| `api_tokens` | Jetons d'API, comme des hashs. Les jetons existants continuent de fonctionner. |
| `push_monitors` | Jetons de battement de cœur, pour que les URL que vos tâches cron appellent déjà restent les mêmes. |

Les appareils sont retrouvés par **type + adresse**, qui est aussi la
clé unique de la table : importer le même paquet deux fois ne crée
jamais une seconde copie de quoi que ce soit.

### Mots de passe de compte

Par défaut, les comptes sont exportés **sans** leur hash de mot de
passe, secret TOTP ou codes de récupération. Les comptes restaurés
existent alors avec le bon nom et rôle, mais personne ne peut s'y
connecter jusqu'à ce qu'un administrateur règle un mot de passe — ou
qu'ils se connectent via SSO, qui n'a besoin de rien du paquet.

Activez **Inclure les mots de passe de compte et les secrets 2FA**
quand vous déplacez une instance plutôt que de copier sa configuration.
Un hash Argon2 dans un fichier qui voyage peut être attaqué hors ligne
aussi longtemps que l'attaquant le veut ; le défaut est le choix sûr.

### Ce qui n'y est pas

L'historique de métriques (qui vit dans VictoriaMetrics, pas dans la
base de données), l'état et l'historique d'alerte, les références
d'anomalie, le journal d'audit, les sessions ouvertes, l'enregistrement
et la dernière vue des agents, et les caches de sonde des panneaux
Proxmox et Synology. Ce sont des observations, pas des réglages : ils
reviennent d'eux-mêmes.

Et **pas le secret de l'instance**. C'est le but : le paquet porte ses
secrets rechiffrés sous votre phrase de passe précisément pour pouvoir
être restauré là où `secret.key` n'a jamais été vu.

### Restaurer sur une instance neuve

1. Démarrez le nouveau conteneur et créez le premier compte
   administrateur à `/setup`, avec le code d'installation affiché dans
   ses logs. Ce compte est le vôtre ; la restauration n'y touchera pas.
2. **Réglages → Sauvegarde → Restaurer un paquet.** Choisissez le
   fichier, saisissez la phrase de passe, et appuyez sur **Vérifier
   cette sauvegarde**.
3. Lisez le rapport. Il dit, section par section, ce qui serait
   **créé**, ce qui serait **mis à jour** parce que ça diffère de ce
   qui est déjà là, et ce qui est déjà identique — plus une note pour
   tout ce qu'il n'a pas pu placer, comme une règle qui pointait vers
   un appareil que le paquet ne contient pas.
4. Appuyez sur **Restaurer pour de vrai**.

Trois règles que la restauration ne casse jamais :

- **Rien n'est jamais supprimé.** Une restauration fait exister ce que
  le paquet décrit ; tout ce que l'instance a en plus reste.
- **Rien n'est jamais dupliqué.** Chaque élément est retrouvé par sa
  clé naturelle — type + adresse pour un appareil, `uid` pour une
  règle, nom pour un canal, slug pour une page d'état. Restaurer le
  même paquet deux fois rapporte que tout est déjà identique et n'écrit
  rien.
- **Les comptes existants sont laissés tranquilles.** Un compte dont le
  nom d'utilisateur est déjà là est ignoré, jamais écrasé : un fichier
  ne doit pas pouvoir changer le rôle ou le mot de passe de
  l'administrateur qui lance la restauration.

Un essai à sec n'est pas une simulation de la restauration, c'est *la*
restauration, lancée dans une transaction annulée à la fin. Le rapport
ne peut pas se tromper sur ce que l'appliquer ferait.

Un paquet écrit par un DumbMonit plus récent est refusé avec une phrase
qui le dit — mettez à jour d'abord, puis restaurez. Un paquet qui a été
modifié, ou dont la phrase de passe est incorrecte, est refusé aussi :
AES-GCM ne peut pas distinguer les deux, et le message non plus.

### Depuis la ligne de commande

Les deux voies ont besoin d'une **session** administrateur, pas d'un
jeton d'API : un jeton vit dans un script, et un script n'a rien à
faire à exporter chaque identifiant de l'instance.

```bash
# Se connecter
curl -s -c jar -X POST http://localhost:8080/api/auth/login \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"username":"admin","password":"…"}'

# Exporter
curl -s -b jar -X POST http://localhost:8080/api/backup \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"passphrase":"three words are enough"}' -o backup.json

# Essai à sec, puis appliquer
jq -n --slurpfile b backup.json \
  '{bundle: $b[0], passphrase: "three words are enough", apply: false}' > restore.json
curl -s -b jar -X POST http://localhost:8080/api/backup/restore \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d @restore.json | jq
```

## Sauvegardes locales planifiées {#scheduled-local-backups}

DumbMonit écrit une copie de sa propre base de données sur un calendrier,
par défaut **une fois par jour, en gardant les sept dernières**, dans
`/data/backups/` :

```
/data/backups/dumbmonit-20260922-030000.db
/data/backups/dumbmonit-20260922-030000.key
```

Le fichier `.db` est produit avec `VACUUM INTO` de SQLite : une copie en
ligne, cohérente, écrite tandis que le serveur continue de tourner, pas
une copie de fichier qui pourrait attraper la base de données en pleine
écriture. `sqlite3` l'ouvre tel quel, sans journal à rejouer. Le `.key`
à côté est la copie de `secret.key` — ensemble, la paire suffit à
reconstruire l'instance.

Ils vivent dans le même volume que la base de données, ils défont donc
une erreur, pas un disque perdu. Copiez-les hors de la machine si vous
voulez plus que ça.

### Réglages

| Variable | Défaut | Rôle |
|---|---|---|
| `DUMBMONIT_BACKUP_ENABLED` | `1` | `0`, `false`, `no` ou `off` pour arrêter d'écrire les sauvegardes planifiées. |
| `DUMBMONIT_BACKUP_DIR` | `<répertoire de données>/backups` | Où elles sont écrites. Pointez-le sur un second volume pour survivre à la perte du premier. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Heures entre deux sauvegardes. La première a lieu un intervalle après le démarrage, pas au démarrage. |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Combien sont gardées. Les plus anciennes sont supprimées, avec leur `.key`. |

Rien d'autre dans le répertoire n'est jamais touché : seuls les fichiers
nommés `dumbmonit-<timestamp>.db` et leur `.key` sont rotés.

Quand le secret de l'instance vient de `DUMBMONIT_SECRET`, il n'y a pas
de fichier à copier, et aucun n'est écrit — gardez cette valeur dans
votre gestionnaire de mots de passe à la place. La section Sauvegarde
dit dans lequel des deux cas vous êtes.

### Savoir que ça a tourné

Chaque exécution publie trois séries :

| Métrique | Signification |
|---|---|
| `dumbmonit_instance_backup_last_success_seconds` | Horodatage Unix de la dernière sauvegarde **réussie**. Une tentative échouée ne la rafraîchit jamais. |
| `dumbmonit_instance_backup_last_status` | `1` si la dernière tentative a réussi, `0` si elle a échoué. |
| `dumbmonit_instance_backup_size_bytes` | Taille du dernier fichier écrit. |

La règle intégrée **La sauvegarde de DumbMonit n'a pas tourné**
(`instance_backup_missing`) se déclenche quand le dernier succès a plus
de deux jours — une nuit manquée peut être un redémarrage, deux est un
problème. La série n'existe qu'une fois une première sauvegarde écrite,
une instance avec le calendrier désactivé n'est donc jamais importunée
pour ça.

**Réglages → Sauvegarde** montre la même chose sans MetricsQL : la
dernière exécution et si elle a réussi, les fichiers gardés et leur
taille, et **Sauvegarder maintenant** pour en écrire une immédiatement.

### En restaurer une {#restoring-one}

Arrêtez la stack, remettez le fichier sous le nom de la base de données,
et restaurez la clé à côté :

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data alpine sh -c '
  cp /data/backups/dumbmonit-20260922-030000.db /data/dumbmonit.db &&
  cp /data/backups/dumbmonit-20260922-030000.key /data/secret.key &&
  rm -f /data/dumbmonit.db-wal /data/dumbmonit.db-shm'
docker compose start
```

Supprimer les fichiers `-wal` et `-shm` compte : ils appartiennent à la
base de données que vous venez de remplacer, et SQLite essaierait de
les appliquer à la nouvelle.

Pour lire une sauvegarde sans la restaurer, copiez-la et ouvrez-la avec
`sqlite3` :

```bash
docker run --rm -v dumbmonit-data:/data -v "$PWD:/out" alpine \
  cp /data/backups/dumbmonit-20260922-030000.db /out/
sqlite3 dumbmonit-20260922-030000.db 'SELECT name, kind, address FROM targets;'
```

Les identifiants dans ce fichier restent illisibles sans le `secret.key`
correspondant.

## Archiver tout le volume

La seule façon de garder aussi les graphiques. Un volume nommé,
`dumbmonit-data`, contient tout : la base de données, le secret, les
sauvegardes locales et les données VictoriaMetrics.

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Pour restaurer, créez le volume, extrayez l'archive dedans de la même
façon, puis `docker compose up -d`.

Si l'espace compte, `--exclude=./vm` garde l'archive petite : la base de
données et le secret sont la configuration, `vm/` n'est que
l'historique.

## Quoi faire avant une mise à jour

1. **Réglages → Sauvegarde → Sauvegarder maintenant** — trente
   secondes, et ça inclut le secret.
2. Téléchargez aussi un paquet si la mise à jour est importante, ou si
   vous déménagez vers une autre machine.
3. `docker compose pull && docker compose up -d`. Les migrations
   tournent au démarrage.

Si ça tourne mal, [restaurez la sauvegarde locale](#restoring-one) et
démarrez le tag d'image précédent.
