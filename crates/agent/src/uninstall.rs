//! Désinstallation de l'agent à la demande du serveur (`agent.uninstall`).
//!
//! Un programme ne s'efface pas proprement lui-même pendant qu'il tourne :
//! Windows verrouille l'exécutable, et le gestionnaire de services arrêterait
//! l'agent au milieu du ménage. L'agent confie donc le travail à un assistant
//! détaché — un script fixe, sans aucun paramètre venu du serveur et sans rien
//! télécharger — qui attend quelques secondes, le temps que l'agent rende compte
//! et s'arrête, puis fait ce que font `install.sh --uninstall` et
//! `install.ps1 -Uninstall` : arrêter et retirer le service, puis effacer le
//! binaire et la configuration, aux mêmes chemins.
//!
//! Refusé, avec un message que l'interface affiche tel quel :
//!
//! - dans un conteneur, où il n'y a ni service ni fichiers à retirer — c'est le
//!   conteneur qu'il faut supprimer (`docker rm -f`) ;
//! - pour un binaire qui n'est pas à l'endroit où les scripts d'installation le
//!   posent : lancé à la main ou empaqueté autrement, l'agent ne sait pas ce qui
//!   l'a installé, et effacer des chemins au hasard serait pire que ne rien faire.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Délai laissé à l'agent pour rendre compte et s'arrêter avant le ménage. Le
/// même nombre est écrit en toutes lettres dans les deux scripts.
pub const HELPER_DELAY_SECS: u64 = 3;

/// Dossier du binaire posé par `install.sh`, et le lien qui y mène — ou le
/// binaire lui-même, pour une installation antérieure à ce dossier.
const UNIX_BIN_DIR: &str = "/usr/local/lib/dumbmonit-agent";
const UNIX_LINK: &str = "/usr/local/bin/dumbmonit-agent";

/// Nom de l'unité transitoire qui porte l'assistant sous systemd.
#[cfg(target_os = "linux")]
const SYSTEMD_UNIT: &str = "dumbmonit-agent-uninstall";

/// Ce que fait `install.sh --uninstall`, chemins compris, en POSIX pur.
///
/// Texte fixe : aucune donnée du serveur n'y entre. Sans un seul `$` ni `%` :
/// sous systemd, la ligne de commande passe par le gestionnaire de services,
/// qui développe les variables de certaines versions à l'autre — le script
/// doit lire la même chose dans tous les cas.
#[cfg(any(unix, test))]
pub const UNIX_SCRIPT: &str = r#"trap '' HUP INT TERM
sleep 3
case `uname -s` in
    Linux)
        if command -v systemctl >/dev/null 2>&1 \
           && { ! command -v rc-update >/dev/null 2>&1 || [ -d /run/systemd/system ]; }; then
            systemctl disable --now dumbmonit-agent 2>/dev/null || true
            systemctl disable --now ezymonit-agent 2>/dev/null || true
            rm -f /etc/systemd/system/dumbmonit-agent.service
            rm -f /etc/systemd/system/ezymonit-agent.service
            systemctl daemon-reload 2>/dev/null || true
        elif command -v rc-update >/dev/null 2>&1; then
            rc-service dumbmonit-agent stop 2>/dev/null || true
            rc-update del dumbmonit-agent default 2>/dev/null || true
            rc-service ezymonit-agent stop 2>/dev/null || true
            rc-update del ezymonit-agent default 2>/dev/null || true
            rm -f /etc/init.d/dumbmonit-agent /etc/init.d/ezymonit-agent
        fi
        rm -rf /etc/dumbmonit /etc/ezymonit
        ;;
    Darwin)
        launchctl bootout system/com.dumbmonit.agent 2>/dev/null || true
        rm -f /Library/LaunchDaemons/com.dumbmonit.agent.plist
        rm -rf /usr/local/etc/dumbmonit
        ;;
    FreeBSD)
        service dumbmonit_agent onestop 2>/dev/null || true
        sysrc -x dumbmonit_agent_enable 2>/dev/null || true
        rm -f /usr/local/etc/rc.d/dumbmonit_agent
        rm -rf /usr/local/etc/dumbmonit
        ;;
esac
rm -f /usr/local/bin/dumbmonit-agent /usr/local/bin/ezymonit-agent
rm -rf /usr/local/lib/dumbmonit-agent
logger -t dumbmonit-agent "agent uninstalled" 2>/dev/null || true
exit 0
"#;

