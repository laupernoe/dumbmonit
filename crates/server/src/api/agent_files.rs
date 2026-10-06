//! Distribution de l'agent : scripts d'installation et binaires.
//!
//! La commande affichée à la création d'un jeton — `curl …/install.sh | sh` —
//! suppose que ce serveur sait livrer l'agent lui-même. Les scripts sont
//! embarqués dans le binaire ; les exécutables, trop lourds pour cela, sont lus
//! dans `Config::agent_dir`, rempli à la construction de l'image.
//!
//! Ces routes sont publiques : la machine qui s'installe n'a pas de session, et
//! rien ici n'est secret — le jeton voyage dans les arguments, jamais dans
//! l'URL du script.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;

use crate::state::AppState;

const INSTALL_SH: &str = include_str!("../../../agent/install/install.sh");
const INSTALL_PS1: &str = include_str!("../../../agent/install/install.ps1");

pub async fn install_sh() -> Response {
    script(INSTALL_SH, "text/x-shellscript; charset=utf-8")
}

pub async fn install_ps1() -> Response {
    script(INSTALL_PS1, "text/plain; charset=utf-8")
}

fn script(body: &'static str, content_type: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type), (header::CACHE_CONTROL, "no-cache")], body)
        .into_response()
}

/// Noms de fichiers que les scripts d'installation demandent.
///
/// Liste fermée : c'est ce qui empêche `/download/../secret.key` ou toute autre
/// fantaisie de sortir du répertoire, sans avoir à normaliser un chemin.
pub const AGENT_FILES: &[&str] = &[
    "dumbmonit-agent-linux-x86_64",
    "dumbmonit-agent-linux-aarch64",
    "dumbmonit-agent-freebsd-x86_64",
    "dumbmonit-agent-windows-x86_64.exe",
];

/// Binaires que cette image ne peut pas contenir.
///
/// Compiler pour macOS exige le SDK d'Apple, que sa licence interdit de
/// redistribuer : l'image ne peut donc pas l'embarquer, et aucune astuce ne
/// changera cela. Les binaires macOS sont construits sur un exécuteur macOS à
/// chaque version et attachés à la publication.
///
/// Ils restent listés ici pour que `/download/<nom>` (et son `.sha256`) renvoie
/// vers la publication : la commande d'installation affichée par l'interface,
/// la même sous Linux et sous macOS, marche alors aussi sur un Mac. Liste
/// fermée, comme l'autre : le serveur ne renvoie nulle part ailleurs.
pub const AGENT_FILES_RELEASED_ELSEWHERE: &[&str] =
    &["dumbmonit-agent-macos-aarch64", "dumbmonit-agent-macos-x86_64"];

/// Où trouver les binaires que l'image ne livre pas.
pub const RELEASES_URL: &str = "https://github.com/laupernoe/dumbmonit/releases/latest";

/// Fichiers de la dernière publication, par leur nom : GitHub sert sous ce
/// préfixe la pièce jointe de ce nom de la version la plus récente.
const RELEASE_DOWNLOAD_URL: &str =
    "https://github.com/laupernoe/dumbmonit/releases/latest/download";

/// Ce que le serveur répond, en plus du renvoi, pour un binaire qu'il ne livre pas.
///
/// Écrit pour être lu sur la machine qu'on est en train d'installer, par qui
/// télécharge sans suivre les renvois (`curl` sans `-L`) : la raison, l'adresse,
/// et la commande pour s'en servir.
fn released_elsewhere_message(name: &str) -> String {
    format!(
        "{name} is not shipped in the DumbMonit image: building the agent for macOS \
         requires Apple's SDK, which cannot be redistributed.\n\
         It is attached to each release: {RELEASE_DOWNLOAD_URL}/{name}\n\
         (all files: {RELEASES_URL}). Install it with:\n\
         \x20 sudo ./install.sh --token=... --url=... --bin=./{name}\n"
    )
}

