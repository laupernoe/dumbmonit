# Changements de site web

Surveillez le *contenu* d'une page web, ou d'un site entier, et soyez
informé quand il change : une page de tarifs ou de conditions, un site de
documentation, l'avis de maintenance d'un fournisseur, une page d'annonce
publique, ou votre propre site après un déploiement. Chaque changement est
conservé avec le texte avant et après, un diff ligne par ligne et, quand un
navigateur est disponible, une capture d'écran de chaque version.

Ce qui est comparé est le **texte visible** de chaque page, pas son HTML :
scripts, styles, commentaires, blocs `noscript`, `svg` et `template` sont
abandonnés, les espaces sont réduits et chaque bloc (paragraphe, titre,
élément de liste, ligne de tableau) devient une ligne. Une nouvelle
compilation de la même page, un attribut réordonné ou un nouveau nom de
ressource anti-cache ne change rien ; une phrase reformulée si.

### Quelle partie de la page {#what-part-of-the-page}

La **navigation, l'en-tête, le pied de page et les encadrés latéraux** du site
sont écartés : ils sont les mêmes sur toutes les pages, et une nouvelle entrée
de menu ferait sinon signaler chaque page comme changée en une seule
vérification. Ils suivent les rôles que leur donne le HTML : chaque `nav`, un
`header` ou un `footer` qui n'est pas dans un `article`, `aside`, `main`, `nav`
ou `section` (l'en-tête d'un article est gardé), un `aside` hors d'un `article`
et d'un `main`, et tout élément de rôle `navigation`, `banner`, `contentinfo` ou
`search`. Leurs liens restent suivis pour trouver les pages d'un site.
**Comparer menus, en-tête et pied de page** désactive cela.

**Zone surveillée** restreint la comparaison à certains éléments, par exemple
`main` ou `#content` ; une page où aucun n'est trouvé est lue en entier plutôt
que comme vide. **Zones exclues** écarte d'autres éléments : un bandeau de
cookies, un encadré « derniers articles », un emplacement publicitaire. Les
deux prennent des sélecteurs simples, séparés par des virgules : une balise
(`main`), un id (`#content`), des classes (`.cookie-banner`), des attributs
(`[role=alert]`, `[data-ad]`), ou plusieurs ensemble
(`div.notice[role=alert]`). Les descendants (`main p`) et autres combinateurs
ne sont pas pris en charge.

## Page ou site

**Page** (le défaut) lit l'adresse de l'appareil et rien d'autre. Ça peut
aussi être un document texte brut ou JSON, comparé ligne par ligne.

**Site** part de l'adresse et suit chaque page sous le même dossier
(`https://example.com/docs/intro.html` couvre tout sous `/docs/` ;
**Préfixe de chemin** choisit un autre dossier). Les pages sont trouvées :

1. dans le sitemap du site : celles que `robots.txt` annonce (lignes
   `Sitemap:`), ou `/sitemap.xml`, index de sitemap inclus (un niveau) ;
2. à défaut, ou quand le sitemap ne liste rien sous le dossier, en suivant
   les liens de chaque page, en largeur d'abord.

Seules les pages de la même origine (schéma, hôte et port) sont suivies ;
les liens vers images, feuilles de style, scripts, PDF, archives et autres
fichiers sont ignorés par leur extension, et toute réponse qui n'est pas
HTML est ignorée. Les fragments (`#section`) sont abandonnés et les
requêtes vides retirées, une page est donc une adresse. `robots.txt` est
respecté pour les pages trouvées ainsi : le groupe qui nomme `DumbMonit`,
ou sinon le groupe `*`, avec `Allow`, `Disallow`, `*` et `$`.

Le crawl est poli : deux requêtes à la fois au plus, une demi-seconde
entre deux lots, le `User-Agent` `DumbMonit/<version>`, un délai de 15
secondes par requête et 5 Mo lus au plus par page. Il s'arrête à
**Pages maximum** (25 par défaut, 200 au plus) ou après dix minutes.

## Comment une vérification fonctionne

L'appareil est sondé comme n'importe quel autre (chaque minute par
défaut), mais les pages ne sont lues que toutes les **Vérifier toutes les**
minutes (60 par défaut, de 5 à 10080). Entre deux, la sonde ne rapporte que
l'état de la dernière vérification : le site n'est pas touché chaque
minute. Une vérification tourne en arrière-plan ; deux vérifications du
même appareil ne se chevauchent jamais, et **Vérifier maintenant** sur la
page de l'appareil en démarre une tout de suite.

