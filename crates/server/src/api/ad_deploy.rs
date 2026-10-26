//! Déploiement de l'agent par stratégie de groupe Active Directory.
//!
//! `POST /api/agent/ad-kit` crée un jeton de flotte limité à un domaine et
//! rend `Deploy-DumbMonit.ps1`, un script à lancer une fois sur un contrôleur
//! de domaine : il pose une GPO dont la tâche planifiée installe l'agent sur
//! chaque serveur visé, qui s'inscrit alors sous son nom complet.
//!
//! La commande d'installation n'est pas recomposée ici : le jeton est créé par
//! le même chemin que depuis l'interface ([`ingest::create_token`]), et ce sont
//! les arguments de *sa* commande Windows que le kit transmet à l'installateur.
//! Tout ce que cette commande portera demain (empreintes, épinglage du
//! transport…) arrive donc dans le kit sans retoucher ce fichier.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::api::ingest::{self, CreatedToken, TokenPayload, TokenView};
use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{AdminIdentity, Authenticated};
use crate::collectors::agent;
use crate::state::AppState;

/// Script lancé une fois sur un contrôleur de domaine.
const DEPLOY_TEMPLATE: &str = include_str!("../../../agent/install/ad/Deploy-DumbMonit.ps1");
/// Script exécuté par la tâche planifiée de la GPO sur chaque serveur.
const BOOTSTRAP_TEMPLATE: &str =
    include_str!("../../../agent/install/ad/Install-DumbMonitAgent.ps1");
/// Installateur Windows, le même que celui de `/install.ps1`.
const INSTALL_PS1: &str = include_str!("../../../agent/install/install.ps1");

/// Nom du fichier proposé au téléchargement.
pub const FILENAME: &str = "Deploy-DumbMonit.ps1";

/// Binaire Windows dont l'empreinte est attendue par le kit.
const WINDOWS_BINARY: &str = "dumbmonit-agent-windows-x86_64.exe";

/// Longueur maximale d'un nom distinctif d'OU ou de groupe saisi.
const MAX_FIELD_LEN: usize = 512;

/// Validité par défaut d'un jeton de flotte, en jours : le temps de déployer,
/// pas davantage. Les machines inscrites continuent après le terme.
const DEFAULT_EXPIRES_IN_DAYS: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct AdKitPayload {
    /// Domaine DNS Active Directory (`corp.example.com`).
    pub domain: String,
    /// URL par laquelle les serveurs joindront DumbMonit (voir [`TokenPayload`]).
    #[serde(default)]
    pub base_url: Option<String>,
    /// Nombre maximal de serveurs inscrits. Absent : sans limite.
    #[serde(default)]
    pub max_uses: Option<i64>,
    /// Jours pendant lesquels le jeton inscrit. Absent : 30 ; `null` explicite
    /// n'est pas distingué de l'absence.
    #[serde(default)]
    pub expires_in_days: Option<i64>,
    /// OU à laquelle lier la GPO (nom distinctif). Absent : le script cherche.
    #[serde(default)]
    pub target_ou: Option<String>,
    /// Lier aussi la GPO à l'OU Domain Controllers.
    #[serde(default)]
    pub include_domain_controllers: bool,
    /// Groupe de sécurité auquel restreindre la GPO.
    #[serde(default)]
    pub security_group: Option<String>,
    /// Copier aussi le binaire dans SYSVOL, pour les serveurs sans accès sortant.
    #[serde(default)]
    pub sysvol_binary: bool,
}

/// Réponse : le jeton (montré cette fois seulement) et le script qui le porte.
#[derive(Debug, Serialize)]
pub struct CreatedAdKit {
    pub token: TokenView,
    pub secret: String,
    pub script: String,
    pub filename: &'static str,
    /// Dossier dans lequel les serveurs inscrits sont rangés.
    pub folder: String,
    pub install_windows: String,
}

