# Carte de topologie

La page **Carte** (**⋯ → Carte** sur la page Surveillances, ou *Carte* dans la
palette de commandes, `/map`) dessine toute votre infrastructure en une image.
Rien n'y est saisi à la main : chaque lien vient de ce que DumbMonit sait déjà.

## D'où viennent les liens

| Lien | Source | Tracé |
|---|---|---|
| Dépend de | Le **parent** défini sur une sonde, celui que suit la suppression des alertes. | trait plein |
| Joint via un relais | L'**agent relais** qui interroge une surveillance sur un site distant. | tirets longs |
| Hébergé par un hyperviseur | Les invités signalés dans les 30 dernières minutes par Proxmox VE, XCP-ng, Nutanix, vSphere ou le module libvirt d'un agent Linux. | tirets courts |
| Câblé à (LLDP) | Un **voisin LLDP** vu par un commutateur SNMP sur l'un de ses ports (profil `ethernet-switch`). | pointillés |

Un invité ou un voisin LLDP n'est relié à une sonde que si son nom désigne une
seule sonde (même nom, ou même nom d'hôte dans l'adresse, domaine ignoré). Un
nom ambigu ne crée aucun lien plutôt qu'un lien faux. Un invité qui ne
correspond à aucune sonde apparaît dans sa propre case (*Machine virtuelle* ou
*Conteneur*) à côté de son hyperviseur ; un voisin LLDP inconnu est ignoré. Une
sonde sans lien connu reste simplement dans son dossier.

Quand deux sources relient la même paire, la carte garde la plus sûre, le lien
déclaré avant le lien déduit. Chaque case pend à un seul lien ; les autres sont
tracés plus légèrement.

## Lire la carte

- L'amont à gauche, ce qui en dépend à droite ; une section par dossier.
- Chaque case montre l'icône du type, le nom et l'état en toutes lettres. Une surveillance **en panne** a un contour épais et un badge `!` ; ce qui se trouve
  derrière lui (par un parent, un relais ou un hyperviseur) est marqué
  **Impacté**, avec un contour en tirets : ses alertes sont supprimées. Les
  liens LLDP ne propagent pas une panne.
- Survoler ou sélectionner une case met ses liens en évidence. Un clic (ou
  Entrée) ouvre la sonde ; la case d'un invité ouvre son hyperviseur.
- Glisser pour déplacer, molette ou pincement pour zoomer, flèches pour
  défiler, `+`, `-` et `0` (ajuster). Les boutons dans le coin font de même.
- **Afficher** restreint la carte à un dossier ou au site d'un relais ; la case
  à cocher masque les machines virtuelles et conteneurs non surveillés.
- Sur un téléphone, la page s'ouvre sur la vue **Liste** : le même arbre, indenté.

## Exporter et imprimer

**Exporter** propose :

- **Image SVG** et **Image PNG** : toute la carte avec un titre, le périmètre,
  la date et la légende, sur fond blanc, quel que soit le thème à l'écran.
- **Imprimer / PDF (A4 paysage)** et **(A3 paysage)** : le navigateur
  n'imprime que cette feuille, mise à l'échelle de la page. Choisissez
  *Enregistrer au format PDF* dans la boîte d'impression pour un PDF.

## API

`GET /api/topology` renvoie les nœuds, les liens et la source de chaque lien
(tous les rôles, lecture seule, sans adresses). Voir la
[référence de l'API](../reference/api.md).
