<#
.SYNOPSIS
    Installs the DumbMonit system agent and its Windows service.

.DESCRIPTION
    Downloads the binary, writes the configuration with the token, registers the
    service and starts it. Running the script again updates the agent: it is also
    the upgrade procedure. A machine still running the agent under its former
    name (EzyMonitAgent) is migrated in place by the same command.

.EXAMPLE
    & ([scriptblock]::Create((irm http://serveur:8080/install.ps1))) -Token dmon_xxx -Url http://serveur:8080

.EXAMPLE
    .\install.ps1 -Token dmon_xxx -Url http://serveur:8080 -Services @('Spooler','MSSQLSERVER')
#>
[CmdletBinding()]
param(
    [string]$Token,
    [string]$Url,
    [int]$Interval = 0,
    [string[]]$Services = @(),
    [hashtable]$Tags = @{},
    [string]$HostName,
    # Local binary to install instead of downloading it.
    [string]$BinPath,
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

$ServiceName = 'DumbMonitAgent'
$InstallDir  = Join-Path $env:ProgramFiles 'DumbMonit'
$ConfigDir   = Join-Path $env:ProgramData 'DumbMonit'
$ExePath     = Join-Path $InstallDir 'dumbmonit-agent.exe'
$ConfigPath  = Join-Path $ConfigDir 'agent.yaml'

# Noms d'avant le renommage EzyMonit → DumbMonit. Une installation qui les porte
# encore est migrée sur place à l'installation, et -Uninstall en fait aussi le
# ménage.
$LegacyServiceName = 'EzyMonitAgent'
$LegacyInstallDir  = Join-Path $env:ProgramFiles 'EzyMonit'
$LegacyConfigDir   = Join-Path $env:ProgramData 'EzyMonit'

function Write-Etape($message) { Write-Host "==> $message" }

function Stop-Sur($message) {
    Write-Error $message
    exit 1
}

# Arrête et retire le service et le dossier d'installation de l'ancien agent.
# Ne touche pas à sa configuration. Silencieux quand rien d'ancien n'est
# présent ; renvoie $true si quelque chose a été retiré.
function Remove-AncienAgent {
    $retire = $false
    if (Get-Service -Name $LegacyServiceName -ErrorAction SilentlyContinue) {
        Stop-Service -Name $LegacyServiceName -Force -ErrorAction SilentlyContinue
        & sc.exe delete $LegacyServiceName | Out-Null
        $retire = $true
    }
    if (Test-Path $LegacyInstallDir) {
        Remove-Item -Path $LegacyInstallDir -Recurse -Force -ErrorAction SilentlyContinue
        $retire = $true
    }
    return $retire
}

# Propriétaires admis pour le dossier de configuration et ce qu'il contient :
# SYSTEM, les administrateurs, TrustedInstaller, et le compte qui exécute
# l'installateur (déjà administrateur, contrôlé plus bas).
$SidSysteme = New-Object Security.Principal.SecurityIdentifier('S-1-5-18')
$SidAdmins  = New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')

function Test-ProprietaireSur($chemin) {
    $proprietaire = (Get-Acl -LiteralPath $chemin).GetOwner([Security.Principal.SecurityIdentifier])
    $admis = @(
        'S-1-5-18',
        'S-1-5-32-544',
        'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464',
        [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    )
    return $admis -contains $proprietaire.Value
}

# Le dossier de configuration porte le jeton, le secret de liaison, et une
# configuration que le service SYSTEM exécute (password_command des dépôts
# restic). Sous ProgramData, il hériterait de droits qui laissent tout
# utilisateur y créer des fichiers : un compte sans privilège qui l'aurait créé
# avant nous en resterait propriétaire, et pourrait réécrire la configuration.
# Un dossier ou un fichier appartenant à un autre compte arrête donc
# l'installation ; sinon, le dossier est réservé à SYSTEM et aux
# administrateurs (héritage coupé, propriétaire Administrators), puis tout son
# contenu est réaligné sur ces seuls droits.
function Protect-ConfigDir {
    if (Test-Path -LiteralPath $ConfigDir) {
        $elements = @(Get-Item -LiteralPath $ConfigDir -Force) + @(Get-ChildItem -LiteralPath $ConfigDir -Recurse -Force)
        foreach ($element in $elements) {
            if (-not (Test-ProprietaireSur $element.FullName)) {
                Stop-Sur "$($element.FullName) belongs to another account: it may have been planted to take over the agent. Check it, remove $ConfigDir, then run the installer again."
            }
        }
    } else {
        New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    }
    & icacls.exe $ConfigDir /setowner '*S-1-5-32-544' /T /C /Q | Out-Null
    $acl = New-Object System.Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($sid in @($SidSysteme, $SidAdmins)) {
        $regle = New-Object System.Security.AccessControl.FileSystemAccessRule(
            $sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
        $acl.AddAccessRule($regle)
    }
    Set-Acl -LiteralPath $ConfigDir -AclObject $acl
    # Le contenu ne garde que ce qu'il hérite du dossier.
    Get-ChildItem -LiteralPath $ConfigDir -Recurse -Force | ForEach-Object {
        & icacls.exe $_.FullName /reset /C /Q | Out-Null
    }
}

# Migration sur place d'une installation d'avant le renommage : l'ancien service
# est arrêté et retiré, et sa configuration reprend sa place sous le nouveau nom.
# Elle est réécrite juste après depuis -Token/-Url, mais les clés qu'un
# utilisateur y aurait ajoutées restent à portée de main. Sans rien d'ancien,
# ne fait rien et ne dit rien.
function Migrate-Legacy {
    $migre = Remove-AncienAgent
    if ((Test-Path $LegacyConfigDir) -and -not (Test-Path $ConfigDir)) {
        # Sur un même volume, Move-Item est un renommage : les droits posés par
        # icacls sur l'ancien fichier de configuration sont conservés.
        Move-Item -Path $LegacyConfigDir -Destination $ConfigDir
        $migre = $true
    }
    if ($migre) { Write-Etape "Migrating the $LegacyServiceName install to $ServiceName" }
}

# --------------------------------------------------------------- privilèges

$identite = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identite)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Stop-Sur "This installer needs a PowerShell console running as administrator."
}

# ------------------------------------------------------------ désinstallation

if ($Uninstall) {
    Write-Etape "Stopping the service"
    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        # `sc.exe delete` plutôt que `Remove-Service` : ce dernier n'existe qu'à
        # partir de PowerShell 6, absent de bien des Windows Server encore en place.
        & sc.exe delete $ServiceName | Out-Null
    }
    Remove-Item -Path $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path $ConfigDir -Recurse -Force -ErrorAction SilentlyContinue
    # Les restes d'une installation d'avant le renommage partent aussi.
    Remove-AncienAgent | Out-Null
    Remove-Item -Path $LegacyConfigDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Etape "Agent uninstalled"
    exit 0
}

if ([string]::IsNullOrWhiteSpace($Token)) { Stop-Sur "The enrollment token is required (-Token)." }
if ([string]::IsNullOrWhiteSpace($Url))   { Stop-Sur "The server URL is required (-Url http://server:8080)." }
$Url = $Url.TrimEnd('/')

# ------------------------------------------------------------------ binaire

# Un seul binaire Windows est construit, pour x86_64. Windows 11 sur ARM64 le fait
# tourner en émulation x64 ; Windows 10 sur ARM64 ne sait émuler que du 32 bits.
$architecture = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x86_64' }
    'ARM64' {
        Write-Warning "No native ARM64 build of the agent: installing the x86_64 one, which Windows 11 runs under x64 emulation."
        'x86_64'
    }
    default { Stop-Sur "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
# Le dossier de configuration n'est créé qu'une fois protégé (Protect-ConfigDir),
# juste avant d'y écrire : pas de fenêtre où il hériterait des droits de
# ProgramData.

# Écriture à côté puis renommage : Windows verrouille le fichier d'un exécutable
# en cours, et un service déjà installé tiendrait le sien.
$exeTemporaire = "$ExePath.nouveau"

if ($BinPath) {
    if (-not (Test-Path $BinPath)) { Stop-Sur "Binary not found: $BinPath" }
    Write-Etape "Installing from $BinPath"
    Copy-Item -Path $BinPath -Destination $exeTemporaire -Force
} else {
    $source = "$Url/download/dumbmonit-agent-windows-$architecture.exe"
    Write-Etape "Downloading $source"
    try {
        # Sans la barre de progression, `Invoke-WebRequest` est plusieurs fois plus
        # rapide sur Windows PowerShell 5 : elle repeint la console à chaque bloc.
        $progression = $ProgressPreference
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $source -OutFile $exeTemporaire -UseBasicParsing
    } catch {
        Stop-Sur "Download failed from ${source}: $_"
    } finally {
        $ProgressPreference = $progression
    }

    # Le serveur publie l'empreinte SHA-256 du binaire à côté (`<url>.sha256`) :
    # un téléchargement tronqué ou remplacé en chemin n'est pas installé.
    $sourceEmpreinte = "$source.sha256"
    $attendu = $null
    try {
        $reponse = Invoke-WebRequest -Uri $sourceEmpreinte -UseBasicParsing
        $attendu = ([string]$reponse.Content).Trim().Split(' ')[0].ToLowerInvariant()
    } catch {
        Write-Warning "No checksum published at ${sourceEmpreinte}: binary not verified"
    }
    if ($attendu) {
        $obtenu = (Get-FileHash -Path $exeTemporaire -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($obtenu -ne $attendu) {
            Remove-Item -Path $exeTemporaire -Force -ErrorAction SilentlyContinue
            Stop-Sur "Checksum mismatch for ${source} (expected $attendu, got $obtenu): the download is corrupt or has been tampered with. Nothing was installed."
        }
        Write-Etape "Checksum verified ($obtenu)"
    }
}

if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
    Write-Etape "Stopping the existing service before replacing it"
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
}
Move-Item -Path $exeTemporaire -Destination $ExePath -Force

# Le nouveau binaire est en place : l'ancien agent, s'il est là, peut être
# arrêté et sa configuration déplacée avant que la nouvelle ne soit écrite. Pas
# avant — un téléchargement raté ne doit pas laisser la machine sans agent.
Migrate-Legacy
Protect-ConfigDir

# ------------------------------------------------------------ configuration

$lignes = New-Object System.Collections.Generic.List[string]
$lignes.Add("# DumbMonit system agent configuration.")
$lignes.Add("# Written by install.ps1 — environment variables override it.")
$lignes.Add("server_url: $Url")
$lignes.Add("token: $Token")
if ($Interval -gt 0)  { $lignes.Add("interval_secs: $Interval") }
if ($HostName)        { $lignes.Add("hostname: $HostName") }
if ($Services.Count -gt 0) {
    $lignes.Add("services:")
    foreach ($service in $Services) { $lignes.Add("  - $service") }
}
if ($Tags.Count -gt 0) {
    $lignes.Add("tags:")
    foreach ($cle in $Tags.Keys) { $lignes.Add("  ${cle}: $($Tags[$cle])") }
}

# UTF-8 sans marque d'ordre : l'analyseur YAML de l'agent lit de l'UTF-8 brut, et
# une marque d'ordre en tête ferait échouer la première clé.
$encodage = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllLines($ConfigPath, $lignes, $encodage)

# Le fichier porte le jeton : seuls le système et les administrateurs doivent le
# lire. L'héritage est coupé, sinon les droits du dossier parent le rouvriraient.
& icacls.exe $ConfigPath /inheritance:r /grant:r 'SYSTEM:(R)' 'Administrators:(F)' | Out-Null
Write-Etape "Configuration written to $ConfigPath"

# --------------------------------------------------------------- vérification

# Un envoi de test avant de démarrer le service : un jeton erroné ou un serveur
# injoignable doit se voir maintenant, pas dans le journal des événements.
Write-Etape "Checking the connection to $Url"
& $ExePath --config="$ConfigPath" --once
if ($LASTEXITCODE -ne 0) {
    Stop-Sur @"
The agent could not reach the server — check the URL and the token.
The configuration is in place: fix $ConfigPath, then run
'Start-Service $ServiceName'.
"@
}

# ------------------------------------------------------------------- service

$commande = "`"$ExePath`" --service --config=`"$ConfigPath`""

if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
    Write-Etape "Updating the existing service"
    # La ligne de commande est écrite directement dans le registre plutôt que
    # passée à `sc.exe config` : Windows PowerShell 5 transmet mal à un programme
    # externe un argument qui contient à la fois des espaces et des guillemets,
    # et `sc.exe` recevrait un chemin coupé à « C:\Program ».
    Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\$ServiceName" `
        -Name ImagePath -Value $commande
    Set-Service -Name $ServiceName -StartupType Automatic
} else {
    Write-Etape "Registering the service"
    New-Service -Name $ServiceName `
        -BinaryPathName $commande `
        -DisplayName "DumbMonit system agent" `
        -Description "Collects this machine's metrics and pushes them to the DumbMonit server." `
        -StartupType Automatic | Out-Null
}

# Redémarrage automatique après une panne : sans cela, un agent tombé une fois ne
# revient qu'au prochain redémarrage de la machine — soit jamais, sur un serveur.
& sc.exe failure $ServiceName reset= 86400 actions= restart/10000/restart/30000/restart/60000 | Out-Null

Start-Service -Name $ServiceName

Write-Etape "Agent installed and started"
Write-Etape "Status: Get-Service $ServiceName"
Write-Etape "Logs:   Get-Content '$(Join-Path $ConfigDir 'agent.log')' -Tail 20 -Wait"
