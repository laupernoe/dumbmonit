//! Sonde gRPC (`kind = "grpc"`) : interroge le protocole de santé standard
//! `grpc.health.v1.Health/Check`, que presque tous les serveurs gRPC et
//! les équilibreurs savent servir.
//!
//! Un port TCP ouvert ne dit pas qu'un service gRPC répond, et une sonde HTTP
//! échoue sur HTTP/2 sans TLS (« h2c »). La sonde parle donc HTTP/2 pour de bon :
//! en TLS avec ALPN `h2`, ou en clair (« prior knowledge ») quand `tls` n'est pas
//! coché, puis envoie un `HealthCheckRequest` et lit la réponse.
//!
//! # Verdict
//!
//! * `SERVING` : succès ;
//! * `NOT_SERVING`, `UNKNOWN`, `SERVICE_UNKNOWN` : le serveur répond mais dit que
//!   le service ne travaille pas (`unhealthy`) ;
//! * `grpc-status` différent de 0, dans les trailers ou dans les en-têtes d'une
//!   réponse « Trailers-Only » : échec, le code et le `grpc-message` figurent dans
//!   le détail. `UNIMPLEMENTED` (12) veut dire que le serveur n'expose pas le
//!   service de santé ; `NOT_FOUND` (5), qu'il ne connaît pas le nom donné dans
//!   `service` (`protocol`). `UNAVAILABLE` (14) est `unhealthy`, `UNAUTHENTICATED`
//!   et `PERMISSION_DENIED` sont `auth`, `DEADLINE_EXCEEDED` est `timeout` ;
//! * un statut HTTP autre que 200 (`status`), ou une réponse qui n'est pas du gRPC
//!   (`protocol`) : on a affaire à un proxy ou à un autre service.
//!
//! Le protobuf, minuscule, est écrit à la main dans [`wire`].
//!
//! # Adresse et étiquettes
//!
//! Adresse : `grpc.exemple.fr:50051`. Pas d'identifiant : le protocole de santé
//! n'en demande pas.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `service` | — | Service dont on demande la santé ; vide : le serveur entier. |
//! | `tls` | `false` | Connexion chiffrée (ALPN `h2`). Sinon HTTP/2 en clair. |
//! | `port` | `50051` / `443` | Port, si l'adresse n'en précise pas ; 443 avec `tls`. |
//! | `server_name` | l'hôte | Nom envoyé en SNI et en `:authority`. |
//! | `insecure_tls` | `false` | Une chaîne non vérifiable ne fait plus échouer. |
//! | `allow_private_targets` | `false` | Autorise la boucle locale (voir `guard`). |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |

pub(crate) mod options;
mod wire;

use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use h2::client::SendRequest;
use http::{HeaderMap, Method, Request};
use tracing::debug;

use super::outcome::{Failure, Report};
use super::session::{self, Deadline, SessionError, Stream};
use options::Options;
use wire::Serving;

/// Méthode du protocole de santé standard.
const CHECK_PATH: &str = "/grpc.health.v1.Health/Check";

/// En-tête `user-agent` envoyé au serveur.
const USER_AGENT: &str = "dumbmonit-grpc-probe";

/// Taille maximale du corps lu : une réponse de santé tient en quelques octets.
const MAX_BODY: usize = 64 * 1024;

/// Échec d'une étape : la raison exposée en métrique et son détail.
type Failed = (Failure, String);

/// Collecteur de disponibilité par protocole de santé gRPC.
#[derive(Default)]
pub struct GrpcCollector;

impl GrpcCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for GrpcCollector {
    fn kind(&self) -> &'static str {
        "grpc"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let mut report = Report::new(self.kind()).label("port", options.port.to_string());
        if !options.service.is_empty() {
            report = report.label("service", options.service.clone());
        }

        check(&mut report, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, port = options.port, detail,
                "sonde gRPC en échec");
        }
        Ok(report.finish())
    }
}

/// Ce que le serveur a répondu, une fois le flux HTTP/2 refermé.
struct Reply {
    /// Durée de l'appel, de la demande de flux à la fin des trailers.
    elapsed: Duration,
    http_status: u16,
    content_type: Option<String>,
    grpc_status: Option<u32>,
    grpc_message: Option<String>,
    body: Vec<u8>,
}

