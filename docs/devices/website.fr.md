# Santé des sites web et SEO

Surveillez les sites web en masse, une surveillance par site : soyez informé quand un site **casse**
(erreur, inaccessible, certificat invalide, boucle de redirection), quand sa page **devient
blanche**, quand elle **disparaît des moteurs de recherche** (`noindex`, `robots.txt`
bloquant tout) et quand les métadonnées que Google affiche **changent** (titre,
description, URL canonique, adresse finale). Chaque site est également trié selon la
plateforme sur laquelle il s'exécute (WordPress, Shopify, Wix, Next.js…).

Pour le contenu d'une page plutôt que sa santé, voir
[Modifications du site web](webchange.md).

## Configuration

1. Dans l'adresse, collez la page à vérifier, généralement la page d'accueil, avec ou sans https://. Les redirections sont suivies : http vers https et example.com vers www.example.com sont normaux, seule une boucle de redirection ou un déplacement vers une autre adresse est signalé.
2. Remplissez optionnellement « Texte attendu » avec une phrase que la page doit toujours afficher, et ajustez « Page vierge en dessous de » si la page est courte à dessein.
3. La première vérification enregistre la référence : adresse finale, titre, description, URL canonique, règles d'indexation. Un changement n'est signalé qu'après **2 vérifications consécutives** (réglable) l'ayant vu, donc les tests A/B et les caches capricieux restent silencieux.
4. Pour ajouter de nombreux sites à la fois, importez un fichier CSV depuis le menu **⋯** de la page Surveillances : chaque site est classé sous `Websites/<catégorie>` (E-commerce, Blog… ou la catégorie que vous donnez) ou, quand la page ne le dit pas, sous `Websites/<platform>` (WordPress, Shopify…), sauf si vous spécifiez un dossier.

!!! warning
    Seul ce que le serveur envoie est lu : une page dessinée entièrement par JavaScript semble vierge ici. Réduisez « Page vierge en dessous de » à 0 pour un tel site.

## Qu'est-ce que chaque vérification fait

À chaque intervalle d'interrogation (60 s par défaut), la page est lue une fois :

- **Les redirections sont suivies** une par une, jusqu'à **Max. redirections** (10 par
  défaut), et la chaîne est enregistrée : chaque adresse et son statut. Passer de
  `http://` à `https://`, ou de `example.com` à `www.example.com`, est ce que
  la plupart des sites font et ne constitue jamais une erreur. Une redirection qui revient à une adresse
  déjà vue est une **boucle**, et trop de redirections que autorisées est aussi une erreur :
  les deux font échouer la vérification avec la chaîne dans le message, comme un site qui ne
  répond pas.
- **La réponse finale** : son statut HTTP, le temps total (redirections incluses)
  et le certificat TLS, vérifié comme le ferait un navigateur. Un certificat refusé
  fait échouer la vérification.
- **Le texte visible** de la page, comme dans [Modifications du site web](webchange.md) :
  les scripts, les styles et les blocs masqués sont laissés de côté. En dessous de **Page vierge en dessous de**
  caractères (20 par défaut, pour que les courtes pages légitimes passent), la page compte comme vierge. Avec
  **Texte attendu**, cette phrase doit y apparaître (casse et espaces ignorés).
- **Les métadonnées SEO** de la page finale : `<title>`, méta-description,
  URL canonique, méta-tags robots et en-têtes `X-Robots-Tag` (`noindex`,
  `nofollow`, `none`), `og:title`, `og:description`, `<html lang>`, le nombre
  de `<h1>` et d'alternates `hreflang`.
- **Une fois par heure**, `robots.txt` du site final : interdit-il à Googlebot
  (ou `*`) d'explorer `/` ? Et le sitemap qu'il annonce, ou `/sitemap.xml` : répond-il
  et se lit-il comme un sitemap ?

L'User-Agent est `DumbMonit/<version>` sauf si **User agent** dit autrement.

## Pages quasi vides {#nearly-empty-pages}

En plus du seuil absolu (**Page vierge en dessous**), DumbMonit garde une référence, qui
évolue lentement, de la taille du texte visible de chaque page. Quand le texte tombe de plus de
**Page quasi vide : chute du texte** pour cent sous cette référence (80 par défaut, soit de
1 000 caractères à moins de 200) et y reste pendant **Confirmer un changement après**
vérifications consécutives, l'alerte « Page du site web quasi vide » se déclenche. Elle
attrape un thème qui a perdu son contenu, un catalogue qui ne charge plus ou un cache qui
sert une coquille, alors que la page répond encore 200. Il faut une référence d'au moins
200 caractères ; la croissance et la décroissance normales ne font pas de bruit, et une chute
qui dure plus d'une semaine devient la nouvelle référence (le site a été refait plus court).
0 désactive la vérification.

