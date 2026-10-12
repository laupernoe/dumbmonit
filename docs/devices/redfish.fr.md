# Matériel serveur (Redfish)

Le propre matériel d'un serveur, lu depuis son contrôleur de gestion
(BMC) : ventilateurs, températures par rapport à leurs propres seuils,
alimentations et leur redondance, disques, mémoire, processeurs et le
journal d'événements.

Redfish est l'interface HTTPS que le DMTF a standardisée pour les
contrôleurs de gestion, et chaque contrôleur actuel la parle : Supermicro,
Dell iDRAC, HPE iLO, Lenovo XClarity Controller, ASRock Rack, OpenBMC.
C'est la bonne façon de surveiller le matériel serveur. L'agent SNMP d'un
BMC, à l'inverse, décrit généralement le BMC lui-même — son petit Linux
ARM, sa mémoire, ses liens réseau — et pas le serveur qu'il gère (voir
[SNMP](snmp.md#server-management-controllers-bmc)).

!!! warning "Testé seulement contre les mockups de référence DMTF"

    Cette intégration a été construite et vérifiée contre les mockups
    Redfish officiels du DMTF (serveur rack, serveur tour avec stockage
    local, serveur refroidi par liquide avec les schémas thermiques et
    d'alimentation plus récents), servis par le serveur de mockups du
    DMTF. Chaque appel est une lecture et les formes de ressource sont les
    standards, mais ça n'a pas encore tourné contre un vrai Supermicro,
    iDRAC ou iLO pendant des semaines. Dites-nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_redfish_`. Les séries de
santé sont **0 (OK), 1 (Avertissement) ou 2 (Critique)**, directement
depuis `Status.Health`.

| Famille | Métriques | Labels |
|---|---|---|
| Service | `info` (valeur 1) | `vendor`, `product`, `redfish_version` |
| Système | `system_health` (la propre), `system_health_rollup` (tout ce qu'il contient), `system_power_on`, `memory_health`, `processor_health` (les résumés du contrôleur, une valeur pour tous les DIMM et tous les CPU), `memory_total_bytes` | `system` |
| Châssis | `chassis_health`, `chassis_health_rollup` | `chassis` |
| Températures | `temperature_celsius`, `temperature_upper_critical_celsius` et `temperature_upper_caution_celsius` (les propres seuils critique et d'avertissement du capteur, quand il les déclare), `temperature_health` | `chassis`, `sensor` |
| Ventilateurs | `fan_speed_rpm` (ou `fan_speed_percent` quand le contrôleur rapporte un pourcentage), `fan_lower_critical_rpm` (ou `_percent` : le propre minimum déclaré du ventilateur, ancien schéma seulement), `fan_health`, `fan_redundancy_health` | `chassis`, `fan` / `group` |
| Alimentation | `psu_health`, `psu_output_watts`, `psu_capacity_watts`, `power_redundancy_health`, `power_consumed_watts` | `chassis`, `psu` / `group` |
| Tensions | `voltage_volts`, `voltage_health` | `chassis`, `sensor` |
| Stockage | `storage_health`, `drive_health`, `drive_failure_predicted`, `drive_life_left_percent` (SSD), `drive_capacity_bytes` | `system`, `storage`, `drive`, `media` |
| Contrôleur | `manager_health`, `manager_info` (valeur 1) | `manager`, `firmware`, `model` |
| Journaux | `log_entries`, `log_critical_entries`, `log_warning_entries` | `owner` (système ou contrôleur), `log` |
| Collection | `scrape_errors`, `scrape_duration_seconds` | |

**Un emplacement vide n'est pas une panne.** Un composant dont `Status.State`
est `Absent` (une baie d'alimentation vide, un emplacement DIMM libre, un
second CPU manquant) ou `Disabled` (un groupe de redondance non configuré)
ne produit aucune série du tout. Un serveur livré avec une alimentation sur
deux est en bonne santé, et se lit comme tel.

**Deux générations du schéma, un seul ensemble de séries.** La plupart des
contrôleurs en service publient encore les anciennes ressources
`Chassis/{id}/Thermal` et `Chassis/{id}/Power`, une requête par capteur ;
les plus récents ne publient que `ThermalSubsystem`, `PowerSubsystem`,
`EnvironmentMetrics` et `Sensors`, une ressource par ventilateur,
alimentation et capteur. DumbMonit lit l'ancienne forme quand elle existe
et la nouvelle sinon ; les deux produisent exactement les mêmes séries et
labels, une règle écrite une fois fonctionne donc sur les deux.

**Les journaux sont comptés, jamais lus.** Les entrées de journal peuvent
nommer des hôtes, adresses et utilisateurs : DumbMonit garde seulement
combien d'entrées il y a, et combien de la première page que le contrôleur
sert sont Critical ou Warning.

Les [règles intégrées](../alerting/rules.md#server-hardware-redfish) qui
s'appliquent : ventilateur serveur en échec, température serveur
au-dessus du critique, redondance d'alimentation perdue, alimentation en
échec, échec de disque prédit, santé serveur critique, plus Appareil
inaccessible.

## La page de l'appareil

Un appareil Redfish obtient son propre panneau sur sa page, au-dessus des
graphiques, lu depuis ce que la dernière sonde a stocké : ouvrir la page
n'interroge jamais le contrôleur. Il répond, dans cet ordre :

1. **Le serveur est-il en bonne santé.** Un mot — Sain, Dégradé ou
   Critique — depuis la propre santé et les agrégats du contrôleur, et en
   dessous chaque composant qui la tire vers le bas, en une phrase :
   *CPU1 Temp est à 47 °C, à ou au-dessus de son seuil critique de 45 °C*,
   *La redondance d'alimentation est perdue*, *Le disque 3 prédit sa
   propre panne*. Quand le contrôleur agrège le serveur comme dégradé sans
   rien nommer que DumbMonit lit, le panneau le dit et pointe vers son
   journal d'événements. En dessous : état d'alimentation, consommation,
   mémoire installée et le firmware du contrôleur.
2. **Ventilateurs et températures sont-ils dans leurs limites.** Chaque
   capteur est comparé aux seuils que le contrôleur déclare **pour ce
   capteur** — ses températures d'avertissement et critique, la vitesse
   minimale d'un ventilateur — jamais avec un chiffre de DumbMonit. Un
   capteur qui ne déclare aucun seuil montre sa lecture et aucun verdict,
   sauf si le contrôleur lui-même le rapporte dégradé. Les problèmes sont
   listés en premier.
3. **L'alimentation est-elle redondante.** Chaque groupe de redondance
   (Redondant, Redondance dégradée, Redondance perdue), puis chaque
   alimentation présente avec sa sortie par rapport à sa capacité et son
   état. Sans groupe de redondance, le panneau dit que le contrôleur n'en
   déclare aucun : une seule alimentation, ou redondance non configurée.
4. **Les disques sont-ils en bonne santé.** Chaque contrôleur de stockage,
   puis chaque disque avec sa capacité, la vie restante d'un SSD et son
   état ; *Panne prédite* vient avant tout le reste.
5. **Mémoire, processeurs et contrôleur** : les résumés en un mot du
   contrôleur pour tous les DIMM et tous les CPU, la propre santé, modèle
   et firmware du contrôleur de gestion, et les tensions.
6. **Journaux d'événements** : entrées comptées par gravité. Les entrées
   restent jusqu'à ce que quelqu'un vide le journal, donc un compteur
   critique peut être une vieille nouvelle : il est montré, mais ne change
   pas le mot de santé du serveur.

Les baies vides et emplacements libres n'apparaissent jamais : ils ne
produisent aucune série. Chaque état est un mot ainsi qu'une couleur, et
une valeur inconnue lit « — », jamais 0.

## Créer un compte en lecture seule sur le contrôleur de gestion

1. Ouvrez l'interface web du contrôleur de gestion (BMC), pas le système d'exploitation du serveur. Créez un nouvel utilisateur local nommé comme suit, avec un mot de passe long utilisé nulle part ailleurs.

    ```
    dumbmonit
    ```

2. Donnez-lui le rôle le plus bas qui puisse lire, et rien de plus. Supermicro : Configuration → Users → Add User, privilège User, et cochez Redfish dans le type de compte. Dell iDRAC : iDRAC Settings → Users → Local Users → Add, rôle Read Only. HPE iLO : Administration → User Administration → New, décochez chaque privilège sauf Login. Lenovo XClarity Controller : BMC Configuration → User/LDAP → Create, rôle Read-only. ASRock Rack : Settings → User Management, privilège User.

3. Vérifiez depuis n'importe quelle machine sur le réseau de gestion que le compte peut lire Redfish. La commande demande le mot de passe et affiche le nom et la santé du serveur.

    ```
    curl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems
    ```

4. Dans DumbMonit, saisissez l'adresse du contrôleur, par exemple « bmc.lan » ou « 10.0.0.50 », puis le nom d'utilisateur et le mot de passe de ce compte.

5. DumbMonit ne fait que lire : ventilateurs, températures, tensions, alimentations, disques, résumés de mémoire et de processeur, et combien d'entrées de journal il y a par gravité. Il n'allume ni n'éteint jamais le serveur, ne change jamais un réglage et ne lit jamais le texte des journaux.

!!! warning
    Ne réutilisez pas le compte d'usine du contrôleur : il peut éteindre le serveur, monter des médias et reflasher le firmware. Les contrôleurs de gestion livrent aussi un certificat autosigné : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options. Un contrôleur répond lentement et une lecture complète prend plusieurs requêtes : si l'appareil rapporte des délais dépassés, augmentez DUMBMONIT_PROBE_TIMEOUT_SECS sur le serveur.

Les noms de menu ci-dessus sont ceux du firmware actuel ; les générations
plus anciennes les déplacent mais gardent les mêmes rôles. Sur Supermicro,
le type de compte montré à la création d'un utilisateur décide quelles
interfaces il peut utiliser : un compte sans accès Redfish est refusé avec
un 401 même avec le bon mot de passe.

## Identifiants

| Identifiant | Champs |
|---|---|
| Compte de contrôleur de gestion | Nom d'utilisateur et mot de passe du compte en lecture seule. |

Adresse : le nom d'hôte ou l'IP du contrôleur, par exemple `bmc.lan` ou
`10.0.0.50`. Le port par défaut est 443 ; écrivez `host:port` pour un
autre, ou une URL complète (`https://bmc.lan:8443`).

## Options

| Option | Défaut | Effet |
|---|---|---|
| `port` | `443` | Port HTTPS, si l'adresse n'en donne pas. |
| `insecure_tls` | `false` | Accepter un certificat autosigné. |
| `request_timeout_seconds` | `8` | Temps accordé à chaque appel Redfish, de 1 à 120. |
| `auth` | `basic` | `basic` envoie le nom d'utilisateur et le mot de passe à chaque requête et ne laisse rien ouvert sur le contrôleur. `session` se connecte une fois et réutilise le jeton de session (`X-Auth-Token`), ne le rouvrant que quand le contrôleur le refuse et fermant celui qu'il remplace. Utilisez-le seulement si le contrôleur refuse l'authentification basique : les contrôleurs n'ont qu'une poignée d'emplacements de session. |
| `storage` | `true` | Lire les contrôleurs de stockage et chaque disque derrière eux. Une requête par disque. |
| `logs` | `true` | Compter les entrées du journal d'événements système et du propre journal du contrôleur, par gravité. |

Une lecture complète est quelques dizaines de requêtes : la racine de
service, puis chaque système, châssis et contrôleur, leurs ressources
thermiques et d'alimentation, les contrôleurs de stockage et disques, les
services de journal. DumbMonit en envoie quatre au plus à la fois. Un
contrôleur lent peut avoir besoin d'un délai de sonde plus long
(`DUMBMONIT_PROBE_TIMEOUT_SECS`, voir [Configuration](../reference/configuration.md))
et un intervalle de sonde de deux minutes ou plus.

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| *Authentification refusée* | Mauvais mot de passe, ou le compte n'est pas autorisé à utiliser Redfish (type de compte Supermicro, privilège « Login » manquant sur iLO). |
| *Privilèges insuffisants* | Le rôle est trop bas pour une ressource ; le reste de la lecture passe quand même. Certains contrôleurs cachent le journal d'événements à leur rôle le plus bas. |
| *Certificat TLS rejeté* | Le certificat autosigné du contrôleur : cochez « Accepter un certificat non vérifiable », ou installez un certificat de confiance sur le contrôleur. |
| *Délai dépassé* | Le contrôleur répond lentement : augmentez `DUMBMONIT_PROBE_TIMEOUT_SECS`, désactivez `storage` ou `logs`, ou allongez l'intervalle. |
| Pas de seuils de température | Le contrôleur ne publie pas `UpperThresholdCritical` pour ce capteur : la température est quand même enregistrée, mais la règle intégrée ne peut la comparer à rien. |