/// Champ libre facultatif : sans blancs autour, `None` s'il est vide.
fn optional_field(value: Option<&str>, what: &str) -> Result<Option<String>, ApiError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > MAX_FIELD_LEN || value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(format!("The {what} is not valid.")));
    }
    Ok(Some(value.to_string()))
}

/// URL telle qu'elle sera écrite, entre apostrophes, dans les scripts.
///
/// Elle vient du navigateur de l'administrateur ; elle finit dans un script
/// exécuté en administrateur du domaine. Rien d'autre qu'une URL HTTP(S)
/// sans apostrophe, guillemet, accent grave ni blanc n'y entre.
fn script_safe_url(url: &str) -> Result<String, ApiError> {
    let url = url.trim().trim_end_matches('/');
    let scheme_ok = url.starts_with("https://") || url.starts_with("http://");
    let forbidden = |c: char| {
        c.is_whitespace()
            || c.is_control()
            || matches!(
                c,
                '\'' | '"' | '`' | '$' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}'
            )
    };
    if !scheme_ok || url.chars().any(forbidden) || url.len() > 2048 {
        return Err(ApiError::BadRequest(
            "The server URL must be a plain http:// or https:// address.".into(),
        ));
    }
    Ok(url.to_string())
}

/// Arguments de la commande d'installation Windows générée par le serveur,
/// sans l'appel de l'installateur qui les précède.
///
/// La commande a la forme `& ([scriptblock]::Create((irm …/install.ps1))) -Token …` :
/// le kit embarque l'installateur, il ne garde que ce qui suit. Repli sur
/// `-Token` si la forme de l'appel venait à changer.
pub fn installer_arguments(command: &str) -> Option<&str> {
    if let Some((_, arguments)) = command.split_once("))) ") {
        return Some(arguments.trim());
    }
    command.find("-Token ").map(|start| command[start..].trim())
}

/// Valeurs substituées dans les modèles de scripts.
pub struct KitParams<'a> {
    pub domain: &'a str,
    pub url: &'a str,
    pub install_args: &'a str,
    pub sha256: &'a str,
    pub token_prefix: &'a str,
    pub target_ou: &'a str,
    pub include_domain_controllers: bool,
    pub security_group: &'a str,
    pub sysvol_binary: bool,
    pub generated: &'a str,
}

fn b64(text: &str) -> String {
    STANDARD.encode(text.as_bytes())
}

/// Produit `Deploy-DumbMonit.ps1`.
///
/// Les valeurs libres (OU, groupe, arguments) passent en base64 : aucune
/// apostrophe, typographique ou non, ne peut sortir de sa chaîne. Les autres
/// (domaine, URL, empreinte, préfixe) ont été réduites à des caractères sûrs.
/// Le texte commence par une marque d'ordre UTF-8 : Windows PowerShell 5 lit
/// sinon le fichier en ANSI.
pub fn render_deploy_script(params: &KitParams<'_>) -> String {
    let bootstrap = BOOTSTRAP_TEMPLATE
        .replace("__DM_URL__", params.url)
        .replace("__DM_SHA256__", params.sha256)
        // Version du binaire du kit : un agent plus récent (mis à jour par le
        // serveur) n'est jamais rétrogradé par la stratégie de groupe.
        .replace("__DM_VERSION__", env!("CARGO_PKG_VERSION"))
        .replace("__DM_DOMAIN__", params.domain)
        .replace("__DM_INSTALL_ARGS_B64__", &b64(params.install_args));
    let script = DEPLOY_TEMPLATE
        .replace("__DM_TARGET_OU_B64__", &b64(params.target_ou))
        .replace(
            "__DM_INCLUDE_DCS__",
            if params.include_domain_controllers { "true" } else { "false" },
        )
        .replace("__DM_SECURITY_GROUP_B64__", &b64(params.security_group))
        .replace("__DM_SYSVOL__", if params.sysvol_binary { "true" } else { "false" })
        .replace("__DM_DOMAIN__", params.domain)
        .replace("__DM_URL__", params.url)
        .replace("__DM_SHA256__", params.sha256)
        .replace("__DM_TOKEN_PREFIX__", params.token_prefix)
        .replace("__DM_GENERATED__", params.generated)
        .replace("__DM_INSTALLER_B64__", &b64(INSTALL_PS1))
        .replace("__DM_BOOTSTRAP_B64__", &b64(&bootstrap));
    format!("\u{feff}{script}")
}

