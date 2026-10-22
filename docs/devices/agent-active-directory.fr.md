# Déployer l'agent avec Active Directory

Installez l'agent sur tous les serveurs Windows d'un domaine avec une seule
stratégie de groupe (GPO), au lieu de lancer la commande d'installation serveur
par serveur.

Dans **Settings → Agents**, **Deploy with Active Directory** crée un jeton de
flotte limité à votre domaine (réutilisable, valable 30 jours par défaut, avec
un nombre maximal de serveurs facultatif) et télécharge `Deploy-DumbMonit.ps1`.
Vous lancez ce script une seule fois, en Domain Admin, sur un contrôleur de
domaine ou sur une machine dotée des modules PowerShell GroupPolicy et
ActiveDirectory (RSAT). Il crée ou met à jour la GPO **DumbMonit Agent** et la
lie à une OU. La GPO exécute une tâche planifiée immédiate, en SYSTEM, à chaque
rafraîchissement des stratégies. Chaque serveur installe l'agent et
s'inscrit sous son nom complet (`srv01.corp.example.com`). Il est rangé dans le
dossier **Active Directory &lt;domaine&gt;**, avec les tags `ad=<domaine>` et
`ad_ou=<OU>`.

## Étapes

1. Dans Settings → Agents, cliquez sur **Deploy with Active Directory**.
   Saisissez le domaine, ajustez les options si besoin, créez le jeton et
   téléchargez `Deploy-DumbMonit.ps1`. Le jeton n'est affiché qu'à ce moment.
2. Copiez le script sur un contrôleur de domaine et ouvrez PowerShell en
   administrateur du domaine. Faites d'abord un essai à blanc, qui ne change
   rien :

    ```powershell
    .\Deploy-DumbMonit.ps1 -WhatIf
    ```

3. Appliquez :

    ```powershell
    .\Deploy-DumbMonit.ps1 -TargetOU "OU=Servers,DC=corp,DC=example,DC=com"
    ```

4. Les serveurs installent l'agent à leur prochain rafraîchissement des
   stratégies, sous 90 minutes, ou tout de suite avec `gpupdate /force` sur un
   serveur. Ils apparaissent dans l'assistant et dans le dossier **Active
   Directory &lt;domaine&gt;** à mesure qu'ils s'inscrivent.
