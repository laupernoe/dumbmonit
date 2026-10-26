# Signer les binaires de l'agent

Les agents [se mettent à jour](../devices/agent.md#automatic-updates) depuis le
serveur, et ne lancent un nouveau binaire que si sa signature Ed25519 est valide
pour une clé publique de release compilée dans l'agent. Cette page s'adresse aux
mainteneurs : création de la clé, où elle vit, et ce que le workflow de release
en fait.

## Créer la clé

```sh
openssl genpkey -algorithm ed25519 -out agent-signing.pem
openssl pkey -in agent-signing.pem -pubout
```

1. Ajoutez le bloc PEM **public** affiché par la seconde commande à
   `crates/agent/signing/release-keys.pem` et committez-le. Plusieurs clés
   peuvent y figurer.
2. Stockez la clé **privée** comme secret de dépôt (pas un secret
   d'environnement) :

    ```sh
    gh secret set AGENT_SIGNING_KEY < agent-signing.pem
    ```

3. Gardez une sauvegarde hors ligne de `agent-signing.pem`, puis supprimez la
   copie locale.

## Ce que fait le workflow de release

À chaque push sur `dev` et à chaque tag, `release.yml`, après avoir extrait les
binaires de l'agent :

- signe chacun avec `openssl pkeyutl -sign -rawin` (signature détachée de 64
  octets bruts, `<binaire>.sig`) ;
- vérifie que la moitié publique de la clé figure dans `release-keys.pem`, et
  échoue sinon ;
- vérifie chaque signature ;
- joint les fichiers `.sig` à l'artefact du workflow et à la release GitHub ;
- les transmet au build de l'image serveur comme contexte de build
  `agent-signatures`, qui les ajoute en une couche supplémentaire.

Sans le secret, le workflow avertit et publie des builds non signés. Les agents
les refusent, sauf si *Accepter les builds non signés (développement)* est
activé (voir [Sécurité](../devices/agent.md#automatic-updates-security)).

Les signatures sont des fichiers séparés pour que les binaires restent
[reproductibles](reproducible-builds.md) ; la couche `.sig` est la seule
différence avec l'image publiée.

## Rotation

1. Ajoutez la nouvelle clé publique à `release-keys.pem` (en gardant l'ancienne)
   et publiez une release : les agents font alors confiance aux deux.
2. Basculez le secret `AGENT_SIGNING_KEY` vers la nouvelle clé privée.
3. Une release plus tard, retirez l'ancienne clé publique.

## Si la clé fuite

Retirez sa clé publique de `release-keys.pem` et publiez avec une nouvelle clé.
Les agents déjà déployés font confiance à ce avec quoi ils ont été compilés : une
clé qui a fuité reste de confiance pour eux jusqu'à ce qu'ils soient mis à jour
au-delà. C'est pourquoi le secret n'est utilisable que par `release.yml` sur
`dev` et les tags.

Faire confiance à un build `:edge` signé revient donc à faire confiance aux
protections de la branche `dev` : qui peut y pousser peut obtenir un build signé.
