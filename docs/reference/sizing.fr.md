# Dimensionnement

Ce qu'il faut à un conteneur DumbMonit pour 100, 1 000 et 5 000 surveillances,
et combien de temps les données peuvent être conservées. **Les chiffres
ci-dessous sont des estimations**, tirées du modèle de stockage décrit plus bas.
Elles sont en cours de confirmation par le [workflow de test de
charge](#mesurer-chez-soi) ; d'ici là, servez-vous-en comme point de départ et
surveillez la courbe de disque dans **Réglages → Système**.

## En un coup d'œil

Hypothèses : période de collecte de 30 s (la valeur par défaut) et environ
20 séries par surveillance (un contrôle HTTP en écrit moins, un hôte Proxmox ou
un commutateur à nombreuses interfaces beaucoup plus). Les données brutes sont
gardées 12 mois, les agrégats de 5 minutes 730 jours.

| | 100 surveillances | 1 000 surveillances | 5 000 surveillances |
|---|---|---|---|
| Séries actives | ~2 000 | ~20 000 | ~100 000 |
| CPU (conteneur entier) | 0,1 cœur | 0,5 cœur | 2 à 3 cœurs |
| RAM, serveur + VictoriaMetrics | 250 Mo | 500 Mo | 1,5 à 2 Go |
| RAM, VictoriaMetrics longue durée (`DUMBMONIT_VM_LONG_MEMORY`) | 64 Mo | 64 Mo | 128 à 256 Mo |
| RAM, VictoriaLogs (facultatif) | 64 Mo | 64 Mo | 128 Mo |
| Données brutes, par jour | ~6 Mo | ~60 Mo | ~300 Mo |
| Données brutes, 12 mois | ~2 Go | ~21 Go | ~105 Go |
| Agrégats de 5 minutes, par jour | ~1,5 Mo | ~15 Mo | ~75 Mo |
| Agrégats de 5 minutes, 730 jours | ~1 Go | ~11 Go | ~55 Go |
| Latence de l'API, p95 (`/api/targets`, graphiques) | < 50 ms | < 150 ms | < 500 ms |

Au-delà d'environ **5 000 surveillances**, ou pour la haute disponibilité,
utilisez un [VictoriaMetrics externe](external-victoriametrics.md).

## Disque

VictoriaMetrics comprime un échantillon à environ un octet en moyenne (moins
pour des valeurs qui bougent peu, plus pour des valeurs bruitées) :

```text
octets par jour = séries x échantillons par jour et par série x ~1 octet
échantillons par jour et par série = 86 400 / période de collecte en secondes   (2 880 à 30 s)
```

Exemple : 20 000 séries à 30 s font 20 000 x 2 880 x 1 octet, soit environ
58 Mo par jour. Gardez 30 % de marge pour les fusions en arrière-plan et les
index, dont VictoriaMetrics a besoin pendant qu'il compacte.

Les agrégats de 5 minutes comptent 288 échantillons par jour et par série, mais
une jauge est stockée trois fois (moyenne, minimum, maximum), et les métriques
de disponibilité ajoutent une somme et un compte. En pratique ils pèsent environ
un quart des données brutes par jour, et se gardent beaucoup plus longtemps :
c'est pourquoi la rétention brute peut rester courte.

## Conseils de rétention

* **Les données brutes** (points de 30 s) répondent à « que s'est-il passé hier
  à 14 h 03 ». 30 à 90 jours suffisent à la plupart des équipes ; gardez plus
  seulement si vous lisez des graphiques à pleine résolution aussi loin.
* **L'historique long** (agrégats de 5 minutes, 730 jours par défaut) répond
  aux tendances, à la disponibilité mensuelle et aux rapports. Un graphique,
  une page de statut ou un rapport dont la plage dépasse la rétention brute est
  servi automatiquement depuis lui.
* Les deux se règlent dans **Réglages → Système → Données et rétention**.
  Changer une valeur redémarre VictoriaMetrics embarqué quelques secondes ; les
  échantillons sont mis en tampon entre-temps.
* La page montre la croissance du disque et le nombre de jours avant que `/data`
  soit plein.

## Mesurer chez soi

Le workflow **Load test** démarre l'image publiée sur un runner GitHub, crée les
surveillances (type « Appareil de démonstration », période de 30 s), les laisse
tourner, puis rapporte le CPU, la mémoire, la croissance du disque par dossier
et la latence de l'API.

1. Dans le dépôt, ouvrez **Actions → Load test → Run workflow**.
2. Choisissez 1 000 ou 5 000 surveillances, une durée (60 minutes ou plus pour
   des chiffres de disque parlants) et l'étiquette d'image (`edge` par défaut).
3. À la fin, lisez le résumé du job, ou téléchargez l'artefact `load-test-<n>`
   (`summary.md` et `summary.json`).

Comment le lire :

* **Le CPU** est un pourcentage d'un cœur ; le runner a 4 vCPU, donc une valeur
  proche de 400 % signifie la saturation.
* **Disque, « per day, extrapolated »** multiplie la croissance mesurée pendant
  l'essai par le temps restant dans la journée. Un essai court la surestime,
  car VictoriaMetrics fusionne ses parts plus tard. `vm` est le brut, `vm-long`
  les agrégats (il se remplit après la première fenêtre de 5 minutes),
  `dumbmonit.db` la configuration et l'état.
* **La latence** est mesurée depuis le runner vers `localhost` : le réseau n'y
  est pas.

Le même script tourne partout où il y a Docker et Python 3 :
`MONITORS=1000 DURATION_MIN=60 scripts/load/run.sh`.
