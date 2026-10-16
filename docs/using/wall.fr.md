# Mode mur et musique

`/wall` montre le bulletin seul, plein écran, pour un écran qui reste
allumé dans une pièce : la phrase, la fenêtre météo, trois grands relevés
et la liste *À traiter*, rafraîchie toutes les 20 secondes, avec l'écran
maintenu éveillé. ++esc++ ou **Quitter** retourne à la vue d'ensemble.
Ouvrez-le depuis la palette de commandes (« Mode mur ») ou en tapant
l'URL. `?theme=dark` ou `?theme=light` force un thème pour cet affichage
sans toucher votre préférence enregistrée.

![Mode mur](../assets/screenshots/wall-light.png){ loading=lazy }

Chaque problème, à gauche, porte l'icône du produit de sa surveillance (les
mêmes marques que dans les listes de surveillances). La liste ne déborde jamais de
sa place : sur un petit écran, ou avec beaucoup de problèmes, les grandes
tuiles se réduisent à une ligne chacune, puis se replient en *Et N de plus*,
au lieu de recouvrir les surveillances en dessous.

## Actualités sur le mur

Un administrateur peut ajouter des **titres** et une **chaîne en direct** au
mur dans **Réglages → Actualités du mur**. Les deux sont désactivés par défaut.

- **Bandeau de titres.** Une bande en bas du mur fait défiler les derniers
  titres, chacun avec sa source et son âge, et se rafraîchit toutes les cinq
  minutes. Si le système de l'écran demande de *réduire les animations*, rien ne
  défile : un titre à la fois apparaît et disparaît en fondu.
- **Vidéo en direct.** Une chaîne en direct, sans le son, dans une vignette, dans
  la colonne de droite sous l'horloge (ou au milieu de la scène), petite, moyenne
  ou grande. La vignette se réduit à la place disponible ; elle ne recouvre
  jamais l'état.
- **Carrousel d'images.** Les derniers articles des mêmes flux qui portent une
  image, un à la fois au milieu du mur, huit secondes chacun, en fondu, avec la
  source, l'âge et le titre sur l'image. Si l'écran demande de *réduire les
  animations*, les images se remplacent sans fondu.

C'est le serveur qui lit les flux, pas le navigateur (un navigateur ne peut pas
lire les flux d'autres sites). Il suit au plus trois redirections, refuse les
adresses internes, abandonne après huit secondes ou 2 Mo par flux, et garde la
réponse cinq minutes pour tous les murs. Il lit RSS 2.0, RSS 1.0 (RDF) et Atom.

L'image d'un article vient du flux : la plus large des images `media:content`,
une `media:thumbnail`, une pièce jointe image (`enclosure`, comme BFM TV), un
`link rel="enclosure"` Atom, ou à défaut la première `<img>` du contenu de
l'article (comme Korben). Quand le flux n'en a pas et que le carrousel est
allumé, le serveur lit une fois le début de la page de l'article (512 Ko au
plus, même garde-fou d'adresses) et prend son `og:image` ; la réponse est
retenue, une page n'est lue qu'une fois. Le mur ne charge jamais d'image depuis
le site d'actualité : le serveur la relaie à `/api/wall/news/image/<id>`,
seulement pour les articles qu'il a publiés, 6 Mo au plus, et seulement si le
fichier est vraiment une image PNG, JPEG, GIF, WebP ou AVIF (jamais de SVG).
L'adresse de l'écran et ses cookies restent à la maison, et la politique de
sécurité du mur n'autorise toujours que les images de DumbMonit.

### Choisir les sources

Choisissez un **pays** (présélectionné d'après la région et la langue de votre
navigateur), puis cochez ses sources : huit flux au plus en tout. Pour un autre
flux, collez son adresse RSS ou Atom sous **Ajouter un flux personnalisé** et
appuyez sur **Tester** : DumbMonit le lit pour de vrai et montre le dernier titre
avant l'ajout.