La première vérification n'enregistre que la **référence** : chaque page
est lue et conservée, rien n'est rapporté. À partir de là, pour chaque
page :

| Situation | Ce qui est conservé |
|---|---|
| Même texte que la dernière fois (après **Ignorer les lignes**) | Rien de nouveau ; la page est marquée vérifiée. |
| Texte différent | Un nouvel instantané et un événement **changée**, avec le nombre de lignes ajoutées et retirées. |
| Une page jamais vue avant, ou revue après avoir disparu | Un instantané et un événement **nouvelle page**. |
| La page répond 404 ou 410 | Un événement **page supprimée**, le dernier instantané comme « avant ». |
| Portée site : un crawl complet n'atteint plus une page connue | Un événement **page supprimée** aussi (« Plus liée »). Un crawl arrêté par **Pages maximum** ou par sa limite de temps ne conclut jamais qu'une page a disparu. |
| Toute autre erreur sur une page (délai dépassé, 403, 500 sur une sous-page) | L'erreur est montrée sur la page ; aucun événement. |

Changer l'adresse, la portée, le préfixe de chemin, **Pages maximum**, les
zones ou **Ignorer les lignes** fait de la prochaine vérification une nouvelle
référence plutôt que de rapporter chaque page comme changée (ou, pour un
**Pages maximum** plus élevé, chaque page ajoutée comme nouvelle). Une mise à
jour de DumbMonit qui lit les pages autrement fait de même, une fois.

Quand une vérification trouve la **même modification sur plusieurs pages**
(les mêmes lignes retirées et ajoutées, comme un bandeau), la page de
l'appareil la montre une seule fois, avec les pages concernées.

### Ignorer les lignes

Des lignes qui changent à chaque visite — une horloge, un compteur de
visiteurs, « mis à jour il y a 5 minutes », une citation aléatoire —
seraient rapportées à chaque vérification. **Ignorer les lignes** contient
des expressions régulières (syntaxe `regex` de Rust), une par ligne ou
jointes avec `|` ; tout texte de ligne que l'une d'elles correspond est
laissé de côté avant comparaison. Par exemple :

```
Updated \d+ minutes ago|Visitors: \d+|© \d{4}
```

### Page de départ inaccessible

Quand la page de départ elle-même ne peut pas être lue (pas de réponse,
connexion refusée, délai dépassé, HTTP 5xx), l'appareil est
**inaccessible** : la sonde échoue, et la règle « Appareil inaccessible »
se déclenche comme pour tout appareil. La page de départ est alors
retentée à chaque sonde, son retour est donc vu dans la minute. Une page de
départ qui refuse l'accès (401, 403) est rapportée comme un problème de
configuration, pas comme une panne. Les erreurs sur d'autres pages du site
ne sont montrées que sur ces pages.

Les adresses que seul l'hôte DumbMonit lui-même peut atteindre (loopback,
link-local, le service de métadonnées cloud) sont refusées sauf si
**Autoriser les cibles loopback et link-local** est activée ; les adresses
LAN privées sont toujours autorisées.

## Alertes