/// Ouvre la connexion, la chiffre au besoin, puis appelle `Health/Check`.
///
/// `Err` seulement quand le garde-fou refuse l'adresse : une erreur de
/// configuration, pas une mesure.
async fn check(report: &mut Report, options: &Options) -> Result<(), ProbeError> {
    let deadline = Deadline::starting_now(options.timeout);

    let connected = match session::connect(
        &options.host,
        options.port,
        options.allow_private,
        deadline,
    )
    .await
    {
        Ok(connected) => connected,
        Err(SessionError::Refused(error)) => return Err(error),
        Err(error) => {
            let (reason, detail) = error.failure();
            report.fail(reason, detail);
            return Ok(());
        }
    };
    report.gauge("connect_seconds", connected.connect.as_secs_f64());

    let stream = if options.tls {
        let secured = match session::upgrade_with_alpn(
            connected.stream,
            &options.server_name,
            &[b"h2"],
            deadline,
        )
        .await
        {
            Ok(secured) => secured,
            Err(error) => {
                let (reason, detail) = error.failure();
                report.fail(reason, detail);
                return Ok(());
            }
        };
        if !session::record_tls(report, &secured, options.allow_untrusted) {
            return Ok(());
        }
        if secured.alpn.as_deref() != Some(&b"h2"[..]) {
            report.fail(
                Failure::Protocol,
                "the server did not negotiate HTTP/2 (ALPN \"h2\"): this port does not look \
                 like a gRPC server",
            );
            return Ok(());
        }
        Stream::Tls(Box::new(secured.stream))
    } else {
        Stream::Plain(connected.stream)
    };

    match call(stream, options, deadline).await {
        Ok(reply) => judge(report, &reply, &options.service),
        Err((reason, detail)) => report.fail(reason, detail),
    }
    Ok(())
}

/// Poignée de main HTTP/2, puis l'appel. La connexion progresse dans sa propre
/// tâche — `h2` l'exige — arrêtée dès que l'appel est terminé.
async fn call(stream: Stream, options: &Options, deadline: Deadline) -> Result<Reply, Failed> {
    let (client, connection) = match deadline.wait(h2::client::handshake(stream)).await {
        Ok(Ok(pair)) => pair,
        Ok(Err(error)) => return Err(h2_failure(&error)),
        Err(error) => return Err(error.failure()),
    };
    let _driver = Driver(tokio::spawn(async move {
        let _ = connection.await;
    }));
    exchange(client, options, deadline).await
}

/// Tâche qui fait progresser la connexion HTTP/2. Elle est arrêtée avec l'appel,
/// y compris quand celui-ci est interrompu de l'extérieur : une tâche détachée
/// survivrait à la sonde et garderait la connexion ouverte.
struct Driver(tokio::task::JoinHandle<()>);

