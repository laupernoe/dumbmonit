# Windows sans agent (WinRM)

Un serveur ou un poste Windows surveillé sans rien installer, par WinRM
(WS-Management) et des requêtes WMI en lecture seule : CPU, mémoire, disques,
services (ceux que vous choisissez et tous les services automatiques arrêtés),
durée de fonctionnement et erreurs récentes du journal d'événements.

DumbMonit n'envoie que des requêtes `SELECT` fixes (WQL `Enumerate`). Il
n'exécute ni commande ni script sur le serveur, et peut passer par un
[agent relais](agent.md) pour atteindre un réseau que le serveur ne voit pas.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 5986 | TCP | WinRM en HTTPS (par défaut) |
| 5985 | TCP | Seulement avec l'option Autoriser le trafic non chiffré |

## Préparer le serveur Windows

1. Sur le serveur, dans un PowerShell élevé, activez WinRM.

    ```powershell
    Enable-PSRemoting -SkipNetworkProfileCheck -Force
    ```

2. Créez l'écouteur HTTPS avec un certificat au nom du serveur (utilisez le
   vôtre plutôt que ce certificat auto-signé si vous en avez un), puis ouvrez
   son port.

    ```powershell
    $cert = New-SelfSignedCertificate -DnsName "srv01.corp.example.com" -CertStoreLocation Cert:\LocalMachine\My
    New-Item -Path WSMan:\LocalHost\Listener -Transport HTTPS -Address * -CertificateThumbPrint $cert.Thumbprint -Force
    New-NetFirewallRule -DisplayName "WinRM HTTPS" -Direction Inbound -Protocol TCP -LocalPort 5986 -Action Allow
    ```

3. Créez le compte avec un long mot de passe aléatoire, puis ajoutez-le aux
   groupes ci-dessous.

    ```powershell
    net user dumbmonit * /add
    net localgroup "Remote Management Users" dumbmonit /add
    net localgroup "Performance Monitor Users" dumbmonit /add
    net localgroup "Event Log Readers" dumbmonit /add
    ```

4. Donnez au compte l'accès à l'espace de noms WMI : wmimgmt.msc → clic droit
   sur Contrôle WMI → Propriétés → Sécurité → Root → CIMV2 → Sécurité,
   ajoutez-le et cochez Activer le compte et Activation à distance.

5. Dans DumbMonit, saisissez le nom du serveur tel que son certificat le porte
   (srv01.corp.example.com) et connectez-vous en dumbmonit, ou DOMAINE\dumbmonit
   pour un compte de domaine.

### Droits

| Droit | Usage |
|---|---|
| Remote Management Users | Autorise la connexion WinRM |
| Performance Monitor Users | Lit la charge CPU par cœur |
| Event Log Readers | Seulement pour les comptes du journal d'événements |
| WMI root/cimv2 : Activer le compte, Activation à distance | Lit les classes WMI |

Le compte n'est pas administrateur et n'exécute rien sur le serveur.

