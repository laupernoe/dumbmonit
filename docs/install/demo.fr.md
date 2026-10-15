# Lancer une instance de démonstration

La démo publique sur **[demo.dumbmonit.app](https://demo.dumbmonit.app)** est
une image DumbMonit ordinaire démarrée avec `DUMBMONIT_DEMO=1`. Vous pouvez
faire la même chose vous-même — pour montrer DumbMonit à une équipe avant de
l'installer pour de vrai, ou pour mettre une vitrine sur votre propre site.

!!! warning "Jamais sur un vrai volume de données"
    Le mode démo **recrée la base de données à chaque démarrage**. Donnez-lui
    un volume qui lui est propre, ou aucun : pointé sur le `/data` d'une
    instance réelle, il remplacerait vos surveillances, règles et historique par
    le parc fictif.

## Ce que fait le mode démo

- **Connexion avec `demo` / `demo`.** La page de connexion l'indique et
  remplit les champs en un clic. Le compte voit tous les écrans qu'un
  administrateur voit.
- **Lecture seule.** Chaque modification — surveillances, règles, silences,
  acquittements, canaux, jetons, utilisateurs, réglages, sauvegardes et
  restaurations, scans de découverte, enrôlement d'agent, outils d'écriture du
  point de terminaison de l'assistant — est refusée par le serveur avec `403`
  et `{"error": "This is a read-only demo…", "demo": true}`. Se connecter et se
  déconnecter sont les seules écritures permises. L'interface affiche le
  message là où le clic a eu lieu.
- **Un parc fictif et fixe.** Au démarrage, la base de données est recréée et
  peuplée avec un petit homelab sous `*.home.arpa` : un cluster Proxmox VE, un
  NAS Synology, un Proxmox Backup Server, un TrueNAS, un pare-feu OPNsense, un
  serveur Redfish avec une pièce défaillante, quelques vérifications HTTP, TLS
  et ping, un hôte avec agent et une page d'état. Les surveillances sont répondues
  par de faux répondeurs à l'intérieur du serveur, liés à `127.0.0.1` et
  rejouant des réponses d'API enregistrées, si bien que les vrais collecteurs
  et panneaux de surveillance fonctionnent. Redémarrer donne le même parc.
- **Sept jours d'historique.** Des métriques synthétiques sont écrites dans
  VictoriaMetrics au démarrage, si bien que les graphiques, *7 derniers jours*,
  les prévisions et l'historique des alertes sont remplis, et quelques alertes
  sont actives pour que *À traiter* contienne quelque chose.
- **Rien ne sort du serveur.** Aucune notification n'est jamais envoyée, et
  aucune sonde ne part vers une adresse tapée par un visiteur.
- **Une visite guidée** s'ouvre après la première connexion ; le bandeau la
  rouvre.

La limitation de débit de la connexion fonctionne comme d'habitude. Placez la
démo derrière votre reverse proxy habituel avec HTTPS et définissez
`DUMBMONIT_COOKIE_SECURE=1` et `DUMBMONIT_TRUSTED_PROXIES` comme pour toute
instance publique.

## La démarrer

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

Ou avec Compose, sans volume pour que chaque redémarrage reparte propre :

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/laupernoe/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

Récupérez l'image régulièrement (ou laissez un outil comme Watchtower le
faire) : la démo montre alors toujours la dernière release avec les mêmes
données.
