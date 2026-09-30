//! VMware vSphere : un vCenter Server ou un ESXi seul.
//!
//! Tout passe par l'API vSphere Web Services (SOAP, `/sdk`) plutôt que par
//! l'API REST `/api/vcenter/*` : un ESXi seul n'a pas d'API REST, et même sur
//! un vCenter l'API REST ne dit ni le mode maintenance, ni l'état de santé
//! global (`overallStatus`), ni l'état des VMware Tools, ni les alarmes
//! déclenchées. Le simulateur officiel (`vcsim`, govmomi) parle le même SOAP.
//!
//! Le compte n'a besoin que du rôle intégré **Read-only** (lecture seule),
//! posé à la racine de l'inventaire avec propagation. Aucun appel ne modifie
//! quoi que ce soit : la seule écriture est la vue d'inventaire que
//! `CreateContainerView` crée dans la session, et que `DestroyView` rend.
//!
//! # Session
//!
//! Ouvrir une session à chaque collecte écrirait un événement « utilisateur
//! connecté » par minute dans le journal du vCenter. Le cookie de session est
//! donc gardé d'une collecte à l'autre, par cible ; une session expirée
//! (`NotAuthenticated`) est rouverte une fois, puis la collecte reprend.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `443` | Port HTTPS du vCenter ou de l'ESXi. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'un ESXi par défaut). |
//! | `request_timeout_seconds` | `20` | Délai par appel. |
//! | `alarms` | `true` | Lit les alarmes déclenchées. |

pub mod metrics;
pub mod soap;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target, TargetId};
use reqwest::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use tracing::warn;

use crate::api_options::{Connection, parse_bool, tag};
use soap::{Fault, MoRef, ObjectContent, ServiceContent};

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Au plus autant de pages d'inventaire : 500 objets chacune.
const MAX_PAGES: usize = 40;

#[derive(Debug, Clone)]
struct Settings {
    connection: Connection,
    alarms: bool,
    username: String,
    password: String,
}

impl Settings {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (username, password) = match &target.credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                (username.trim().to_string(), password.clone())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "vSphere expects a user name and password, configured: {other}"
                )));
            }
        };
        let mut target = target.clone();
        // Une adresse copiée depuis le navigateur finit souvent par `/sdk` ou `/ui`.
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed
            .strip_suffix("/sdk")
            .or_else(|| trimmed.strip_suffix("/ui"))
            .unwrap_or(trimmed)
            .to_string();
        Ok(Self {
            connection: Connection::from_target(
                &target,
                "https",
                80,
                DEFAULT_PORT,
                DEFAULT_REQUEST_TIMEOUT,
            )?,
            alarms: parse_bool(tag(&target, "alarms"), true)?,
            username,
            password,
        })
    }

    /// Clé du cache de sessions : une autre adresse ou un autre compte ouvre
    /// une autre session.
    fn session_key(&self, id: TargetId) -> String {
        format!("{id}\u{0}{}\u{0}{}", self.connection.base_url, self.username)
    }
}

/// Un appel SOAP, avec ou sans cookie de session.
struct Client {
    http: reqwest::Client,
    url: String,
    timeout: Duration,
    cookie: Option<String>,
}

impl Client {
    async fn call(&self, body: String) -> Result<(String, Option<String>), ProbeError> {
        let mut request = self
            .http
            .post(&self.url)
            .header(CONTENT_TYPE, "text/xml; charset=utf-8")
            .header("SOAPAction", soap::SOAP_ACTION)
            .timeout(self.timeout)
            .body(body);
        if let Some(cookie) = &self.cookie {
            request = request.header(COOKIE, cookie);
        }
        let response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                ProbeError::Timeout(self.timeout)
            } else {
                ProbeError::Unreachable(format!("{}: {error}", self.url))
            }
        })?;
        let status = response.status();
        let cookie = session_cookie(response.headers());
        let text = response
            .text()
            .await
            .map_err(|error| ProbeError::Unreachable(format!("{}: {error}", self.url)))?;
        // Une faute SOAP arrive en 500 : c'est le corps qui dit ce qu'il en est.
        if status.is_success() || status == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
            return Ok((text, cookie));
        }
        Err(match status.as_u16() {
            401 | 403 => ProbeError::Auth(format!("vSphere refused {} ({status})", self.url)),
            404 => ProbeError::Protocol(format!(
                "No vSphere API at {}: check the address and the port (443 for a vCenter or ESXi).",
                self.url
            )),
            code if code >= 500 => ProbeError::Unreachable(format!("vSphere answered {status}")),
            _ => ProbeError::Protocol(format!("vSphere answered {status}")),
        })
    }

    /// Un appel dont la réponse est décodée ; une faute reste une faute.
    async fn request<T>(
        &self,
        body: String,
        decode: impl FnOnce(roxmltree::Node) -> Result<T, ProbeError>,
    ) -> Result<Result<T, Fault>, ProbeError> {
        let (text, _) = self.call(body).await?;
        soap::read(&text, decode)
    }
}