/// `POST /api/agent/ad-kit` — jeton de flotte Active Directory et son script.
pub async fn create_kit(
    _: AdminIdentity,
    State(state): State<AppState>,
    Authenticated(me): Authenticated,
    ClientIp(ip): ClientIp,
    Json(payload): Json<AdKitPayload>,
) -> ApiResult<(StatusCode, Json<CreatedAdKit>)> {
    let domain = agent::clean_ad_domain(&payload.domain).ok_or_else(|| {
        ApiError::BadRequest(
            "Enter the DNS name of the Active Directory domain, for example corp.example.com."
                .into(),
        )
    })?;
    let target_ou = optional_field(payload.target_ou.as_deref(), "organisational unit")?;
    let security_group = optional_field(payload.security_group.as_deref(), "security group")?;
    let expires_in_days = payload.expires_in_days.unwrap_or(DEFAULT_EXPIRES_IN_DAYS);
    // Refusée avant de créer quoi que ce soit, plutôt qu'avec un jeton déjà fait.
    if let Some(url) = payload.base_url.as_deref().filter(|url| !url.trim().is_empty()) {
        script_safe_url(url)?;
    }

    // Le jeton est créé par le chemin ordinaire : mêmes contrôles, même
    // journal d'audit, même commande d'installation. Construit depuis du JSON
    // pour suivre les champs que ce chemin gagnera.
    let token_payload: TokenPayload = serde_json::from_value(serde_json::json!({
        "name": agent::ad_folder(&domain),
        "base_url": payload.base_url,
        "reusable": true,
        "max_uses": payload.max_uses,
        "expires_in_days": expires_in_days,
    }))
    .map_err(|error| ApiError::Internal(error.into()))?;
    let (_, Json(created)) = ingest::create_token(
        State(state.clone()),
        Authenticated(me.clone()),
        ClientIp(ip),
        Json(token_payload),
    )
    .await?;
    let CreatedToken { mut token, secret, install_windows, .. } = created;

    agent::set_token_ad_domain(&state.pool, token.id, &domain).await?;
    token.ad_domain = Some(domain.clone());
    audit::record(&state.pool, Some(&me.username), "agent_token.ad_kit", Some(&domain), ip).await;

    let arguments = installer_arguments(&install_windows)
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("unexpected install command shape")))?;
    // L'URL que porte la commande est celle que les serveurs utiliseront.
    let url = arguments
        .split_whitespace()
        .skip_while(|word| *word != "-Url")
        .nth(1)
        .map(str::to_string)
        .unwrap_or_default();
    let url = script_safe_url(&url)?;
    let sha256 = crate::api::agent_files::sha256_of(&state.config.agent_dir, WINDOWS_BINARY)
        .await
        .unwrap_or_default()
        .unwrap_or_default();
    let generated = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();

    let script = render_deploy_script(&KitParams {
        domain: &domain,
        url: &url,
        install_args: arguments,
        sha256: &sha256,
        token_prefix: &token.prefix,
        target_ou: target_ou.as_deref().unwrap_or(""),
        include_domain_controllers: payload.include_domain_controllers,
        security_group: security_group.as_deref().unwrap_or(""),
        sysvol_binary: payload.sysvol_binary,
        generated: &generated,
    });

    Ok((
        StatusCode::CREATED,
        Json(CreatedAdKit {
            folder: agent::ad_folder(&domain),
            token,
            secret,
            script,
            filename: FILENAME,
            install_windows,
        }),
    ))
}

