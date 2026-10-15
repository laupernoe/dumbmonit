# Score de sécurité

Le score de sécurité est une note **facultative** de **A** à **F** (et un score
de 0 à 100) affichée à côté des appareils pour lesquels quelques contrôles
disent quelque chose d'utile : hôtes Linux et Windows avec l'agent, Proxmox VE
et Proxmox Backup Server, Synology, OPNsense, sites web et Active Directory.
Les autres appareils n'affichent rien.

!!! warning "Indicatif, pas un audit"
    Le score est **approximatif et non exhaustif**. Il lit uniquement ce que
    DumbMonit collecte déjà sur un appareil et le compare à une poignée de
    bonnes pratiques du fournisseur. Une note A ne signifie pas qu'un appareil
    est sûr, et un contrôle absent ne signifie pas que le réglage est correct.
    Servez-vous-en comme d'une invitation à regarder, puis utilisez un vrai
    outil d'évaluation (PingCastle, Mozilla HTTP Observatory, Lynis, le
    conseiller de sécurité du fournisseur…) quand cela compte.

## L'activer

Le score est **désactivé par défaut**. Un administrateur l'active dans
**Paramètres → Compte et sécurité → Score de sécurité**. Une fois activé :

- chaque appareil noté reçoit un petit **badge en forme de bouclier** avec sa
  lettre dans la liste des appareils, et un badge **Security** avec le score
  sur sa page ;
- un clic sur le badge ouvre le **rapport** de l'appareil : ce qui échoue, avec
  la preuve et le correctif, ce qui réussit, les étapes pour améliorer le score
  sur ce type d'appareil, et un lien vers la section correspondante ci-dessous.

Le désactiver masque les badges, arrête le calcul périodique, et les séries
`dumbmonit_security_score` ne sont plus écrites.

!!! note "Mise à niveau depuis 0.1.0-alpha.8 ou antérieure"
    Le score était auparavant toujours actif, avec sa propre page
    **Security**. Une installation qui surveillait déjà un appareil noté garde
    le score activé après la mise à niveau ; une nouvelle installation démarre
    avec le score désactivé. L'ancienne adresse `/security` ouvre désormais le
    réglage.

## Comment le score est calculé

DumbMonit ne demande jamais rien de plus à l'appareil pour calculer le score.
Les contrôles lisent les dernières valeurs que les collecteurs ont déjà
enregistrées (en remontant sur 6 heures) : un score ne coûte donc rien à
l'appareil.

