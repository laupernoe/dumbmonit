//! Sonde IMAP (`kind = "imap"`).
//!
//! « Puis-je encore relever mon courrier ? » : un serveur IMAP peut ouvrir son
//! port, présenter un beau certificat et refuser tout le monde depuis que la
//! base des comptes est indisponible. La sonde mène le début d'une vraie session :
//! bannière `* OK`, `STARTTLS` si demandé, `LOGIN` si la cible porte des
//! identifiants, puis `LOGOUT`. Elle ne sélectionne aucune boîte et ne lit aucun
//! message.
//!
//! # Adresse et étiquettes
//!
//! Adresse : `imap.exemple.fr`, ou `imap.exemple.fr:1993`. Identifiants :
//! facultatifs (nom d'utilisateur et mot de passe).
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `security` | `tls` | `tls` (993), `starttls` (143) ou `none` (143, identifiants en clair). |
//! | `port` | selon `security` | Port, si l'adresse n'en précise pas. |
//! | `server_name` | l'hôte | Nom envoyé en SNI. |
//! | `insecure_tls` | `false` | Une chaîne non vérifiable ne fait plus échouer. |
//! | `allow_private_targets` | `false` | Autorise la boucle locale (voir `guard`). |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |

pub(crate) mod options;

use std::time::Instant;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use tracing::debug;

use super::mailbox::{self, Aborted, MAX_LINE_BYTES, Security};
use super::outcome::{Failure, Report};
use super::session::{Deadline, LineReader, Stream};
use options::Options;

/// Étiquettes des commandes. Une seule est en vol à la fois : des tags fixes suffisent.
const TAG_STARTTLS: &str = "a001";
const TAG_LOGIN: &str = "a002";
const TAG_LOGOUT: &str = "a003";

/// Réponses non étiquetées tolérées avant la réponse étiquetée.
const MAX_UNTAGGED_LINES: usize = 128;

/// Collecteur de disponibilité d'un serveur IMAP.
#[derive(Default)]
pub struct ImapCollector;

impl ImapCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for ImapCollector {
    fn kind(&self) -> &'static str {
        "imap"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = options::from_target(target)?;
        let mut report = Report::new(self.kind())
            .label("port", options.port.to_string())
            .label("security", options.security.as_str());

        converse(&mut report, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, port = options.port, detail,
                "sonde IMAP en échec");
        }
        Ok(report.finish())
    }
}

/// Où en est la session après la bannière.
enum Opening {
    /// Rien d'autre à négocier. `true` si le serveur a répondu `PREAUTH`.
    Ready { preauthenticated: bool },
    /// Le serveur a accepté `STARTTLS` : la suite se passe chiffrée.
    Upgrade,
}

/// `Err` seulement quand le garde-fou refuse l'adresse.
async fn converse(report: &mut Report, options: &Options) -> Result<(), ProbeError> {
    let deadline = Deadline::starting_now(options.timeout);
    let Some(mut stream) = mailbox::open(report, options, deadline).await? else {
        return Ok(());
    };
    let mut reader = LineReader::new(MAX_LINE_BYTES);

    let preauthenticated = match opening(report, &mut stream, &mut reader, options, deadline).await
    {
        Err(Aborted) => {
            stream.close().await;
            return Ok(());
        }
        Ok(Opening::Ready { preauthenticated }) => preauthenticated,
        Ok(Opening::Upgrade) => {
            let plain = stream.into_plain().expect("la session est encore en clair");
            let Some(secured) = mailbox::upgrade(report, plain, options, deadline).await else {
                return Ok(());
            };
            stream = secured;
            reader = LineReader::new(MAX_LINE_BYTES);
            false
        }
    };

    if login(report, &mut stream, &mut reader, options, preauthenticated, deadline).await.is_ok()
    {
        // Sans attendre la réponse : le service a déjà tout dit.
        let _ = stream.write_line(&format!("{TAG_LOGOUT} LOGOUT"), deadline).await;
    }
    stream.close().await;
    Ok(())
}

