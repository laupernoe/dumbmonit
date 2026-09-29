//! Les services de messagerie, vus de l'extérieur : un port ouvert, et pour les
//! protocoles texte, une bannière d'accueil qui dit « en service ».
//!
//! C'est la mesure qui ne demande aucun identifiant et qui ne dépend d'aucune
//! API : elle fonctionne sur toutes les versions des deux produits. La
//! connexion est refermée poliment (`QUIT`, `LOGOUT`) après la bannière, pour
//! ne laisser dans les journaux du serveur qu'une session courte et propre —
//! jamais une tentative d'authentification.

use std::time::{Duration, Instant};

use dumbmonit_proto::ProbeError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Longueur maximale lue d'une bannière.
const MAX_BANNER: usize = 512;

/// Nombre maximal de services surveillés sur un même équipement.
const MAX_SERVICES: usize = 16;

/// Ce que le service doit dire en premier pour être jugé en service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greeting {
    /// `220` (RFC 5321 § 4.2).
    Smtp,
    /// `+OK` (RFC 1939 § 4).
    Pop3,
    /// `* OK` ou `* PREAUTH` (RFC 9051 § 7.1).
    Imap,
    /// Rien à lire : la connexion acceptée suffit (HTTP, TLS implicite, XMPP).
    None,
}

/// Un service connu d'un produit, avec son port par défaut.
#[derive(Debug, Clone, Copy)]
pub struct KnownService {
    pub name: &'static str,
    pub port: u16,
    pub greeting: Greeting,
}

const fn known(name: &'static str, port: u16, greeting: Greeting) -> KnownService {
    KnownService { name, port, greeting }
}

/// Les ports par défaut de MDaemon Email Server, tels que MDaemon Technologies
/// les liste pour le pare-feu (base de connaissances, « What ports need to be
/// open for MDaemon »).
pub const MDAEMON_SERVICES: &[KnownService] = &[
    known("smtp", 25, Greeting::Smtp),
    known("msa", 587, Greeting::Smtp),
    known("smtps", 465, Greeting::None),
    known("pop3", 110, Greeting::Pop3),
    known("pop3s", 995, Greeting::None),
    known("imap", 143, Greeting::Imap),
    known("imaps", 993, Greeting::None),
    known("webmail", 3000, Greeting::None),
    known("remote_admin", 1000, Greeting::None),
    known("remote_admin_https", 444, Greeting::None),
    known("xmpp", 5222, Greeting::None),
];

/// Défaut prudent : SMTP, IMAP et la messagerie web sont ouverts sur presque
/// toutes les installations ; POP3, MSA ou XMPP souvent désactivés, et un port
/// fermé volontairement ne doit pas alerter.
pub const MDAEMON_DEFAULT: &str = "smtp,imap,webmail";

/// Les ports par défaut de SecurityGateway (aide « HTTP Interface » et
/// « Ports »).
pub const SECURITY_GATEWAY_SERVICES: &[KnownService] = &[
    known("smtp", 25, Greeting::Smtp),
    known("smtps", 465, Greeting::None),
    known("web", 4000, Greeting::None),
    known("web_https", 4443, Greeting::None),
];

pub const SECURITY_GATEWAY_DEFAULT: &str = "smtp,web";

/// Un service à vérifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSpec {
    pub name: String,
    pub port: u16,
    pub greeting: Greeting,
}