Chaque contrôle a une catégorie (exposition, correctifs, authentification,
chiffrement, sauvegarde, configuration), une **gravité** qui est son poids
(critical 10, high 6, medium 3, low 1), un résultat (**réussi**, **échoué**, ou
**inconnu** quand la donnée n'est pas collectée), la preuve, le correctif et un
lien vers la page du fournisseur sur laquelle il se base.

**Score** = 100 × (poids des contrôles réussis) ÷ (poids des contrôles réussis
+ échoués). **Les contrôles inconnus sont affichés mais jamais comptés** : un
score ne doit pas baisser parce qu'un compte de supervision manque d'un
privilège, ni monter parce que rien n'est connu. Un appareil dont tous les
contrôles sont inconnus n'est *pas noté*.

| Note | Score | Mot |
|---|---|---|
| A | 90–100 | Fort |
| B | 75–89 | Bon |
| C | 60–74 | Correct |
| D | 40–59 | Faible |
| F | 0–39 | Mauvais |

**Un contrôle critique en échec plafonne la note à C**, quel que soit le score.

## Historique et alertes

Tant que le score est activé, le serveur le recalcule toutes les 15 minutes et
le stocke sous la forme `dumbmonit_security_score{target="<id>"}` (et le nombre
de contrôles en échec sous `dumbmonit_security_checks_failed`). Deux règles
intégrées l'utilisent : **Security score dropped** (10 points ou plus sous la
meilleure valeur des dernières 24 heures) et **Security score low** (sous 40).
Voir [Règles d'alerte](../alerting/rules.md#security-score).

## Contrôles et comment les améliorer

### Hôtes Linux { #linux-hosts }

Machines avec l'agent. Les contrôles SSH et pare-feu demandent un agent de la
même version que le serveur ; avec un agent plus ancien, ils sont inconnus.

| Contrôle | Gravité | Donnée |
|---|---|---|
| Security updates installed | high | mises à jour de sécurité en attente (là où le gestionnaire de paquets les distingue) |
| No pending updates | low | mises à jour en attente |
| No reboot pending | medium | redémarrage requis |
| SSH root login with a password refused | high | `PermitRootLogin` de `sshd -T` (`prohibit-password` réussit) |
| SSH accepts keys only | medium | `PasswordAuthentication`, et `KbdInteractiveAuthentication` avec `UsePAM` |
| Host firewall filters inbound traffic | high | une chaîne input nftables ou iptables qui rejette ou abandonne |
| SELinux enforcing | medium | mode SELinux (inconnu là où SELinux est absent) |
| Container images up to date | medium | image plus récente dans le registre (uniquement avec des conteneurs) |
| No container image older than 180 days | low | âge de l'image (uniquement avec des conteneurs) |

Pour l'améliorer :

1. Installez les mises à jour de sécurité en attente, puis activez les mises à
   jour de sécurité automatiques : `unattended-upgrades` sur Debian et Ubuntu,
   `dnf-automatic` sur Fedora et RHEL.
2. Redémarrez après les mises à jour du noyau ou des bibliothèques.
3. Vérifiez que votre clé SSH fonctionne, puis définissez `PermitRootLogin no`,
   `PasswordAuthentication no` et `KbdInteractiveAuthentication no` dans
   `/etc/ssh/sshd_config` et rechargez sshd.
4. Activez un pare-feu d'hôte qui rejette le trafic entrant non sollicité :
   `ufw enable` sur Ubuntu, firewalld sur Fedora et RHEL, un jeu de règles
   nftables ailleurs.
5. Gardez SELinux en mode enforcing là où la distribution le fournit ; tirez
   régulièrement les images de conteneurs plus récentes.

Sources : [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Hôtes Windows { #windows-hosts }

Machines avec l'agent Windows, de la même version que le serveur (lus depuis le
registre ; un agent plus ancien ne remonte aucun de ces contrôles).

| Contrôle | Gravité | Donnée |
|---|---|---|
| Windows Firewall on for every profile | high | profils domaine, privé et public (stratégie de groupe d'abord) |
| No reboot pending | medium | Windows Update ou maintenance de composants en attente d'un redémarrage |
| Remote Desktop requires Network Level Authentication | high | réussit quand le Bureau à distance est désactivé |
| SMBv1 disabled | high | paramètre du serveur SMBv1 |

Pour l'améliorer :

1. Installez les mises à jour Windows (Paramètres → Windows Update) et
   redémarrez quand c'est demandé.
2. Gardez le Pare-feu Windows Defender activé pour les profils domaine, privé
   et public.
3. Si le Bureau à distance est activé, exigez l'authentification au niveau du
   réseau (NLA) et accédez-y par un VPN, jamais directement depuis Internet.
4. Supprimez SMBv1 : `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Sources : [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE { #proxmox-ve }

| Contrôle | Gravité | Donnée |
|---|---|---|
| Security updates installed | high | mises à jour de sécurité en attente par nœud (demande `Sys.Modify` sur le jeton, sinon inconnu) |
| Proxmox VE packages up to date | medium | paquets Proxmox pouvant être mis à niveau par nœud |
| Running the installed kernel | medium | noyau plus récent installé que celui qui tourne |
| No test repository in use | medium | dépôts standard activés |
| Package repositories correctly configured | medium | erreurs et avertissements que Proxmox VE remonte sur les dépôts |
| Enterprise repository subscription | low | état de l'abonnement par nœud |
| Web interface certificate valid for 14+ days | medium | expiration du certificat du nœud |
| Every guest is in a backup job | high | invités non sélectionnés par aucun job de sauvegarde |
| Backup jobs succeed | high | dernier résultat de chaque job de sauvegarde |

Pour l'améliorer :

1. Utilisez un dépôt auquel vous avez droit : enterprise avec un abonnement,
   sinon `pve-no-subscription` ; jamais `pvetest` sur un nœud qui compte.
2. Installez les mises à jour depuis le panneau **Updates** du nœud, puis
   redémarrez sur le nouveau noyau.
3. Mettez chaque VM et conteneur dans un job de sauvegarde, idéalement vers un
   Proxmox Backup Server, et corrigez tout job qui échoue.
4. Remplacez le certificat web auto-signé par un certificat ACME.

Sources : [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server { #proxmox-backup-server }

| Contrôle | Gravité | Donnée |
|---|---|---|
| System updates installed | high | mises à jour en attente (demande `Sys.Modify` sur `/system`) |
| Running the installed Backup Server version | medium | démons tournant encore sur la version précédant la mise à niveau |
| Verify job scheduled | high | nombre de jobs de vérification |
| Prune job scheduled | medium | nombre de jobs de prune |
| Backups copied elsewhere (sync or tape) | medium | jobs de synchronisation et de sauvegarde sur bande (règle 3-2-1) |
| Garbage collection clean, no bad chunks | high | dernier résultat du GC et chunks défectueux par datastore |
| Certificate valid for 14+ days | medium | expiration du certificat |

Pour l'améliorer :

1. Installez les mises à jour et redémarrez quand le noyau ou `proxmox-backup`
   a changé.
2. Planifiez un job de vérification et un job de prune sur chaque datastore.
3. Gardez une copie ailleurs : un job de synchronisation vers un second Backup
   Server, ou une bande.
4. Utilisez un certificat ACME pour l'interface web.

Sources : [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html).

### Synology { #synology }

| Contrôle | Gravité | Donnée |
|---|---|---|
| DSM 7 or later | high | version de DSM |
| DSM up to date | high | une mise à jour de DSM est disponible (compte administrateur uniquement) |
| Built-in admin account disabled | medium | état du compte `admin` (compte administrateur uniquement) |
| Auto Block enabled | medium | réglage Auto Block (compte administrateur uniquement) |
| DSM firewall enabled | medium | réglage du pare-feu (compte administrateur uniquement) |
| Hyper Backup task configured | medium | nombre de tâches Hyper Backup |
| Hyper Backup tasks succeed | high | dernier résultat de chaque tâche |

Les quatre réglages sont lus une fois par heure et seulement quand DumbMonit se
connecte avec un compte administrateur ; avec un compte ordinaire ils restent
inconnus et ne comptent pas. La validation en deux étapes et la redirection
HTTPS ne sont pas lues.

Pour l'améliorer :

1. Utilisez la dernière version de DSM 7 pour votre modèle et laissez DSM
   installer automatiquement les mises à jour importantes (Panneau de
   configuration → Mise à jour et restauration).
2. Désactivez le compte intégré `admin` ; connectez-vous avec un administrateur
   personnel protégé par la validation en deux étapes.
3. Activez Auto Block et le pare-feu (Panneau de configuration → Sécurité), et
   ne redirigez pas les ports de DSM vers Internet : utilisez un VPN.
4. Sauvegardez vers un autre appareil ou une destination cloud avec Hyper
   Backup, et corrigez toute tâche qui échoue : RAID et instantanés ne sont pas
   des sauvegardes.

Sources : [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense { #opnsense }

| Contrôle | Gravité | Donnée |
|---|---|---|
| Firmware updates installed | high | mises à jour de paquets en attente (inconnu tant qu'OPNsense n'a pas vérifié les mises à jour) |
| On the current major release series | medium | nouvelle série majeure disponible |
| No reboot pending after an update | medium | redémarrage requis |
| Packet filter enabled | **critical** | état de `pf` |

Pour l'améliorer :

1. Vérifiez les mises à jour dans System → Firmware → Status, installez-les,
   redémarrez quand c'est demandé.
2. Passez à la nouvelle version majeure dès qu'elle est sortie.
3. Gardez le filtre de paquets activé.
4. N'autorisez l'interface web et SSH que depuis le LAN ou un VPN, jamais
   depuis le WAN.

Sources : [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Sites web { #websites }

Pour les moniteurs **HTTP(S)** et **TLS** qui négocient TLS, et pour les
appareils **Website health & SEO**. Un moniteur HTTP en clair n'a pas de score.

| Contrôle | Gravité | Donnée |
|---|---|---|
| Certificate valid for 14+ days | high | jours avant expiration (moniteurs HTTP(S) et TLS) |
| Certificate chain trusted | high | vérification de la chaîne (moniteurs HTTP(S) et TLS) |
| TLS 1.3 negotiated | low | version du protocole négocié (moniteurs HTTP(S) et TLS) |
| Served over HTTPS | high | adresse finale après redirections (Website health) |
| Strict-Transport-Security header | medium | en-tête avec un `max-age` supérieur à 0 |
| X-Content-Type-Options: nosniff | low | valeur de l'en-tête |
| Protected against clickjacking | low | `X-Frame-Options` ou `frame-ancestors` de la CSP |
| Content-Security-Policy header | low | une politique appliquée (le mode report-only ne compte pas) |

Les en-têtes ne sont lus que sur les réponses HTTPS.

Pour l'améliorer :

1. Servez le site uniquement en HTTPS, et redirigez le HTTP en clair vers
   HTTPS.
2. Automatisez le renouvellement du certificat (ACME, p. ex. Let's Encrypt) et
   servez la chaîne complète.
3. Ajoutez des en-têtes de sécurité dans le serveur web ou le reverse proxy :
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff`, et une
   `Content-Security-Policy` avec `frame-ancestors`.
4. Recoupez avec [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory),
   qui teste bien plus que ce score.

Sources : [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory { #active-directory }

Noté à partir des constats de type PingCastle du collecteur Active Directory :
comptes Kerberoastables, comptes sans pré-authentification Kerberos,
délégation non contrainte, ancienneté du mot de passe `krbtgt`, taille des
groupes privilégiés, couverture LAPS, politique de mots de passe, comptes
obsolètes. Chaque constat est un contrôle qui réussit quand il concerne zéro
objet.

Pour l'améliorer :

1. Corrigez d'abord les constats critiques et élevés : ce sont les chemins
   habituels vers les Domain Admins.
2. Si le mot de passe `krbtgt` est ancien, réinitialisez-le deux fois, à
   quelques heures d'intervalle, et retirez la délégation non contrainte des
   ordinateurs qui ne sont pas des contrôleurs de domaine.
3. Faites passer les services qui tournent sous des comptes utilisateurs avec
   SPN vers des comptes de service gérés de groupe (gMSA).
4. Lancez [PingCastle](https://www.pingcastle.com/) pour une évaluation
   complète.

Sources : [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Limites

- Le score ne reflète que ce que DumbMonit peut voir ; de nombreuses bonnes
  pratiques ne sont pas contrôlées faute de collecteur qui lise la donnée :
  pare-feu et authentification à deux facteurs de Proxmox, interface web
  d'OPNsense exposée sur le WAN, conteneurs Docker tournant en privilégié ou
  avec le socket Docker monté, TLS 1.0/1.1 encore proposés (la sonde ne
  négocie que TLS 1.2 et 1.3), et bien d'autres.
- Un contrôle n'est aussi frais que sa donnée : certains collecteurs ne relisent
  les mises à jour que toutes les quelques heures.
- Les poids sont un choix de produit, identique pour toutes les installations.
- Seuls les types ci-dessus sont notés. Les autres (switchs, imprimantes,
  UniFi, pfSense…) n'affichent volontairement aucun badge : quelques contrôles
  donneraient une note qui ne veut rien dire.