Pour la vidéo, choisissez une chaîne du pays ou **Un autre lien…** et collez une
adresse YouTube, Dailymotion ou Twitch ordinaire (une vidéo, un direct, ou la
page `/live` d'une chaîne YouTube) ; DumbMonit la convertit en lecteur
intégrable. Seuls ces trois lecteurs peuvent être affichés. Un direct n'est
proposé que lorsque la chaîne en a un officiel et stable.

| Pays | Titres | Vidéo en direct |
|---|---|---|
| France | BFM TV, franceinfo, Le Monde, France 24 | BFM TV, franceinfo, France 24, Euronews |
| Suisse | RTS Info, SRF News, Le Temps, NZZ | aucune |
| Belgique | RTBF, VRT NWS, La DH | aucune |
| Allemagne | Tagesschau, ZDFheute, Spiegel, Deutsche Welle | DW News |
| Royaume-Uni | BBC News, The Guardian | Sky News |
| États-Unis | NPR, PBS NewsHour, CBS News, The New York Times | ABC News Live |
| Canada | Radio-Canada, CBC News | CBC News |
| Espagne | RTVE, 20minutos, El País, El Mundo | aucune |
| Italie | ANSA, RaiNews | aucune |
| Portugal | RTP Notícias, Público | aucune |
| Brésil | G1, Folha de S.Paulo | CNN Brasil |
| Russie | Interfax, Meduza, Kommersant | aucune |
| Chine | CGTN, South China Morning Post, Xinhua | CGTN |
| International | Deutsche Welle, France 24, BBC News (monde) | DW News, France 24 English, Euronews English |

Le flux de BFM TV est `https://www.bfmtv.com/rss/news-24-7/`. La liste a été
vérifiée à sa rédaction ; un site peut déplacer son flux à tout moment, le
bandeau l'ignore alors simplement. **Testez** le remplaçant et ajoutez-le comme
flux personnalisé.

La politique de sécurité du mur n'autorise que les lecteurs YouTube
(`youtube-nocookie.com`), Dailymotion, Twitch, Spotify et Deezer dans un cadre.

### API

`GET /api/wall/news` (ce que les murs interrogent), `GET|PUT
/api/wall/news/settings` et `POST /api/wall/news/test` (admin), `GET
/api/wall/news/presets`, `GET /api/wall/news/image/{id}` (l'image d'un article,
relayée).

## Widgets sur le mur

Un administrateur peut ajouter des **widgets** au mur dans **Paramètres → Mur →
Widgets du mur** : la météo de quelques villes, le cours de cryptomonnaies, d'actions,
les taux de change et les dernières failles critiques. Chacun est désactivé par
défaut et s'allume séparément. Ils se partagent une carte dans la colonne de
droite, sous l'horloge : un widget à la fois, chacun pendant le nombre de
secondes choisi (20 par défaut), puis le suivant apparaît en fondu. Les longues
listes sont coupées en pages de cinq. Des points sous la carte montrent où en
est le tour. Si l'écran demande de *réduire les animations*, les cartes se
remplacent sans fondu.

Une hausse ou une baisse porte toujours une flèche et un signe (▲ +1,04 %,
▼ −0,57 %), jamais seulement une couleur. Chaque carte cite sa source,
discrètement, et dit de quand datent ses chiffres.

| Widget | Ce qu'il montre | Source | Clé | Le serveur demande |
|---|---|---|---|---|
| Météo | Maintenant, toutes les trois heures jusqu'à +12 h, les quatre jours suivants, pour quatre villes au plus | [Open-Meteo](https://open-meteo.com) (prévisions et géocodage) | aucune | toutes les 15 min, un appel pour toutes les villes |
| Crypto | Cours et variation sur 24 h de dix monnaies au plus, dans la devise choisie | API publique de [CoinGecko](https://www.coingecko.com) | aucune (clé Demo gratuite facultative) | chaque minute, un appel |
| Actions | Dernier cours et variation du jour de dix symboles au plus | [Finnhub](https://finnhub.io) | **clé gratuite obligatoire** | toutes les 5 min, un appel par symbole |
| Devises | Ce que coûte une unité (ou 100 d'une petite monnaie) dans la devise de base, et sa variation depuis la veille | [Frankfurter](https://frankfurter.dev) (taux de référence des banques centrales) | aucune | chaque heure |
| Failles critiques | Les dernières CVE notées critiques (CVSS 9 ou plus) de la semaine et/ou les dernières entrées du catalogue de la CISA des failles activement exploitées, avec note et date | API 2.0 de la [NVD](https://nvd.nist.gov), [CISA KEV](https://www.cisa.gov/known-exploited-vulnerabilities-catalog) | aucune | chaque heure |

**Les sources et leurs conditions** (vérifiées en octobre 2026) :

- **Open-Meteo** est gratuit sans clé pour un usage non commercial (jusqu'à
  10 000 appels par jour) ; ses données sont sous CC BY 4.0.
- L'API sans clé de **CoinGecko** partage une réserve d'environ 10 à 30 appels
  par minute et par adresse, et n'est pas faite pour des interrogations
  intensives ; DumbMonit demande une fois par minute pour tous les murs. Une
  **clé Demo** gratuite (coingecko.com) donne à l'instance sa propre limite.
- Les **actions** demandent une clé **Finnhub** gratuite : aucune source de
  cours sans clé n'est à la fois fiable et autorisée (Stooq demande désormais
  une clé, Yahoo interdit son API non documentée). L'offre gratuite couvre les
  titres cotés aux États-Unis, 60 appels par minute ; les cours peuvent être
  différés.
- **Frankfurter** n'a ni clé ni quota ; les taux changent une fois par jour
  ouvré.
- La **NVD** accepte 5 requêtes par 30 secondes sans clé ; DumbMonit en fait
  une par heure. *This product uses the NVD API but is not endorsed or
  certified by the NVD.* Le catalogue **CISA KEV** est un fichier JSON public.

**Tout passe par le serveur.** Aucun écran ne parle à ces services : ni son
adresse ni les limites des services ne dépendent du nombre d'écrans. Chaque
source a son cache. Quand une source tombe, la carte garde ses derniers chiffres
jusqu'à six heures, marqués *Obsolète*, et le serveur attend une minute avant
de redemander ; sans rien à montrer, la carte dit *Indisponible*. Un widget ne
casse jamais le mur.

**Les clés d'API ne reviennent jamais.** Elles sont chiffrées avec le secret de
l'instance ; les réglages montrent seulement qu'une clé est enregistrée. En
taper une nouvelle la remplace, **Supprimer la clé** l'efface.

### Choisir ce qui s'affiche

- **Météo :** tapez une ville et **Rechercher** (géocodage d'Open-Meteo, dans
  votre langue), puis ajoutez jusqu'à quatre villes. Unités : métriques ou
  impériales.
- **Crypto :** cochez parmi les principales monnaies (BTC, ETH, SOL, XRP, BNB,
  ADA, DOGE, TRX, TON, AVAX, DOT, LINK, LTC, XLM, XMR, USDT, USDC) ou ajoutez
  n'importe laquelle par son identifiant CoinGecko (la fin de son adresse sur
  coingecko.com, par exemple `avalanche-2`). Devise d'affichage : USD, EUR,
  GBP, CHF, JPY et d'autres.
- **Actions :** tapez les symboles, séparés par des virgules
  (`AAPL, MSFT, NVDA`), et collez votre clé Finnhub.
- **Devises :** choisissez la devise de base et jusqu'à dix autres parmi les
  principales (USD, EUR, GBP, JPY, CHF, CNY, CAD, AUD…).
- **Failles critiques :** *Les deux* (critiques et exploitées ; une CVE
  exploitée déjà dans la liste de la NVD porte la marque *Exploitée*),
  *Critiques* (NVD) ou *Activement exploitées* (CISA KEV), et combien en
  montrer (1 à 10).

### API

`GET /api/wall/widgets` (ce que les murs interrogent chaque minute : une
section par widget allumé, avec `data`, `updated`, `error`, `needs_key`,
`source`), `GET|PUT /api/wall/widgets/settings` et
`GET /api/wall/widgets/places?q=` (admin), `GET /api/wall/widgets/catalog`. Sur
`PUT`, `secrets.finnhub` et `secrets.coingecko` enregistrent une clé, une chaîne
vide l'efface, et les omettre les garde.

## Musique sur le mur

Le mur peut aussi jouer de la musique et montrer ce qui joue, pour que
vous puissiez lancer une playlist depuis votre téléphone, l'entendre
depuis la TV, et la voir depuis l'autre bout de la pièce. La musique se
trouve toujours dans le coin bas de la fenêtre météo, par-dessus le ciel :
elle ne prend jamais de place au bulletin ou à la liste *À traiter*, et
disparaît quand rien ne joue. Chaque carte se replie en une petite pastille
(l'affichage s'en souvient).

Il y a deux façons, et vous pouvez utiliser les deux.

| | Spotify Connect | Un lien envoyé aux murs |
|---|---|---|
| Démarré depuis | l'application Spotify sur votre téléphone (**Appareils → DumbMonit Wall**) | n'importe quel écran DumbMonit, votre téléphone inclus (**Jouer sur le mur**) |
| Services | Spotify | Spotify, Deezer, YouTube et YouTube Music |
| A besoin de | Spotify Premium, une application Spotify Developer qui vous est propre, le téléphone sur le même compte Spotify, le mur servi en HTTPS (ou localhost) dans un navigateur avec DRM | rien ; sans être connecté sur le mur, Spotify et Deezer jouent des aperçus de 30 secondes |
| Montre ce qui joue | oui, où que ça joue : le mur, l'application Spotify d'une TV, un haut-parleur cast | le propre lecteur du service |

Connecter Spotify et envoyer des liens sont des actions d'administrateur ;
un compte lecteur sur le mur joue et montre tout.

### Spotify Connect : le mur devient un haut-parleur

L'objectif : sur votre téléphone, ouvrez Spotify, touchez l'icône des
appareils, choisissez la TV — la musique joue sur la TV tandis que le mur
montre ce qui joue. Le mur fait ça avec le **Web Playback SDK** de
Spotify : le navigateur du mur s'enregistre comme un appareil Spotify
Connect. Ça ne fonctionne que quand **toutes** ces conditions tiennent, et
le mur vous dit sur son propre écran laquelle manque :

| A besoin de | Pourquoi | Quand ça manque |
|---|---|---|
| **Spotify Premium** sur le compte connecté | Spotify ne laisse que les comptes Premium utiliser des haut-parleurs navigateur | le mur montre *Spotify Premium is required* ; Réglages montre *Premium : Non* |
| **Le téléphone connecté au même compte Spotify** | un haut-parleur navigateur n'est listé que pour le compte dont il détient le jeton — il n'est pas découvert sur le réseau comme un haut-parleur intelligent, et les autres membres d'un plan Family ne le voient pas | le téléphone ne liste simplement pas le mur |
| **HTTPS** (ou `http://localhost` sur l'affichage lui-même) | les navigateurs n'autorisent les médias protégés (DRM) que dans une page sécurisée | le mur montre *opened over plain HTTP* |
| **Un navigateur dont le DRM fonctionne** (Widevine, PlayReady ou FairPlay) | Spotify diffuse de l'audio chiffré | le mur montre *This browser cannot play Spotify* |
| **Un appui ou une pression de touche sur le mur par chargement de page** | les navigateurs gardent une page silencieuse jusqu'à ce que quelqu'un interagisse avec elle | le mur montre *Tap to enable sound* |

Quels navigateurs fonctionnent : Chrome, Edge et Firefox sur un ordinateur
(Firefox : autorisez *Play DRM-controlled content*), Safari sur un Mac,
Chromium avec Widevine sur un Raspberry Pi (`libwidevinecdm0`). La plupart
des **navigateurs de smart-TV** (Samsung Tizen, LG webOS), des
**navigateurs kiosque** et beaucoup de **builds Chromium Linux** sont
livrés sans Widevine utilisable : ils ne peuvent pas être un haut-parleur
Spotify, quoi que fasse DumbMonit. Pour une telle TV, voir
[Quand le navigateur de la TV ne peut pas jouer Spotify](#when-the-tvs-browser-cannot-play-spotify).

#### Le configurer

1. **Créez une application Spotify.** Ouvrez le
   [Spotify Developer Dashboard](https://developer.spotify.com/dashboard),
   créez une application (n'importe quel nom), cochez **Web API** et
   **Web Playback SDK**, et ajoutez l'URI de redirection que
   **Réglages → Musique du mur** montre :
    - DumbMonit servi en HTTPS :
      `https://<votre adresse>/api/music/spotify/callback`. Spotify
      revient directement sur DumbMonit après votre approbation.
    - DumbMonit servi en HTTP simple : `http://127.0.0.1:8888/callback`.
      Spotify n'accepte `http://` que pour une adresse loopback (et pas
      `localhost`), depuis 2025. Rien ne répond à cette adresse : après
      votre approbation, votre navigateur montre une page d'erreur dont
      l'adresse porte le code — copiez-le et collez-le dans DumbMonit.
2. **Connectez.** Dans **Réglages → Musique du mur**, collez le
   **Client ID** de l'application (il n'y a pas de secret : DumbMonit
   utilise Authorization Code avec PKCE) et choisissez **Connecter
   Spotify**, puis approuvez sur Spotify **avec le compte que votre
   téléphone utilise**.
3. **Sur la TV**, ouvrez le `/wall` de DumbMonit en HTTPS, connecté (un
   compte lecteur suffit), et touchez l'écran ou appuyez sur **OK** sur
   la télécommande une fois : n'importe quel geste débloque le son. Un
   navigateur démarré avec `--autoplay-policy=no-user-gesture-required`
   (un kiosque) n'a besoin d'aucun appui.
4. **Sur votre téléphone**, dans l'application Spotify, touchez l'icône
   des appareils et choisissez **DumbMonit Wall**. Le son sort de la TV, et
   le mur montre la pochette, le titre et la progression.

**Réglages → Musique du mur → Haut-parleur** montre si tout tient, sans
marcher jusqu'à la TV : le compte, Premium, si Spotify liste le
haut-parleur parmi les appareils du compte en ce moment (et quels
appareils il liste), et chaque mur qui s'est signalé dans les quinze
dernières minutes — son navigateur (« Chrome 130 sur Linux »), s'il est
prêt, et pourquoi pas sinon. **Tester le son** fait jouer le compte sur le
haut-parleur (il reprend ce qui jouait en dernier) ; chaque mur prêt a
aussi son propre **Tester le son**. Sur le mur lui-même, le panneau
**Musique** a **Jouer ici**, qui déplace ce qui joue sur le compte vers
cet affichage.

**Le nom.** Renommez le haut-parleur dans
**Réglages → Musique du mur → Haut-parleur** (« TV du salon ») ; chaque
mur reprend le nouveau nom en quelques secondes. Pour nommer un seul mur,
ajoutez `?speaker=Cuisine` à son adresse — l'affichage s'en souvient
(`?speaker=` l'oublie).

**Un mur reste allumé pendant des jours.** Le mur demande à DumbMonit un
jeton frais quand celui de Spotify expire (chaque heure) ; quand
l'appareil se déconnecte (Wi-Fi, une carte réseau qui dort) et ne revient
pas en trente secondes, le mur s'enregistre à nouveau ; un échec passager
est retenté de lui-même (15 s, 30 s, … puis toutes les cinq minutes) ; et
si Spotify arrête de lister le mur alors qu'il se croit prêt, il
s'enregistre à nouveau. Seul ce que l'attente ne peut pas réparer — pas de
DRM, pas de Premium — s'arrête pour de bon, jusqu'à **Réessayer**.

DumbMonit garde le jeton de rafraîchissement Spotify chiffré avec le
secret de l'instance et ne l'envoie jamais à un navigateur. Le mur ne
reçoit qu'un jeton d'accès de courte durée pour son lecteur, et seulement
via une session connectée (un jeton d'API ne peut pas en obtenir un).
**Déconnecter** oublie le compte ; pour aussi retirer DumbMonit de votre
compte Spotify, utilisez
[spotify.com/account/apps](https://www.spotify.com/account/apps/).

Bon à savoir aussi :

- Une application Spotify personnelle (*development mode*) appartient à
  un compte Premium et admet jusqu'à cinq comptes : ajoutez les autres
  sous **User Management** dans le tableau de bord.
- **Reconnectez tous les six mois.** Spotify termine une connexion six
  mois après son approbation ; Réglages montre la date, et le mur le dit
  quand ça arrive.

**« En cours de lecture » fonctionne sans rien de tout ça.** Le serveur de
DumbMonit demande à Spotify ce qui joue sur le compte toutes les quelques
secondes (une requête pour tous les murs) et le mur montre la pochette, le
titre, l'artiste, la progression et l'appareil sur lequel ça joue — le mur
lui-même, l'application Spotify d'une smart TV, un Chromecast, votre
ordinateur portable — même en HTTP simple et dans un navigateur qui ne
peut pas jouer Spotify. Pour utiliser seulement ça, désactivez
**Utiliser cet affichage comme haut-parleur Spotify** dans le panneau
Musique du mur.

#### Quand le navigateur de la TV ne peut pas jouer Spotify {#when-the-tvs-browser-cannot-play-spotify}

Choisissez ce qui correspond à votre installation — dans chaque cas le
mur continue de montrer ce qui joue, parce que *en cours de lecture* lit
le compte, pas le haut-parleur :

- **La TV a une application Spotify, ou un Chromecast / Google TV / Fire
  TV stick.** Choisissez-le dans la liste d'appareils de Spotify ; le mur
  montre « Joue sur TV du salon ». C'est l'option la plus simple et la
  plus fiable.
- **Un petit ordinateur pilote la TV** (un Raspberry Pi, un mini PC, un
  vieil ordinateur portable). Ouvrez le mur dans Chrome ou Chromium avec
  Widevine dessus : il devient le haut-parleur comme décrit plus haut.
- **Vous voulez un haut-parleur toujours présent, même sans navigateur.**
  Lancez un récepteur Spotify Connect basé sur
  [librespot](https://github.com/librespot-org/librespot) (MIT) — par
  exemple [raspotify](https://github.com/dtcooper/raspotify) ou spotifyd
  — **sur la machine dont la sortie audio atteint la TV ou le
  haut-parleur**. Il apparaît dans la liste d'appareils du téléphone pour
  tout le monde sur le réseau.

DumbMonit ne fournit pas un tel récepteur lui-même, volontairement : le
conteneur DumbMonit tourne généralement sur un serveur dans un placard, pas
sur la machine branchée à la TV, son son sortirait donc de la mauvaise
boîte ; un récepteur a besoin de la carte son de l'hôte et du réseau hôte
(mDNS) qu'un conteneur de surveillance ne devrait pas avoir ; et librespot
dépend de protocoles Spotify non officiels qui cassent de temps en temps.
Un récepteur installé sur la bonne machine, à côté de DumbMonit, fait
mieux le travail.

### Un lien envoyé à chaque mur

Dans **Réglages → Musique du mur** (ou le propre panneau **Musique** du
mur), collez un lien Spotify, Deezer ou YouTube vers un titre, un album,
une playlist, un épisode ou une vidéo et choisissez **Jouer sur le mur**.
Chaque mur le joue dans le lecteur embarqué officiel du service ; un lien
envoyé pendant qu'un mur est ouvert démarre de lui-même là où le service
le permet (YouTube, Deezer) une fois le mur touché. **Arrêter** le retire
de chaque mur.

Deezer et YouTube n'ont pas d'équivalent Spotify Connect qu'une page web
puisse utiliser — Deezer n'accepte plus de nouvelles applications
développeur et son API n'a jamais exposé ce qui joue — un lien est donc
la voie pour eux. Les lecteurs embarqués jouent des aperçus de 30 secondes
sauf si le navigateur du mur est connecté au service.

### Confidentialité

Rien de tiers ne se charge sur le mur jusqu'à ce que Spotify soit connecté
ou qu'un lien soit réglé. La politique de sécurité du contenu de la page
autorise exactement les trois lecteurs embarqués, le script de lecteur de
Spotify (`sdk.scdn.co/spotify-player.js`) et sa frame, et couvre depuis
`i.scdn.co` — rien d'autre, et rien hors de l'interface web.

### API

| Route | Qui | Quoi |
|---|---|---|
| `GET /api/music/now` | toute session ou jeton | Ce qui joue sur le compte Spotify (mis en cache quelques secondes) et le lien que les murs jouent |
| `PUT /api/music/link` `{"link": "…"}` | admin | Chaque mur joue ce lien |
| `DELETE /api/music/link` | admin | Les murs arrêtent le lecteur embarqué |
| `GET /api/music/spotify` | toute session ou jeton | Le compte connecté : statut, Client ID, date de reconnexion — jamais un jeton |
| `POST /api/music/spotify/authorize` | session admin | Démarre une connexion : `{"client_id", "redirect_uri"}` → l'URL d'approbation de Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | session admin | Termine une connexion loopback avec l'adresse sur laquelle le navigateur a atterri |
| `GET /api/music/spotify/callback` | le navigateur de l'admin | La redirection directe de Spotify (installations HTTPS) |
| `DELETE /api/music/spotify` | admin | Déconnecte et oublie les jetons |
| `GET /api/music/spotify/token` | session navigateur seulement | Un jeton d'accès de courte durée pour le lecteur du mur |
| `GET /api/music/speaker` | toute session ou jeton | Le haut-parleur : nom, Premium, les appareils Spotify Connect du compte, si le haut-parleur est listé, ce que chaque mur a rapporté |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Renomme le haut-parleur (`null` revient à « DumbMonit Wall ») |
| `POST /api/music/speaker/report` | session navigateur seulement | Un mur rapporte son haut-parleur (envoyé chaque minute) ; la réponse dit si Spotify le liste |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | session navigateur seulement | Joue le compte sur cet appareil, ou sur l'appareil nommé comme le haut-parleur (« Tester le son », « Jouer ici ») |