/// Lit la liste `services` : `smtp,imap` ou `smtp:2525,webmail:8080`. Un nom
/// inconnu est accepté s'il porte son port — il est alors vérifié par simple
/// connexion. `none` désactive la vérification des ports.
pub fn parse_services(
    raw: Option<&str>,
    catalog: &[KnownService],
    default: &str,
) -> Result<Vec<ServiceSpec>, ProbeError> {
    let raw = raw.unwrap_or(default).trim();
    if raw.eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    let mut specs: Vec<ServiceSpec> = Vec::new();
    for item in raw.split(',').map(str::trim).filter(|item| !item.is_empty()) {
        let (name, port) = match item.split_once(':') {
            Some((name, port)) => (name.trim(), Some(super::address::parse_port(port)?)),
            None => (item, None),
        };
        let name = name.to_ascii_lowercase();
        if name.is_empty()
            || name.len() > 32
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ProbeError::Config(format!("Invalid service name: \"{item}\"")));
        }
        let entry = catalog.iter().find(|known| known.name == name);
        let spec = match (entry, port) {
            (Some(known), port) => {
                ServiceSpec { name, port: port.unwrap_or(known.port), greeting: known.greeting }
            }
            (None, Some(port)) => ServiceSpec { name, port, greeting: Greeting::None },
            (None, None) => {
                let names: Vec<&str> = catalog.iter().map(|known| known.name).collect();
                return Err(ProbeError::Config(format!(
                    "Unknown service \"{name}\": expected one of {}, or name:port for another port",
                    names.join(", ")
                )));
            }
        };
        if !specs.iter().any(|s| s.name == spec.name && s.port == spec.port) {
            specs.push(spec);
        }
    }
    if specs.len() > MAX_SERVICES {
        return Err(ProbeError::Config(format!(
            "At most {MAX_SERVICES} services can be watched on one device"
        )));
    }
    Ok(specs)
}

/// Résultat d'une vérification.
#[derive(Debug, Clone)]
pub struct ServiceCheck {
    pub name: String,
    pub port: u16,
    pub up: bool,
    /// Temps jusqu'à la bannière (ou jusqu'à la connexion, sans bannière).
    pub seconds: Option<f64>,
    /// Pourquoi le service est jugé arrêté.
    pub reason: Option<String>,
    /// Première ligne lue, pour la version annoncée.
    pub banner: Option<String>,
}

/// Vérifie tous les services en parallèle.
pub async fn check_all(host: &str, specs: &[ServiceSpec], timeout: Duration) -> Vec<ServiceCheck> {
    futures::future::join_all(specs.iter().map(|spec| check(host, spec, timeout))).await
}

async fn check(host: &str, spec: &ServiceSpec, timeout: Duration) -> ServiceCheck {
    let started = Instant::now();
    let mut outcome = ServiceCheck {
        name: spec.name.clone(),
        port: spec.port,
        up: false,
        seconds: None,
        reason: None,
        banner: None,
    };
    let stream = match tokio::time::timeout(timeout, TcpStream::connect((host, spec.port))).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(error)) => {
            outcome.reason = Some(format!("connection failed: {error}"));
            return outcome;
        }
        Err(_) => {
            outcome.reason = Some(format!("no answer within {}s", timeout.as_secs()));
            return outcome;
        }
    };
    let mut stream = stream;
    if spec.greeting == Greeting::None {
        outcome.up = true;
        outcome.seconds = Some(started.elapsed().as_secs_f64());
        return outcome;
    }
    let line = match tokio::time::timeout(timeout, read_line(&mut stream)).await {
        Ok(Ok(line)) => line,
        Ok(Err(error)) => {
            outcome.reason = Some(format!("no greeting: {error}"));
            return outcome;
        }
        Err(_) => {
            outcome.reason = Some(format!("no greeting within {}s", timeout.as_secs()));
            return outcome;
        }
    };
    outcome.seconds = Some(started.elapsed().as_secs_f64());
    match judge(spec.greeting, &line) {
        Ok(()) => outcome.up = true,
        Err(reason) => outcome.reason = Some(reason),
    }
    outcome.banner = Some(line);
    // Au mieux : un serveur qui a déjà coupé ne rend pas le service arrêté.
    let goodbye: &[u8] = match spec.greeting {
        Greeting::Imap => b"a1 LOGOUT\r\n",
        _ => b"QUIT\r\n",
    };
    let _ = tokio::time::timeout(Duration::from_secs(1), async {
        let _ = stream.write_all(goodbye).await;
        let _ = stream.shutdown().await;
    })
    .await;
    outcome
}