## Changements, confirmés deux fois

L'adresse finale et les métadonnées sont comparées à une référence. Une valeur différente
vue une seule fois est seulement **en attente** ; elle devient un changement quand **Confirmer un
changement après** vérifications consécutives (2 par défaut, de 1 à 10) ont lu la même nouvelle valeur.
Une valeur qui revient en arrière, ou une troisième valeur, relance le décompte. Une page servie en deux versions (un test A/B), un cache qui
hésite ou une réponse tronquée restent donc silencieux. Les valeurs sont comparées en
ignorant les espaces et la casse ; les adresses ignorent le schéma, un `www.` initial, un `/`
final et, avec **Ignorer les paramètres de requête** (activé par défaut), les paramètres URL
comme `?utm_source=…`.

Chaque type de vérification peut être désactivé séparément, dans les options « Métadonnées
suivies » : adresse finale, titre, description, URL canonique, directives robots, Open Graph,
langue, nombre de `<h1>`, `hreflang` et balise de vérification Google. Un champ désactivé n'est ni
comparé, ni enregistré, et ne déclenche aucune alerte ; réactivé, il repart d'une référence
neuve. Tout est activé par défaut. Un champ qui disparaît de la page (valeur vide) est un
changement comme un autre.

Un changement confirmé est conservé dans l'historique de la surveillance (champ, avant, après,
date), et son jauge (`dumbmonit_website_title_changed`…) reste à 1 pendant 24
heures. La première vérification enregistre seulement la référence. Changer l'adresse ou
**Ignorer les paramètres de requête** démarre une nouvelle référence au lieu de signaler un changement.

## Options