La règle intégrée **Site web changé** (`webchange_detected`, Info) se
déclenche après chaque vérification qui a trouvé au moins un changement
(page changée, nouvelle ou supprimée), et la notification nomme les pages :
« /tarifs, /contact and 2 more ». Deux vérifications de suite qui trouvent
toutes deux des changements donnent deux notifications, même sur les mêmes
pages ; l'alerte d'une vérification se résout à la suivante. Elle lit
`dumbmonit_webchange_last_check_changes` ; attachez-la à un canal comme
toute autre règle, ou augmentez sa gravité, dans **Alertes → Règles**. Voir
[Règles intégrées](../alerting/rules.md#website-changes).

| Métrique | Quoi |
|---|---|
| `dumbmonit_webchange_pages` | Pages suivies (pages supprimées exclues). |
| `dumbmonit_webchange_pages_failed` | Pages suivies dont la dernière lecture a échoué. |
| `dumbmonit_webchange_last_check_changes` | Changements trouvés par la dernière vérification ; quand il y en a, l'étiquette `changes` liste les pages et `check` l'heure de la vérification. |

Les métriques apparaissent une fois la première vérification faite.

## Captures d'écran {#screenshots}

L'image DumbMonit n'a pas de navigateur : les captures d'écran viennent
d'un Chromium headless optionnel, piloté via le protocole Chrome DevTools.
Le fichier Compose en fournit un comme service `browser`, sous le profil
`screenshots`, il ne démarre donc que si on le demande :

```
DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d
```

`DUMBMONIT_BROWSER_URL` pointe DumbMonit vers le navigateur (tout Chromium
démarré avec `--remote-debugging-port`, par exemple
`chromedp/headless-shell` ; le serveur lit `/json/version` et parle à
l'adresse que le nom d'hôte résout). Mettez la variable dans un fichier
`.env` à côté de `docker-compose.yml` pour la garder.

Une capture n'est prise que quand une page obtient un nouvel instantané
(la référence, et chaque changement), jamais sur une vérification sans
changement : une fenêtre de 1366 × 900, la page complète jusqu'à 5000
pixels, en JPEG. L'option **Captures d'écran** les désactive pour un
appareil. Sans navigateur, ou quand il ne répond pas, la comparaison de
texte fonctionne pareil et les instantanés n'ont simplement pas d'image ;
la page de l'appareil le dit.

Le navigateur charge la page comme le ferait un visiteur, avec ses
scripts : ce qu'il montre peut différer du texte que DumbMonit a comparé,
qui est le HTML envoyé par le serveur.

## Stockage

Le texte de chaque instantané est gardé dans la base de données (1 Mo au
plus par page) ; les captures d'écran sont des fichiers sous
`/data/webchange/<id de l'appareil>/`. Par appareil, les 500 derniers
changements sont gardés ; un instantané est gardé tant qu'il est le plus
récent d'une page ou qu'un changement gardé s'y réfère, et tout le reste,
fichiers inclus, est supprimé après chaque vérification. Supprimer
l'appareil retire tout cela.

## Surveiller un site web pour des changements

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.

1. Dans l'adresse, collez la page à surveiller, avec http:// ou https://.
   Pour surveiller tout un site ou une de ses sections, collez sa page de
   départ et réglez « Portée » sur « site » : chaque page sous le même
   dossier est suivie, jusqu'à « Pages maximum ».
2. Les lignes qui changent à chaque visite (une horloge, un compteur de
   visiteurs, « mis à jour il y a 5 minutes ») seraient rapportées chaque
   fois : listez-les dans « Ignorer les lignes » comme expressions
   régulières.
3. La première vérification n'enregistre que la référence, rien n'est
   rapporté. À partir de là, chaque vérification compare le texte visible
   page par page et garde une copie avant/après de chaque changement,
   montrée sur la page de l'appareil ; l'alerte « Site web changé » vous
   le dit.
4. Pour une capture d'écran de chaque version, démarrez le service
   navigateur optionnel du fichier Compose et pointez DumbMonit vers lui :

    ```
    DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d
    ```

!!! warning
    Seul le texte envoyé par le serveur est comparé : une page dessinée entièrement par JavaScript montre peu ou rien à la vérification. Les pages derrière une connexion ne sont pas accessibles non plus.

Identifiants : aucun. Les vérifications tournent toujours sur le serveur
DumbMonit : une surveillance de site web ne peut pas passer par un agent
relais.

## Options

| Option | Défaut | Signification |
|---|---|---|
| Portée (`scope`) | `page` | `page` : l'adresse seulement. `site` : chaque page sous le même dossier. |
| Pages maximum (`max_pages`) | `25` | Portée site : pages lues au plus par vérification, de 1 à 200. |
| Préfixe de chemin (`path_prefix`) | dossier de l'adresse | Portée site : seulement les pages dont le chemin commence par ceci (`/docs/`). |
| Ignorer les lignes (`ignore`) | — | Expressions régulières ; les lignes de texte correspondantes ne sont pas comparées. |
| Zone surveillée (`content_selector`) | — | Sélecteurs simples ; seul le texte qu'ils contiennent est comparé. |
| Zones exclues (`exclude_selector`) | — | Sélecteurs simples ; leur texte n'est pas comparé. |
| Comparer menus, en-tête et pied de page (`keep_page_chrome`) | `false` | Comparer aussi la navigation, l'en-tête, le pied de page et les encadrés du site. |
| Captures d'écran (`screenshots`) | `true` | Garder une capture d'écran de chaque nouvelle version, quand un navigateur est configuré. |
| Vérifier toutes les (`check_interval_minutes`) | `60` | Minutes entre deux vérifications, de 5 à 10080. |
| Accepter un certificat non vérifiable (`insecure_tls`) | `false` | Pour un site propre derrière un certificat autosigné. |
| Autoriser les cibles loopback et link-local (`allow_private_targets`) | `false` | Atteindre un site servi par l'hôte DumbMonit lui-même. |
