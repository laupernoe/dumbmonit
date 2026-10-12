# Bandeau pour site web

Affichez un bandeau sur votre propre site quand des services sont en panne
(« 2 services sont en panne »), alimenté par une [page d'état](../using/status-pages.md)
publiée. Quand tout va bien, le bandeau reste caché, à moins que vous ne
demandiez à l'afficher quand même (« Tous les systèmes sont opérationnels »).

Trois façons de l'ajouter, de la plus simple à la plus contrôlée. Des fichiers
prêts à copier se trouvent dans le dossier `integrations/` du dépôt.

## Extrait HTML

Collez ceci avant `</body>`, avec l'adresse de votre serveur et celle de la page :

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Attribut | Valeurs | Défaut |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (seulement quand quelque chose ne va pas), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | langue de la page, sinon `en` |

Le script n'a aucune dépendance, dessine à l'intérieur d'un Shadow DOM pour que
vos styles ne le touchent pas, suit la préférence clair ou sombre du visiteur
et peut être fermé (jusqu'à la fermeture de l'onglet). Il lit le JSON public de
la page, que le serveur laisse récupérer par n'importe quel site
(`Access-Control-Allow-Origin: *`, pas de cookies). Seul le document public de
la page d'état est ouvert ainsi ; le reste de l'API garde ses règles.

## WordPress

Copiez `integrations/wordpress/dumbmonit-status-banner` dans
`wp-content/plugins/`, activez-le, puis ouvrez **Réglages > DumbMonit Banner**
et saisissez l'URL de la page d'état
(`https://monit.example.com/api/public/status/my-page`), la position et le
mode d'affichage. Le plugin récupère le JSON côté serveur, le met en cache une
minute et échappe tout ce qu'il affiche. WordPress refuse les requêtes vers des
adresses privées, le serveur DumbMonit doit donc être accessible sur une
adresse publique.

## Inclusion PHP

Pour n'importe quel site PHP, sans framework :

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

L'assistant n'accepte que les URL `http` et `https`, utilise un délai de 3
secondes, ne suit pas les redirections, garde un cache fichier dans le
répertoire temporaire et n'affiche rien si le serveur est inaccessible. Voir
`integrations/php/example.php`.
