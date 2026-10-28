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
- **Deux clients fictifs.** Au démarrage, la base de données est recréée et
  remplie d'un homelab sous `*.home.arpa` — Proxmox VE, un NAS Synology avec un
  LUN iSCSI, Proxmox Backup Server, TrueNAS, OPNsense, UniFi, Kopia, un serveur
  Redfish avec une pièce en panne, un commutateur SNMP, des sondes HTTP, TLS,
  DNS et ping, un agent Linux qui fait tourner Docker — et, en mode MSP, d'un
  petit bureau d'architectes à Genève : un contrôleur de domaine Windows et son
  agent, Veeam Backup & Replication, un réseau Omada, Microsoft 365 et
  Entra ID, interrogés par un agent relais posé dans le bureau. Les
  surveillances sont servies par de faux équipements internes au serveur,
  liés à `127.0.0.1`, qui rejouent des réponses d'API enregistrées : les vrais
  collecteurs et les vrais panneaux tournent. Un redémarrage redonne le même
  parc.
- **Trente jours d'historique, et le reste du produit.** Des métriques
  synthétiques sont écrites dans VictoriaMetrics au démarrage, par blocs
  bornés (la dernière semaine au pas de cinq minutes) : graphiques, *7 derniers
  jours*, disponibilité sur 30 jours, prévisions, matrice des sauvegardes et
  historique des alertes sont remplis, et quelques alertes sont actives. Le
  semis remplit aussi ce qui se règle d'habitude à la main : équipes et
  dossiers, comptes et plannings d'astreinte avec politiques d'escalade,
  silences, consignes, deux tableaux de bord, rapports planifiés, canaux de
  notification (jamais utilisés), sites sur la carte du monde, failles
  connues, quatre pages de statut (une par scène de nuit) avec un incident en
  cours, et un balayage réseau avec un appareil nouveau. Tout est calculé à
  partir d'une graine fixe : rien n'est récupéré sur le réseau, pas même les
  sources de failles. La recherche dans les journaux reste éteinte : la démo
  n'a pas de magasin de journaux.
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