/// `vmware_soap_session="…"; Path=/; HttpOnly` devient `vmware_soap_session="…"`.
fn session_cookie(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .find(|pair| pair.trim_start().starts_with("vmware_soap_session="))
        .map(|pair| pair.trim().to_string())
}

/// Collecteur vCenter / ESXi.
#[derive(Default)]
pub struct VsphereCollector {
    /// Cookies de session par cible (voir le module).
    sessions: Mutex<HashMap<String, String>>,
}

impl VsphereCollector {
    pub fn new() -> Self {
        Self::default()
    }

    fn cached(&self, key: &str) -> Option<String> {
        self.sessions.lock().ok()?.get(key).cloned()
    }

    fn remember(&self, key: &str, cookie: Option<String>) {
        if let Ok(mut sessions) = self.sessions.lock() {
            match cookie {
                Some(cookie) => sessions.insert(key.to_string(), cookie),
                None => sessions.remove(key),
            };
        }
    }

    async fn login(
        &self,
        client: &mut Client,
        settings: &Settings,
        content: &ServiceContent,
    ) -> Result<(), ProbeError> {
        client.cookie = None;
        let body = soap::login(&content.session_manager, &settings.username, &settings.password);
        let (text, cookie) = client.call(body).await?;
        soap::read(&text, soap::decode_nothing)?.map_err(Fault::into_probe_error)?;
        let cookie = cookie.ok_or_else(|| {
            ProbeError::Protocol("vSphere accepted the login but set no session cookie".into())
        })?;
        client.cookie = Some(cookie);
        Ok(())
    }

    async fn collect(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = Settings::from_target(target)?;
        let key = settings.session_key(target.id);
        let mut client = Client {
            http: crate::http::client(settings.connection.insecure_tls)?,
            url: format!("{}/sdk", settings.connection.base_url),
            timeout: settings.connection.request_timeout,
            cookie: self.cached(&key),
        };
        let ts_ms = chrono::Utc::now().timestamp_millis();

        let content = client
            .request(soap::retrieve_service_content(), soap::decode_service_content)
            .await?
            .map_err(Fault::into_probe_error)?;
        if client.cookie.is_none() {
            self.login(&mut client, &settings, &content).await?;
        }
        let inventory = match read_inventory(&client, &content).await? {
            Ok(objects) => objects,
            Err(Fault::NotAuthenticated) => {
                // Session expirée : une seule nouvelle tentative.
                self.login(&mut client, &settings, &content).await?;
                read_inventory(&client, &content).await?.map_err(|fault| {
                    self.remember(&key, None);
                    fault.into_probe_error()
                })?
            }
            Err(fault) => {
                self.remember(&key, None);
                return Err(fault.into_probe_error());
            }
        };
        self.remember(&key, client.cookie.clone());

        let mut out = metrics::about_samples(&content.about, ts_ms);
        out.extend(metrics::inventory_samples(&inventory, ts_ms));
        let mut errors = 0u32;
        if settings.alarms {
            match read_alarms(&client, &content, &inventory, ts_ms).await {
                Ok(samples) => out.extend(samples),
                Err(error) => {
                    errors += 1;
                    warn!(target_id = target.id, %error, "alarmes vSphere illisibles");
                }
            }
        }
        out.push(Sample::new("vsphere_scrape_errors", f64::from(errors), MetricKind::Gauge, ts_ms));
        Ok(out)
    }
}

/// Crée la vue, lit toutes les pages, rend la vue.
async fn read_inventory(
    client: &Client,
    content: &ServiceContent,
) -> Result<Result<Vec<ObjectContent>, Fault>, ProbeError> {
    let view = match client
        .request(
            soap::create_container_view(&content.view_manager, &content.root_folder),
            soap::decode_moref,
        )
        .await?
    {
        Ok(view) => view,
        Err(fault) => return Ok(Err(fault)),
    };
    let collector = &content.property_collector;
    let outcome = read_pages(client, collector, &view).await;
    // La vue est rendue même si la lecture a échoué.
    let _ = client.request(soap::destroy_view(&view), soap::decode_nothing).await;
    outcome
}