/// Bannière, puis `STARTTLS` le cas échéant.
async fn opening(
    report: &mut Report,
    stream: &mut Stream,
    reader: &mut LineReader,
    options: &Options,
    deadline: Deadline,
) -> Result<Opening, Aborted> {
    let started = Instant::now();
    let greeting = mailbox::read_line(report, stream, reader, deadline).await?;
    report.gauge("imap_greeting_seconds", started.elapsed().as_secs_f64());

    let upper = greeting.to_ascii_uppercase();
    let preauthenticated = upper.starts_with("* PREAUTH");
    if !preauthenticated && !upper.starts_with("* OK") {
        report.fail(
            Failure::Protocol,
            format!("not an IMAP greeting: \"{}\"", truncated(&greeting)),
        );
        return Err(Aborted);
    }
    if options.security != Security::StartTls {
        return Ok(Opening::Ready { preauthenticated });
    }

    mailbox::send(report, stream, &format!("{TAG_STARTTLS} STARTTLS"), deadline).await?;
    let (status, text) = tagged_status(report, stream, reader, TAG_STARTTLS, deadline).await?;
    if status != "OK" {
        report.fail(Failure::Tls, format!("STARTTLS refused: {status} {}", truncated(&text)));
        return Err(Aborted);
    }
    // Des octets déjà lus seraient du texte glissé avant la négociation, ce
    // qu'aucun serveur honnête ne fait.
    if !reader.take_buffered().is_empty() {
        report.fail(
            Failure::Protocol,
            "the server sent data before the TLS negotiation, which STARTTLS forbids",
        );
        return Err(Aborted);
    }
    Ok(Opening::Upgrade)
}

/// `LOGIN`, si la cible porte des identifiants.
async fn login(
    report: &mut Report,
    stream: &mut Stream,
    reader: &mut LineReader,
    options: &Options,
    preauthenticated: bool,
    deadline: Deadline,
) -> Result<(), Aborted> {
    let Some((username, password)) = &options.login else { return Ok(()) };
    if preauthenticated {
        report.gauge("imap_authenticated", 1.0);
        return Ok(());
    }

    let started = Instant::now();
    let command = format!("{TAG_LOGIN} LOGIN {} {}", quote(username), quote(password));
    mailbox::send(report, stream, &command, deadline).await?;
    let (status, text) = tagged_status(report, stream, reader, TAG_LOGIN, deadline).await?;
    report.gauge("imap_login_seconds", started.elapsed().as_secs_f64());
    if status == "OK" {
        report.gauge("imap_authenticated", 1.0);
        return Ok(());
    }

    // `NO` : compte ou mot de passe refusé. `[UNAVAILABLE]` : le serveur ne
    // sait pas répondre pour l'instant, ce qui n'est pas la faute du compte.
    let service_side = text.to_ascii_uppercase().contains("[UNAVAILABLE]");
    let reason = if status == "NO" && !service_side { Failure::Auth } else { Failure::Protocol };
    // Détail journalisé, jamais mis en étiquette : il peut citer l'identifiant.
    report.fail(reason, format!("login refused: {status} {}", truncated(&text)));
    Err(Aborted)
}

/// Lit jusqu'à la réponse étiquetée `tag`, et rend son statut (`OK`, `NO`,
/// `BAD`) et le texte qui suit.
async fn tagged_status(
    report: &mut Report,
    stream: &mut Stream,
    reader: &mut LineReader,
    tag: &str,
    deadline: Deadline,
) -> Result<(String, String), Aborted> {
    let prefix = format!("{tag} ");
    for _ in 0..MAX_UNTAGGED_LINES {
        let line = mailbox::read_line(report, stream, reader, deadline).await?;
        if line.starts_with("* BYE") {
            report.fail(Failure::Protocol, format!("the server hung up: {}", truncated(&line)));
            return Err(Aborted);
        }
        let Some(rest) = line.strip_prefix(&prefix) else { continue };
        let (status, text) = rest.split_once(' ').unwrap_or((rest, ""));
        return Ok((status.to_ascii_uppercase(), text.to_string()));
    }
    report.fail(Failure::Protocol, "the server's reply never ends");
    Err(Aborted)
}