impl Drop for Driver {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn exchange(
    client: SendRequest<Bytes>,
    options: &Options,
    deadline: Deadline,
) -> Result<Reply, Failed> {
    let started = Instant::now();
    let mut client = deadline
        .wait(client.ready())
        .await
        .map_err(|error| error.failure())?
        .map_err(|error| h2_failure(&error))?;

    let request = build_request(options)?;
    let (response, mut upload) =
        client.send_request(request, false).map_err(|error| h2_failure(&error))?;
    let message = wire::frame(&wire::encode_request(&options.service));
    upload.send_data(Bytes::from(message), true).map_err(|error| h2_failure(&error))?;

    let response = deadline
        .wait(response)
        .await
        .map_err(|error| error.failure())?
        .map_err(|error| h2_failure(&error))?;
    let (head, mut stream) = response.into_parts();

    let mut body = Vec::new();
    while let Some(chunk) = deadline.wait(stream.data()).await.map_err(|error| error.failure())? {
        let chunk = chunk.map_err(|error| h2_failure(&error))?;
        // Rend la place dans la fenêtre de réception, faute de quoi le serveur
        // cesserait d'envoyer.
        let _ = stream.flow_control().release_capacity(chunk.len());
        if body.len() + chunk.len() > MAX_BODY {
            return Err((Failure::Protocol, "the reply is far larger than a health reply".into()));
        }
        body.extend_from_slice(&chunk);
    }
    let trailers = deadline
        .wait(stream.trailers())
        .await
        .map_err(|error| error.failure())?
        .map_err(|error| h2_failure(&error))?;

    // Une réponse « Trailers-Only » porte `grpc-status` dans ses en-têtes.
    let source = trailers
        .as_ref()
        .filter(|trailers| read_status(trailers).is_some())
        .unwrap_or(&head.headers);
    Ok(Reply {
        elapsed: started.elapsed(),
        http_status: head.status.as_u16(),
        content_type: head
            .headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
        grpc_status: read_status(source),
        grpc_message: read_message(source),
        body,
    })
}

fn build_request(options: &Options) -> Result<Request<()>, Failed> {
    let scheme = if options.tls { "https" } else { "http" };
    // Une adresse IPv6 s'écrit entre crochets dans une autorité.
    let host = if options.server_name.contains(':') && !options.server_name.starts_with('[') {
        format!("[{}]", options.server_name)
    } else {
        options.server_name.clone()
    };
    Request::builder()
        .method(Method::POST)
        .uri(format!("{scheme}://{host}:{}{CHECK_PATH}", options.port))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .header("grpc-accept-encoding", "identity")
        .header("user-agent", USER_AGENT)
        .body(())
        .map_err(|error| (Failure::Protocol, format!("cannot build the request: {error}")))
}

/// Traduit une erreur HTTP/2 en raison d'échec.
fn h2_failure(error: &h2::Error) -> Failed {
    if error.is_io() {
        (Failure::Connect, format!("connection lost: {error}"))
    } else {
        (Failure::Protocol, format!("HTTP/2 error: {error}"))
    }
}

fn read_status(headers: &HeaderMap) -> Option<u32> {
    headers.get("grpc-status")?.to_str().ok()?.trim().parse().ok()
}

fn read_message(headers: &HeaderMap) -> Option<String> {
    let text = wire::decode_message(headers.get("grpc-message")?.to_str().ok()?);
    let text = text.trim();
    if text.is_empty() { None } else { Some(text.to_string()) }
}

/// Publie les métriques et rend le verdict.
fn judge(report: &mut Report, reply: &Reply, service: &str) {
    report.gauge("grpc_check_seconds", reply.elapsed.as_secs_f64());
    report.gauge("http_status_code", f64::from(reply.http_status));

    if reply.http_status != 200 {
        report.fail(
            Failure::Status,
            format!(
                "HTTP {}: the endpoint answered something other than gRPC (a proxy, or another \
                 service)",
                reply.http_status
            ),
        );
        return;
    }
    let is_grpc =
        reply.content_type.as_deref().is_some_and(|value| value.starts_with("application/grpc"));
    if !is_grpc && reply.grpc_status.is_none() {
        report.fail(
            Failure::Protocol,
            format!(
                "not a gRPC reply (content-type: {})",
                reply.content_type.as_deref().unwrap_or("none")
            ),
        );
        return;
    }
    let Some(code) = reply.grpc_status else {
        report.fail(Failure::Protocol, "the reply ended without a grpc-status");
        return;
    };
    report.gauge("grpc_status_code", f64::from(code));
    if code != 0 {
        let (reason, detail) = status_failure(code, reply.grpc_message.as_deref());
        report.fail(reason, detail);
        return;
    }

    let message = match wire::unframe(&reply.body) {
        Ok(Some(message)) => message,
        Ok(None) => {
            report.fail(Failure::Protocol, "the server answered OK without a health message");
            return;
        }
        Err(detail) => {
            report.fail(Failure::Protocol, detail);
            return;
        }
    };
    match wire::decode_response(message) {
        Ok(status) => {
            report.gauge("grpc_serving_status", status.value());
            if status != Serving::Serving {
                let subject = if service.is_empty() {
                    "the server".to_string()
                } else {
                    format!("service \"{service}\"")
                };
                report.fail(Failure::Unhealthy, format!("{subject} reports {}", status.name()));
            }
        }
        Err(detail) => report.fail(Failure::Protocol, format!("unreadable health reply: {detail}")),
    }
}

/// Raison et détail d'un `grpc-status` différent de 0.
fn status_failure(code: u32, message: Option<&str>) -> Failed {
    let reason = match code {
        4 => Failure::Timeout,
        7 | 16 => Failure::Auth,
        14 => Failure::Unhealthy,
        _ => Failure::Protocol,
    };
    let hint = match code {
        12 => {
            ": the server does not expose the standard health service (grpc.health.v1.Health): \
             register it, or monitor this port with a TCP check"
        }
        5 => ": the server does not know the service named in the options",
        _ => "",
    };
    let message = message.map(|text| format!(" ({text})")).unwrap_or_default();
    (reason, format!("grpc-status {code} {}{message}{hint}", wire::status_name(code)))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use http::{HeaderValue, Response};
    use tokio::net::TcpListener;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(GrpcCollector::new().kind(), "grpc");
    }