!!! tip "Stratégie de groupe (GPO)"

    Pour déployer sur de nombreux serveurs : *Configuration ordinateur →
    Stratégies → Modèles d'administration → Composants Windows → Gestion à
    distance de Windows (WinRM) → Service WinRM* (autoriser la gestion à
    distance du serveur, avec les filtres IP de votre réseau d'administration,
    et l'authentification de base laissée désactivée), la règle entrante du
    *Pare-feu Windows Defender* pour le TCP 5986, et le certificat par
    inscription automatique. Quand l'écouteur HTTPS fonctionne, supprimez
    l'écouteur HTTP : `winrm delete winrm/config/Listener?Address=*+Transport=HTTP`.

## Authentification et chiffrement

- **NTLM** (par défaut) : comptes locaux et de domaine. L'en-tête Negotiate
  porte des jetons NTLMv2. En HTTPS, le canal est chiffré par TLS.
- **Basic** : en HTTPS seulement, et si le serveur l'autorise
  (`winrm set winrm/config/service/auth @{Basic="true"}`).
- **Kerberos** n'est pas pris en charge : il n'existe pas de client libre en
  Rust pur qui tienne dans le binaire statique. Utilisez NTLM.
- **HTTP non chiffré (5985)** est refusé sauf si vous cochez *Autoriser le
  trafic non chiffré*. Le serveur doit alors l'accepter
  (`winrm set winrm/config/service @{AllowUnencrypted="true"}`), ce qui expose
  données et identifiants à quiconque est sur le chemin réseau. Préférez HTTPS.
- Un certificat d'écouteur auto-signé demande *Accepter un certificat non
  vérifiable* ; sinon il doit être approuvé par l'hôte DumbMonit et porter le
  nom saisi pour le serveur.

Un serveur injoignable (connexion refusée, délai) est un équipement *en panne* ;
un 401 ou un *accès refusé* est une erreur de configuration affichée sur
l'équipement, pas une panne.

## Ce qui est surveillé

Ces métriques portent les noms et étiquettes de l'agent Windows : tableaux de
bord et règles fonctionnent de la même façon.

| Métrique | Source |
|---|---|
| `cpu_usage_percent`, `cpu_core_usage_percent`, `cpu_count` | `Win32_PerfFormattedData_PerfOS_Processor` |
| `memory_total_bytes`, `memory_used_bytes`, `memory_available_bytes`, `memory_used_percent` | `Win32_OperatingSystem` |
| `swap_total_bytes`, `swap_used_bytes`, `swap_used_percent` | `Win32_PageFileUsage` |
| `filesystem_*{mountpoint,device,fstype}` | `Win32_LogicalDisk` (disques fixes) |
| `uptime_seconds`, `boot_time_seconds`, `process_count`, `windows_version_info` | `Win32_OperatingSystem` |
| `service_up{service}` | `Win32_Service` |
| `windows_services_auto_stopped`, `windows_service_auto_stopped{service}` | services automatiques arrêtés |
| `windows_eventlog_errors{log}`, `windows_eventlog_events{log,event_id,source}` | `Win32_NTLogEvent` |
| `winrm_section_readable{section}` | 0 quand une lecture est refusée |

- **Services** : listez des noms dans *Services à suivre* pour obtenir une série
  up/down chacun (un nom absent du serveur compte comme arrêté). Tout service en
  démarrage automatique et arrêté est aussi signalé, sauf les programmes de mise
  à jour et services à démarrage différé (`gupdate`, `edgeupdate`, `sppsvc`…)
  et les noms de *Services arrêtés à ignorer* (`*` en début ou fin = joker). La
  règle intégrée *Services Windows automatiques arrêtés* alerte dessus.
- **Journal d'événements** : les erreurs des N dernières minutes (60 par défaut)
  des journaux choisis, en comptes par journal et les cinq Event ID les plus
  fréquents. Le journal Security compte les échecs d'audit. La lecture passe par
  `Win32_NTLogEvent` (journaux classiques) ; pour des règles par événement
  (journal, source, Event ID, texte), utilisez les
  [alertes du journal d'événements](agent-windows-events.md) de l'agent Windows.
- Les **mises à jour en attente** ne se lisent pas en WQL : elles ne sont pas
  collectées.

## Options

| Option | Défaut | Rôle |
|---|---|---|
| Autoriser le trafic non chiffré | non | HTTP sur 5985 au lieu de HTTPS sur 5986 |
| Port | 5986 | Utilisé si l'adresse n'en donne pas |
| Authentification | `ntlm` | `ntlm` ou `basic` (HTTPS seulement) |
| Accepter un certificat non vérifiable | non | Pour un écouteur auto-signé |
| Délai par requête | 15 s | 1 à 120 |
| Services à suivre | aucun | Noms séparés par des virgules |
| Services arrêtés à ignorer | aucun | Noms, joker `*` possible |
| Journaux d'événements à compter | `System,Application` | `none` désactive |
| Fenêtre des événements | 60 min | 1 à 1440 |
