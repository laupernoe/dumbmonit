# AWS (CloudWatch)

L'état d'un compte AWS par CloudWatch : alarmes qui se déclenchent, instances
EC2 qui échouent à leurs contrôles d'état ou chauffent, bases RDS avec un CPU
élevé ou peu de stockage libre.

DumbMonit interroge l'API CloudWatch d'une région
(`monitoring.<région>.amazonaws.com`) avec une clé d'accès IAM en lecture
seule, une fois par minute : `DescribeAlarms` pour les alarmes à l'état
`ALARM`, `ListMetrics` pour les instances EC2 et bases RDS qui publient des
métriques, et une requête `GetMetricData` pour leurs 10 dernières minutes.
DumbMonit signe lui-même les requêtes en AWS Signature Version 4 ; aucun
logiciel AWS n'est nécessaire. Facultativement, DumbMonit endosse un rôle IAM
par STS (`sts:AssumeRole`) et signe avec ses identifiants temporaires, gardés
une heure.

C'est une API Internet : l'adresse de la surveillance est le point d'accès
CloudWatch de la région, pas celle d'une machine. Une surveillance couvre une
région d'un compte ; ajoutez-en une par région.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation d'AWS
    (référence de l'API CloudWatch, Signature Version 4, STS), et la
    signature est vérifiée avec le vecteur de test officiel d'AWS, pas
    contre un compte en fonctionnement. Elle ne fait que lire. Dites-nous ce
    qui casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API CloudWatch (HTTPS, monitoring.&lt;région&gt;.amazonaws.com) |
| 443 | TCP | STS AssumeRole, seulement avec un ARN de rôle (HTTPS, sts.&lt;région&gt;.amazonaws.com) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_aws_`.

| Métrique | Labels | Signification |
|---|---|---|
| `alarms_in_alarm` | | Nombre d'alarmes CloudWatch (de métrique et composites) à l'état `ALARM`. |
| `alarm_in_alarm` | `alarm` | 1 pour chaque alarme à l'état `ALARM`. |
| `ec2_instances` | | Instances EC2 ayant publié du CPU ces 3 dernières heures. |
| `ec2_cpu_percent` | `instance` | Plus haute valeur de `CPUUtilization` sur les 10 dernières minutes. |
| `ec2_status_check_failed` | `instance` | 1 quand `StatusCheckFailed` (instance ou système) a dépassé 0 sur les 10 dernières minutes. |
| `rds_instances` | | Instances de base RDS ayant publié du CPU ces 3 dernières heures. |
| `rds_cpu_percent` | `db` | Plus haute valeur de `CPUUtilization` sur les 10 dernières minutes. |
| `rds_free_storage_bytes` | `db` | Dernière valeur de `FreeStorageSpace`. |
| `rds_connections` | `db` | Dernière valeur de `DatabaseConnections`. |
| `ec2_readable`, `rds_readable` | | 0 quand les permissions IAM ne permettent pas de lister ou lire les métriques de ce service (la surveillance continue avec les alarmes). |

Au plus 100 instances EC2 et 100 bases RDS sont lues (une requête
`GetMetricData` est limitée à 500 requêtes) ; les comptes restent exacts. Une
instance sans point de mesure sur les 10 dernières minutes (arrêtée, ou tout
juste lancée) n'a pas de série. Les métriques agrégées (par groupe Auto
Scaling, image ou type d'instance) sont ignorées, comme les clusters Aurora :
seules les instances de base sont lues.

Les [règles intégrées](../alerting/rules.md#aws-cloudwatch) qui s'appliquent :

- **Alarme CloudWatch AWS en ALARM** : une alarme se déclenche depuis 5
  minutes (Avis, rappelée toutes les 6 heures).
- **Contrôle d'état EC2 AWS en échec** : un contrôle d'état d'instance ou de
  système échoue depuis 10 minutes (Avis).
- **CPU EC2 AWS élevé** : un pic de CPU au-dessus de 90 % depuis 30 minutes
  (Avis).
- **CPU RDS AWS élevé** : un pic de CPU au-dessus de 90 % depuis 30 minutes
  (Avis).
- **Stockage libre RDS AWS bas** : moins de 5 Gio libres depuis 15 minutes
  (Avis).

## La page de la surveillance

La page dit en une phrase ce qui ne va pas, montre les alarmes à l'état ALARM,
le nombre d'instances EC2 et de bases RDS et les contrôles d'état en échec,
puis liste les alarmes déclenchées et les instances avec leur contrôle d'état.

## Créer un utilisateur IAM en lecture seule pour DumbMonit

1. Dans la console AWS, ouvrez IAM, Policies, Create policy, onglet JSON, collez cette politique en lecture seule et nommez-la DumbMonitReadOnly.

    `{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":["cloudwatch:DescribeAlarms","cloudwatch:ListMetrics","cloudwatch:GetMetricData"],"Resource":"*"}]}`

2. Ouvrez IAM, Users, Create user : nommez-le dumbmonit, laissez l'accès à la console désactivé et attachez la politique DumbMonitReadOnly.

3. Ouvrez l'utilisateur, Security credentials, Create access key, choisissez Application running outside AWS. Copiez l'identifiant de la clé d'accès et la clé d'accès secrète : le secret n'est affiché qu'une fois.

4. Facultatif, pour passer par un rôle : attachez la politique à un rôle qui fait confiance à l'utilisateur, donnez à l'utilisateur seulement sts:AssumeRole sur ce rôle, et copiez l'ARN du rôle dans l'option Role ARN.

5. Dans DumbMonit, saisissez monitoring.&lt;région&gt;.amazonaws.com comme adresse, collez la clé et son secret, et renseignez la même région dans l'option Region.

### Permissions nécessaires

- **cloudwatch:DescribeAlarms** — Liste les alarmes à l'état ALARM, ne modifie rien.
- **cloudwatch:ListMetrics** — Trouve les instances EC2 et bases RDS qui publient des métriques.
- **cloudwatch:GetMetricData** — Lit le CPU, les contrôles d'état, le stockage libre et les connexions des 10 dernières minutes.
- **sts:AssumeRole (seulement avec un ARN de rôle)** — Permet à l'utilisateur d'endosser le rôle en lecture seule ; autorisez-le sur ce rôle uniquement.

!!! warning
    Utilisez un utilisateur IAM dédié, avec ces seules permissions de lecture, jamais une clé capable de modifier des ressources. AWS facture GetMetricData par métrique demandée (0,01 USD pour 1 000) : un intervalle de contrôle long limite le coût sur les grands comptes.

Documentation du fournisseur : https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/Welcome.html

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'accès | Identifiant de clé d'accès (`AKIA…`) et clé d'accès secrète de l'utilisateur IAM. |

Adresse : `monitoring.<région>.amazonaws.com`, par exemple
`monitoring.eu-west-3.amazonaws.com`. Options :

- **Region** (requise) : la région de l'adresse, par exemple `eu-west-3`.
  Elle fait partie de la signature : une différence est refusée par AWS.
- **Role ARN** : un rôle à endosser par STS
  (`arn:aws:iam::123456789012:role/dumbmonit-read`). Vide : les permissions
  de l'utilisateur sont utilisées.
- **STS endpoint URL** : seulement avec un rôle ; remplace
  `https://sts.<région>.amazonaws.com` (VPC endpoint, régions Chine ou GovCloud).
- **Délai par requête** (15 s).

## Dépannage

« AWS refused the request (UnrecognizedClientException / InvalidClientTokenId) » :
l'identifiant de clé d'accès est faux, ou la clé a été désactivée ou supprimée.

« AWS refused the request (SignatureDoesNotMatch) » : la clé d'accès secrète
est fausse, ou l'option Region ne correspond pas à l'adresse.

« AWS refused the request (AccessDeniedException) » : la politique n'autorise
pas `cloudwatch:DescribeAlarms`. L'absence de `ListMetrics` ou de
`GetMetricData` ne fait pas échouer la surveillance : les séries EC2 et RDS
sont simplement absentes et `ec2_readable` / `rds_readable` valent 0.

« AWS rejected the request date » : l'horloge du serveur DumbMonit est décalée
de plus de quelques minutes ; synchronisez-la.

« AWS refused to assume the role » : l'utilisateur n'a pas `sts:AssumeRole`
sur le rôle, ou la politique de confiance du rôle ne le nomme pas.
