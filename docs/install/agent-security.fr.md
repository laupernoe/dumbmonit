# Sécurité du canal des agents

Les agents (et les agents relais) se connectent au serveur ; le serveur ne se
connecte jamais à eux. Ils le font sur un **port
agents dédié et chiffré** (`8443` par défaut), conçu pour être exposé sur
Internet : personne sur le chemin, y compris quelqu'un sur le réseau local de
l'agent ou du serveur, ne peut lire ni modifier ce que les agents envoient ou
reçoivent.

Rien à configurer : le serveur crée sa propre autorité de certification au
premier démarrage, et la commande d'installation affichée dans DumbMonit porte
tout ce dont l'agent a besoin.

## Ports à ouvrir

| Où | Port | Pourquoi |
|----|------|----------|
| Serveur, entrant | **TCP 8443** | Canal chiffré des agents. Le seul port nécessaire aux agents d'autres réseaux. |
| Serveur, entrant | TCP 8080 | Interface web, scripts d'installation et téléchargement de l'agent. Peut rester privé (LAN, VPN). |
| Agents | aucun | Les agents n'établissent que des connexions sortantes. |

Avec Docker, publiez le port agents à côté du port web :

```yaml
    ports:
      - "8080:8080"
      - "8443:8443"
```

Si un routeur ou un mandataire TCP le publie sous un autre numéro, réglez
`DUMBMONIT_AGENT_PUBLIC_PORT` pour que les commandes d'installation l'utilisent.

## Ce qui protège le canal

- **TLS 1.3 seulement**, suites AEAD (AES-GCM, ChaCha20-Poly1305). Aucune
  version plus ancienne ne peut être négociée.
- **Échange de clés hybride post-quantique X25519MLKEM768.** L'agent ne propose
  rien d'autre : un trafic enregistré aujourd'hui reste protégé face à un futur
  ordinateur quantique.
- **Un certificat client par machine (mTLS).** L'agent génère sa paire de clés
  sur la machine (ECDSA P-256) ; la clé privée ne la quitte jamais. À
  l'enrôlement, il envoie la clé publique avec son jeton, une fois, et reçoit
  un certificat valable 30 jours, qu'il renouvelle de lui-même 10 jours avant
  l'échéance, avec une nouvelle clé, sous le certificat en cours.
- **Révocation immédiate.** Un certificat n'est accepté que tant que sa machine
  existe : supprimer l'équipement dans DumbMonit le refuse dès la requête
  suivante.
- **Serveur épinglé.** L'agent ne reconnaît le serveur qu'à l'empreinte SHA-256
  de l'autorité de DumbMonit (`--ca-pin` dans la commande d'installation). Un
  certificat d'une autorité publique, même valide, est refusé : un attaquant ne
  peut pas se faire passer pour le serveur, un mandataire qui intercepte TLS ne
  peut pas lire le trafic.
- **Pas de retour en arrière.** Un agent installé avec l'empreinte, ou enrôlé
  une fois, ne parle plus jamais en clair ; si le port agents est injoignable,
  il garde ses mesures et réessaie.
- **Deux secrets par machine.** En plus du certificat, chaque requête porte le
  jeton d'enrôlement et le secret de liaison de la machine : un certificat ne
  peut pas parler pour une autre machine.
- **Enrôlement freiné.** Les jetons erronés sont comptés par adresse et bloqués
  après quelques essais. Un jeton est à usage unique, sauf s'il est créé
  réutilisable.
- **Rien ne fuit avant l'authentification.** Le port agents ne sert que le
  protocole des agents : ni interface, ni version, ni description de l'API.
- **Clés au repos.** La clé privée de l'autorité est chiffrée par la clé de
  l'instance (`/data/secret.key`). Sur l'agent, `agent-tls.json` (clé et
  certificat) est rangé à côté d'`agent.yaml`, lisible par son seul
  propriétaire (0600 sous Linux, macOS et FreeBSD ; SYSTEM et Administrateurs
  seulement sous Windows).

Les commandes envoyées aux agents (redémarrage et mise à jour de conteneurs) et
les sondes déléguées aux agents relais passent par le même canal.

## Modèle de menace

Protégé contre :

- quiconque peut écouter ou modifier le réseau entre l'agent et le serveur
  (Wi-Fi public, FAI, routeur compromis, voisin sur le même LAN) ;
- un attaquant muni d'un certificat public valide pour votre domaine, ou un
  mandataire qui intercepte TLS ;
- une commande d'installation volée une fois le jeton utilisé (jeton à usage
  unique), un certificat volé une fois la machine supprimée ;