async fn read_pages(
    client: &Client,
    collector: &MoRef,
    view: &MoRef,
) -> Result<Result<Vec<ObjectContent>, Fault>, ProbeError> {
    let mut objects = Vec::new();
    let mut body = soap::retrieve_inventory(collector, view);
    for _ in 0..MAX_PAGES {
        let page = match client.request(body, soap::decode_page).await? {
            Ok(page) => page,
            Err(fault) => return Ok(Err(fault)),
        };
        if soap::page_not_authenticated(&page) {
            return Ok(Err(Fault::NotAuthenticated));
        }
        objects.extend(page.objects);
        match page.token {
            Some(token) => body = soap::continue_retrieve(collector, &token),
            None => return Ok(Ok(objects)),
        }
    }
    Err(ProbeError::Protocol(format!(
        "The inventory spans more than {} objects: only the first ones were read.",
        MAX_PAGES as u32 * soap::PAGE_SIZE
    )))
}

async fn read_alarms(
    client: &Client,
    content: &ServiceContent,
    inventory: &[ObjectContent],
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let collector = &content.property_collector;
    let states = client
        .request(
            soap::retrieve_alarm_states(collector, &content.root_folder),
            soap::decode_alarm_states,
        )
        .await?
        .map_err(Fault::into_probe_error)?;
    let mut alarm_names = HashMap::new();
    let mut wanted: Vec<&MoRef> = states.iter().map(|s| &s.alarm).collect();
    wanted.sort();
    wanted.dedup();
    if !wanted.is_empty() {
        // Un nom manquant n'empêche rien : l'alarme garde son identifiant.
        if let Ok(Ok(page)) =
            client.request(soap::retrieve_alarm_names(collector, &wanted), soap::decode_page).await
        {
            for object in page.objects {
                if let Some(name) = object.text("info.name") {
                    alarm_names.insert(object.obj.clone(), name.to_string());
                }
            }
        }
    }
    Ok(metrics::alarm_samples(&states, &metrics::names(inventory), &alarm_names, ts_ms))
}