| Option | Par défaut | Sens |
|---|---|---|
| Texte attendu | vide | Une phrase que la page doit afficher. |
| Page vierge en dessous (caractères) | 20 | Le texte visible plus court que ceci est une page vierge ; 0 désactive la vérification. |
| Max. redirections | 10 | Redirections suivies au maximum (0 à 30). |
| User agent | `DumbMonit/<version>` | Envoyé à la place du propre DumbMonit. |
| Ignorer les paramètres de requête | activé | Les paramètres URL ne comptent pas lors de la comparaison des adresses. |
| Catégorie | vide | Votre propre étiquette du site, affichée avec lui et dans ses métriques (posée par l'import CSV). Vide : devinée, voir [Catégories](#categories). |
| Confirmer un changement après (vérifications) | 2 | Lectures consécutives d'une nouvelle valeur avant de la signaler, de 1 à 10. |
| Page quasi vide : chute du texte (%) | 80 | Alerte quand le texte visible tombe autant sous sa taille habituelle, de 0 à 99 ; 0 désactive. |
| Suivre l'adresse finale, le titre, la description, l'URL canonique, les directives robots, Open Graph, la langue, le nombre de h1, hreflang, la balise de vérification Google | tout activé | Chaque contrôle de métadonnées peut être désactivé séparément. |
| Autoriser les cibles loopback et link-local | désactivé | Pour un site servi par l'hôte DumbMonit lui-même. |

## Détection de plateforme

La plateforme provient, dans l'ordre : de la balise `<meta name="generator">`, des en-têtes
spécifiques à un hôte ou moteur (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js`…), des traces dans le balisage (`/wp-content/`,
`cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`, `/_nuxt/`…) et enfin de
l'hôte lui-même (`*.myshopify.com`, `*.wixsite.com`…). Reconnus : WordPress,
Shopify, Wix, Squarespace, Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento,
Next.js, Nuxt, Gatsby, Hugo, Jekyll ; tout le reste est **Other**. La plateforme
est l'étiquette `platform` de `dumbmonit_website_info`.

## Catégories {#categories}

Chaque site reçoit une catégorie, affichée sur sa page, dans l'étiquette `category` de
`dumbmonit_website_info` et comme dossier à l'import. Vous pouvez la fixer vous-même (option
**Catégorie** ou colonne CSV) ; sinon elle est devinée à partir de signes clairs seulement :

- **E-commerce** : Shopify, PrestaShop ou Magento, balisage WooCommerce ou « ajouter au panier »,
  `og:type` à `product`.
- **Blog** : Ghost, `og:type` à `article`, un hôte `blog.` ou une adresse `*.blogspot.com`,
  `*.substack.com`, `*.wordpress.com`.
- **Application web** : Next.js ou Nuxt, ou une page qui n'est qu'un conteneur `#root` / `#app` vide.
- **Site vitrine** : Wix, Squarespace ou Webflow.

Une plateforme généraliste (WordPress, Drupal, Hugo…) sans autre signe n'est pas devinée : le site
reste alors sous sa plateforme (`Websites/WordPress`), et ceux d'une plateforme inconnue sous
`Websites/Other`. Rangez-les à la main avec les colonnes `folder` ou `category`, ou déplacez-les dans la
liste des surveillances.

## Importer une liste de sites (CSV)

À partir de la liste des surveillances ou de la page d'ajout de surveillance, **Importer des sites web (CSV)** prend
un fichier ou du texte collé avec une ligne d'en-tête et jusqu'à 1000 lignes :

| Colonne | Requis | Sens |
|---|---|---|
| `url` | oui | Adresse du site. |
| `name` | non | Nom de la surveillance ; l'hôte par défaut. |
| `folder` | non | Dossier de la surveillance ; vide : `Websites/<catégorie>` s'il y a une catégorie, sinon `Websites/<platform>`, détecté en lisant le site une fois. |
| `category` | non | Texte libre (60 caractères, sans `/`), enregistré comme l'option **Catégorie**. Vide : devinée depuis la page, voir [Catégories](#categories). |
| `tags` | non | `key=value;key=value`. |
| `interval` | non | Intervalle d'interrogation. |
| `expect_text` | non | Même que l'option. |

```csv
url,name,folder,tags,interval,expect_text,category
https://example.com,Example,,client=acme,60s,Example Inc.,
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart,
https://blog.example.net,,,,,,Clients
```

Un aperçu affiche chaque ligne et ses erreurs avant que rien ne soit créé ; un site
déjà surveillé (même adresse finale) est ignoré.

## Métriques

| Métrique | Sens |
|---|---|
| `dumbmonit_website_up` | 1 quand la réponse finale est un succès (2xx ou 3xx), 0 sinon. |
| `dumbmonit_website_status_code` | Statut HTTP de la réponse finale. |
| `dumbmonit_website_response_seconds` | Temps total, redirections incluses. |
| `dumbmonit_website_redirect_hops` | Redirections suivies. |
| `dumbmonit_website_https` | 1 quand l'adresse finale est en https. |
| `dumbmonit_website_security_header{header}` | 0/1 par étiquette `header` (https seulement) : HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `dumbmonit_website_info{platform,category}` | Toujours 1 ; les étiquettes portent la plateforme et, si elle est connue, la catégorie. |
| `dumbmonit_website_text_chars` | Caractères de texte visible (réponses 2xx). |
| `dumbmonit_website_blank` | 1 quand ce texte est en dessous du seuil. |
| `dumbmonit_website_expect_text_missing` | 1 quand le texte attendu est manquant (option définie). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Directives d'indexation. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | Titres `<h1>`, alternates `hreflang`. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 pendant 24 heures après un changement confirmé. |
| `dumbmonit_website_text_dropped` | 1 quand le texte visible est très en dessous de sa taille habituelle (option activée). |
| `dumbmonit_website_robots_changed`, `…_open_graph_changed`, `…_lang_changed`, `…_h1_changed`, `…_hreflang_changed`, `…_google_verification_changed` | 1 pendant 24 heures après un changement confirmé de ces métadonnées. |
| `dumbmonit_website_robots_blocks_all` | 1 quand `robots.txt` interdit tout à Googlebot. |
| `dumbmonit_website_sitemap_ok` | 1 quand un sitemap répond et se lit. |

## Alertes intégrées

| Alerte | Quand | Sévérité |
|---|---|---|
| Surveillance inaccessible | Pas de réponse, certificat refusé, boucle de redirection ou trop de redirections | critical |
| Site web indisponible | La réponse finale est une erreur 4xx ou 5xx, pendant 3 minutes | critical |
| Page du site web vierge | Texte visible en dessous du seuil, pendant 3 minutes | critical |
| Texte attendu du site web manquant | Le texte attendu est manquant, pendant 3 minutes | warning |
| Site web non indexable | `noindex` sur la page, pendant 10 minutes | critical |
| Site web bloqué par robots.txt | `robots.txt` interdit tout, pendant 10 minutes | critical |
| URL canonique du site web modifiée | Changement confirmé | warning |
| Adresse du site web modifiée | L'hôte ou le chemin final a changé, confirmé | warning |
| Titre ou description du site web modifié | Changement confirmé | info |
| Page du site web quasi vide | Le texte visible est tombé très en dessous de sa taille habituelle, pendant 3 minutes | warning |
| Vérification Google du site web modifiée | La balise `google-site-verification` a changé ou disparu, confirmé | warning |
| Balisage SEO du site web modifié | Directives robots, Open Graph, langue, nombre de h1 ou hreflang modifiés, confirmé | info |
| Site web lent | Plus de 5 secondes, pendant 10 minutes | warning |

La vérification s'exécute sur le serveur DumbMonit lui-même : elle ne peut pas passer par un agent
relais.
