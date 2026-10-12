# Score de sécurité

Chaque appareil supporté obtient un **score de sécurité** de 0 à 100 et
une note de **A** à **F**, dans l'esprit de PingCastle ou Microsoft
Secure Score : une liste de contrôles tirés des propres recommandations
de bonnes pratiques du fournisseur, chacun réussi, en échec ou non évalué.

Le score apparaît comme une carte **Sécurité** sur la page de l'appareil,
et sur la page **Sécurité**, qui liste chaque appareil noté, le pire
d'abord.

## Comment le score est calculé

DumbMonit ne demande jamais rien d'extra à l'appareil pour calculer le
score. Les contrôles lisent les dernières valeurs que les collecteurs ont
déjà enregistrées (en regardant 6 heures en arrière), un score ne coûte
donc rien à l'appareil et fonctionne hors ligne depuis la dernière sonde.

Chaque contrôle a :

| Champ | Signification |
|---|---|
| Catégorie | exposition, correctifs, authentification, chiffrement, sauvegarde ou configuration |
| Gravité | critical, high, medium ou low — son poids dans le score : 10, 6, 3, 1 |
| Résultat | **réussi**, **échoué**, ou **inconnu** quand la donnée n'est pas collectée |
| Preuve | ce qui a été observé : `pve1 : 3 mises à jour de sécurité`, `Expire dans 9 jours`… |
| Remédiation | quoi faire, dans les termes du fournisseur |
| Référence fournisseur | la page du fournisseur (ou du standard) sur laquelle le contrôle est basé |

**Score** = 100 × (poids des contrôles réussis) ÷ (poids des réussis +
échoués). **Les contrôles inconnus sont montrés mais jamais comptés**,
ni pour ni contre : un score ne doit pas baisser parce qu'un jeton de
surveillance manque un privilège, ni monter parce que rien n'est connu.
Un appareil dont tous les contrôles sont inconnus n'est *pas noté*.

| Note | Score | Mot |
|---|---|---|
| A | 90–100 | Fort |
| B | 75–89 | Bon |
| C | 60–74 | Correct |
| D | 40–59 | Faible |
| F | 0–39 | Mauvais |

**Un contrôle critique en échec plafonne la note à C**, quel que soit le
score : un pare-feu qui ne filtre plus, ou un firmware activement
exploité laissé non corrigé, n'est pas compensé par dix petits bons
réglages. La carte le dit quand ça arrive.

## Historique et alertes

Le serveur recalcule chaque score toutes les 15 minutes et le stocke
comme `dumbmonit_security_score{target="<id>"}` (et le nombre de
contrôles en échec comme `dumbmonit_security_checks_failed`), vous
pouvez donc le tracer et alerter sur lui comme toute autre métrique. Deux
règles intégrées l'utilisent :

- **Score de sécurité en baisse** — le score a chuté de 10 points ou
  plus par rapport à sa meilleure valeur des dernières 24 heures :
  quelque chose de nouveau échoue.
- **Score de sécurité faible** — le score est sous 40 (note F). Modifiez
  la règle pour déplacer le seuil à votre propre barre.

Voir [Règles d'alerte](../alerting/rules.md#security-score).

## Contrôles par type

### Proxmox VE

| Contrôle | Gravité | Donnée |
|---|---|---|
| Mises à jour de sécurité installées | high | mises à jour de sécurité en attente par nœud (a besoin de `Sys.Modify` sur le jeton, sinon inconnu) |
| Paquets Proxmox VE à jour | medium | paquets Proxmox pouvant être mis à niveau par nœud |
| Fait tourner le noyau installé | medium | noyau plus récent installé que celui qui tourne |
| Aucun dépôt de test utilisé | medium | dépôts standards activés |
| Dépôts de paquets correctement configurés | medium | erreurs et avertissements que Proxmox VE lève sur les dépôts (p. ex. dépôt enterprise sans abonnement) |
| Abonnement du dépôt enterprise | low | statut d'abonnement par nœud |
| Certificat de l'interface web valide pour 14+ jours | medium | expiration du certificat de nœud |
| Chaque invité est dans un job de sauvegarde | high | invités non sélectionnés par aucun job de sauvegarde |
| Les jobs de sauvegarde réussissent | high | dernier résultat de chaque job de sauvegarde |

Sources : [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server

| Contrôle | Gravité | Donnée |
|---|---|---|
| Mises à jour système installées | high | mises à jour en attente (a besoin de `Sys.Modify` sur `/system`) |
| Fait tourner la version Backup Server installée | medium | démons tournant encore la version pré-mise à niveau |
| Job de vérification planifié | high | nombre de jobs de vérification |
| Job de nettoyage planifié | medium | nombre de jobs de nettoyage (prune) |
| Sauvegardes copiées ailleurs (sync ou bande) | medium | jobs de sauvegarde sync et bande (règle 3-2-1) |
| Ramasse-miettes propre, pas de chunks corrompus | high | dernier résultat de GC et chunks corrompus par datastore |
| Certificat valide pour 14+ jours | medium | expiration du certificat |

Sources : [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html),
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html),
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html).