#[async_trait]
impl Collector for VsphereCollector {
    fn kind(&self) -> &'static str {
        "vsphere"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        self.collect(target).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.collect(target).await?;
        Ok(Some("vsphere".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::post;

    use super::*;
    use crate::api_options::test_support::target;

    const DIR: &str = "testdata/vcsim_0.56.0/vcenter";

    fn fixture(name: &str) -> &'static str {
        match name {
            "service_content" => include_str!("testdata/vcsim_0.56.0/vcenter/service_content.xml"),
            "login" => include_str!("testdata/vcsim_0.56.0/vcenter/login.xml"),
            "login_refused" => include_str!("testdata/vcsim_0.56.0/vcenter/login_refused.xml"),
            "create_view" => include_str!("testdata/vcsim_0.56.0/vcenter/create_view.xml"),
            "create_view_not_authenticated" => {
                include_str!("testdata/vcsim_0.56.0/vcenter/create_view_not_authenticated.xml")
            }
            "after_logout" => include_str!("testdata/vcsim_0.56.0/vcenter/after_logout.xml"),
            "page1" => include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page1.xml"),
            "page2" => include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page2.xml"),
            "page3" => include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page3.xml"),
            "page4" => include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page4.xml"),
            "page5" => include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page5.xml"),
            "alarms" => include_str!("testdata/documented/alarm_states.xml"),
            "alarm_names" => include_str!("testdata/documented/alarm_names.xml"),
            other => panic!("{DIR}: pas de fixture {other}"),
        }
    }

    fn token_of(page: &str) -> Option<String> {
        let start = page.find("<token>")? + "<token>".len();
        let end = page[start..].find("</token>")? + start;
        Some(page[start..end].to_string())
    }

    /// Un faux vCenter qui rejoue les réponses de `vcsim` : il exige le cookie
    /// de session, suit les jetons de pagination et compte les ouvertures de
    /// session. `expire` fait oublier la session une fois.
    #[derive(Default)]
    struct Fake {
        logins: AtomicUsize,
        expire: AtomicUsize,
        destroyed: AtomicUsize,
    }

    async fn serve(fake: Arc<Fake>) -> String {
        let app = Router::new().route(
            "/sdk",
            post(move |headers: HeaderMap, body: String| {
                let fake = fake.clone();
                async move {
                    let cookie = headers.get("cookie").and_then(|v| v.to_str().ok()).unwrap_or("");
                    let ok = |name: &str| (StatusCode::OK, fixture(name)).into_response();
                    if body.contains("<RetrieveServiceContent>") {
                        return ok("service_content");
                    }
                    if body.contains("<Login>") {
                        if !body.contains("<password>S3cret&amp;&lt;pw&gt;</password>") {
                            return (StatusCode::INTERNAL_SERVER_ERROR, fixture("login_refused"))
                                .into_response();
                        }
                        let n = fake.logins.fetch_add(1, Ordering::SeqCst) + 1;
                        return (
                            StatusCode::OK,
                            [("set-cookie", format!("vmware_soap_session=\"s{n}\"; Path=/; HttpOnly"))],
                            fixture("login"),
                        )
                            .into_response();
                    }
                    let current = format!("vmware_soap_session=\"s{}\"", fake.logins.load(Ordering::SeqCst));
                    if cookie != current || fake.expire.load(Ordering::SeqCst) > 0 {
                        fake.expire.store(0, Ordering::SeqCst);
                        // Ce que `vcsim` répond, selon l'appel, sans session.
                        if body.contains("<CreateContainerView>") {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                fixture("create_view_not_authenticated"),
                            )
                                .into_response();
                        }
                        return ok("after_logout");
                    }
                    if body.contains("<CreateContainerView>") {
                        return ok("create_view");
                    }
                    if body.contains("<DestroyView>") {
                        fake.destroyed.fetch_add(1, Ordering::SeqCst);
                        return (StatusCode::OK, "<?xml version=\"1.0\"?><soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\"><soapenv:Body><DestroyViewResponse xmlns=\"urn:vim25\"/></soapenv:Body></soapenv:Envelope>").into_response();
                    }
                    if body.contains("triggeredAlarmState") {
                        return ok("alarms");
                    }
                    if body.contains("<type>Alarm</type>") {
                        return ok("alarm_names");
                    }
                    if body.contains("<ContinueRetrievePropertiesEx>") {
                        for (previous, next) in
                            [("page1", "page2"), ("page2", "page3"), ("page3", "page4"), ("page4", "page5")]
                        {
                            let token = token_of(fixture(previous)).unwrap();
                            if body.contains(&format!("<token>{token}</token>")) {
                                return ok(next);
                            }
                        }
                        return (StatusCode::INTERNAL_SERVER_ERROR, "unknown token").into_response();
                    }
                    if body.contains("<RetrievePropertiesEx>") {
                        return ok("page1");
                    }
                    (StatusCode::INTERNAL_SERVER_ERROR, "unexpected").into_response()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn credential(password: &str) -> Credential {
        Credential::UsernamePassword { username: "dumbmonit".into(), password: password.into() }
    }

    fn value(samples: &[Sample], metric: &str, key: &str, label: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == metric && s.labels.get(key).map(String::as_str) == Some(label))
            .map(|s| s.value)
    }

    #[tokio::test]
    async fn la_collecte_suit_les_pages_garde_la_session_et_rend_la_vue() {
        let fake = Arc::new(Fake::default());
        let base = serve(fake.clone()).await;
        let collector = VsphereCollector::new();
        let target = target("vsphere", &format!("{base}/sdk/"), &[], credential("S3cret&<pw>"));

        let samples = collector.probe(&target).await.unwrap();
        assert_eq!(value(&samples, "vsphere_hosts", "state", "disconnected"), Some(1.0));
        assert_eq!(value(&samples, "vsphere_host_maintenance", "host", "DC0_C0_H2"), Some(1.0));
        assert_eq!(
            value(&samples, "vsphere_datastore_accessible", "datastore", "LocalDS_0"),
            Some(1.0)
        );
        assert_eq!(value(&samples, "vsphere_alarms", "status", "red"), Some(1.0));
        assert_eq!(value(&samples, "vsphere_info", "product", "vcenter"), Some(1.0));
        assert_eq!(
            value(&samples, "vsphere_alarm", "alarm", "Host connection and power state"),
            Some(2.0)
        );
        assert_eq!(fake.destroyed.load(Ordering::SeqCst), 1);

        // La deuxième collecte réutilise la session.
        collector.probe(&target).await.unwrap();
        assert_eq!(fake.logins.load(Ordering::SeqCst), 1);

        // Une session expirée est rouverte une fois, sans erreur.
        fake.expire.store(1, Ordering::SeqCst);
        let samples = collector.probe(&target).await.unwrap();
        assert_eq!(fake.logins.load(Ordering::SeqCst), 2);
        assert!(value(&samples, "vsphere_vms", "state", "poweredOn").is_some());
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_n_est_pas_une_panne() {
        let base = serve(Arc::new(Fake::default())).await;
        let error = VsphereCollector::new()
            .probe(&target("vsphere", &base, &[], credential("faux")))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error:?}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn sans_identifiant_la_configuration_est_refusee() {
        let error = VsphereCollector::new()
            .probe(&target("vsphere", "vcenter.lan", &[], Credential::None))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        let error = VsphereCollector::new()
            .probe(&target("vsphere", "vcenter.lan", &[("alarms", "peut-être")], credential("x")))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }

    #[tokio::test]
    async fn un_serveur_eteint_est_une_panne() {
        // Port réservé, rien n'écoute.
        let error = VsphereCollector::new()
            .probe(&target("vsphere", "http://127.0.0.1:9", &[], credential("x")))
            .await
            .unwrap_err();
        assert!(error.means_down(), "{error:?}");
    }
}