/// Chaîne entre guillemets au sens d'IMAP (RFC 3501 §4.3) : seuls `"` et `\`
/// s'échappent. Les sauts de ligne sont exclus en amont, à la lecture des options.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Texte de serveur raccourci pour le journal.
fn truncated(text: &str) -> String {
    text.chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dumbmonit_proto::Credential;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn metric(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn reason(samples: &[Sample]) -> Option<String> {
        samples
            .iter()
            .find(|s| s.metric == "probe_failure_info")
            .and_then(|s| s.labels.get("reason").cloned())
    }

    /// Faux serveur IMAP. `script` associe une commande à la réponse envoyée, où
    /// `{tag}` est remplacé par l'étiquette du client. Rend le port et les lignes
    /// reçues.
    async fn faux_imap(
        greeting: &'static str,
        script: Vec<(&'static str, &'static str)>,
    ) -> (u16, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let received = Arc::new(Mutex::new(Vec::new()));
        let log = received.clone();
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            write.write_all(format!("{greeting}\r\n").as_bytes()).await.unwrap();
            let mut lines = BufReader::new(read).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log.lock().unwrap().push(line.clone());
                let mut words = line.splitn(3, ' ');
                let tag = words.next().unwrap_or("");
                let command = words.next().unwrap_or("").to_ascii_uppercase();
                let answer = script
                    .iter()
                    .find(|(name, _)| *name == command)
                    .map(|(_, answer)| answer.replace("{tag}", tag))
                    .unwrap_or_else(|| format!("{tag} BAD unknown command\r\n"));
                if write.write_all(answer.as_bytes()).await.is_err() || command == "LOGOUT" {
                    break;
                }
            }
        });
        (port, received)
    }

    fn cible_locale(port: u16, tags: &[(&str, &str)]) -> Target {
        let mut all = vec![
            ("timeout_seconds", "3"),
            ("allow_private_targets", "true"),
            ("security", "none"),
        ];
        all.extend_from_slice(tags);
        cible("imap", &format!("127.0.0.1:{port}"), &all)
    }

    fn avec_identifiants(mut target: Target, password: &str) -> Target {
        target.credential = Credential::UsernamePassword {
            username: "monit".into(),
            password: password.into(),
        };
        target
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(ImapCollector::new().kind(), "imap");
    }

    #[test]
    fn les_chaines_sont_citees_selon_imap() {
        assert_eq!(quote("simple"), "\"simple\"");
        assert_eq!(quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[tokio::test]
    async fn la_boucle_locale_est_refusee_sans_loption() {
        let target = cible("imap", "127.0.0.1:143", &[("timeout_seconds", "2")]);
        let error = ImapCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
        assert!(error.to_string().contains("allow_private_targets"), "{error}");
    }

    #[tokio::test]
    async fn une_banniere_seule_suffit_sans_identifiants() {
        let (port, received) = faux_imap("* OK IMAP4rev1 ready", vec![]).await;
        let samples = ImapCollector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0), "{:?}", reason(&samples));
        assert!(metric(&samples, "probe_imap_greeting_seconds").is_some());
        assert!(metric(&samples, "probe_imap_authenticated").is_none());
        assert!(received.lock().unwrap().iter().all(|line| !line.contains("LOGIN")));
    }

    #[tokio::test]
    async fn une_connexion_reussie_est_authentifiee_et_citee_correctement() {
        let (port, received) =
            faux_imap("* OK ready", vec![("LOGIN", "{tag} OK LOGIN completed\r\n")]).await;
        let target = avec_identifiants(cible_locale(port, &[]), "pa\"ss\\w");
        let samples = ImapCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0), "{:?}", reason(&samples));
        assert_eq!(metric(&samples, "probe_imap_authenticated"), Some(1.0));
        assert!(metric(&samples, "probe_imap_login_seconds").is_some());
        let lines = received.lock().unwrap();
        assert!(lines.contains(&"a002 LOGIN \"monit\" \"pa\\\"ss\\\\w\"".to_string()), "{lines:?}");
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_se_distingue_dune_panne() {
        let (port, _) = faux_imap(
            "* OK ready",
            vec![("LOGIN", "{tag} NO [AUTHENTICATIONFAILED] Authentication failed.\r\n")],
        )
        .await;
        let target = avec_identifiants(cible_locale(port, &[]), "faux");
        let samples = ImapCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("auth"));
    }

    #[tokio::test]
    async fn un_serveur_indisponible_nest_pas_une_erreur_de_compte() {
        let (port, _) = faux_imap(
            "* OK ready",
            vec![("LOGIN", "* OK noise\r\n{tag} NO [UNAVAILABLE] backend down\r\n")],
        )
        .await;
        let target = avec_identifiants(cible_locale(port, &[]), "x");
        let samples = ImapCollector::new().probe(&target).await.unwrap();
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn preauth_dispense_de_login() {
        let (port, received) = faux_imap("* PREAUTH ready", vec![]).await;
        let target = avec_identifiants(cible_locale(port, &[]), "x");
        let samples = ImapCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        assert_eq!(metric(&samples, "probe_imap_authenticated"), Some(1.0));
        assert!(received.lock().unwrap().iter().all(|line| !line.contains("LOGIN")));
    }

    /// Le point de sécurité : si `STARTTLS` est refusé, le mot de passe ne part pas.
    #[tokio::test]
    async fn starttls_refuse_ninterrompt_la_sonde_avant_tout_identifiant() {
        let (port, received) = faux_imap(
            "* OK ready",
            vec![
                ("STARTTLS", "{tag} NO TLS not available\r\n"),
                ("LOGIN", "{tag} OK logged in\r\n"),
            ],
        )
        .await;
        let target = avec_identifiants(cible_locale(port, &[("security", "starttls")]), "secret");
        let samples = ImapCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("tls"));
        assert!(received.lock().unwrap().iter().all(|line| !line.contains("LOGIN")));
    }

    #[tokio::test]
    async fn un_service_qui_ne_parle_pas_imap_est_signale() {
        let (port, _) = faux_imap("SSH-2.0-OpenSSH_9.6", vec![]).await;
        let samples = ImapCollector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn un_serveur_injoignable_produit_un_echantillon_a_zero() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let samples = ImapCollector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("connect"));
    }
}
