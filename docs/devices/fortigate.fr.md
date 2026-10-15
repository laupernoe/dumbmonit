# FortiGate

Un Fortinet FortiGate via l'API REST FortiOS : tunnels IPsec site à site en
panne, interfaces activées qui ont perdu leur lien, cluster HA désynchronisé
ou avec un membre manquant, mémoire approchant du mode conserve, licences
FortiGuard et FortiCare arrivant à expiration, et releases de correctif
proposées pour la branche FortiOS en cours.

DumbMonit utilise un utilisateur d'API REST dont le profil d'accès ne fait
que lire, et qui n'accepte que les appels depuis l'adresse du serveur
DumbMonit (hôte de confiance). Son jeton est envoyé comme
`Authorization: Bearer`, jamais dans l'URL : FortiOS 7.4.5 et 7.6.1 le
refusent là de toute façon.

Chaque minute, DumbMonit lit trois pages requises —
`/api/v2/monitor/system/status`, `/api/v2/monitor/system/resource/usage`
et `/api/v2/monitor/system/interface/select` — et six optionnelles qu'un
profil plus restreint, ou un FortiGate sans cluster ni tunnels, peut
refuser ou laisser vides sans faire échouer le reste : l'état
administratif des interfaces (`/api/v2/cmdb/system/interface`), les
statistiques et sommes de contrôle HA, les tunnels IPsec, le statut de
licence et le firmware disponible.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation
    FortiOS de Fortinet et des formes de réponse enregistrées par
    l'exportateur Prometheus communautaire pour FortiGate, pas contre un
    FortiGate en fonctionnement. Chaque appel est une lecture. Les pages
    de licence et de firmware sont les moins documentées : dites-nous ce
    qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_fortigate_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Valeur 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Usage actuel. |
| `sessions`, `session_setup_rate` | | Sessions actives, nouvelles sessions par seconde. |
| `log_disk_ok` | | 0 quand le disque de logs a besoin d'être formaté. |
| `interface_link_up` | `interface`, `alias` | Lien physique. |
| `interface_enabled` | `interface`, `alias` | État administratif ; chaque interface compte comme activée quand le profil ne peut pas lire la configuration. |
| `interface_down` | `interface`, `alias` | 1 quand l'interface est activée, a une adresse et n'a pas de lien. Les ports libres sans adresse sont laissés tranquilles. |
| `interface_speed_mbps` | `interface`, `alias` | Tant que le lien est actif. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Compteurs. |
| `interfaces_down` | | Compteur. |
| `ha_members` | | Membres dans le cluster ; 0 pour un FortiGate autonome. |
| `ha_in_sync` | | Avec deux membres ou plus : 1 quand chaque membre a la même somme de contrôle de configuration. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | Par membre. |
| `ipsec_tunnel_up` | `tunnel` | 1 quand au moins une phase 2 du tunnel est active. Les tunnels dial-up (accès distant) sont laissés de côté. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | Par sélecteur de phase 2. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Compteurs. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Valeur 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Compteurs. |
| `license_status_info` | `license`, `status` | Valeur 1, pour chaque licence que le FortiGate détient (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…) ; les entrées `no_license` sont laissées de côté. |
| `license_expired` | `license` | 1 quand le statut est `expired`. |
| `license_expiry_seconds` | `license` | Jusqu'à l'expiration ; négatif une fois expiré. |
| `firmware_update_available` | | 1 quand FortiGuard propose une release de correctif plus récente de la branche en cours (7.4.4 → 7.4.5). Une nouvelle branche (7.6) est un projet, pas une alerte. |
| `firmware_latest_info` | `version` | Valeur 1 : cette release de correctif. |

Les [règles intégrées](../alerting/rules.md#fortigate) qui s'appliquent :

- **Tunnel IPsec FortiGate en panne** : aucune phase 2 active depuis cinq
  minutes (Avertissement).
- **Interface FortiGate sans lien** : depuis cinq minutes (Avis).
- **HA FortiGate désynchronisé** : depuis quinze minutes (Avis).
- **Membre HA FortiGate perdu** : moins de membres qu'à tout moment dans
  les dernières 24 heures (Avertissement). Le compteur attendu n'est publié
  nulle part, donc un membre retiré pour de bon arrête d'alerter le jour
  suivant.
- **Mémoire FortiGate proche du mode conserve** : au-dessus de 85 % pendant
  dix minutes (Avis). À 88 % par défaut, FortiOS entre en mode conserve et
  arrête d'inspecter les nouvelles sessions.
- **CPU FortiGate élevé** : au-dessus de 90 % pendant quinze minutes
  (Avis).
- **Licence FortiGate arrivant à expiration** : moins de 30 jours
  restants, ou expirée (Avis, rappelé chaque jour).
- **Mise à jour firmware FortiGate** : une release de correctif est
  proposée (Info, rappelée chaque semaine).

## La page de la surveillance

Le panneau Firewall dit ce qui ne va pas en une phrase, montre le CPU, la
mémoire, les sessions et l'état HA, puis liste les tunnels IPsec (ceux en
panne d'abord), les interfaces sans lien, les licences qui expirent dans
les 60 jours ou expirées, et une release de correctif disponible.

## Créer un utilisateur d'API REST en lecture seule pour DumbMonit

1. Depuis la CLI du FortiGate, créez un profil d'accès qui ne peut lire que l'état système, réseau et VPN, et rien d'autre.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Créez l'utilisateur d'API REST dumbmonit avec ce profil, ne faisant confiance qu'à l'adresse du serveur DumbMonit (remplacez 192.0.2.10), puis générez son jeton : il est affiché une fois.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. Dans DumbMonit, saisissez l'adresse du FortiGate, par exemple « fw.lan » (ajoutez le port si l'interface d'administration n'écoute pas sur 443), et collez le jeton comme Jeton d'API. FortiGate livre un certificat autosigné : cochez Accepter un certificat non vérifiable sauf si vous avez installé le vôtre.

!!! warning
    Une interface ne compte comme en panne que si elle est activée, a une adresse et n'a pas de lien : les ports libres sont laissés tranquilles. Les mises à jour de firmware sont celles que FortiGuard propose au FortiGate lui-même : sans accès à FortiGuard, aucune n'est rapportée.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'API | Le jeton affiché par `execute api-user generate-key dumbmonit`, envoyé comme `Authorization: Bearer`. |

Adresse : le nom d'hôte ou l'IP du FortiGate, `host:port`, ou une URL.
Options :

- **Port** (443) : le port HTTPS de l'interface d'administration (`set
  admin-sport` sous `config system global`).
- **VDOM** : le domaine virtuel à lire ; vide pour celui auquel
  appartient l'utilisateur d'API REST.
- **Accepter un certificat non vérifiable** : le propre certificat du
  FortiGate est autosigné.
- **Délai par requête** (15 s).

## Dépannage

« FortiGate refused the API token (401) » : le jeton est incorrect ou a
été régénéré. Générez-en un nouveau avec
`execute api-user generate-key dumbmonit`.

« FortiGate refused … (403) » : l'adresse DumbMonit n'est pas parmi les
hôtes de confiance de l'utilisateur, ou son profil ne peut pas lire cette
page. Une page optionnelle refusée ainsi est simplement laissée de côté.

Aucun chiffre IPsec, de licence ou de firmware : le profil ne peut pas les
lire, ou le FortiGate n'a pas de tunnel ; le reste est quand même lu.
