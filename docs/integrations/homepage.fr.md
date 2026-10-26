# Homepage et Homarr

Affichez l'état de DumbMonit sur votre tableau de bord : combien de moniteurs
fonctionnent ou sont tombés, combien d'alertes sont en cours. `GET /api/summary`
renvoie des compteurs plats prévus pour cela.

```json
{
  "status": "ok",
  "sentence": "Ciel dégagé.",
  "devices": { "total": 24, "up": 23, "down": 1, "waiting": 0, "disabled": 0 },
  "alerts": { "firing": 2, "critical": 1, "warning": 1, "info": 0, "building_up": 0, "suppressed": 0 }
}
```

`status` vaut `critical` quand une alerte est critique ou qu'un moniteur est
injoignable, `warning` pour toute autre alerte en cours, `ok` sinon. Les chiffres
sont ceux de l'interface et de l'[assistant](../using/assistant.md).

## 1. Créer un jeton en lecture seule

**Réglages → Jetons d'API → Nouveau jeton**, portée **read**. Le jeton peut lire
et rien d'autre (pas d'acquittement, pas de modification, pas de création d'un
autre jeton). Copiez-le : il n'est affiché qu'une fois. Vous pouvez aussi le
limiter à l'adresse réseau de votre tableau de bord et lui donner une date
d'expiration.

## 2. Homepage

[Homepage](https://gethomepage.dev) lit du JSON avec son widget `customapi`.
Dans `services.yaml` :

```yaml
- Monitoring:
    - DumbMonit:
        icon: mdi-radar
        href: https://monit.example.org
        description: État du homelab
        widget:
          type: customapi
          url: https://monit.example.org/api/summary
          refreshInterval: 30000
          headers:
            Authorization: Bearer dmt_xxxxxxxxxxxxxxxx
          mappings:
            - field: devices.up
              label: Actifs
              format: number
            - field: devices.down
              label: Tombés
              format: number
            - field: alerts.firing
              label: Alertes
              format: number
```

`field` est un chemin pointé dans le JSON. Pour afficher le verdict en texte,
associez `status` (`ok`, `warning`, `critical`) et traduisez-le avec `remap` :

```yaml
            - field: status
              label: État
              remap:
                - value: ok
                  to: Tout va bien
                - value: warning
                  to: Attention
                - value: critical
                  to: Panne
```

Mettez le jeton dans un secret Homepage plutôt que dans le fichier
(`{{HOMEPAGE_VAR_DUMBMONIT_TOKEN}}` avec la variable d'environnement
`HOMEPAGE_VAR_DUMBMONIT_TOKEN`), car `services.yaml` est souvent versionné.

!!! note
    Il n'existe pas aujourd'hui de widget Homepage officiel (un type `dumbmonit`
    dédié) : `customapi` fait le travail sans l'attendre.

## 3. Homarr

Homarr n'a pas, à l'heure où ces lignes sont écrites, de widget générique
« lire ce JSON ». Deux solutions saines :

- **Page de statut dans une iframe.** Publiez une
  [page de statut](../using/status-pages.md) et ajoutez le widget *iFrame* de
  Homarr vers `https://monit.example.org/s/<slug>/embed` : rien à authentifier,
  la page est faite pour être intégrée.
- **Tuile « App ».** Ajoutez DumbMonit comme *App* avec son adresse ; le ping de
  Homarr dit si l'instance répond.

`/api/summary` reste disponible pour tout ce qui sait envoyer un en-tête
`Authorization: Bearer` (une intégration personnalisée Homarr, n8n, un script,
une barre d'état…).

```bash
curl -s -H "Authorization: Bearer $TOKEN" https://monit.example.org/api/summary
```

## Dépannage

| Symptôme | Cause |
|---|---|
| `401` | Jeton absent, mal recopié, révoqué ou expiré ; ou la requête ne vient pas du réseau auquel le jeton est limité. |
| `403` | Le jeton n'est pas du bon type (un jeton d'agent ne marche pas ici) : créez un jeton d'API. |
| Les chiffres diffèrent de la page | Les compteurs sont calculés par le serveur au moment de la demande ; `refreshInterval` règle la fréquence de votre tableau de bord. |