/// Ce que fait `install.ps1 -Uninstall`, en PowerShell. Texte fixe lui aussi.
///
/// Le dossier du binaire est retenté quelques secondes : l'exécutable reste
/// verrouillé tant que le processus de l'agent n'a pas tout à fait disparu.
#[cfg(any(windows, test))]
pub const WINDOWS_SCRIPT: &str = r#"Start-Sleep -Seconds 3
$ServiceName = 'DumbMonitAgent'
$InstallDir  = Join-Path $env:ProgramFiles 'DumbMonit'
$ConfigDir   = Join-Path $env:ProgramData 'DumbMonit'
if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    & sc.exe delete $ServiceName | Out-Null
}
for ($i = 0; ($i -lt 15) -and (Test-Path $InstallDir); $i++) {
    Remove-Item -Path $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    if (Test-Path $InstallDir) { Start-Sleep -Seconds 1 }
}
Remove-Item -Path $ConfigDir -Recurse -Force -ErrorAction SilentlyContinue
if (Get-Service -Name 'EzyMonitAgent' -ErrorAction SilentlyContinue) {
    Stop-Service -Name 'EzyMonitAgent' -Force -ErrorAction SilentlyContinue
    & sc.exe delete 'EzyMonitAgent' | Out-Null
}
Remove-Item -Path (Join-Path $env:ProgramFiles 'EzyMonit') -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path (Join-Path $env:ProgramData 'EzyMonit') -Recurse -Force -ErrorAction SilentlyContinue
"#;

/// Message de refus d'un agent en conteneur.
pub const IN_CONTAINER: &str = "refused: this agent runs in a container; remove the container \
                                instead (docker rm -f dumbmonit-agent)";

/// Vérifie que la désinstallation a un sens ici, avant de rien lancer.
///
/// `exe` est le binaire en cours d'exécution ; `program_files`, sous Windows,
/// le dossier `%ProgramFiles%`. Séparé du lancement pour être testé partout.
pub fn check(in_container: bool, exe: Option<&Path>, program_files: Option<&Path>) -> Result<()> {
    if in_container {
        bail!(IN_CONTAINER);
    }
    let exe = exe.context("refused: the agent cannot tell where its own binary is")?;
    let installed = if cfg!(windows) {
        let expected = program_files
            .map(|dir| dir.join("DumbMonit"))
            .context("refused: %ProgramFiles% is not set")?;
        same_dir_ignoring_case(exe.parent(), &expected)
    } else {
        installed_by_script_unix(exe)
    };
    if !installed {
        bail!(
            "refused: {} was not installed by the DumbMonit install script; remove it by hand",
            exe.display()
        );
    }
    Ok(())
}

/// Vrai pour un binaire à l'endroit où `install.sh` le pose. Un binaire
/// remplacé sur place (mise à jour) peut se lire « … (deleted) » sous Linux.
fn installed_by_script_unix(exe: &Path) -> bool {
    let text = exe.to_string_lossy();
    let path = Path::new(text.strip_suffix(" (deleted)").unwrap_or(&text));
    path.parent() == Some(Path::new(UNIX_BIN_DIR)) || path == Path::new(UNIX_LINK)
}

fn same_dir_ignoring_case(dir: Option<&Path>, expected: &Path) -> bool {
    let normalise = |path: &Path| {
        path.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
    };
    dir.is_some_and(|dir| normalise(dir) == normalise(expected))
}

/// Le binaire en cours d'exécution, tel que le système le désigne.
pub fn current_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if cfg!(windows) {
        // `canonicalize` rendrait un chemin `\\?\C:\…`, qui ne se compare plus
        // à `%ProgramFiles%`.
        return Some(exe);
    }
    Some(exe.canonicalize().unwrap_or(exe))
}

/// `%ProgramFiles%`, sous Windows seulement.
pub fn program_files() -> Option<PathBuf> {
    if cfg!(windows) { std::env::var_os("ProgramFiles").map(PathBuf::from) } else { None }
}