- une machine compromise du parc qui voudrait rapporter pour une autre, ou
  prendre ses commandes.

Non protégé contre :

- un administrateur de la machine agent (il peut lire sa clé, comme tout le
  reste) ;
- quiconque accède à la fois à `/data` et à la clé de l'instance du serveur ;
- la toute première installation en HTTP clair : le script et le binaire
  viennent du port web. Servez l'interface en HTTPS (ou vérifiez le SHA-256
  affiché à côté de la commande) si le réseau entre vous et le serveur n'est
  pas de confiance. Le canal des agents, lui, est épinglé dès la première
  requête.

## Agents existants et transition

Sur une instance qui avait déjà des agents lors de la mise à jour, le canal en
clair du port web reste ouvert, et chaque réponse à un agent annonce le canal
chiffré (port et empreinte). Un agent à jour s'enrôle alors de lui-même sur le
port agents avec son jeton et son secret de liaison, puis bascule. Cette
première empreinte est une confiance au premier usage : relancez la commande
d'installation de DumbMonit (qui porte l'empreinte) sur les machines dont le
réseau n'est pas de confiance.

**Réglages → Agents** indique pour chaque agent s'il rapporte chiffré (avec
l'échange de clés utilisé) ou par le canal en clair, et propose l'interrupteur
**Exiger le canal chiffré**. Activé, le canal en clair refuse les agents ; une
installation neuve démarre avec. `DUMBMONIT_AGENT_LEGACY_DAYS=N` ferme
automatiquement le canal en clair N jours après la mise à jour.

Les agents antérieurs au canal chiffré ne connaissent pas le canal chiffré : mettez-les à
jour en relançant la commande d'installation.

## Mises à jour de l'agent

Les agents se mettent à jour depuis le serveur (voir
[Mises à jour automatiques](../devices/agent.md#automatic-updates)). L'intégrité
vient du SHA-256 de l'offre ; l'authenticité d'une signature Ed25519, vérifiée
avec une clé publique de release compilée dans l'agent : un serveur compromis ou
usurpé ne peut donc pas pousser du code arbitraire. Les builds sans signature
sont refusés, sauf si un administrateur active *Accepter les builds non signés
(développement)* **et** que l'offre arrive par le canal chiffré. Le
téléchargement passe par le canal épinglé de l'agent, avec son jeton, son secret
de liaison et sa clé de machine.

## Mandataires inverses

Ne placez pas de mandataire inverse qui termine TLS devant le port agents : le
certificat client s'arrêterait au mandataire. Un relais TCP transparent
(HAProxy `mode tcp`, nginx `stream`, routeur TCP Traefik en passthrough,
redirection de port) fonctionne. Le port web peut rester derrière votre
mandataire HTTPS habituel.

## Rotation de l'autorité de certification

Dans **Réglages → Agents → Canal des agents → Rotation de l'autorité de
certification** :

1. **Démarrer la rotation** crée une nouvelle autorité. Les agents apprennent
   son empreinte par le canal chiffré à leur prochain rapport ; elle ne signe
   encore rien.
2. Attendez que tous les agents la connaissent (le compteur l'indique).
3. **Terminer la rotation** : le serveur présente désormais un certificat de la
   nouvelle autorité. Les agents qui n'ont pas rapporté entre-temps sont
   refusés et doivent être réinstallés avec la nouvelle commande.

## Réglages

| Variable | Défaut | Rôle |
|----------|--------|------|
| `DUMBMONIT_AGENT_LISTEN` | `0.0.0.0:8443` | Adresse d'écoute du canal des agents ; `off` le ferme. |
| `DUMBMONIT_AGENT_PUBLIC_PORT` | port d'écoute | Port écrit dans les commandes d'installation. |
| `DUMBMONIT_AGENT_LEGACY_DAYS` | absent | Fermer le canal en clair N jours après la mise à jour. |

Sur l'agent (`agent.yaml` ou environnement) : `tls_url` /
`DUMBMONIT_AGENT_TLS_URL` (par défaut : l'hôte de `server_url`, port 8443) et
`ca_pin` / `DUMBMONIT_AGENT_CA_PIN`.

## Paramètres d'installation pour les outils de déploiement

`GET /api/agent/install?base_url=…` renvoie l'URL web, l'URL du canal des
agents, l'empreinte de l'autorité, les empreintes des binaires, et les
commandes d'installation Unix et PowerShell avec `{TOKEN}` à remplacer. Un
générateur GPO ou Ansible doit s'en servir plutôt que de reconstruire la
commande.