/// Réponse donnée pour un binaire (ou son empreinte, `file` finissant alors par
/// `.sha256`) qui n'est, par nature, jamais dans l'image : un renvoi vers la
/// pièce jointe du même nom de la dernière publication.
fn released_elsewhere(name: &str, file: &str) -> Response {
    let location = format!("{RELEASE_DOWNLOAD_URL}/{file}");
    (
        StatusCode::TEMPORARY_REDIRECT,
        [(header::LOCATION, location), (header::CACHE_CONTROL, "no-cache".to_string())],
        released_elsewhere_message(name),
    )
        .into_response()
}

/// Suffixe sous lequel l'empreinte d'un binaire est servie : `/download/<nom>.sha256`.
const CHECKSUM_SUFFIX: &str = ".sha256";

/// Empreintes déjà calculées, par chemin. Les binaires sont figés dans l'image :
/// une empreinte ne change pas pendant la vie du processus, et la recalculer à
/// chaque installation relirait dix mégaoctets pour rien.
static CHECKSUMS: LazyLock<Mutex<HashMap<PathBuf, String>>> = LazyLock::new(Mutex::default);

/// `GET /download/{name}` — le binaire, ou son empreinte SHA-256 quand `name` se
/// termine par `.sha256`.
///
/// Les scripts d'installation vérifient le binaire téléchargé contre cette
/// empreinte, et l'interface l'affiche à côté de la commande d'installation.
/// Servie par la même origine que le binaire, elle détecte un téléchargement
/// tronqué ou corrompu, mais ne protège pas d'un intermédiaire qui, en HTTP en
/// clair, remplacerait le script, le binaire et l'empreinte ensemble : seul
/// HTTPS (ou la comparaison avec l'empreinte affichée) le fait.
pub async fn download(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    if let Some(binary) = name.strip_suffix(CHECKSUM_SUFFIX) {
        return checksum(&state, binary).await;
    }
    if AGENT_FILES_RELEASED_ELSEWHERE.contains(&name.as_str()) {
        return released_elsewhere(&name, &name);
    }
    if !AGENT_FILES.contains(&name.as_str()) {
        return (StatusCode::NOT_FOUND, "Unknown file.").into_response();
    }

    let path = state.config.agent_dir.join(&name);
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!(path = %path.display(), "agent binary missing from this image");
            return (
                StatusCode::NOT_FOUND,
                "This image does not ship the agent binaries. Install the agent with --bin=PATH.",
            )
                .into_response();
        }
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "failed to read agent binary");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response();
        }
    };

    let mut response = Body::from_stream(ReaderStream::new(file)).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
    if let Ok(value) = format!("attachment; filename=\"{name}\"").parse() {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

/// Empreinte au format de `sha256sum` (`<hex>  <nom>`), pour que le script
/// d'installation puisse la passer telle quelle à `sha256sum -c`.
async fn checksum(state: &AppState, name: &str) -> Response {
    if AGENT_FILES_RELEASED_ELSEWHERE.contains(&name) {
        return released_elsewhere(name, &format!("{name}{CHECKSUM_SUFFIX}"));
    }
    if !AGENT_FILES.contains(&name) {
        return (StatusCode::NOT_FOUND, "Unknown file.").into_response();
    }
    let digest = match sha256_of(&state.config.agent_dir, name).await {
        Ok(Some(digest)) => digest,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "This image does not ship the agent binaries.")
                .into_response();
        }
        Err(error) => {
            tracing::error!(name, %error, "failed to hash agent binary");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response();
        }
    };
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")],
        format!("{digest}  {name}\n"),
    )
        .into_response()
}