/// `POST /api/agent/tokens/{id}/close` — plus d'inscriptions, sans couper les
/// machines déjà inscrites (voir [`agent::close_enrolment`]).
pub async fn close_token(
    _: AdminIdentity,
    State(state): State<AppState>,
    Authenticated(me): Authenticated,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if agent::close_enrolment(&state.pool, id).await? {
        let subject = id.to_string();
        audit::record(&state.pool, Some(&me.username), "agent_token.closed", Some(&subject), ip)
            .await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Token {id} not found or revoked.")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(args: &str) -> KitParams<'_> {
        KitParams {
            domain: "corp.example.com",
            url: "https://dm.example.com",
            install_args: args,
            sha256: "ab".repeat(32).leak(),
            token_prefix: "dmon_12345678",
            target_ou: "OU=Servers,DC=corp,DC=example,DC=com",
            include_domain_controllers: true,
            security_group: "Serveurs l'équipe",
            sysvol_binary: false,
            generated: "2026-10-22 12:00",
        }
    }

    /// Décode la chaîne base64 qui suit `marker` dans un script : le texte
    /// entre les deux apostrophes suivantes.
    fn decode(script: &str, marker: &str) -> String {
        let after = &script[script.find(marker).expect("repère") + marker.len()..];
        let start = after.find('\'').expect("début de chaîne") + 1;
        let end = start + after[start..].find('\'').expect("fin de chaîne");
        String::from_utf8(STANDARD.decode(&after[start..end]).expect("base64")).expect("utf-8")
    }

    #[test]
    fn the_installer_arguments_are_those_of_the_generated_command() {
        let command = "& ([scriptblock]::Create((irm https://dm/install.ps1))) -Token dmon_x \
                       -Url https://dm -Sha256 windows-x86_64:abc";
        assert_eq!(
            installer_arguments(command),
            Some("-Token dmon_x -Url https://dm -Sha256 windows-x86_64:abc")
        );
        // Une autre forme d'appel garde ce qui commence à -Token.
        assert_eq!(installer_arguments("installer.exe -Token t -Url u"), Some("-Token t -Url u"));
        assert_eq!(installer_arguments("nothing here"), None);
    }

    #[test]
    fn the_kit_keeps_the_encrypted_agent_channel_of_the_install_command() {
        // La commande vient de `ingest::install_parameters` : canal chiffré et
        // empreinte épinglée passent tels quels dans le script de la tâche.
        let pin = "a".repeat(64);
        let command = format!(
            "& ([scriptblock]::Create((irm https://dm/install.ps1))) -Token dmon_x -Url https://dm \
             -Sha256 windows-x86_64:abc -TlsUrl https://dm:8443 -CaPin {pin}"
        );
        let arguments = installer_arguments(&command).expect("arguments");
        assert!(
            arguments.ends_with(&format!("-TlsUrl https://dm:8443 -CaPin {pin}")),
            "{arguments}"
        );
    }

    #[test]
    fn the_script_carries_every_value_and_no_placeholder() {
        let args = "-Token dmon_secret -Url https://dm.example.com -Sha256 windows-x86_64:00";
        let script = render_deploy_script(&params(args));

        assert!(script.starts_with('\u{feff}'), "marque d'ordre pour Windows PowerShell 5");
        assert!(!script.contains("__DM_"), "un paramètre n'a pas été remplacé");
        assert!(script.contains("$Domain         = 'corp.example.com'"));
        assert!(script.contains("$Url            = 'https://dm.example.com'"));
        assert!(script.contains("[bool]::Parse('true')"), "contrôleurs de domaine inclus");
        assert!(script.contains("[bool]::Parse('false')"), "pas de binaire dans SYSVOL");
        // Le jeton n'apparaît pas en clair : il n'est que dans le script de la
        // tâche, encodé, lui-même encodé dans celui-ci.
        assert!(!script.contains("dmon_secret"));

        let bootstrap = decode(&script, "$BootstrapB64");
        assert!(!bootstrap.contains("__DM_"), "un paramètre du script de tâche manque");
        assert!(bootstrap.contains("$Domain         = 'corp.example.com'"));
        assert_eq!(decode(&bootstrap, "$InstallArgsB64"), args);
        // Un agent mis à jour par le serveur, plus récent que le kit, reste en
        // place : la stratégie de groupe ne le rétrograde pas.
        let version = format!("$KitVersion     = '{}'", env!("CARGO_PKG_VERSION"));
        assert!(bootstrap.contains(&version), "version du kit");
        assert!(bootstrap.contains("($current -ge $kit)"));

        // Les champs libres voyagent encodés : l'apostrophe ne s'échappe pas.
        assert_eq!(decode(&script, "[string]$SecurityGroup"), "Serveurs l'équipe");
        assert_eq!(decode(&script, "[string]$TargetOU"), "OU=Servers,DC=corp,DC=example,DC=com");
        assert_eq!(decode(&script, "$InstallerB64"), INSTALL_PS1);
    }

    #[test]
    fn the_deployment_script_does_what_the_wizard_promises() {
        // Prérequis, simulation, GPO idempotente, tâche immédiate SYSTEM,
        // filtrage, liens, SYSVOL et retrait.
        for needle in [
            "SupportsShouldProcess = $true",
            "Get-Module -ListAvailable -Name $module",
            "-512\"",
            "New-GPO -Name $GpoName",
            "Get-GPO -Name $GpoName",
            "ImmediateTaskV2",
            "runAs=\"NT AUTHORITY\\System\"",
            "{AADCED64-746C-4633-A97C-D61349046527}",
            "Set-GPPermission",
            "New-GPLink",
            "[string]$TargetOU",
            "[switch]$IncludeDomainControllers",
            "[string]$SecurityGroup",
            "[switch]$UseSysvol",
            "[switch]$Remove",
            "Remove-GPO",
            "SetAccessRuleProtection($true, $false)",
        ] {
            assert!(DEPLOY_TEMPLATE.contains(needle), "Deploy-DumbMonit.ps1 : {needle}");
        }
        for needle in [
            "Write-EventLog",
            "Get-FileHash",
            "-EncodedCommand",
            "Hide-Secrets",
            "-HostName",
            "ad_ou",
            "-BinPath",
            "-Uninstall",
        ] {
            assert!(BOOTSTRAP_TEMPLATE.contains(needle), "Install-DumbMonitAgent.ps1 : {needle}");
        }
    }

    #[test]
    fn a_url_that_could_break_out_of_its_string_is_refused() {
        assert_eq!(
            script_safe_url("https://dm.example.com/").ok().as_deref(),
            Some("https://dm.example.com")
        );
        assert!(script_safe_url("http://10.0.0.5:8080").is_ok());
        for bad in [
            "",
            "ftp://dm",
            "https://dm'; Remove-Item C:\\ -Recurse; '",
            "https://dm\u{2019}x",
            "https://dm/$(calc)",
            "https://d m",
            "https://dm`n",
        ] {
            assert!(script_safe_url(bad).is_err(), "{bad:?} doit être refusée");
        }
    }

    #[test]
    fn free_fields_are_trimmed_and_bounded() {
        assert!(matches!(optional_field(Some("  "), "x"), Ok(None)));
        assert!(matches!(optional_field(None, "x"), Ok(None)));
        assert!(
            matches!(optional_field(Some(" OU=A,DC=b "), "x"), Ok(Some(ref v)) if v == "OU=A,DC=b")
        );
        assert!(optional_field(Some("a\nb"), "x").is_err());
        assert!(optional_field(Some(&"a".repeat(MAX_FIELD_LEN + 1)), "x").is_err());
    }
}
