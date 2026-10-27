# Carte de topologie

La page **Carte** (**⋯ → Carte** sur la page Surveillances, ou *Carte* dans la
palette de commandes, `/map`) dessine toute votre infrastructure en une image.
Rien n'y est saisi : chaque lien vient de ce que DumbMonit sait déjà, et un
administrateur peut [corriger la carte](#corriger-la-carte) là où elle se trompe.

## D'où viennent les liens

| Lien | Source | Tracé |
|---|---|---|
| Dépend de | Le **parent** défini sur une sonde, celui que suit la suppression des alertes. | trait plein |
| Joint via un relais | L'**agent relais** qui interroge une surveillance sur un site distant. | tirets longs |
| Hébergé par un hyperviseur | Les invités signalés dans les 30 dernières minutes par Proxmox VE, XCP-ng, Nutanix, vSphere ou le module libvirt d'un agent Linux. | tirets courts |
| Tourne sur un invité | Une sonde dont l'adresse IP est l'une de celles que l'hyperviseur rapporte pour une VM ou un conteneur (agent invité Proxmox, interfaces du conteneur ou configuration statique ; VMware Tools sur vSphere). | tirets courts serrés |
| Câblé à (LLDP) | Un **voisin LLDP** vu par un commutateur SNMP sur l'un de ses ports (profil `ethernet-switch`). | pointillés |
| Câblé à (ports de commutateur) | Un **lien de niveau 2** lu dans les tables des commutateurs : commutateur à commutateur (LLDP, ou CDP sur du matériel Cisco) et hôte à port d'accès (table d'adresses du pont rapprochée de l'ARP). | tirets-points |

Un invité ou un voisin LLDP n'est relié à une sonde que si son nom désigne une
seule sonde (même nom, ou même nom d'hôte dans l'adresse, domaine ignoré). Un
nom ambigu ne crée aucun lien plutôt qu'un lien faux. Un invité qui ne
correspond à aucune sonde apparaît dans sa propre case (*Machine virtuelle* ou
*Conteneur*) à côté de son hyperviseur ; un voisin LLDP inconnu est ignoré. Une
sonde sans lien connu reste simplement dans son dossier.

Les adresses des invités relient aussi les sondes à l'invité sur lequel elles
tournent : un contrôle HTTP, un port TCP ou une base de données visant l'une
des adresses IP d'une VM pend sous cette VM. Quand le nom d'un invité ne
correspond à aucune sonde, le seul agent, ping ou SNMP à son adresse représente
l'invité au lieu d'une seconde case. Une adresse que plusieurs invités
annoncent (adresse virtuelle partagée) ne relie rien, un parent déclaré passe
toujours avant, et les noms d'hôte ne sont pas résolus. Les adresses de
l'invité sont affichées dans son panneau de détail.

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

## Corriger la carte

La déduction se trompe parfois : un équipement « passe par le pare-feu » alors
qu'il est derrière un commutateur, un contrôle HTTP ne sait pas sur quelle VM il
tourne. Les administrateurs cliquent sur **Corriger la carte** au-dessus de la
carte :

- **Rattacher** : glissez un équipement sur celui dont il dépend vraiment. Sans
  souris, sélectionnez l'équipement et utilisez **Rattacher** dans son panneau
  (recherche par nom ou par type). **Utiliser aussi pour les alertes** (cochée
  par défaut, équipements seulement) en fait aussi le parent déclaré, celui que
  suit la suppression des alertes : quand le parent tombe, les alertes de
  l'équipement sont masquées. Décochée, seule la carte change. Une boucle de
  parents est refusée.
- **Ce lien est faux** : cliquez sur un lien, puis marquez-le faux. Il disparaît
  et n'est plus jamais déduit. Marquer faux un parent déclaré le retire aussi
  pour les alertes. Un service trouvé par un agent reste toujours sous sa
  machine.
- **Lien** : tracez un lien d'un type choisi entre deux nœuds.
- **Site** et **Groupe** : déplacez un nœud dans un autre cadre de site, ou
  regroupez-le dans la vue par dossier de la carte (son dossier sur la page des
  sondes ne change pas).

**Annuler** et **Rétablir** (Ctrl+Z, Ctrl+Maj+Z) reviennent sur chaque étape.
Sous la carte, la liste des corrections dit qui a fait chacune et quand ;
retirez-en une et la carte (et le parent déclaré qu'elle avait changé)
redevient comme avant. Les corrections s'appliquent avant la déduction, dans
cet ordre : liens faux, liens tracés et rattachements à la main, puis règles
apprises. Chaque lien dit qui l'a tracé : déduit par DumbMonit, corrigé à la
main (un losange vide sur le lien) ou règle apprise (un losange plein). Les
corrections et les règles font partie de la sauvegarde.

### Ce que DumbMonit apprend

À partir des corrections, DumbMonit propose des règles, par des heuristiques
simples et prévisibles (aucune statistique) :

- un lien marqué faux vers un équipement d'un sous-réseau propose de ne plus
  tracer ce type de lien dans tout le sous-réseau (par exemple « Ne jamais
  tracer de liens « Derrière le pare-feu » dans 10.0.5.0/24 ») ;
- deux équipements d'une même adresse rattachés sous le même parent proposent
  « ce qui est à cette adresse en dépend » (limité à leur type s'ils ont le
  même : « Les surveillances HTTP à 10.0.0.40 dépendent de vm-web ») ;
- deux équipements d'un même sous-réseau rattachés sous le même parent
  proposent « Les équipements de 10.0.5.0/24 dépendent de switch-core ».

Une suggestion n'est jamais appliquée d'elle-même : **Appliquer la règle** ou
**Pas celle-ci** (une règle refusée n'est plus proposée). Une règle appliquée
place aussi les équipements ajoutés ensuite, sauf si quelque chose de plus sûr
les place (parent déclaré, hyperviseur, service d'agent, correction). Retirez
une règle et ses liens s'en vont.

## Exporter et imprimer

**Exporter** propose :

- **Image SVG** et **Image PNG** : toute la carte avec un titre, le périmètre,
  la date et la légende, sur fond blanc, quel que soit le thème à l'écran.
- **Imprimer / PDF (A4 paysage)** et **(A3 paysage)** : le navigateur
  n'imprime que cette feuille, mise à l'échelle de la page. Choisissez
  *Enregistrer au format PDF* dans la boîte d'impression pour un PDF.

## API

`GET /api/topology` renvoie les nœuds, les liens et la source de chaque lien
(tous les rôles, lecture seule ; sans les adresses des sondes, seulement celles
que l'hyperviseur rapporte pour ses invités). `/api/topology/corrections`,
`/api/topology/rules` et `/api/topology/suggestions` portent les corrections,
les règles apprises et les suggestions (écriture réservée aux administrateurs).
Voir la
[référence de l'API](../reference/api.md).

## Liens de niveau 2 (LLDP, CDP, table du pont)

Le profil SNMP `ethernet-switch` (inclus par les profils Zyxel et HPE / Aruba)
lit aussi les voisins LLDP et CDP, la table d'adresses du pont (FDB, `dot1d` et
`dot1q`), la table ARP et les agrégats 802.3ad. La carte en tire des liens `l2` :

- **Commutateur à commutateur** : un voisin est rapproché d'une sonde par son
  nom système, à défaut par l'adresse MAC de son châssis (vue dans une table
  ARP). Le lien indique le port de chaque côté au survol ou à l'ouverture. Des
  câbles parallèles ou un agrégat entre les deux mêmes commutateurs ne font
  qu'un lien.
- **Hôte à commutateur** : une adresse MAC apprise sur un port est rapprochée,
  par l'ARP, de la sonde qui a cette adresse IP. Seuls les ports d'accès
  comptent : un port avec un voisin LLDP/CDP, membre d'un agrégat, ou qui
  apprend plus de 24 adresses est une liaison montante et ne porte aucun hôte.
  Une adresse vue sur plusieurs commutateurs est rattachée là où elle est seule
  sur son port.

Les liens sont relus à chaque collecte et disparaissent quand leurs mesures ont
plus de 30 minutes. Ils fonctionnent via un agent relais comme toute sonde SNMP.
Les corrections faites à la main l'emportent toujours, et un lien `l2` ne
propage pas une panne et ne supprime pas d'alertes à lui seul : rattachez
l'équipement à son commutateur depuis la carte (« aussi pour les alertes »).