/// Empreinte SHA-256 (hexadécimal) d'un binaire livré par l'image, ou `None`
/// s'il n'y est pas. Mise en cache : les binaires sont figés dans l'image.
pub async fn sha256_of(agent_dir: &std::path::Path, name: &str) -> std::io::Result<Option<String>> {
    let path = agent_dir.join(name);
    if let Some(digest) = CHECKSUMS.lock().expect("cache des empreintes").get(&path).cloned() {
        return Ok(Some(digest));
    }
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let digest = hex::encode(Sha256::digest(&bytes));
            CHECKSUMS.lock().expect("cache des empreintes").insert(path, digest.clone());
            Ok(Some(digest))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Empreintes des binaires que l'image livre, par plateforme (`linux-x86_64`,
/// `windows-x86_64`…), dans l'ordre de [`AGENT_FILES`]. Ce sont elles que la
/// commande d'installation embarque, et que les scripts exigent.
pub async fn platform_checksums(agent_dir: &std::path::Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for name in AGENT_FILES {
        match sha256_of(agent_dir, name).await {
            Ok(Some(digest)) => {
                let platform = name.trim_start_matches("dumbmonit-agent-").trim_end_matches(".exe");
                found.push((platform.to_string(), digest));
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(name, %error, "failed to hash agent binary"),
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_checksums_of_the_shipped_binaries_are_named_by_platform() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("dumbmonit-agent-linux-x86_64"), b"abc").unwrap();
        std::fs::write(dir.path().join("dumbmonit-agent-windows-x86_64.exe"), b"").unwrap();
        let found = platform_checksums(dir.path()).await;
        assert_eq!(
            found,
            vec![
                (
                    "linux-x86_64".to_string(),
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_string()
                ),
                (
                    "windows-x86_64".to_string(),
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()
                ),
            ]
        );
        // Une image sans binaires n'a rien à embarquer.
        assert!(platform_checksums(&dir.path().join("absent")).await.is_empty());
    }

    /// La vérification est obligatoire : sans empreinte, ou sans outil pour la
    /// calculer, rien n'est installé, sauf échappatoire explicite et bruyante.
    #[test]
    fn the_scripts_refuse_a_download_without_its_checksum() {
        assert!(INSTALL_SH.contains("--sha256=*)"));
        assert!(INSTALL_SH.contains("--insecure-skip-checksum)"));
        assert!(INSTALL_SH.contains("no expected checksum for $PLATFORM-$ARCH"));
        assert!(INSTALL_SH.contains("WARNING: --insecure-skip-checksum"));
        assert!(!INSTALL_SH.contains("binary not verified\" >&2\n        return 0"));
        assert!(!INSTALL_SH.contains("Warning: no SHA-256 tool found"));
        // La vérification précède l'installation du binaire.
        let verify = INSTALL_SH.find("verifier_empreinte \"$source_url\"").expect("appel");
        let install = INSTALL_SH.find("mv -f \"$TMP_BIN\" \"$BIN_PATH\"").expect("installation");
        assert!(verify < install);

        assert!(INSTALL_PS1.contains("[string]$Sha256"));
        assert!(INSTALL_PS1.contains("[switch]$InsecureSkipChecksum"));
        assert!(INSTALL_PS1.contains("No expected checksum for"));
        assert!(!INSTALL_PS1.contains("binary not verified\"\n"));
        let verify = INSTALL_PS1.find("Get-FileHash").expect("vérification");
        let install = INSTALL_PS1.find("Move-Item -Path $exeTemporaire").expect("installation");
        assert!(verify < install);
    }

    #[test]
    fn les_scripts_embarques_demandent_les_fichiers_que_le_serveur_sait_servir() {
        // Un renommage d'un côté sans l'autre ne casserait rien à la compilation,
        // seulement l'installation chez l'utilisateur, en 404.
        assert!(INSTALL_SH.contains("/download/dumbmonit-agent-$PLATFORM-$ARCH"));
        assert!(INSTALL_PS1.contains("/download/dumbmonit-agent-windows-$architecture.exe"));
        for arch in ["x86_64", "aarch64"] {
            assert!(AGENT_FILES.contains(&format!("dumbmonit-agent-linux-{arch}").as_str()));
        }
        // Les noms que le script compose à partir de `uname` doivent tous être
        // connus d'un côté ou de l'autre : servis par l'image, ou publiés avec
        // la version. Un nom qu'aucune des deux listes ne connaît est un 404
        // que personne ne saura expliquer.
        // La matrice exacte que `install.sh` accepte : tout ce qu'il refuse, il
        // le refuse avec une phrase, avant le moindre téléchargement.
        for name in [
            "dumbmonit-agent-linux-x86_64",
            "dumbmonit-agent-linux-aarch64",
            "dumbmonit-agent-freebsd-x86_64",
            "dumbmonit-agent-macos-x86_64",
            "dumbmonit-agent-macos-aarch64",
        ] {
            assert!(
                AGENT_FILES.contains(&name) || AGENT_FILES_RELEASED_ELSEWHERE.contains(&name),
                "{name} is composed by install.sh but known nowhere"
            );
        }
        // Et ils vérifient ce qu'ils ont téléchargé contre l'empreinte que porte
        // la commande d'installation.
        assert!(INSTALL_SH.contains("sha256sum"));
        assert!(INSTALL_SH.contains("$PLATFORM-$ARCH"));
        assert!(INSTALL_PS1.contains("windows-$architecture"));
        assert!(INSTALL_PS1.contains("Get-FileHash"));
    }

    /// Le dossier de configuration Windows est protégé avant que le jeton n'y
    /// soit écrit, et un dossier préparé par un autre compte arrête tout.
    #[test]
    fn le_script_windows_protege_la_configuration_avant_d_y_ecrire() {
        let body = INSTALL_PS1
            .split("# --------------------------------------------------------------- privilèges")
            .nth(1)
            .expect("corps du script");
        let protect = body.find("Protect-ConfigDir\n").expect("Protect-ConfigDir appelée");
        let write = body.find("WriteAllLines($ConfigPath").expect("écriture de la configuration");
        assert!(protect < write, "la protection doit précéder l'écriture du jeton");
        assert!(
            !body.contains("New-Item -ItemType Directory -Path $ConfigDir  -Force"),
            "le dossier ne doit pas être créé avec les droits hérités de ProgramData"
        );
        assert!(INSTALL_PS1.contains("SetAccessRuleProtection($true, $false)"));
        assert!(INSTALL_PS1.contains("/setowner '*S-1-5-32-544'"));
        assert!(INSTALL_PS1.contains("belongs to another account"));
    }

    #[test]
    fn les_binaires_publies_ailleurs_ne_sont_jamais_servis_par_l_image() {
        // Les deux listes ne doivent pas se recouvrir : un nom présent dans les
        // deux serait servi ou expliqué selon l'ordre du code, ce qui est
        // exactement le genre de détail dont personne ne se souvient.
        for name in AGENT_FILES_RELEASED_ELSEWHERE {
            assert!(!AGENT_FILES.contains(name), "{name} is in both lists");
        }
    }

    #[test]
    fn un_binaire_absent_par_nature_renvoie_vers_la_publication() {
        // C'est ce renvoi qui fait marcher sur un Mac la commande que l'interface
        // affiche : `curl -fsSL` (install.sh) le suit, binaire comme empreinte.
        let name = "dumbmonit-agent-macos-aarch64";
        let location = |response: &Response| {
            response.headers().get(header::LOCATION).unwrap().to_str().unwrap().to_owned()
        };

        let binary = released_elsewhere(name, name);
        assert_eq!(binary.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            location(&binary),
            "https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64"
        );

        // L'empreinte suit le même chemin : le fichier `.sha256` publié à côté du
        // binaire, au format de `shasum -a 256` que le script sait lire.
        let checksum = released_elsewhere(name, &format!("{name}{CHECKSUM_SUFFIX}"));
        assert_eq!(checksum.status(), StatusCode::TEMPORARY_REDIRECT);
        assert!(
            location(&checksum).ends_with("/latest/download/dumbmonit-agent-macos-aarch64.sha256")
        );
    }

    #[test]
    fn un_binaire_absent_par_nature_dit_ou_le_trouver() {
        // Pour qui télécharge sans suivre les renvois, le corps est la seule chose
        // visible : il doit porter la raison, l'adresse et la commande — pas
        // seulement un regret.
        let name = "dumbmonit-agent-macos-aarch64";
        let message = released_elsewhere_message(name);
        assert!(message.contains(name));
        assert!(message.contains(RELEASES_URL));
        assert!(message.contains("--bin="));
        assert!(message.contains("SDK"));
    }

    #[test]
    fn le_script_dinstallation_sait_reconnaitre_les_quatre_systemes() {
        for marker in ["systemd", "openrc", "launchd", "rcd"] {
            assert!(INSTALL_SH.contains(marker), "install.sh ignores {marker}");
        }
        // Et il sait nommer le binaire de chaque plateforme.
        for platform in ["linux", "freebsd", "macos"] {
            assert!(INSTALL_SH.contains(platform), "install.sh never mentions {platform}");
        }
    }
}
