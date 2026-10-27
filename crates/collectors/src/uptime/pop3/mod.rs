//! Sonde POP3 (`kind = "pop3"`).
//!
//! Même question que pour IMAP — « le serveur laisse-t-il encore relever le
//! courrier ? » — pour les boîtes qui parlent POP3. La sonde lit la bannière
//! `+OK`, négocie `STLS` si demandé, s'identifie (`USER` / `PASS`) si la cible
//! porte des identifiants, puis envoie `QUIT`. Aucune commande de lecture n'est
//! émise : ni `STAT`, ni `RETR`, ni `DELE`.
//!
//! # Adresse et étiquettes
//!
//! Adresse : `pop.exemple.fr`, ou `pop.exemple.fr:1995`. Identifiants :
//! facultatifs (nom d'utilisateur et mot de passe).
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `security` | `tls` | `tls` (995), `starttls` (110, commande `STLS`) ou `none` (110, identifiants en clair). |
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

/// Collecteur de disponibilité d'un serveur POP3.
#[derive(Default)]
pub struct Pop3Collector;

impl Pop3Collector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for Pop3Collector {
    fn kind(&self) -> &'static str {
        "pop3"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = options::from_target(target)?;
        let mut report = Report::new(self.kind())
            .label("port", options.port.to_string())
            .label("security", options.security.as_str());

        converse(&mut report, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, port = options.port, detail,
                "sonde POP3 en échec");
        }
        Ok(report.finish())
    }
}

/// `Err` seulement quand le garde-fou refuse l'adresse.
async fn converse(report: &mut Report, options: &Options) -> Result<(), ProbeError> {
    let deadline = Deadline::starting_now(options.timeout);
    let Some(mut stream) = mailbox::open(report, options, deadline).await? else {
        return Ok(());
    };
    let mut reader = LineReader::new(MAX_LINE_BYTES);

    match greeting(report, &mut stream, &mut reader, options, deadline).await {
        Err(Aborted) => {
            stream.close().await;
            return Ok(());
        }
        Ok(false) => {}
        Ok(true) => {
            let plain = stream.into_plain().expect("la session est encore en clair");
            let Some(secured) = mailbox::upgrade(report, plain, options, deadline).await else {
                return Ok(());
            };
            stream = secured;
            reader = LineReader::new(MAX_LINE_BYTES);
        }
    }

    if login(report, &mut stream, &mut reader, options, deadline).await.is_ok() {
        // Sans attendre la réponse : le service a déjà tout dit.
        let _ = stream.write_line("QUIT", deadline).await;
    }
    stream.close().await;
    Ok(())
}

/// Bannière, puis `STLS` le cas échéant. Renvoie `true` quand la connexion doit
/// maintenant être chiffrée.
async fn greeting(
    report: &mut Report,
    stream: &mut Stream,
    reader: &mut LineReader,
    options: &Options,
    deadline: Deadline,
) -> Result<bool, Aborted> {
    let started = Instant::now();
    let line = mailbox::read_line(report, stream, reader, deadline).await?;
    report.gauge("pop3_greeting_seconds", started.elapsed().as_secs_f64());
    if !is_ok(&line) {
        report.fail(Failure::Protocol, format!("not a POP3 greeting: \"{}\"", truncated(&line)));
        return Err(Aborted);
    }
    if options.security != Security::StartTls {
        return Ok(false);
    }

    mailbox::send(report, stream, "STLS", deadline).await?;
    let answer = mailbox::read_line(report, stream, reader, deadline).await?;
    if !is_ok(&answer) {
        report.fail(Failure::Tls, format!("STLS refused: {}", truncated(&answer)));
        return Err(Aborted);
    }
    // Des octets déjà lus seraient du texte glissé avant la négociation.
    if !reader.take_buffered().is_empty() {
        report.fail(
            Failure::Protocol,
            "the server sent data before the TLS negotiation, which STLS forbids",
        );
        return Err(Aborted);
    }
    Ok(true)
}

/// `USER` puis `PASS`, si la cible porte des identifiants.
async fn login(
    report: &mut Report,
    stream: &mut Stream,
    reader: &mut LineReader,
    options: &Options,
    deadline: Deadline,
) -> Result<(), Aborted> {
    let Some((username, password)) = &options.login else { return Ok(()) };
    let started = Instant::now();

    mailbox::send(report, stream, &format!("USER {username}"), deadline).await?;
    let answer = mailbox::read_line(report, stream, reader, deadline).await?;
    if !is_ok(&answer) {
        return refused(report, &answer);
    }
    mailbox::send(report, stream, &format!("PASS {password}"), deadline).await?;
    let answer = mailbox::read_line(report, stream, reader, deadline).await?;
    report.gauge("pop3_login_seconds", started.elapsed().as_secs_f64());
    if !is_ok(&answer) {
        return refused(report, &answer);
    }
    report.gauge("pop3_authenticated", 1.0);
    Ok(())
}

/// Un `-ERR` à l'identification est un compte refusé, sauf quand le serveur dit
/// que la boîte est verrouillée ou le système en difficulté (codes de la RFC
/// 3206) : c'est alors le service, pas le compte.
fn refused(report: &mut Report, answer: &str) -> Result<(), Aborted> {
    let upper = answer.to_ascii_uppercase();
    let service_side =
        ["[IN-USE]", "[SYS/TEMP]", "[SYS/PERM]"].iter().any(|code| upper.contains(code));
    let reason = if service_side { Failure::Protocol } else { Failure::Auth };
    // Détail journalisé, jamais mis en étiquette : il peut citer l'identifiant.
    report.fail(reason, format!("login refused: {}", truncated(answer)));
    Err(Aborted)
}