/// Lit la première ligne envoyée par le serveur, sans son retour à la ligne.
async fn read_line(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buffer = Vec::with_capacity(128);
    let mut chunk = [0u8; 128];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.contains(&b'\n') || buffer.len() >= MAX_BANNER {
            break;
        }
    }
    if buffer.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "connection closed before any greeting",
        ));
    }
    let end = buffer.iter().position(|&b| b == b'\n').unwrap_or(buffer.len()).min(MAX_BANNER);
    Ok(String::from_utf8_lossy(&buffer[..end]).trim_end().to_string())
}

/// Juge la bannière. Un service qui répond mais refuse (`421`, `554`, `-ERR`,
/// `* BYE`) n'accepte pas de courrier : il est arrêté pour qui l'attend.
pub fn judge(greeting: Greeting, line: &str) -> Result<(), String> {
    let accepted = match greeting {
        Greeting::Smtp => line.starts_with("220"),
        Greeting::Pop3 => line.starts_with("+OK"),
        Greeting::Imap => line.starts_with("* OK") || line.starts_with("* PREAUTH"),
        Greeting::None => true,
    };
    if accepted {
        Ok(())
    } else {
        let shown: String = line.chars().take(80).collect();
        Err(format!("refused service: \"{shown}\""))
    }
}