/// Lance l'assistant. Rend une ligne pour le compte rendu.
///
/// Ne rend la main qu'une fois l'assistant effectivement parti : c'est la
/// condition pour annoncer au serveur que la désinstallation est engagée.
#[cfg(target_os = "linux")]
pub fn spawn() -> Result<String> {
    // Sous systemd, l'agent tourne dans une unité durcie (système de fichiers
    // en lecture seule hors de ses dossiers) dont l'arrêt emporte tous ses
    // processus : l'assistant doit vivre dans une unité à lui, que seul
    // systemd peut créer.
    if !Path::new("/run/systemd/system").is_dir() {
        return spawn_detached_sh();
    }
    // `--collect` (systemd 236) fait oublier l'unité une fois finie ; un
    // systemd plus ancien ne le connaît pas, et s'en passe.
    let mut last_error = String::new();
    for collect in [true, false] {
        let mut command = std::process::Command::new("systemd-run");
        command.arg("--quiet");
        if collect {
            command.arg("--collect");
        }
        let output = command
            .args(["--unit", SYSTEMD_UNIT, "--description", "Remove the DumbMonit agent"])
            .args(["/bin/sh", "-c", UNIX_SCRIPT])
            .current_dir("/")
            .stdin(std::process::Stdio::null())
            .output()
            .context("starting systemd-run")?;
        if output.status.success() {
            return Ok(format!("clean-up queued in the transient unit {SYSTEMD_UNIT}"));
        }
        last_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    bail!("systemd-run refused the clean-up unit: {last_error}")
}

#[cfg(all(unix, not(target_os = "linux")))]
pub fn spawn() -> Result<String> {
    spawn_detached_sh()
}

/// Un `sh` dans son propre groupe de processus, hors de celui du service :
/// l'arrêt du service par launchd, rc.d ou OpenRC ne l'emporte pas.
#[cfg(unix)]
fn spawn_detached_sh() -> Result<String> {
    use std::os::unix::process::CommandExt as _;
    std::process::Command::new("/bin/sh")
        .args(["-c", UNIX_SCRIPT])
        .current_dir("/")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .context("starting the clean-up helper")?;
    Ok("clean-up helper started".to_string())
}

#[cfg(windows)]
pub fn spawn() -> Result<String> {
    use std::os::windows::process::CommandExt as _;
    // Processus détaché de la console et du groupe du service, comme
    // l'assistant de mise à jour.
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let root = PathBuf::from(root);
    // Chemin complet : pas de `powershell.exe` trouvé au hasard du PATH.
    let powershell = root.join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    std::process::Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass"])
        .arg("-EncodedCommand")
        .arg(encoded_powershell(WINDOWS_SCRIPT))
        .current_dir(&root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .context("starting the clean-up helper")?;
    Ok("clean-up helper started".to_string())
}

#[cfg(not(any(unix, windows)))]
pub fn spawn() -> Result<String> {
    bail!("refused: removing the agent remotely is not supported on this system")
}

/// `-EncodedCommand` : le script en UTF-16LE, en base64. Rien à échapper, et
/// rien que la ligne de commande de Windows puisse réinterpréter.
#[cfg(any(windows, test))]
fn encoded_powershell(script: &str) -> String {
    use base64::Engine as _;
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_agent_in_a_container_refuses() {
        let exe = Path::new("/usr/local/lib/dumbmonit-agent/dumbmonit-agent");
        let error = check(true, Some(exe), None).unwrap_err().to_string();
        assert!(error.contains("docker rm -f"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn only_a_binary_where_the_installer_puts_it_is_removed() {
        for exe in [
            "/usr/local/lib/dumbmonit-agent/dumbmonit-agent",
            "/usr/local/lib/dumbmonit-agent/dumbmonit-agent (deleted)",
            "/usr/local/bin/dumbmonit-agent",
        ] {
            assert!(check(false, Some(Path::new(exe)), None).is_ok(), "{exe}");
        }
        for exe in ["/home/me/dumbmonit-agent", "/opt/dumbmonit/dumbmonit-agent", "/usr/bin/x"] {
            let error = check(false, Some(Path::new(exe)), None).unwrap_err().to_string();
            assert!(error.contains("install script"), "{exe}: {error}");
        }
        assert!(check(false, None, None).is_err());
    }

    #[test]
    fn windows_paths_are_compared_without_case() {
        let expected = Path::new("C:\\Program Files\\DumbMonit");
        let found = Path::new("c:\\program files\\dumbmonit\\");
        assert!(same_dir_ignoring_case(Some(found), expected));
        assert!(!same_dir_ignoring_case(Some(Path::new("C:\\Tools")), expected));
        assert!(!same_dir_ignoring_case(None, expected));
    }

    #[test]
    fn the_unix_script_mirrors_the_installer_and_never_downloads() {
        for needle in [
            UNIX_BIN_DIR,
            UNIX_LINK,
            "/etc/systemd/system/dumbmonit-agent.service",
            "/etc/init.d/dumbmonit-agent",
            "/Library/LaunchDaemons/com.dumbmonit.agent.plist",
            "/usr/local/etc/rc.d/dumbmonit_agent",
            "rm -rf /etc/dumbmonit",
            "rm -rf /usr/local/etc/dumbmonit",
            "systemctl disable --now dumbmonit-agent",
            "launchctl bootout system/com.dumbmonit.agent",
            "sysrc -x dumbmonit_agent_enable",
        ] {
            assert!(UNIX_SCRIPT.contains(needle), "{needle} manque");
        }
        assert!(UNIX_SCRIPT.contains(&format!("sleep {HELPER_DELAY_SECS}\n")));
        // Ni variable ni spécificateur que systemd pourrait développer.
        assert!(!UNIX_SCRIPT.contains('$') && !UNIX_SCRIPT.contains('%'));
        for forbidden in ["curl", "wget", "fetch ", "http", "eval", "rm -rf / ", "rm -rf /\n"] {
            assert!(!UNIX_SCRIPT.contains(forbidden), "{forbidden} dans le script");
        }
    }

    #[test]
    fn the_windows_script_mirrors_the_installer_and_never_downloads() {
        for needle in ["'DumbMonitAgent'", "sc.exe delete", "'DumbMonit'", "$env:ProgramData"] {
            assert!(WINDOWS_SCRIPT.contains(needle), "{needle} manque");
        }
        assert!(WINDOWS_SCRIPT.contains(&format!("Start-Sleep -Seconds {HELPER_DELAY_SECS}\n")));
        for forbidden in ["WebClient", "Invoke-WebRequest", "http", "Invoke-Expression"] {
            assert!(!WINDOWS_SCRIPT.contains(forbidden), "{forbidden} dans le script");
        }
        // UTF-16LE puis base64 : « S » (0x53 0x00) donne « UwA ».
        assert!(encoded_powershell("S").starts_with("UwA"));
    }
}