    // --- Verdict, sans réseau ---

    fn reply(code: Option<u32>, message: Option<&str>, body: Vec<u8>) -> Reply {
        Reply {
            elapsed: Duration::from_millis(3),
            http_status: 200,
            content_type: Some("application/grpc".to_string()),
            grpc_status: code,
            grpc_message: message.map(str::to_string),
            body,
        }
    }

    fn verdict(reply: &Reply, service: &str) -> Report {
        let mut report = Report::new("grpc");
        judge(&mut report, reply, service);
        report
    }

    fn reason_of(report: Report) -> String {
        let samples = report.finish();
        let info = samples.iter().find(|s| s.metric == "probe_failure_info");
        info.and_then(|s| s.labels.get("reason").cloned()).unwrap_or_default()
    }

    #[test]
    fn un_service_en_marche_reussit() {
        let report = verdict(&reply(Some(0), None, wire::frame(&[0x08, 1])), "");
        assert!(report.is_up());
    }

    #[test]
    fn un_service_qui_ne_travaille_pas_est_unhealthy_avec_son_etat() {
        for (status, name) in [(0u8, "UNKNOWN"), (2, "NOT_SERVING"), (3, "SERVICE_UNKNOWN")] {
            let report = verdict(&reply(Some(0), None, wire::frame(&[0x08, status])), "db");
            assert!(!report.is_up());
            let detail = report.detail().unwrap().to_string();
            assert!(detail.contains(name) && detail.contains("\"db\""), "{detail}");
            assert_eq!(reason_of(report), "unhealthy");
        }
    }

    #[test]
    fn un_message_vide_vaut_unknown() {
        let report = verdict(&reply(Some(0), None, wire::frame(&[])), "");
        assert!(report.detail().unwrap().contains("UNKNOWN"));
    }

    #[test]
    fn unimplemented_dit_que_le_service_de_sante_manque() {
        let report = verdict(&reply(Some(12), Some("unknown service"), vec![]), "");
        let detail = report.detail().unwrap().to_string();
        assert!(detail.contains("UNIMPLEMENTED") && detail.contains("health service"), "{detail}");
        assert!(detail.contains("unknown service"), "{detail}");
        assert_eq!(reason_of(report), "protocol");
    }

    #[test]
    fn chaque_code_grpc_a_sa_raison() {
        let cas = [
            (4, "timeout"),
            (5, "protocol"),
            (7, "auth"),
            (14, "unhealthy"),
            (16, "auth"),
            (13, "protocol"),
        ];
        for (code, attendu) in cas {
            assert_eq!(reason_of(verdict(&reply(Some(code), None, vec![]), "")), attendu, "{code}");
        }
    }

    #[test]
    fn une_reponse_qui_nest_pas_du_grpc_est_distinguee() {
        let mut http = reply(None, None, vec![]);
        http.http_status = 503;
        assert_eq!(reason_of(verdict(&http, "")), "status");

        let mut html = reply(None, None, b"<html>".to_vec());
        html.content_type = Some("text/html".to_string());
        assert_eq!(reason_of(verdict(&html, "")), "protocol");

        // Application/grpc mais sans statut : flux interrompu.
        let report = verdict(&reply(None, None, vec![]), "");
        assert!(report.detail().unwrap().contains("grpc-status"));
        assert_eq!(reason_of(report), "protocol");
    }

