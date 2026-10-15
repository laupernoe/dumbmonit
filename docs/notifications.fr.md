# Notifications DumbMonit

Ce guide explique comment recevoir les alertes DumbMonit sur le service de
votre choix. Il ne demande aucune compétence de programmation : chaque
section dit où trouver l'adresse ou le jeton à copier, et quoi saisir dans
le formulaire.

---

## Sommaire

- [Comment ça fonctionne](#how-it-works)
- [Créer et tester un canal](#creating-and-testing-a-channel)
- [Réglages ou secrets : la différence](#settings-or-secrets-the-difference)
- [Acquitter ou mettre en pause depuis la notification](#acknowledge-or-snooze-from-the-notification)
- [Le canal personnalisé](#the-custom-channel)
  - [Variables disponibles](#available-variables)
  - [Exemple 1 — une API JSON interne](#example-1--an-internal-json-api)
  - [Exemple 2 — un service qui attend un formulaire](#example-2--a-service-that-expects-a-form)
  - [Exemple 3 — un SMS en texte brut via un opérateur](#example-3--a-plain-text-sms-through-a-carrier)
  - [Exemple 4 — une requête GET sans corps](#example-4--a-get-request-without-a-body)
  - [Erreurs de template courantes](#common-template-mistakes)
- [Canaux supportés](#supported-channels)
- [Dépannage](#troubleshooting)

---

## Comment ça fonctionne {#how-it-works}

Un **canal** est une destination : un canal Discord, un numéro de
téléphone, une boîte mail. Vous en créez autant que vous voulez, et vos
règles d'alerte choisissent lesquels notifier.

Trois principes s'appliquent à chaque canal :

1. **Les alertes sont groupées.** Un message par appareil et par cycle,
   même si cinq disques se remplissent en même temps. Sans ça, vous
   coupriez les notifications en une semaine.
2. **Un canal cassé n'en bloque jamais un autre.** L'échec est enregistré
   dans la colonne « dernière erreur » du canal, et la livraison continue
   vers les autres.
3. **Un service qui ne répond pas est abandonné après 15 secondes.** Une
   notification qui arrive une minute en retard n'est plus utile, et la
   boucle de surveillance ne doit pas attendre derrière un serveur
   inaccessible.

---

## Créer et tester un canal {#creating-and-testing-a-channel}

1. Ouvrez **Alertes → Notifications → Ajouter un canal**
   (`/alerts#notifications`).
2. Donnez-lui un nom significatif (« Discord — famille », « SMS —
   astreinte »). C'est le nom que vous choisirez plus tard dans vos règles
   d'alerte.
3. Choisissez le type de canal, puis remplissez les champs décrits
   ci-dessous.
4. Enregistrez, puis cliquez sur **Envoyer un test**.

Le message de test prend **exactement le même chemin** qu'une vraie
alerte : même requête, même jeton, même formatage. S'il arrive, le canal
fonctionne. S'il échoue, le message d'erreur nomme le réglage à corriger.

> **Un jeton n'est jamais remontré.** Une fois enregistré, il est chiffré
> dans la base de données et l'interface dit seulement « configuré ». Pour
> modifier un canal sans ressaisir son jeton, laissez simplement le champ
> vide.

---

## Réglages ou secrets : la différence {#settings-or-secrets-the-difference}

Chaque canal a deux groupes de champs.

| | **Réglages** | **Secrets** |
|---|---|---|
| Contenu | adresse serveur, nom de canal, sujet | jetons, mots de passe, URL de webhook |
| Stockage | texte clair | chiffré |
| Relu par l'interface | oui | jamais |

L'interface **refuse** d'enregistrer un champ sensible parmi les réglages,
et vous le dit. Ce n'est pas une formalité : les réglages sont renvoyés
tels quels par l'API, un jeton stocké là serait donc publié chaque fois
que la page est ouverte.

Les tableaux de la section [Canaux supportés](#supported-channels)
disent, pour chaque canal, ce qui va où.

---

## Acquitter ou mettre en pause depuis la notification {#acknowledge-or-snooze-from-the-notification}

Quand une URL publique est réglée (**Alertes → Notifications → Politique de
notification**, ou `DUMBMONIT_PUBLIC_URL`), chaque message d'alerte porte deux
actions :

- **Acquitter** (« Acknowledge ») : plus de rappels jusqu'à la résolution de
  l'alerte. La résolution reste annoncée.
- **Pause 1 h** (« Snooze 1 h ») : plus de rappels pendant une heure ; ils
  reprennent ensuite si l'alerte est toujours active. Une pause n'écourte jamais
  un acquittement plus long.

Leur présentation dépend du canal :

| Canal | Présentées comme |
|---|---|
| ntfy, Telegram, Web Push | Boutons sous la notification |
| Discord, Slack, courriel et tous les autres canaux | Liens en fin de message |
| Webhook personnalisé | Liens dans `{{message}}`, et les variables `{{ack_url}}` / `{{snooze_url}}` |
| PagerDuty, Opsgenie | Rien : acquittez dans le service lui-même |

Telegram n'accepte de boutons que vers une adresse `https` avec un nom de domaine ;
avec une adresse IP ou une URL en `http`, les actions restent des liens dans le
texte. Web Push ne montre les boutons que sur les appareils des opérateurs et des
administrateurs : un lecteur voit l'alerte, pas de quoi la faire taire.

Un lien ouvre une courte page de confirmation dans DumbMonit : **rien ne se passe
tant que vous n'appuyez pas sur le bouton**. Les messageries, applications de chat
et antivirus ouvrent les liens pour construire un aperçu ; ils ne font que charger
la page. Aucune connexion n'est nécessaire : le lien est l'autorisation, et
seulement pour ce geste, sur les alertes de ce message, une seule fois, pendant
24 heures. L'action est inscrite au journal d'audit, et l'alerte indique
« acknowledged by via notification link ».

La sécurité, en bref : chaque lien est signé avec une clé dérivée du secret
d'instance (changer le secret invalide tous les liens), ne porte aucune session, et
voyage après le `#` de l'adresse, que les navigateurs n'envoient jamais au serveur :
il n'apparaît donc dans aucun journal d'accès ni de mandataire. Un lien forgé,
altéré ou expiré reçoit la même réponse que n'importe quel lien invalide, et les
tentatives répétées depuis une même adresse sont freinées.

Désactivez **Acquitter et mettre en pause depuis les notifications** dans la
politique de notification pour ne plus les ajouter ; cela invalide aussi les liens
déjà envoyés. Vider l'URL publique a le même effet.

---

## Le canal personnalisé {#the-custom-channel}

Le canal **`webhook`** est la trappe de secours de DumbMonit. Vous décrivez
toute la requête HTTP — méthode, adresse, en-têtes, type de contenu et
corps —, ce qui vous laisse brancher presque n'importe quel service, y
compris une API interne que DumbMonit ne connaîtra jamais.

**Sans aucune configuration particulière**, il envoie déjà une charge
utile JSON structurée qu'un script peut utiliser :

```json
{
  "source": "dumbmonit",
  "target": "nas",
  "rule": "Disk full",
  "severity": "warning",
  "status": "firing",
  "title": "nas — Disk full",
  "text": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "value": 95.0,
  "threshold": 90.0,
  "unit": "%",
  "fingerprint": "nas/disk-full",
  "count": 1,
  "at": "2026-09-01T14:12:05+00:00",
  "link": ""
}
```

### Réglages du canal personnalisé

| Réglage | Requis | Défaut | Rôle |
|---|---|---|---|
| `url` | oui (ici ou comme secret) | — | Adresse à appeler. Peut contenir des variables. |
| `method` | non | `POST` | `POST`, `PUT` ou `GET`. |
| `content_type` | non | `json` | `json`, `form` ou `text`. Détermine aussi comment les variables sont échappées. |
| `body_template` | non | — | Modèle de corps. Quand absent, la charge utile ci-dessus est envoyée. |
| `headers` | non | — | En-têtes supplémentaires, comme un objet `{"X-Key": "value"}`. |
| `base_url` | non | — | Adresse publique de votre DumbMonit, pour que `{{link}}` donne un lien cliquable. |
| `username` | non | — | Nom d'utilisateur pour l'authentification HTTP « Basic ». Demande le secret `password`. |

| Secret | Rôle |
|---|---|
| `url` | Utilisez-le au lieu du réglage `url` si l'adresse contient déjà un jeton. |
| `token` | Envoyé comme en-tête `Authorization: Bearer …`, sauf si votre modèle utilise `{{token}}`. |
| `password` | Mot de passe pour l'authentification « Basic ». |

### Variables disponibles {#available-variables}

Une variable s'écrit entre **doubles accolades** : `{{title}}`. Les
espaces autour du nom sont tolérés (`{{ title }}`). Les accolades
simples dans un corps JSON restent littérales, vous pouvez donc écrire
votre JSON normalement.

| Variable | Ce qu'elle contient | Exemple |
|---|---|---|
| `{{title}}` | Titre complet du message | `nas — Disk full` |
| `{{message}}` | Corps en texte brut, une ligne par alerte | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | Le même corps en Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Nom de la règle d'alerte | `Disk full` |
| `{{target}}` | Nom de l'appareil surveillé | `nas` |
| `{{target_id}}` | Identifiant numérique de l'appareil | `42` |
| `{{severity}}` | Gravité technique | `info`, `warning` ou `critical` |
| `{{severity_fr}}` | Gravité comme un mot lisible | `information`, `avertissement`, `critique` |
| `{{status}}` | Statut technique | `firing` ou `resolved` |
| `{{status_fr}}` | Statut comme un mot lisible | `en cours` ou `résolu` |
| `{{value}}` | Valeur mesurée, unité incluse | `95 %` |
| `{{value_raw}}` | Valeur nue, utilisable comme **nombre** JSON | `95` |
| `{{threshold}}` | Seuil franchi, unité incluse | `90 %` |
| `{{threshold_raw}}` | Seuil nu, utilisable comme **nombre** JSON | `90` |
| `{{unit}}` | Unité de la valeur | `%` |
| `{{operator}}` | Comparaison de seuil | `>` |
| `{{count}}` | Nombre d'alertes groupées dans ce message | `3` |
| `{{fingerprint}}` | Identifiant stable de l'alerte | `nas/disk-full` |
| `{{timestamp}}` | Horodatage ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Horodatage en secondes depuis 1970 | `1788358325` |
| `{{date}}` | Horodatage lisible, en UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Priorité de 1 (basse) à 5 (haute) | `4` |
| `{{color}}` | Couleur d'accent, sans dièse | `D98A00` |
| `{{color_hex}}` | Couleur d'accent, avec dièse | `#D98A00` |
| `{{emoji}}` | Pictogramme de statut | `🔴`, `⚠️`, `ℹ️` ou `✅` |
| `{{link}}` | Lien vers l'appareil dans DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Toujours `dumbmonit` | `dumbmonit` |
| `{{token}}` | Le secret `token` du canal | *(votre jeton)* |
| `{{ack_url}}` | Lien signé qui acquitte l'alerte jusqu'à sa résolution, vide si les [liens d'action](#acknowledge-or-snooze-from-the-notification) sont désactivés | `https://dumbmonit.home/alert-action#…` |
| `{{snooze_url}}` | Lien signé qui met l'alerte en pause une heure, vide si les liens d'action sont désactivés | `https://dumbmonit.home/alert-action#…` |

Deux remarques utiles :

- **Certaines variables peuvent être vides.** Une alerte de disponibilité
  (« hôte inaccessible ») n'a ni valeur ni seuil : `{{value}}` et
  `{{value_raw}}` rendent alors une chaîne vide, jamais `null` ni `NaN`.
  Si vous les placez comme un nombre JSON, gérez ce cas — ou utilisez
  `"{{value}}"` entre guillemets.
- **`{{link}}` reste vide** jusqu'à ce que vous remplissiez `base_url`. Un
  lien faux gaspillerait plus de temps qu'une alerte sans lien.

**Vous n'avez jamais besoin d'échapper quoi que ce soit vous-même.** Selon
le `content_type` choisi, DumbMonit protège automatiquement les valeurs
substituées : guillemets et retours à la ligne échappés en JSON,
encodage pourcentage dans un formulaire, aucune transformation en texte
brut. Une règle nommée `"Système" disque plein` ne cassera pas votre
modèle JSON.

### Exemple 1 — une API JSON interne {#example-1--an-internal-json-api}

Une application maison expose `POST /incidents` et attend un jeton dans un
en-tête personnalisé.

**Réglages**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "headers": {
    "X-Application": "dumbmonit"
  },
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Secrets**

```json
{ "token": "your-application-token" }
```

Le jeton est envoyé automatiquement comme en-tête
`Authorization: Bearer …`. Voici ce que l'application reçoit :

```json
{
  "title": "nas — Disk full",
  "device": "nas",
  "severity": "warning",
  "state": "firing",
  "value": 95,
  "threshold": 90,
  "detail": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "link": "https://dumbmonit.home/targets/42",
  "seen_at": "2026-09-01T14:12:05+00:00"
}
```

### Exemple 2 — un service qui attend un formulaire {#example-2--a-service-that-expects-a-form}

Beaucoup de passerelles SMS et d'anciennes API attendent
`application/x-www-form-urlencoded` plutôt que du JSON, avec le jeton
dans le corps.

**Réglages**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Secrets**

```json
{ "token": "gateway-key" }
```

Comme le modèle utilise déjà `{{token}}`, DumbMonit **n'ajoute pas**
d'en-tête `Authorization` : certains services rejettent une requête qui
porte deux authentifications. Les valeurs sont encodées automatiquement,
les espaces devenant `+` et les caractères accentués leur équivalent
encodé en pourcentage.

### Exemple 3 — un SMS en texte brut via un opérateur {#example-3--a-plain-text-sms-through-a-carrier}

Certains services, comme l'API SMS d'un opérateur ou une automatisation
maison, veulent une seule ligne de texte.

**Réglages**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

Le message envoyé, sans aucun échappement :

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Exemple 4 — une requête GET sans corps {#example-4--a-get-request-without-a-body}

Quand le service n'accepte que des paramètres d'URL, mettez les variables
dans `url` et choisissez la méthode `GET`. Une requête `GET` n'a pas de
corps : `body_template` est alors refusé, pour que personne ne croie
qu'il fait quelque chose.

**Réglages**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Secrets**

```json
{ "token": "api-key" }
```

Seules les **valeurs substituées** sont encodées ; le reste de l'adresse
est copié tel quel. La requête réellement envoyée ressemble à :

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Erreurs de template courantes {#common-template-mistakes}

Un modèle défectueux est **refusé à l'enregistrement**, pendant que vous
regardez le formulaire, pas la nuit où l'incident arrive.

| Ce que vous écrivez | Ce qui se passe |
|---|---|
| `{{severite}}` | Refusé : variable inconnue « severite », suivie de la liste des noms valides. Le bon nom est `severity`. |
| `{"t": "{{title"}` | Refusé : « accolades non fermées ». Le `}}` manque. |
| `{{}}` ou `{{  }}` | Refusé : une variable sans nom ne signifie rien. |
| `{"a": {"b": 1}}` | **Accepté.** Les accolades simples sont littérales : écrivez votre JSON normalement. |
| `{{token}}` sans secret `token` | Refusé : le modèle demande un jeton que le canal n'a pas. |
| `method: GET` avec `body_template` | Refusé, avec l'alternative : mettez les variables dans `url`. |
| `method: DELETE` | Refusé : seuls `POST`, `PUT` et `GET` sont acceptés. |

---

## Canaux supportés {#supported-channels}

### Discord

Dans Discord : **Réglages du canal → Intégrations → Webhooks → Nouveau
webhook**, puis **Copier l'URL du webhook**. Cette URL contient le
jeton : elle va dans les secrets.

- **Secrets** — `webhook_url`

### Slack

Sur <https://api.slack.com/apps>, créez une application, activez
**Incoming Webhooks**, puis **Add New Webhook to Workspace** et choisissez
le canal. Copiez l'URL `https://hooks.slack.com/services/…`.

- **Secrets** — `webhook_url`

### Telegram

Il vous faut deux choses : le **jeton de bot** (secret) et le
**chat ID** (`chat_id`) où il doit écrire. Le premier prend une minute à
obtenir ; le second dépend d'où vous voulez recevoir les alertes.

#### 1. Créer le bot et obtenir son jeton

1. Dans Telegram, cherchez **@BotFather** et cliquez sur *Start*.
2. Envoyez `/newbot`, puis répondez aux questions (un nom, puis un nom
   d'utilisateur qui doit se terminer par `bot`).
3. BotFather termine avec un message comme ceci :

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   La ligne `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` est le **jeton de
   bot**. Ne le partagez pas : il donne le contrôle complet du bot.

Dans les exemples ci-dessous, remplacez `<TOKEN>` par ce jeton. Notez le
préfixe `bot` collé devant dans les URL :
`https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Trouver le `chat_id`

**Chat privé** (le bot vous écrit directement)

1. Ouvrez votre bot (`t.me/my_homelab_bot`) et cliquez sur *Start*, ou
   envoyez-lui n'importe quel message. Sans ça, il ne pourra jamais vous
   écrire.
2. Ouvrez dans un navigateur : `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Vous obtenez un document JSON ; cherchez `"chat"` :

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   Le `chat_id` est la valeur de `chat.id` : ici `21xxxxx38` (un nombre
   positif).

   Si `result` est vide (`"result":[]`), envoyez un autre message au bot
   et rechargez la page.

**Canal**

1. Ajoutez le bot comme administrateur du canal (Manage channel →
   Administrators → Add).
2. Postez un message dans le canal.
3. Ouvrez `https://api.telegram.org/bot<TOKEN>/getUpdates` : la réponse
   contient un bloc `channel_post` avec
   `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   Le `chat_id` est ce nombre négatif, `-100` inclus : `-1001xxxxxx062`.

**Groupe**

Le plus simple passe par l'application de bureau :

1. Ajoutez le bot au groupe.
2. Envoyez un message dans le groupe, clic droit sur lui → *Copy message
   link*. Vous obtenez `https://t.me/c/194xxxx987/13` (ou
   `https://t.me/c/194xxxx987/11/13` si le groupe a des sujets).
3. Le nombre juste après `/c/` est l'identifiant interne du groupe :
   `194xxxx987`. Le `chat_id` à saisir est ce nombre **préfixé avec
   `-100`** : `-100194xxxx987`.

Sans l'application de bureau, la méthode `getUpdates` ci-dessus
fonctionne aussi : `message.chat.id` est directement le bon nombre (déjà
négatif). Si le bot ne voit pas les messages du groupe, demandez à
BotFather `/setprivacy` → *Disable*, puis retirez le bot du groupe et
rajoutez-le.

**Sujet de groupe** (groupes en mode « Topics »)

Le `chat_id` est celui du groupe. En plus, remplissez **Sujet de groupe**
(`message_thread_id`) avec le nombre du milieu du lien copié : dans
`https://t.me/c/194xxxx987/11/13`, le sujet est `11`. Sans ce réglage, le
message atterrit dans le fil *General*.

#### 3. Vérifier avant d'enregistrer

Ouvrez dans le navigateur :

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(ajoutez `&message_thread_id=11` pour un sujet). Si `test123` arrive, le
jeton et le `chat_id` sont bons : saisissez-les dans DumbMonit et cliquez
sur *Test*.

- **Réglages** — `chat_id` (requis), `message_thread_id` (sujet,
  optionnel), `api_base` (défaut `https://api.telegram.org`, changez-le
  seulement pour passer par un relais)
- **Secrets** — `bot_token`

> **« Telegram cannot find this chat »** — pour un chat privé, le bot n'a
> jamais reçu de message de vous : envoyez-lui `/start`. Pour un groupe ou
> un canal, le préfixe `-100` manque probablement, ou le bot n'y a pas été
> ajouté.
>
> **« thread not found »** — le `message_thread_id` ne correspond à aucun
> sujet du groupe, ou le groupe n'est pas en mode « Topics ».

### Microsoft Teams

**Attention : la méthode a changé.** Les anciens « Office 365 connectors »
ont été retirés en mai 2026 ; une URL `outlook.office.com/webhook/…` ne
fonctionne plus.

La procédure actuelle :

1. Dans le canal Teams, **⋯ → Workflows**.
2. Choisissez le modèle **« Post to a channel when a webhook request is
   received »**.
3. Confirmez les étapes, puis **copiez l'URL HTTPS POST** montrée à la
   fin.

- **Secrets** — `webhook_url`

DumbMonit envoie une *Adaptive Card* encapsulée, le seul format que ces
flux affichent correctement. Si vous obtenez une erreur mentionnant Power
Automate, l'URL vient probablement d'un ancien connecteur.

### Matrix

1. Créez un compte dédié à DumbMonit sur votre homeserver, et
   **invitez-le dans la salle**. Un compte qui n'en est pas membre ne
   peut pas écrire.
2. Obtenez son jeton d'accès. Dans Element : **Settings → Help & About →
   Advanced → Access token**.
3. Obtenez l'identifiant **interne** de la salle — pas son nom
   d'affichage. Dans Element : **Room settings → Advanced → Internal room
   ID**. Il commence par `!` (par exemple `!aBcDeF:matrix.org`).

- **Réglages** — `server_url` (par exemple `https://matrix.org`), `room_id`
- **Secrets** — `token`

### Mattermost

**Integrations → Incoming Webhooks → Add**. Choisissez le canal par
défaut, puis copiez l'URL.

- **Réglages** — `channel` (optionnel, pour écrire ailleurs que le canal
  par défaut), `username` (optionnel, défaut `DumbMonit`)
- **Secrets** — `webhook_url`

### Rocket.Chat

**Administration → Integrations → New integration → Incoming webhook**.
Choisissez la salle, activez l'intégration, puis copiez l'*URL du
webhook*.

- **Réglages** — `channel` (optionnel), `alias` (optionnel, défaut
  `DumbMonit`)
- **Secrets** — `webhook_url`

### Google Chat

Dans l'espace Google Chat : **nom de l'espace → Apps & integrations →
Webhooks → Add webhook**. Copiez l'URL, qui contient déjà la clé et le
jeton.

- **Secrets** — `webhook_url`

### Zulip

1. Créez un **bot** : **Personal settings → Bots → Add a new bot**, type
   « Generic ». Notez son adresse email et sa clé d'API.
2. Abonnez le bot au stream cible, sinon l'envoi sera refusé.

- **Réglages** — `server_url` (par exemple `https://your-org.zulipchat.com`),
  `email` (l'adresse du bot), `stream` (le nom du stream), `topic`
  (optionnel, défaut `DumbMonit`)
- **Secrets** — `api_key`

### ntfy

Le canal le plus simple de tous : choisissez un nom de sujet difficile à
deviner, puis abonnez-vous-y depuis l'application ntfy.

- **Réglages** — `topic`, `server_url` (optionnel, défaut `https://ntfy.sh`)
- **Secrets** — `token` (seulement si votre serveur exige une
  authentification)

> Sur l'instance publique, **n'importe qui connaissant le nom d'un sujet
> peut le lire**. Choisissez un nom long et aléatoire, ou hébergez votre
> propre serveur.

### Gotify

Dans l'interface Gotify : **Apps → Create Application**. Copiez le jeton
affiché.

- **Réglages** — `server_url` (par exemple `https://gotify.home`)
- **Secrets** — `token`

### Pushover

1. Sur <https://pushover.net>, votre **User Key** est affichée directement
   sur la page d'accueil.
2. Puis créez une application : **Create an Application/API Token**. Vous
   obtenez un **jeton d'application**.

- **Réglages** — `priority` (optionnel, de -2 à 2), `sound` (optionnel),
  `retry` et `expire` (seulement pour `priority: 2`)
- **Secrets** — `token` (jeton d'application), `user_key` (clé
  utilisateur)

Sans réglage `priority`, DumbMonit choisit selon la gravité : `-1` pour un
avis ou une résolution, `0` pour un avertissement, `1` pour une alerte
critique. Un avis ne fait donc pas sonner le téléphone.

> La priorité `2` (« emergency ») répète la notification jusqu'à ce qu'elle
> soit acquittée. DumbMonit ajoute alors `retry` et `expire`
> automatiquement, sans quoi Pushover rejetterait le message.

### Pushbullet

Sur <https://www.pushbullet.com/#settings/account>, cliquez sur **Create
Access Token**.

- **Réglages** — `device_iden` (optionnel ; sans lui, tous vos appareils
  sont notifiés)
- **Secrets** — `token`

### Bark (iOS)

Ouvrez l'application Bark sur votre iPhone : elle montre une URL
contenant votre **clé d'appareil**, la longue chaîne de caractères après
le nom de domaine.

- **Réglages** — `server_url` (optionnel, défaut `https://api.day.app`),
  `group` (optionnel), `level` (optionnel), `sound` (optionnel)
- **Secrets** — `token` (la clé d'appareil)

Sans réglage `level`, DumbMonit choisit : `passive` pour un avis, `active`
pour un avertissement, `timeSensitive` pour une alerte critique. Le niveau
`critical`, qui traverse « Ne pas déranger », n'est jamais choisi
automatiquement — réglez-le explicitement si vous le voulez.

### Web Push

Notifications natives sur vos téléphones et ordinateurs, directement
depuis votre propre DumbMonit, sans application à installer et sans
compte tiers : le navigateur (installé comme application ou non) reçoit
les alertes même avec chaque onglet fermé.

1. Sur chaque téléphone ou ordinateur, ouvrez DumbMonit et allez à
   **Réglages → Notifications push → Activer sur cet appareil**, puis
   **Envoyer un test**. Sur iPhone et iPad, ajoutez d'abord DumbMonit à
   l'écran d'accueil. Le guide pas à pas est dans
   [Notifications push (PWA)](using/push-notifications.md).
2. Créez un canal **Web Push** ici. Il n'a pas de secret : ses
   destinataires sont les appareils abonnés à l'étape 1.

- **Réglages** — `users` (optionnel ; un compte par ligne, vide : chaque
  appareil abonné de chaque compte), `urgency` (`high` par défaut,
  `normal` ou `low`), `ttl` (secondes que le service de push garde le
  message pour un appareil éteint, défaut `86400`), `contact` (adresse
  `mailto:` ou `https:` optionnelle donnée aux services de push ; vide :
  `DUMBMONIT_PUBLIC_URL` quand elle est en HTTPS, sinon la page du projet)
- **Secrets** — aucun. La paire de clés VAPID du serveur est générée au
  premier démarrage et stockée chiffrée ; la clé privée ne quitte jamais
  le serveur.

Le titre de la notification est la gravité et l'appareil
(`Critical · nas01`), le corps un court résumé ; toucher ouvre la page
**Alertes**. DumbMonit a besoin de HTTPS sortant vers les services de
push (`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`) ; les messages
sont chiffrés de bout en bout, ces services ne peuvent pas les lire. Un
abonnement que le service de push rapporte comme disparu (HTTP 404 ou
410, par exemple après que l'application a été supprimée) est supprimé
automatiquement.

### Apprise

Apprise est une **passerelle** : avec une seule configuration elle donne
accès à des dizaines de services que DumbMonit n'implémente pas
lui-même. Déployez le conteneur `caronc/apprise`, puis choisissez l'un
des deux modes.

**Mode recommandé — configuration stockée dans Apprise.** Créez une
configuration dans l'interface Apprise, notez sa clé, et ne mettez rien
d'autre ici.

- **Réglages** — `server_url`, `config_key`

**Mode direct — DumbMonit fournit les destinations.** Les URL Apprise
contiennent souvent un mot de passe (`mailto://user:password@…`) : elles
vont donc dans les secrets.

- **Réglages** — `server_url`
- **Secrets** — `urls` (une ou plusieurs URL Apprise, séparées par des
  virgules)

Réglages communs optionnels : `tag` (pour ne cibler qu'une partie des
destinations d'une configuration) et `format` (`text` par défaut, ou
`markdown`).

> Si le test répond « Apprise has no configuration under this config_key »,
> la clé ne correspond à rien. DumbMonit le rapporte plutôt que de
> prétendre que le message a été envoyé.

### Home Assistant

1. Dans Home Assistant : **votre profil → Security → Long-lived access
   tokens → Create token**. Copiez-le, il ne sera plus montré.
2. Choisissez le service à appeler. Par défaut, DumbMonit crée une
   notification persistante sur le tableau de bord.

- **Réglages** — `server_url` (par exemple
  `http://homeassistant.local:8123`), `service` (optionnel, défaut
  `persistent_notification.create`), `target` (optionnel), `data`
  (optionnel, objet fusionné dans l'appel)
- **Secrets** — `token`

Quelques services utiles :

| `service` | Effet |
|---|---|
| `persistent_notification.create` | Bannière dans l'interface Home Assistant |
| `notify.notify` | Toutes les cibles de notification configurées |
| `notify.mobile_app_<device>` | L'application mobile d'un appareil spécifique |
| `light.turn_on` | Allume une lumière — utilisez `data` pour l'entité et la couleur |

Le réglage `data` est fusionné **en dernier** : il peut donc surcharger le
titre ou le message que DumbMonit a préparé.

### Email (SMTP)

Le seul canal qui ne dépend d'aucun service tiers.

- **Réglages** — `host`, `from`, `to` (liste d'adresses), `security`
  (`starttls` par défaut, ou `tls`, ou `none`), `port` (dérivé de
  `security` : 587, 465 ou 25), `username` (optionnel)
- **Secrets** — `password`

> Avec Gmail, un mot de passe de compte ne fonctionne pas : créez un
> **mot de passe d'application** dans les réglages de sécurité de votre
> compte Google.

### Signal

Passe par une instance
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api)
que vous hébergez et où vous enregistrez d'abord votre numéro.

- **Réglages** — `server_url` (par exemple `http://signal.home:8080`),
  `number` (le numéro expéditeur enregistré), `recipients` (liste de
  numéros ou d'identifiants de groupe), `username` (optionnel)
- **Secrets** — `password` (optionnel, si votre instance est protégée)

Tous les numéros s'écrivent au **format international**,
`+33612345678`.

### SMS via Twilio

Sur la console Twilio, la page d'accueil montre votre **Account SID** et
votre **Auth Token**. Achetez ensuite un numéro qui peut envoyer des SMS.

- **Réglages** — `account_sid`, `from` (votre numéro Twilio) **ou**
  `messaging_service_sid`, `to` (liste de destinataires)
- **Secrets** — `token` (l'*Auth Token*)

Le corps du SMS est tronqué à 300 caractères : au-delà, Twilio le découpe
en segments facturés séparément.

> Sur un compte d'essai, Twilio n'envoie qu'aux numéros que vous avez
> vérifiés.

### PagerDuty

Dans PagerDuty : **Services → votre service → Integrations → Add
integration**, et choisissez **Events API v2**. Copiez l'*Integration
Key*.

- **Réglages** — `region` (`eu` si votre compte est hébergé en Europe),
  `source` (optionnel, défaut `dumbmonit`)
- **Secrets** — `token` (l'*Integration Key*)

Une alerte ouvre un incident ; sa résolution **le ferme
automatiquement**. La correspondance se fait sur l'empreinte de
l'alerte, préfixée avec `dumbmonit-`, un rappel n'ouvre donc jamais un
second incident.

> Une clé d'un autre type (« Events API v1 », « REST API ») obtient un
> refus sec. Vérifiez que l'intégration est bien de type *Events API v2*.

### Opsgenie

**Teams → votre équipe → Integrations → Add integration → API**. Copiez
la clé d'API.

- **Réglages** — `region` (`eu` pour un compte européen), `responders`
  (liste de noms d'équipe, optionnel), `tags` (liste, optionnel)
- **Secrets** — `api_key`

Comme avec PagerDuty, une résolution ferme l'alerte, identifiée par son
alias.

> Atlassian a fixé la **fin de service d'Opsgenie au 5 avril 2027**. Ce
> canal reste pour les installations existantes ; pour une nouvelle,
> préférez PagerDuty ou un canal personnalisé pointant vers Jira Service
> Management.

### Webhook personnalisé

Voir [Le canal personnalisé](#the-custom-channel) ci-dessus.

---

## Dépannage {#troubleshooting}

**Le test échoue immédiatement, avec « missing setting … ».**
Un champ requis est vide. Le message nomme le réglage exact.

**Le test échoue avec « no answer within 15 s ».**
Le serveur cible est inaccessible depuis DumbMonit. S'il s'agit d'un
service sur votre réseau local, vérifiez que le conteneur DumbMonit peut
l'atteindre — c'est fréquemment un problème de réseau Docker, pas un
problème de configuration de canal.

**Le test réussit, mais je ne reçois rien.**
Le service a accepté le message et l'a envoyé ailleurs : mauvais canal,
mauvais sujet ntfy, mauvais appareil Pushbullet. Vérifiez le réglage de
destination.

**Le message d'erreur contient `***`.**
C'est volontaire : votre jeton a été retiré du message avant qu'il ne
soit enregistré. Aucun secret n'apparaît jamais dans les logs, dans la
colonne « dernière erreur », ni dans une réponse d'API.

**Où puis-je voir la dernière erreur d'un canal ?**
Dans la liste des canaux : chaque ligne montre la date de la dernière
livraison réussie et, s'il y en a, l'erreur du dernier échec.