5. Le parc déployé, supprimez le script et cliquez sur **Stop new
   enrolments** (voir [Sécurité](#securite)).

Le script est idempotent : le relancer met la GPO à jour sur place.

## Paramètres du script

Les options de l'assistant sont inscrites comme valeurs par défaut ; chaque
paramètre peut être remplacé en ligne de commande.

| Paramètre | Effet |
|---|---|
| `-TargetOU` | Nom distinctif de l'OU à laquelle la GPO est liée ; une OU parente couvre ses enfants. Omis : si une seule OU (hors contrôleurs de domaine) contient des ordinateurs Windows Server, elle est utilisée ; s'il y en a plusieurs, le script les liste et s'arrête pour que vous en choisissiez une. |
| `-IncludeDomainControllers` | Lie aussi la GPO à l'OU Domain Controllers. |
| `-SecurityGroup` | Filtrage de sécurité : seuls les ordinateurs de ce groupe appliquent la GPO. Restreint aussi la lecture du kit SYSVOL. |
| `-UseSysvol` | Copie aussi le binaire de l'agent dans SYSVOL, pour les serveurs qui ne peuvent pas le télécharger depuis DumbMonit. Ils doivent tout de même joindre le serveur pour envoyer leurs mesures. |
| `-GpoName` | Nom de la GPO. `DumbMonit Agent` par défaut. |
| `-UninstallAgents` | Fait désinstaller l'agent des serveurs auxquels la GPO s'applique. |
| `-Remove` | Supprime la GPO, ses liens et le kit SYSVOL. Les agents installés continuent de tourner. |
| `-WhatIf` | Affiche ce qui serait fait, ne change rien. |

## Ce qui se passe sur chaque serveur

La tâche exécute `Install-DumbMonitAgent.ps1`, depuis le kit placé dans SYSVOL
(`\\<domaine>\SYSVOL\<domaine>\DumbMonit`).

- **Rien à faire si c'est à jour.** Si l'agent est installé et que le SHA-256
  de son binaire correspond à celui attendu, le script s'arrête. Un service
  arrêté est redémarré.
- **Le binaire est vérifié.** Téléchargé depuis le serveur DumbMonit
  (vérifié par l'installateur) ou copié depuis SYSVOL (copié en local, puis
  comparé au SHA-256 attendu avant d'être installé).
- **Une installation existante garde son nom**, pour qu'un serveur renommé
  n'apparaisse pas en double.
- **Les résultats vont dans le journal Application**, source `DumbMonit` :

| Event ID | Niveau | Signification |
|---|---|---|
| 1000 | Information | Agent déjà à jour ; son service était arrêté et a été relancé. |
| 1001 | Information | Agent installé ou mis à jour. |
| 1002 | Erreur | Échec (erreur de l'installateur, ou binaire SYSVOL au SHA-256 inattendu). |
| 1003 | Avertissement | L'ordinateur n'est pas joint à un domaine ; rien n'est installé. |
| 1004 | Information | Agent désinstallé. |

Le jeton d'inscription n'apparaît jamais dans ces journaux.

## Sécurité {#securite}

- Le jeton ne sert qu'à inscrire des machines **du domaine** : un nom d'hôte
  hors du domaine (`srv01`, `srv01.autre.example.com`) est refusé. Chaque
  serveur reçoit ensuite son propre secret de liaison, si bien qu'aucun ne
  peut se faire passer pour un autre.
- Le jeton est stocké haché et n'est jamais réaffiché. Il se trouve dans le
  script et dans le kit SYSVOL : **traitez le script comme un mot de passe et
  supprimez-le après usage.**
- Le dossier SYSVOL `\\<domaine>\SYSVOL\<domaine>\DumbMonit` n'est lisible que
  par Domain Computers (ou le `-SecurityGroup`), Domain Controllers et les
  administrateurs.
- **Stop new enrolments** (bouton de l'assistant et du jeton) clôt le jeton :
  les serveurs déjà inscrits continuent de remonter leurs mesures.
  **Revoke** (Settings → Agents) coupe tous les agents qui utilisent le jeton.
  Arrêtez les inscriptions une fois le parc déployé ; ne révoquez que pour
  couper des agents.
- Utilisez HTTPS pour l'URL de DumbMonit, puisque les agents y envoient leurs
  mesures.

### Scripts signés

Si votre stratégie d'exécution impose `AllSigned` (définie par GPO, elle prime
sur le `-ExecutionPolicy Bypass` que passe la tâche), signez
`Deploy-DumbMonit.ps1` avant de le lancer. Signez ensuite les scripts du kit
dans SYSVOL (`install.ps1` et `Install-DumbMonitAgent.ps1`) après chaque
exécution, car le script les réécrit.

```powershell
$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Select-Object -First 1
Set-AuthenticodeSignature .\Deploy-DumbMonit.ps1 $cert
Get-ChildItem "\\corp.example.com\SYSVOL\corp.example.com\DumbMonit\*.ps1" |
    ForEach-Object { Set-AuthenticodeSignature $_.FullName $cert }
```

Les machines doivent faire confiance à l'éditeur du certificat.

## Mettre à jour DumbMonit

Le kit embarque le SHA-256 du binaire de l'agent livré par votre serveur. Après
une mise à jour de DumbMonit, **téléchargez un nouveau kit et relancez le
script** : il met la GPO à jour sur place. Sinon les serveurs refusent le
nouveau binaire (SHA-256 différent, Event ID 1002).

## Retirer le déploiement

1. Lancez `.\Deploy-DumbMonit.ps1 -UninstallAgents`.
2. Une fois les stratégies rafraîchies sur les serveurs (Event ID 1004),
   lancez `.\Deploy-DumbMonit.ps1 -Remove`.

Sans la première étape, `-Remove` laisse les agents installés en marche.
Arrêtez aussi les inscriptions du jeton dans DumbMonit.

## Dépannage

- **Un module est introuvable.** Sur un serveur :
  `Install-WindowsFeature GPMC, RSAT-AD-PowerShell`. Sous Windows 10/11,
  ajoutez les capacités RSAT `Rsat.GroupPolicy.Management.Tools` et
  `Rsat.ActiveDirectory.DS-LDS.Tools`.
- **« This enrollment token is limited to an Active Directory domain ».** Le serveur s'inscrit sous un nom qui
  ne se termine pas par le domaine du jeton : vérifiez que la machine est
  jointe au domaine et que son suffixe DNS principal correspond. Le kit est
  aussi lié à un seul domaine : lancez le script dans le domaine pour lequel il
  a été généré.
- **Plusieurs OU contiennent des serveurs.** Passez `-TargetOU` avec une OU
  parente commune, ou lancez le script une fois par OU.
- **Rien ne se passe sur un serveur.** Lancez `gpresult /r` pour vérifier que
  la GPO s'applique, puis ouvrez l'Observateur d'événements → Journaux
  Windows → Application et filtrez sur la source `DumbMonit`.

## Pas de MSI

DumbMonit ne fournit pas de MSI : l'agent Windows est compilé en croisé sous
Linux. La GPO à tâche planifiée remplace le déploiement de logiciel par MSI,
et ajoute la mise à jour de l'agent et la vérification de son SHA-256.

## Linux : Ansible ou SSH

Il n'y a pas d'équivalent de la GPO sous Linux, mais un jeton **réutilisable**
(cochez *Reusable for a fleet* dans Settings → Agents) permet d'exécuter telle
quelle, sur autant de machines que voulu, la commande d'installation affichée
par le serveur.

Rangez la commande complète dans une variable de vault
`dumbmonit_install_command`
(`curl -sSL https://dumbmonit.example.com/install.sh | sh -s -- --token=dmon_xxx --url=https://dumbmonit.example.com`) :

```yaml
- name: Install the DumbMonit agent
  ansible.builtin.shell: "{{ dumbmonit_install_command }}"
  args:
    creates: /etc/dumbmonit/agent.yaml
  become: true
  no_log: true
```

`creates` ignore les machines qui ont déjà une configuration d'agent, et
`no_log` garde le jeton hors de la sortie d'Ansible. Même idée en SSH simple :

```sh
for h in srv01 srv02 srv03; do
  ssh "$h" "sudo sh -c 'test -f /etc/dumbmonit/agent.yaml || { $INSTALL_COMMAND; }'"
done
```

Arrêtez les inscriptions du jeton une fois les machines entrées, comme pour
Active Directory.