### Synology DSM

| Contrôle | Gravité | Donnée |
|---|---|---|
| DSM 7 ou plus récent | high | version DSM |
| Tâche Hyper Backup configurée | medium | nombre de tâches Hyper Backup |
| Les tâches Hyper Backup réussissent | high | dernier résultat de chaque tâche |

Sources : [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense et pfSense

| Contrôle | Gravité | Donnée |
|---|---|---|
| OPNsense : mises à jour de firmware installées | high | mises à jour de paquet en attente (inconnu jusqu'à ce qu'OPNsense ait vérifié les mises à jour) |
| OPNsense : sur la série de version majeure actuelle | medium | nouvelle série majeure disponible |
| OPNsense : aucun redémarrage en attente après une mise à jour | medium | redémarrage requis |
| OPNsense : filtre de paquets activé | **critical** | statut `pf` |
| pfSense : paquet d'API REST à jour | low | mise à jour du paquet d'API REST disponible |

Sources : [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html),
[pfSense REST API](https://pfrest.org/).

### UniFi

| Contrôle | Gravité | Donnée |
|---|---|---|
| Firmware d'appareil à jour | high | indicateur de mise à niveau de chaque appareil adopté |

Source : [UniFi releases](https://community.ui.com/releases).

### Moniteurs TLS et HTTPS

Pour les moniteurs `tls`, `http`, `smtp`, `mqtt` et `websocket` qui
négocient TLS. Un moniteur en texte clair n'a pas de carte Sécurité.

| Contrôle | Gravité | Donnée |
|---|---|---|
| Certificat valide pour 14+ jours | high | jours avant expiration |
| Chaîne de certificat de confiance | high | vérification de chaîne |
| TLS 1.3 négocié | low | version de protocole négociée |

Sources : [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final),
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/).

### Serveurs de temps NTP

| Contrôle | Gravité | Donnée |
|---|---|---|
| Le serveur rapporte une horloge synchronisée | medium | stratum et indicateur de saut |
| Décalage d'horloge dans les 100 ms | low | décalage mesuré |

Un stratum de 16 (ou 0, un « kiss-o'-death ») et un indicateur de saut
« non synchronisé » font tous les deux échouer le premier contrôle quel
que soit le décalage : un serveur qui ne s'est jamais synchronisé n'a
rien de fiable à offrir. Le seuil de 100 ms est fixe ici et ne suit pas
l'option « Décalage toléré » par appareil, de la même façon que le
contrôle de certificat avertit toujours à quatorze jours quels que
soient les propres réglages d'un moniteur.

Source : [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905).

### Nginx et Apache httpd

| Contrôle | Gravité | Donnée |
|---|---|---|
| L'en-tête serveur ne révèle pas la version | low | l'en-tête de réponse `Server` capturé en même temps que la page de statut |

Un en-tête qui nomme encore le produit (« nginx », « Apache ») sans
numéro de version réussit : c'est ce que produisent `server_tokens off`
et `ServerTokens Prod`, et un en-tête absent réussit aussi. Un en-tête
comme « nginx/1.25.3 » ou « Apache/2.4.58 (Unix) » échoue : il dit à un
scanner exactement quels CVE essayer.

Sources : [Nginx core module reference](https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens),
[Apache core module reference](https://httpd.apache.org/docs/2.4/mod/core.html#servertokens).

### Machines avec l'agent (et leurs conteneurs Docker)

| Contrôle | Gravité | Donnée |
|---|---|---|
| Mises à jour de sécurité installées | high | mises à jour de sécurité en attente (là où le gestionnaire de paquets les distingue) |
| Pas de mise à jour en attente | low | mises à jour en attente |
| Pas de redémarrage en attente | medium | redémarrage requis |
| SELinux en mode enforcing | medium | mode SELinux (inconnu là où SELinux est absent) |
| Images de conteneur à jour | medium | image plus récente dans le registre (seulement sur les machines avec des conteneurs) |
| Aucune image de conteneur plus vieille que 180 jours | low | âge de l'image (seulement sur les machines avec des conteneurs) |

Sources : [CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index),
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often).

### Mises à jour fournisseur et autres contrôles

Un contrôle de correctifs par produit, depuis l'information de mise à
jour que chaque collecteur lit déjà :

| Type | Contrôle | Gravité |
|---|---|---|
| FortiGate | Correctif FortiOS installé | **critical** |
| FortiGate | Licences FortiGuard valides | high |
| MikroTik | RouterOS à jour | high |
| MikroTik | Firmware RouterBOOT correspond à RouterOS | low |
| Home Assistant | Composants Home Assistant à jour | medium |
| Nextcloud | Serveur Nextcloud à jour | high |
| Nextcloud | Applications Nextcloud à jour | medium |
| AdGuard Home | AdGuard Home à jour | medium |
| Pi-hole | Pi-hole à jour | medium |
| Plex | Plex Media Server à jour | medium |
| Paperless-ngx | Paperless-ngx à jour | medium |
| Tailscale | Clients Tailscale à jour | medium |

### Active Directory

Un appareil Active Directory est noté depuis les constats de style
PingCastle de son collecteur : comptes Kerberoastable, comptes sans
préauthentification Kerberos, délégation sans contrainte, âge du mot de
passe `krbtgt`, taille des groupes privilégiés, couverture LAPS,
politique de mot de passe, comptes inactifs. Chaque constat est un
contrôle qui réussit quand il concerne zéro objet ; la gravité que le
collecteur rapporte est celle utilisée. Les constats pas encore
rapportés sont listés comme inconnus.

Sources : [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview),
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password).

### GitLab

Lu depuis l'API d'administration, avec un jeton d'accès personnel
d'administrateur ; inconnu avec un ordinaire.

| Contrôle | Gravité | Donnée |
|---|---|---|
| Double authentification requise | high | réglage `require_two_factor_authentication` |
| Inscription publique désactivée | medium | réglage `signup_enabled` |
| Migrations de base de données appliquées | medium | migrations en attente après une mise à niveau |

Source : [GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html).

### Forgejo et Gitea

| Contrôle | Gravité | Donnée |
|---|---|---|
| Les tâches planifiées tournent à l'heure | medium | tâches dont la prochaine exécution est déjà en retard |

Inconnu sans un jeton d'accès d'administrateur du site : Forgejo et
Gitea réservent la liste des tâches planifiées à l'administration.

Source : [Forgejo API](https://forgejo.org/docs/latest/user/api/).

## Limites

Le score ne reflète que ce que DumbMonit peut voir. C'est une invitation
à regarder, pas un audit, et une note A ne signifie pas qu'un appareil
est sûr.

Bonnes pratiques qui ne sont **pas encore** vérifiées parce que les
collecteurs ne lisent pas la donnée aujourd'hui :

| Type | Non vérifié (a besoin de nouvelle donnée) |
|---|---|
| Proxmox VE | pare-feu datacenter/nœud activé ; double authentification sur `root@pam` et autres comptes admin |
| Proxmox Backup Server | chiffrement côté client des sauvegardes (a besoin de lire les manifestes de sauvegarde) |
| Synology DSM | vérification en 2 étapes pour les administrateurs ; compte `admin` par défaut désactivé ; blocage automatique ; pare-feu DSM ; redirection HTTP vers HTTPS ; exposition QuickConnect ; si une mise à jour DSM plus récente est disponible |
| OPNsense / pfSense | interface web ou SSH accessible depuis le WAN ; identifiants par défaut ; mises à jour système pfSense (pas l'API REST) |
| UniFi | SSH d'appareil avec identifiants par défaut ; réglages de site |
| Moniteurs TLS | TLS 1.0/1.1 ou suites de chiffrement faibles encore proposées — la sonde ne négocie que TLS 1.2 et 1.3, donc un serveur limité à des versions plus anciennes se montre comme une sonde échouée à la place |
| Hôtes agent | statut du pare-feu hôte ; configuration SSH ; statut Windows Defender |
| Docker | conteneurs tournant en privileged, en root, ou avec le socket Docker monté |

Autres limites :

- Un contrôle n'est frais que comme sa donnée : certains collecteurs
  relisent les mises à jour ou abonnements toutes les quelques heures.
- Les poids sont un choix produit, le même pour chaque installation.
- Les types sans aucun contrôle n'ont pas de carte Sécurité et
  n'apparaissent pas sur la page Sécurité.