    #[test]
    fn un_corps_mal_forme_est_un_echec_de_protocole() {
        assert_eq!(reason_of(verdict(&reply(Some(0), None, vec![]), "")), "protocol");
        assert_eq!(reason_of(verdict(&reply(Some(0), None, vec![0, 0, 0]), "")), "protocol");
        let bad_protobuf = wire::frame(&[0x08]);
        assert_eq!(reason_of(verdict(&reply(Some(0), None, bad_protobuf), "")), "protocol");
    }

    #[test]
    fn les_metriques_du_verdict_sont_publiees() {
        let samples = verdict(&reply(Some(0), None, wire::frame(&[0x08, 2])), "").finish();
        let value = |name: &str| samples.iter().find(|s| s.metric == name).map(|s| s.value);
        assert_eq!(value("probe_grpc_serving_status"), Some(2.0));
        assert_eq!(value("probe_grpc_status_code"), Some(0.0));
        assert_eq!(value("probe_http_status_code"), Some(200.0));
        assert!(value("probe_grpc_check_seconds").is_some());
    }

    #[test]
    fn les_en_tetes_grpc_se_lisent() {
        let mut headers = HeaderMap::new();
        headers.insert("grpc-status", HeaderValue::from_static(" 12 "));
        headers.insert("grpc-message", HeaderValue::from_static("not%20here"));
        assert_eq!(read_status(&headers), Some(12));
        assert_eq!(read_message(&headers).as_deref(), Some("not here"));
        assert_eq!(read_status(&HeaderMap::new()), None);
        assert_eq!(read_message(&HeaderMap::new()), None);
    }

    // --- Serveur gRPC factice, HTTP/2 en clair ---

    #[derive(Clone)]
    enum Scenario {
        /// `HealthCheckResponse` avec ce statut, puis `grpc-status: 0` en trailers.
        Health(u8),
        /// Réponse « Trailers-Only » portant un code et un message.
        TrailersOnly(u32, &'static str),
        /// Statut HTTP sans corps, hors gRPC.
        Http(u16),
        /// Page HTML en 200.
        Html,
    }