/// Réponse positive : `+OK`, suivi ou non d'un commentaire.
fn is_ok(line: &str) -> bool {
    line.strip_prefix("+OK").is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
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

    /// Faux serveur POP3. `script` associe une commande (`USER`, `PASS`, `STLS`…)
    /// à la ligne de réponse. Rend le port et les lignes reçues.
    async fn faux_pop3(
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
                let command = line.split(' ').next().unwrap_or("").to_ascii_uppercase();
                let answer = script
                    .iter()
                    .find(|(name, _)| *name == command)
                    .map(|(_, answer)| *answer)
                    .unwrap_or("-ERR unknown command");
                if write.write_all(format!("{answer}\r\n").as_bytes()).await.is_err()
                    || command == "QUIT"
                {
                    break;
                }
            }
        });
        (port, received)
    }

    fn cible_locale(port: u16, tags: &[(&str, &str)]) -> Target {
        let mut all =
            vec![("timeout_seconds", "3"), ("allow_private_targets", "true"), ("security", "none")];
        all.extend_from_slice(tags);
        cible("pop3", &format!("127.0.0.1:{port}"), &all)
    }

    fn avec_identifiants(mut target: Target) -> Target {
        target.credential =
            Credential::UsernamePassword { username: "monit".into(), password: "secret".into() };
        target
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(Pop3Collector::new().kind(), "pop3");
    }

    #[test]
    fn seule_une_reponse_positive_est_un_ok() {
        assert!(is_ok("+OK"));
        assert!(is_ok("+OK Dovecot ready."));
        assert!(!is_ok("+OKAY"));
        assert!(!is_ok("-ERR no"));
        assert!(!is_ok(""));
    }

    #[tokio::test]
    async fn la_boucle_locale_est_refusee_sans_loption() {
        let target = cible("pop3", "127.0.0.1:110", &[("timeout_seconds", "2")]);
        let error = Pop3Collector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
        assert!(error.to_string().contains("allow_private_targets"), "{error}");
    }

    #[tokio::test]
    async fn une_banniere_seule_suffit_sans_identifiants() {
        let (port, received) = faux_pop3("+OK ready", vec![]).await;
        let samples = Pop3Collector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0), "{:?}", reason(&samples));
        assert!(metric(&samples, "probe_pop3_greeting_seconds").is_some());
        assert!(received.lock().unwrap().iter().all(|line| !line.starts_with("USER")));
    }

    #[tokio::test]
    async fn une_connexion_reussie_est_authentifiee() {
        let (port, received) =
            faux_pop3("+OK ready", vec![("USER", "+OK"), ("PASS", "+OK logged in")]).await;
        let target = avec_identifiants(cible_locale(port, &[]));
        let samples = Pop3Collector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0), "{:?}", reason(&samples));
        assert_eq!(metric(&samples, "probe_pop3_authenticated"), Some(1.0));
        assert!(metric(&samples, "probe_pop3_login_seconds").is_some());
        let lines = received.lock().unwrap();
        assert_eq!(&lines[..2], ["USER monit", "PASS secret"]);
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_se_distingue_dune_panne() {
        let (port, _) = faux_pop3(
            "+OK ready",
            vec![("USER", "+OK"), ("PASS", "-ERR [AUTH] Authentication failed.")],
        )
        .await;
        let target = avec_identifiants(cible_locale(port, &[]));
        let samples = Pop3Collector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("auth"));
    }

    #[tokio::test]
    async fn une_boite_verrouillee_nest_pas_un_compte_refuse() {
        let (port, _) =
            faux_pop3("+OK ready", vec![("USER", "+OK"), ("PASS", "-ERR [IN-USE] mailbox locked")])
                .await;
        let target = avec_identifiants(cible_locale(port, &[]));
        let samples = Pop3Collector::new().probe(&target).await.unwrap();
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn un_utilisateur_inconnu_est_un_compte_refuse() {
        let (port, _) = faux_pop3("+OK ready", vec![("USER", "-ERR no such user")]).await;
        let target = avec_identifiants(cible_locale(port, &[]));
        let samples = Pop3Collector::new().probe(&target).await.unwrap();
        assert_eq!(reason(&samples).as_deref(), Some("auth"));
    }

    /// Le point de sécurité : si `STLS` est refusé, le mot de passe ne part pas.
    #[tokio::test]
    async fn stls_refuse_ninterrompt_la_sonde_avant_tout_identifiant() {
        let (port, received) = faux_pop3(
            "+OK ready",
            vec![("STLS", "-ERR TLS not available"), ("USER", "+OK"), ("PASS", "+OK")],
        )
        .await;
        let target = avec_identifiants(cible_locale(port, &[("security", "starttls")]));
        let samples = Pop3Collector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("tls"));
        let lines = received.lock().unwrap();
        assert!(lines.iter().all(|line| !line.starts_with("USER") && !line.starts_with("PASS")));
    }

    #[tokio::test]
    async fn un_service_qui_ne_parle_pas_pop3_est_signale() {
        let (port, _) = faux_pop3("220 mail ESMTP", vec![]).await;
        let samples = Pop3Collector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn un_serveur_injoignable_produit_un_echantillon_a_zero() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let samples = Pop3Collector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("connect"));
    }
}
