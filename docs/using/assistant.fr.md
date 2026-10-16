# Connecter un assistant

DumbMonit a un serveur [MCP](https://modelcontextprotocol.io) intégré :
le Model Context Protocol, la façon dont les assistants comme Claude,
ChatGPT, Cursor ou l'agent de VS Code appellent des outils externes.
Connectez-en un, puis demandez-lui des choses comme :

- « Est-ce que tout va bien ? »
- « Mets le NAS en silence pour deux heures, je change un disque. »
- « Qu'est-ce qui s'est passé cette nuit ? »
- « À quel point le serveur de sauvegarde est plein, et est-ce que ça
  s'aggrave ? »
- « Poste une mise à jour sur la page d'état : le serveur mail est de
  retour. »

L'assistant voit ce que l'interface voit — surveillances, alertes,
historique, métriques, agents, pages d'état — et utilise les mêmes
mots : *actif*, *inaccessible*, *en attente*, *avis*, *avertissement*,
*supprimée par le parent*. Il tourne dans le serveur : rien à installer
à côté de DumbMonit, et les outils appellent le même code que l'UI web.

## 1. Créer un jeton

Réglages → **API & assistants** (`/settings#assistant`). Nommez le jeton
d'après l'assistant ou la machine sur laquelle il tourne (« Claude sur
mon portable »), puis choisissez :

| Réglage | Choix |
| --- | --- |
| **Portée** | **Lecture** : regarder — statut, surveillances, alertes, historique, métriques, règles, agents, pages d'état. Il ne peut jamais rien changer. **Lecture et écriture** : aussi agir — mettre en silence, acquitter, sonder, ajouter une surveillance, redémarrer un conteneur, poster un incident… (les [outils d'écriture](#what-the-assistant-can-call)). |
| **Expire** | 30 jours, 90 jours (le défaut), 1 an ou jamais. Un jeton expiré arrête de fonctionner ; créez-en un nouveau. |
| **Réseaux autorisés** | Optionnel. Adresses ou plages CIDR depuis lesquelles le jeton peut être utilisé (`192.168.1.0/24, 10.8.0.5`). Vide : n'importe où. |

Le jeton ressemble à `dmt_` suivi de 32 caractères hexadécimaux. Il est
montré **une fois**, juste après la création ; DumbMonit ne garde qu'un
hash. Perdu ? Révoquez-le et créez-en un autre.

Préférez un jeton de lecture sauf si vous voulez vraiment que
l'assistant agisse.

Le même jeton ouvre aussi l'[API REST](../reference/api.md#authentication)
aux scripts et tableaux de bord — `curl -H "Authorization: Bearer dmt_…"
https://monit.example.lan/api/targets` — où `read` voit ce qu'un lecteur
voit et `write` fait ce qu'un administrateur fait, comptes, jetons et
sauvegardes exceptés.

## 2. Coller l'extrait

La page de réglages les remplit avec l'adresse de votre serveur et le
nouveau jeton. Ci-dessous, `https://monit.example.lan` représente votre
serveur.

### Claude Code

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

Ajoutez `--scope user` pour l'avoir dans chaque projet, pas seulement le
courant.

### Claude Desktop

Claude Desktop démarre des serveurs locaux depuis son fichier de
configuration ; [`mcp-remote`](https://www.npmjs.com/package/mcp-remote)
fait le pont vers le point de terminaison HTTP de DumbMonit (il a besoin
de [Node.js](https://nodejs.org)). DumbMonit s'authentifie avec un
en-tête bearer plutôt qu'OAuth, ajoutez-le donc via ce fichier plutôt que
l'écran Connectors. Settings → Developer → **Edit Config**, puis ajoutez
à `claude_desktop_config.json` et redémarrez Claude Desktop :

```json
{
  "mcpServers": {
    "dumbmonit": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote", "https://monit.example.lan/api/mcp",
        "--header", "Authorization:${DUMBMONIT_AUTH}"
      ],
      "env": { "DUMBMONIT_AUTH": "Bearer dmt_…" }
    }
  }
}
```

L'en-tête passe par une variable d'environnement, sans espace après les
deux-points, parce que certaines plateformes découpent les arguments sur
les espaces. Pour un serveur atteint en HTTP simple sur votre réseau,
ajoutez `"--allow-http"` à `args` — la page de réglages le fait pour vous
quand elle est elle-même servie en HTTP.

### VS Code

`.vscode/mcp.json` dans un espace de travail (ou la commande
**MCP: Add Server** → HTTP) :

```json
{
  "servers": {
    "dumbmonit": {
      "type": "http",
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

### Cursor et autres clients

`.cursor/mcp.json`, et la même forme pour la plupart des clients qui
supportent les serveurs Streamable HTTP avec des en-têtes personnalisés :

```json
{
  "mcpServers": {
    "dumbmonit": {
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

Un client qui ne démarre que des serveurs locaux (stdio) peut utiliser
le pont `mcp-remote` montré pour Claude Desktop.

### ChatGPT

Settings → Connectors → **Create** (mode développeur). URL du serveur
MCP : `https://monit.example.lan/api/mcp`, en-tête d'autorisation :
`Bearer dmt_…`.

ChatGPT se connecte depuis les serveurs d'OpenAI, pas depuis votre
navigateur : DumbMonit doit être **accessible depuis l'internet en
HTTPS**. Voir les [notes de sécurité](#security-notes) avant de faire ça.

### Tout autre client

Ce qu'un client a besoin de savoir : le point de terminaison est
`https://<serveur>/api/mcp`, le transport est Streamable HTTP, et chaque
requête porte `Authorization: Bearer dmt_…`. Un appel brut, pour
vérifier le jeton et le chemin réseau depuis un terminal :

```bash
curl -s https://monit.example.lan/api/mcp \
  -H 'Authorization: Bearer dmt_…' \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

## Ce que l'assistant peut appeler {#what-the-assistant-can-call}

Vingt-sept outils. Chacun a un titre, une description, un schéma
d'entrée, un schéma de sortie et des annotations (`readOnlyHint`,
`destructiveHint`, `idempotentHint`), un client peut donc dire lesquels
ne font que regarder et vous demander avant de lancer les autres.

| Outil | Portée | Fait |
| --- | --- | --- |
| `get_status` | read | Le bulletin, la réponse à « est-ce que tout va bien ? » : compteurs de surveillances, alertes actives et en construction, une phrase (« 2 avis, 1 inaccessible. »). |
| `list_devices` | read | Surveillances et services avec état, parent et étiquettes ; filtre par sous-chaîne ou état. |
| `get_device` | read | Une surveillance : configuration, état, alertes actives, résumé CPU / mémoire / disque le plus plein sur 24 heures (disponibilité et latence pour une vérification de service). |
| `list_alerts` | read | Alertes actives avec les mots de gravité, depuis quand, valeur, suppression, acquittement. |
| `alert_history` | read | Transitions d'alerte, les plus récentes d'abord (`since` ou `hours`, surveillance optionnelle, limite). |
| `query_metrics` | read | Une requête de plage MetricsQL, 60 points au plus par série. |
| `list_silences` | read | Fenêtres de maintenance et si elles sont actives maintenant. |
| `list_rules` | read | Règles d'alerte avec type, gravité, seuil, activé. |
| `list_device_types` | read | Les types de surveillance que ce serveur peut surveiller, avec les options et identifiants que chacun attend — ce dont `add_device` a besoin. |
| `list_agents` | read | Machines faisant tourner l'agent : OS, version d'agent, mode relais, site, surveillances relayées, dernier contact, liaison. |
| `list_containers` | read | Les conteneurs Docker d'une machine faisant tourner l'agent : état, santé, redémarrages, mise à jour disponible, dernière commande. |
| `list_heartbeats` | read | Surveillances battement de cœur : intervalle attendu, dernier appel, verdict (en attente, à l'heure, manqué, rapporté en panne). L'URL secrète n'est jamais montrée. |
| `list_status_pages` | read | Pages d'état, publiées ou non, avec leur chemin public (`/s/<slug>`), les surveillances qu'elles montrent et leurs incidents ouverts. |
| `list_channels` | read | Canaux de notification : type, activé, dernière erreur, dernier message envoyé. Jamais leurs secrets. |
| `list_packs` | read | Packs d'intégration installés : version, type de surveillance, activé, surveillances qui l'utilisent, erreurs. |
| `silence_device` | write | Une fenêtre de maintenance ponctuelle sur une surveillance, commençant maintenant (1 heure par défaut, une semaine au plus). |
| `remove_silence` | write | Retire une fenêtre de maintenance. |
| `schedule_maintenance` | write | Une fenêtre de maintenance ponctuelle à une date donnée, sur une surveillance ou toute l'instance, une semaine au plus. |
| `acknowledge_alert` | write | Acquitte une alerte par empreinte pour un nombre d'heures (4 par défaut) : les rappels se mettent en pause, la résolution est quand même notifiée. `hours: 0` l'enlève. |
| `probe_device` | write | Sonde une surveillance immédiatement et rapporte ce qui a été mesuré. |
| `set_device_enabled` | write | Active ou désactive les vérifications d'une surveillance. |
| `set_rule_enabled` | write | Active ou désactive une règle d'alerte. |
| `add_device` | write | Ajoute une surveillance : nom, type, adresse, intervalle, parent, options, identifiants, agent relais. |
| `discover_network` | write | Scanne un réseau (CIDR, 4096 adresses au plus) pour des appareils SNMP et suggère un profil pour chacun. |
| `restart_container` | write | Demande à l'agent d'une machine de redémarrer un de ses conteneurs. |
| `test_channel` | write | Envoie un message de test via un canal de notification. |
| `post_incident` | write | Ouvre un incident sur une page d'état, ou poste une mise à jour sur un existant. |

Un jeton de lecture qui appelle un outil d'écriture obtient un refus
clair comme résultat de l'outil (`isError: true`) ; l'assistant
l'explique et rien ne change. C'est pareil pour tout ce que l'outil ne
peut pas faire — une surveillance inconnue, un nom ambigu, une fenêtre plus
longue qu'une semaine : le message dit quoi corriger.

## Détails du protocole {#protocol-details}

`POST /api/mcp` implémente MCP sur le transport **Streamable HTTP** :
JSON-RPC 2.0, une requête par `POST`, répondue en JSON simple (pas
d'événements envoyés par le serveur). C'est sans état : aucun
`Mcp-Session-Id` n'est émis, celui qu'un client envoie est ignoré, et
`GET` ou `DELETE` sur le point de terminaison répondent `405`. Seule la
capacité `tools` est offerte — pas de resources, prompts, sampling ou
OAuth.

Deux générations du protocole sont servies sur le même point de
terminaison :

| Versions | Comment un client le parle |
| --- | --- |
| `2026-07-28` | Sans état. Chaque requête porte `io.modelcontextprotocol/protocolVersion` et `io.modelcontextprotocol/clientCapabilities` dans `params._meta`, et les en-têtes `MCP-Protocol-Version`, `Mcp-Method` et, pour `tools/call`, `Mcp-Name`, qui doit correspondre au corps. Méthodes : `server/discover`, `tools/list`, `tools/call`. Les résultats portent `resultType: "complete"` et l'identité du serveur dans `_meta` (`io.modelcontextprotocol/serverInfo`) ; `tools/list` porte aussi `ttlMs` et `cacheScope`. |
| `2025-11-25`, `2025-06-18`, `2025-03-26` | La poignée de main `initialize` choisit la version (une inconnue obtient la plus récente de ces trois) ; `notifications/initialized` est acquittée avec `202` ; puis `ping`, `tools/list`, `tools/call`. |

Erreurs, en plus des `401`, `403` et `429` des
[vérifications de jeton](#security-notes), qui viennent avant tout
JSON-RPC :

| HTTP | Code JSON-RPC | Quand |
| --- | --- | --- |
| `400` | `-32020` (en-tête incohérent) | Une requête `2026-07-28` sans `MCP-Protocol-Version`, `Mcp-Method` ou `Mcp-Name`, ou avec des valeurs qui diffèrent du corps. |
| `400` | `-32022` (version de protocole non supportée) | Une version que ce serveur ne parle pas ; `error.data` liste celles `supported` et celle `requested`. |
| `400` | `-32602` (paramètres invalides) | Une requête `2026-07-28` manquant les champs `_meta` requis. |
| `404` | `-32601` (méthode non trouvée) | Une requête `2026-07-28` pour une méthode que ce serveur n'implémente pas. |
| `200` | `-32602` | Un outil inconnu, ou `arguments` qui n'est pas un objet. |
| | `-32700`, `-32600` | JSON illisible ; pas une requête JSON-RPC 2.0 (les lots ne sont pas supportés). |
| `202` | — | Une notification : acceptée, pas de corps. |

Un outil qui tourne mais échoue — portée manquante, surveillance inconnue —
n'est pas une erreur de protocole : il répond normalement avec
`isError: true` et une phrase que l'assistant peut relayer.

## Notes de sécurité {#security-notes}

- **Un jeton est un mot de passe.** Quiconque le détient lit tout ce que
  l'interface montre — et, avec un jeton d'écriture, change tout ce
  qu'un administrateur peut via l'API REST (surveillances, règles, canaux),
  comptes, jetons et sauvegardes exceptés. Gardez-le hors des chats
  partagés, captures d'écran et dépôts. Révoquez un jeton dont vous
  n'êtes pas sûr ; créez-en un autre en une minute.
- **Préférez la lecture.** La plupart des questions (« est-ce que tout
  va bien ? ») n'ont besoin d'aucune portée d'écriture. Créez un jeton
  d'écriture seulement pour un assistant que vous voulez vraiment voir
  agir, et nommez-le pour le reconnaître dans la liste. La plupart des
  clients demandent avant de lancer un outil qui n'est pas marqué
  lecture seule ; laissez cette confirmation active.
- **Donnez-lui une durée de vie et un réseau.** Un jeton qui expire en
  90 jours et ne fonctionne que depuis votre LAN ou VPN
  (`192.168.1.0/24`) vaut peu pour quiconque le trouve. Derrière un
  reverse proxy, déclarez le proxy dans `DUMBMONIT_TRUSTED_PROXIES`,
  sinon chaque appel semble venir du proxy et une restriction réseau les
  refuse tous.
- **Lié à son créateur.** Un jeton arrête de fonctionner quand le compte
  qui l'a créé est désactivé, perd son pouvoir d'écriture quand ce
  compte n'est plus administrateur, et est révoqué quand ce compte est
  supprimé.
- **Rien de secret ne revient.** Les outils ne renvoient jamais les
  identifiants de surveillance (seulement leur type), les secrets de canal,
  les URL de battement de cœur ou les jetons. `add_device` prend bien
  des identifiants en entrée : ils voyagent dans la conversation de
  votre assistant, et donc chez son fournisseur. Pour une surveillance avec
  des identifiants sensibles, préférez l'interface web.
- **Navigateurs et `Origin`.** Une requête qui porte un en-tête
  `Origin` (un client basé navigateur) est refusée avec `403` sauf si
  cette origine est celle du serveur ou est autorisée par
  `DUMBMONIT_API_CORS_ORIGINS` (par défaut n'importe quelle origine,
  puisque le jeton est requis de toute façon). Listez vos origines là
  pour restreindre ; voir
  [requêtes cross-origin](../reference/api.md#cross-origin-requests-cors).
- **Limite de débit.** Chaque jeton est limité à 120 appels par minute,
  REST et MCP ensemble : un assistant coincé dans une boucle obtient
  `429` avec `Retry-After`, pas toute l'attention de votre serveur.
- **HTTPS avant l'internet.** Les connecteurs de ChatGPT ont besoin
  d'une URL publique. Mettez DumbMonit derrière un reverse proxy avec
  TLS (Caddy, Traefik, nginx…) et réglez `DUMBMONIT_COOKIE_SECURE=1`.
  N'exposez jamais le port HTTP simple. Claude Desktop, Claude Code, VS
  Code et Cursor tournent sur votre machine et peuvent atteindre une
  adresse LAN directement, ils n'ont donc besoin d'aucune exposition du
  tout.
- **Surveillez « dernière utilisation ».** La liste de jetons montre
  quand chaque jeton a été utilisé pour la dernière fois et depuis
  quelle adresse. Un jeton utilisé à un moment où vous ne parliez pas à
  votre assistant, ou depuis une adresse que vous ne connaissez pas,
  mérite une révocation.
- **Logs.** Chaque appel d'outil est journalisé au niveau `info` avec le
  nom du jeton et l'outil, jamais les arguments ; chaque écriture via
  l'API REST avec le nom du jeton, la méthode, le chemin et le statut,
  jamais le corps. Créer et révoquer des jetons est dans le journal
  d'audit (Réglages → Sécurité).

## Dépannage

| Symptôme | Cause et correction |
| --- | --- |
| `401` / « A valid API token is required. » | L'en-tête manque ou est mal saisi (`Authorization: Bearer dmt_…`, un espace après `Bearer`), ou le jeton a été révoqué. Créez-en un nouveau. |
| `401` / « The API token "…" expired on … » | Créez un nouveau jeton ; choisissez une durée de vie plus longue si elle était trop courte. |
| `403` / « cannot be used from … » | Le jeton est restreint à d'autres réseaux. L'adresse dans le message est celle que le serveur voit : derrière un reverse proxy, déclarez le proxy dans `DUMBMONIT_TRUSTED_PROXIES`. |
| `403` sur chaque appel depuis un client basé navigateur | Son `Origin` n'est pas autorisé : ajoutez-le à `DUMBMONIT_API_CORS_ORIGINS`. |
| Un outil répond « needs a token with the "write" scope » | Le jeton est en lecture seule, ou son créateur n'est plus administrateur. Créez un jeton d'écriture si vous voulez que l'assistant agisse. |
| `429` | 120 appels en une minute : l'assistant boucle probablement. Attendez `Retry-After`, puis posez une question plus précise. |
| « Unsupported protocol version » | Le client parle une version que DumbMonit ne connaît pas ; mettez le client à jour. |
| Claude Desktop montre le serveur en échec | Node.js doit être installé pour `npx` ; en HTTP simple, `--allow-http` doit être dans `args`. |
| ChatGPT ne peut pas se connecter | Il a besoin d'une URL HTTPS publique ; une adresse LAN ou HTTP simple ne fonctionne pas depuis les serveurs d'OpenAI. |
