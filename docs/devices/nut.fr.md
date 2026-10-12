# Onduleur avec NUT

Un onduleur branché sur un NAS, un Raspberry Pi ou un serveur par USB est
généralement surveillé par Network UPS Tools (NUT) : son démon, `upsd`,
publie ce que l'onduleur dit sur le port TCP 3493. DumbMonit lit ce serveur,
comme n'importe quel client NUT : si l'onduleur tourne sur secteur ou sur
batterie, si la batterie est faible ou usée, la charge, le temps restant, la
charge de sortie, la tension d'entrée et l'âge de la batterie. Un serveur
publie souvent plusieurs onduleurs ; tous sont lus en une connexion, et
chaque série porte le nom de son onduleur.

Pour un onduleur avec sa propre carte réseau, le type [appareil SNMP](snmp.md)
lit à la place la MIB standard UPS-MIB.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_nut_` et portent le label
`ups`, le nom de l'onduleur dans `ups.conf` (`ups` sur un NAS Synology,
`qnapups` sur un QNAP). Une variable que l'onduleur ne publie pas ne produit
aucune série.

| Métrique | Quoi | Labels |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | Les indicateurs de `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`) : 1 quand actif, 0 sinon, tous présents à chaque lecture | `ups` |
| `ups_status_info` | valeur 1, le `ups.status` brut | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | charge, et la charge sous laquelle l'onduleur rapporte une batterie faible | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | temps restant sur batterie à la charge actuelle, et le temps restant à batterie faible | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | temps depuis `battery.date` (installée ou remplacée), sinon `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | côté secteur ; 0 V pendant une coupure | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | côté charge | `ups` |
| `ups_load_percent` | charge, en pourcentage de la capacité de l'onduleur | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | puissance consommée et la valeur nominale, quand l'onduleur les rapporte | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 quand le dernier autotest (`ups.test.result`) rapporte une erreur ou un avertissement, 0 quand il a réussi ; absent quand aucun test n'a tourné | `ups`, `result` |
| `ups_info` | valeur 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 quand `upsd` répond mais n'a pas de données fraîches de l'onduleur (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 quand le pilote NUT de cet onduleur ne tourne pas | `ups` |
| `server_ups` | nombre d'onduleurs que le serveur publie | |
| `server_info` | valeur 1 | `version` |

Les numéros de série ne sont jamais mis dans un label. 32 onduleurs au plus
sont lus par serveur.

Les [règles intégrées](../alerting/rules.md#ups-behind-nut) qui
s'appliquent : onduleur sur batterie, batterie d'onduleur faible, batterie
d'onduleur à remplacer, charge d'onduleur élevée, temps restant d'onduleur
court, données d'onduleur périmées, plus Appareil inaccessible quand le
serveur NUT lui-même arrête de répondre.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page ne se connecte jamais au serveur NUT. Une carte par onduleur : son état
en mots (sur secteur, sur batterie, batterie faible, batterie à remplacer,
bypass, surcharge…), la charge, le temps restant, la charge de sortie, la
tension d'entrée et l'âge de la batterie.

## Laisser DumbMonit lire le serveur NUT

1. Trouvez le serveur NUT : la machine sur laquelle l'onduleur est branché (un NAS, un Raspberry Pi, un serveur) fait tourner upsd sur le port TCP 3493. Depuis l'hôte DumbMonit, vérifiez qu'il répond et notez les noms de ses onduleurs.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Avec les outils client NUT installés, `upsc -l nas.lan` donne la même liste.

2. Sur une installation NUT classique, faites écouter upsd sur une adresse que l'hôte DumbMonit peut atteindre : ajoutez cette ligne à upsd.conf, ouvrez le port TCP 3493 dans le pare-feu pour l'hôte DumbMonit, et redémarrez upsd. La lecture ne demande aucun compte : upsd laisse tout hôte qu'il accepte lire les variables, et ne change rien sans un utilisateur qui en a le droit.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Sur un NAS, le même interrupteur a un autre nom. Synology DSM : Control Panel → Hardware & Power → UPS, cochez Enable network UPS server, et ajoutez l'hôte DumbMonit sous Permitted DiskStation Devices ; l'onduleur est nommé ups. QNAP : la page UPS du Control Panel, Enable network UPS master, avec l'hôte DumbMonit parmi les adresses autorisées ; l'onduleur est nommé qnapups. TrueNAS : le service UPS avec Remote Monitor cochée.

4. Optionnel : DumbMonit n'envoie un nom d'utilisateur et un mot de passe que si vous en saisissez un, et upsd n'en demande aucun pour lire. Si votre politique veut que chaque client soit nommé, ajoutez un utilisateur sans actions ni instcmds à upsd.users, avec un mot de passe long et aléatoire, et redémarrez upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. Dans DumbMonit, saisissez l'adresse du serveur NUT, par exemple « nas.lan ». Chaque onduleur qu'il publie est surveillé ; pour n'en surveiller que certains, listez leurs noms dans l'option Onduleurs.

6. DumbMonit ne fait que lire : il envoie LIST UPS et LIST VAR, jamais LOGIN, SET, INSTCMD ou FSD, si bien que le serveur ne le compte jamais comme un secondaire à attendre avant de s'arrêter.

!!! warning
    DumbMonit surveille l'onduleur ; il n'arrête rien. Les machines que l'onduleur alimente ont toujours besoin de leur propre client NUT (upsmon) pour s'arrêter proprement quand la batterie faiblit.

## Identifiants

| Identifiant | Champs |
|---|---|
| Aucune authentification | Le défaut : `upsd` laisse tout hôte qu'il accepte lire les variables de l'onduleur. |
| Nom d'utilisateur / mot de passe upsd | Envoyés avec `USERNAME` et `PASSWORD` avant la lecture. `upsd` ne les vérifie que pour les commandes, que DumbMonit n'envoie jamais. |

Adresse : un nom d'hôte ou une IP (`nas.lan`), `host:port`, ou la forme
`ups@host` de `upsc`, dont seul l'hôte est gardé. Options : port (3493),
l'onduleur à surveiller (tous par défaut) et le délai par échange (10 s).

Une connexion refusée ou aucune réponse compte comme l'appareil inaccessible.
`ACCESS-DENIED` est affiché comme une erreur d'identifiant, et un nom
d'onduleur que le serveur ne publie pas comme une erreur de configuration ;
aucun des deux ne réveille personne la nuit. La connexion est en TCP simple :
`STARTTLS` n'est pas utilisé, et rien de lu n'est secret.