    struct FakeServer {
        address: String,
        /// Corps (préfixe gRPC compris) de chaque requête reçue.
        requests: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    async fn spawn(scenario: Scenario) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else { return };
                let (scenario, seen) = (scenario.clone(), seen.clone());
                tokio::spawn(async move {
                    let Ok(mut connection) = h2::server::handshake(socket).await else { return };
                    // `accept` fait aussi progresser la connexion : les requêtes
                    // sont donc traitées dans des tâches à part.
                    while let Some(Ok((request, respond))) = connection.accept().await {
                        tokio::spawn(answer(request, respond, scenario.clone(), seen.clone()));
                    }
                });
            }
        });
        FakeServer { address, requests }
    }

    async fn answer(
        request: Request<h2::RecvStream>,
        mut respond: h2::server::SendResponse<Bytes>,
        scenario: Scenario,
        seen: Arc<Mutex<Vec<Vec<u8>>>>,
    ) {
        let mut body = request.into_body();
        let mut received = Vec::new();
        while let Some(Ok(chunk)) = body.data().await {
            let _ = body.flow_control().release_capacity(chunk.len());
            received.extend_from_slice(&chunk);
        }
        seen.lock().unwrap().push(received);

        let grpc = || Response::builder().status(200).header("content-type", "application/grpc");
        match scenario {
            Scenario::Health(status) => {
                let Ok(mut send) = respond.send_response(grpc().body(()).unwrap(), false) else {
                    return;
                };
                let _ = send.send_data(Bytes::from(wire::frame(&[0x08, status])), false);
                let mut trailers = HeaderMap::new();
                trailers.insert("grpc-status", HeaderValue::from_static("0"));
                let _ = send.send_trailers(trailers);
            }
            Scenario::TrailersOnly(code, message) => {
                let head = grpc()
                    .header("grpc-status", code.to_string())
                    .header("grpc-message", message)
                    .body(())
                    .unwrap();
                let _ = respond.send_response(head, true);
            }
            Scenario::Http(code) => {
                let _ =
                    respond.send_response(Response::builder().status(code).body(()).unwrap(), true);
            }
            Scenario::Html => {
                let head =
                    Response::builder().status(200).header("content-type", "text/html").body(());
                let Ok(mut send) = respond.send_response(head.unwrap(), false) else { return };
                let _ = send.send_data(Bytes::from_static(b"<html>hello</html>"), true);
            }
        }
    }

    fn target(address: &str, tags: &[(&str, &str)]) -> Target {
        let mut all = vec![("allow_private_targets", "true"), ("timeout_seconds", "3")];
        all.extend_from_slice(tags);
        cible("grpc", address, &all)
    }

    async fn probe(address: &str, tags: &[(&str, &str)]) -> Vec<Sample> {
        GrpcCollector::new().probe(&target(address, tags)).await.expect("une mesure")
    }

    fn metric(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn reason(samples: &[Sample]) -> Option<String> {
        samples
            .iter()
            .find(|s| s.metric == "probe_failure_info")
            .and_then(|s| s.labels.get("reason").cloned())
    }

    #[tokio::test]
    async fn un_serveur_en_marche_reussit_et_recoit_une_requete_vide() {
        let server = spawn(Scenario::Health(1)).await;
        let samples = probe(&server.address, &[]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        assert_eq!(metric(&samples, "probe_grpc_serving_status"), Some(1.0));
        assert!(metric(&samples, "probe_connect_seconds").is_some());
        assert_eq!(*server.requests.lock().unwrap(), vec![wire::frame(&[])]);
    }

    #[tokio::test]
    async fn le_nom_du_service_part_dans_la_requete() {
        let server = spawn(Scenario::Health(1)).await;
        let samples = probe(&server.address, &[("service", "pkg.Echo")]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        let expected = wire::frame(&wire::encode_request("pkg.Echo"));
        assert_eq!(*server.requests.lock().unwrap(), vec![expected]);
        let labelled = samples.iter().all(|s| s.labels.contains_key("service"));
        assert!(labelled, "l'étiquette service est posée sur tous les échantillons");
    }

    #[tokio::test]
    async fn not_serving_produit_un_echec_unhealthy() {
        let server = spawn(Scenario::Health(2)).await;
        let samples = probe(&server.address, &[]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(metric(&samples, "probe_grpc_serving_status"), Some(2.0));
        assert_eq!(reason(&samples).as_deref(), Some("unhealthy"));
    }

    #[tokio::test]
    async fn service_unknown_produit_un_echec_unhealthy() {
        let server = spawn(Scenario::Health(3)).await;
        let samples = probe(&server.address, &[("service", "inconnu")]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("unhealthy"));
    }

    #[tokio::test]
    async fn unimplemented_en_trailers_only_est_un_echec_lisible() {
        let server =
            spawn(Scenario::TrailersOnly(12, "unknown service grpc.health.v1.Health")).await;
        let samples = probe(&server.address, &[]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(metric(&samples, "probe_grpc_status_code"), Some(12.0));
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn un_statut_http_hors_grpc_est_signale() {
        let server = spawn(Scenario::Http(503)).await;
        let samples = probe(&server.address, &[]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(metric(&samples, "probe_http_status_code"), Some(503.0));
        assert_eq!(reason(&samples).as_deref(), Some("status"));
    }

    #[tokio::test]
    async fn une_page_html_nest_pas_une_reponse_grpc() {
        let server = spawn(Scenario::Html).await;
        let samples = probe(&server.address, &[]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn un_port_ferme_produit_un_echec_mesure_et_non_une_erreur() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let samples = probe(&address, &[]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("connect"));
    }

    /// Un serveur HTTP/1.1 sur le port : la poignée de main HTTP/2 échoue ou
    /// reste sans réponse, mais la sonde écrit un zéro dans le délai.
    #[tokio::test]
    async fn un_serveur_http_1_est_un_echec_mesure() {
        use tokio::io::AsyncWriteExt;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    drop(socket);
                });
            }
        });
        let samples = probe(&address, &[("timeout_seconds", "2")]).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert!(reason(&samples).is_some());
    }

    #[tokio::test]
    async fn la_boucle_locale_est_refusee_sans_loption() {
        let target = cible("grpc", "127.0.0.1:50051", &[("timeout_seconds", "2")]);
        let error = GrpcCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
        assert!(error.to_string().contains("allow_private_targets"), "{error}");
    }
}