/// Version annoncée dans une bannière : `… ESMTP MDaemon 25.0.1; …` ou
/// `… (MDaemon PRO v11.0.3) …`. `None` si la bannière ne la donne pas — c'est
/// réglable côté serveur, et beaucoup d'administrateurs la masquent.
pub fn banner_version(banner: &str, product: &str) -> Option<String> {
    let tokens: Vec<&str> = banner
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | ',' | '(' | ')' | '[' | ']'))
        .filter(|token| !token.is_empty())
        .collect();
    let at = tokens.iter().position(|token| token.eq_ignore_ascii_case(product))?;
    tokens[at + 1..].iter().take(2).find_map(|token| {
        let token = token.strip_prefix(['v', 'V']).unwrap_or(token);
        let looks_like_version = token.split('.').count() >= 2
            && token
                .split('.')
                .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
        looks_like_version.then(|| token.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_liste_par_defaut_de_mdaemon_vise_les_ports_documentes() {
        let specs = parse_services(None, MDAEMON_SERVICES, MDAEMON_DEFAULT).unwrap();
        let got: Vec<(&str, u16)> = specs.iter().map(|s| (s.name.as_str(), s.port)).collect();
        assert_eq!(got, [("smtp", 25), ("imap", 143), ("webmail", 3000)]);
        let sg = parse_services(None, SECURITY_GATEWAY_SERVICES, SECURITY_GATEWAY_DEFAULT).unwrap();
        let got: Vec<(&str, u16)> = sg.iter().map(|s| (s.name.as_str(), s.port)).collect();
        assert_eq!(got, [("smtp", 25), ("web", 4000)]);
    }

    #[test]
    fn un_port_se_remplace_et_un_service_libre_porte_le_sien() {
        let specs = parse_services(
            Some(" SMTP:2525, pop3 ,archive:8443,smtp:2525 "),
            MDAEMON_SERVICES,
            MDAEMON_DEFAULT,
        )
        .unwrap();
        assert_eq!(
            specs,
            [
                ServiceSpec { name: "smtp".into(), port: 2525, greeting: Greeting::Smtp },
                ServiceSpec { name: "pop3".into(), port: 110, greeting: Greeting::Pop3 },
                ServiceSpec { name: "archive".into(), port: 8443, greeting: Greeting::None },
            ]
        );
        assert!(
            parse_services(Some("none"), MDAEMON_SERVICES, MDAEMON_DEFAULT).unwrap().is_empty()
        );
    }

    #[test]
    fn une_liste_invalide_est_une_erreur_de_configuration() {
        for bad in ["gopher", "smtp:0", "smtp:abc", "sm tp", "a;b"] {
            assert!(
                matches!(
                    parse_services(Some(bad), MDAEMON_SERVICES, MDAEMON_DEFAULT),
                    Err(ProbeError::Config(_))
                ),
                "« {bad} » devrait être refusée"
            );
        }
        let many: Vec<String> = (1..=17).map(|i| format!("p{i}:{}", 1000 + i)).collect();
        assert!(parse_services(Some(&many.join(",")), MDAEMON_SERVICES, "").is_err());
    }

    #[test]
    fn les_bannieres_sont_jugees_selon_le_protocole() {
        assert!(judge(Greeting::Smtp, "220 mail.example.com ESMTP MDaemon 25.0.1; Mon").is_ok());
        assert!(judge(Greeting::Smtp, "220-mail.example.com ESMTP").is_ok());
        assert!(judge(Greeting::Smtp, "421 Service not available").is_err());
        assert!(judge(Greeting::Smtp, "554 No SMTP service here").is_err());
        assert!(judge(Greeting::Pop3, "+OK mail.example.com POP MDaemon ready").is_ok());
        assert!(judge(Greeting::Pop3, "-ERR too many connections").is_err());
        assert!(judge(Greeting::Imap, "* OK mail.example.com IMAP4rev1 MDaemon ready").is_ok());
        assert!(judge(Greeting::Imap, "* BYE server shutting down").is_err());
    }

    #[test]
    fn la_version_est_lue_dans_la_banniere_quand_elle_y_est() {
        assert_eq!(
            banner_version(
                "220 mail.example.com ESMTP MDaemon 25.0.1; Mon, 29 Sep 2026 10:00:00 +0200",
                "MDaemon"
            )
            .as_deref(),
            Some("25.0.1")
        );
        assert_eq!(
            banner_version("by mail.example.com (MDaemon PRO v11.0.3) with ESMTP", "MDaemon")
                .as_deref(),
            Some("11.0.3")
        );
        assert_eq!(banner_version("220 mail.example.com ESMTP ready", "MDaemon"), None);
        assert_eq!(banner_version("220 mail.example.com ESMTP MDaemon; ready", "MDaemon"), None);
    }

    #[tokio::test]
    async fn un_faux_serveur_smtp_est_juge_en_service_puis_arrete() {
        async fn serveur(greeting: &'static str) -> u16 {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    let _ = socket.write_all(greeting.as_bytes()).await;
                    let mut sink = [0u8; 64];
                    let _ = socket.read(&mut sink).await;
                }
            });
            port
        }
        let timeout = Duration::from_secs(2);
        let up = serveur("220 mx.test ESMTP MDaemon 26.0.4; ready\r\n").await;
        let spec = ServiceSpec { name: "smtp".into(), port: up, greeting: Greeting::Smtp };
        let checks = check_all("127.0.0.1", &[spec], timeout).await;
        assert!(checks[0].up, "{:?}", checks[0].reason);
        assert_eq!(checks[0].banner.as_deref(), Some("220 mx.test ESMTP MDaemon 26.0.4; ready"));

        let busy = serveur("421 mx.test too busy\r\n").await;
        let spec = ServiceSpec { name: "smtp".into(), port: busy, greeting: Greeting::Smtp };
        let checks = check_all("127.0.0.1", &[spec], timeout).await;
        assert!(!checks[0].up);
        assert!(checks[0].reason.as_deref().unwrap().contains("421"));

        // Un port où personne n'écoute : la connexion est refusée.
        let closed = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap().port()
        };
        let spec = ServiceSpec { name: "webmail".into(), port: closed, greeting: Greeting::None };
        let checks = check_all("127.0.0.1", &[spec], timeout).await;
        assert!(!checks[0].up);
        assert!(checks[0].seconds.is_none());
    }
}
