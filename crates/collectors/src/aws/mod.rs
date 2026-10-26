//! Amazon Web Services, par l'API CloudWatch.
//!
//! La cible porte l'hôte du point d'accès CloudWatch de la région
//! (`monitoring.eu-west-3.amazonaws.com`) ; la région elle-même est une option,
//! car elle entre dans la signature des requêtes. Les requêtes sont signées en
//! « Signature Version 4 » (`sigv4.rs`), sans le SDK d'AWS.
//!
//! Identifiants : une clé d'accès (identifiant et secret) d'un utilisateur IAM en
//! lecture seule. Si l'option `role_arn` est renseignée, DumbMonit endosse ce
//! rôle (`sts:AssumeRole`, point d'accès régional de STS) et signe avec les
//! identifiants temporaires obtenus, gardés une heure moins une minute.
//!
//! Lectures, par l'API JSON 1.0 de CloudWatch (`X-Amz-Target:
//! GraniteServiceVersion20100801.<Action>`) :
//!
//! * `DescribeAlarms` (état `ALARM`, alarmes de métrique et composites) : l'appel
//!   principal, dont l'échec fait échouer la sonde ;
//! * `ListMetrics` (`AWS/EC2` et `AWS/RDS`, `CPUUtilization`, actives dans les trois
//!   dernières heures) : l'inventaire des instances EC2 et des bases RDS ;
//! * `GetMetricData`, un seul lot borné à 500 requêtes : processeur et contrôles
//!   d'état des instances, processeur, espace libre et connexions des bases, sur
//!   les dix dernières minutes.
//!
//! Les deux derniers appels sont facultatifs : un refus (permission IAM absente)
//! n'échoue pas la sonde, il retire les séries concernées et `ec2_readable` /
//! `rds_readable` valent 0. Seule une panne du réseau les fait échouer.
//!
//! Limites : les instances et bases sont plafonnées à 100 chacune (le lot de
//! `GetMetricData` compte au plus 500 requêtes) ; seules les instances EC2 sans
//! dimension supplémentaire et les bases `DBInstanceIdentifier` sont lues ; les
//! métriques d'une instance sans surveillance détaillée ont un point toutes les
//! cinq minutes, d'où la période de 300 secondes.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `region` | — | Région AWS (`eu-west-3`), requise. |
//! | `role_arn` | vide | Rôle à endosser par STS. |
//! | `endpoint_url` | vide | Point d'accès STS, pour un VPC endpoint, une région isolée ou les tests. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

mod sigv4;

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::cloud::{figure, item};
use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms, number, text};
use crate::selfhosted::options::tag;
use sigv4::{Signer, sha256_hex};

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "monitoring.eu-west-3.amazonaws.com";

/// Préfixe des séries : exactement le `kind`.
const PREFIX: &str = "aws";
const CLOUDWATCH_TARGET: &str = "GraniteServiceVersion20100801";
const JSON_CONTENT_TYPE: &str = "application/x-amz-json-1.0";
const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded; charset=utf-8";
/// Fenêtre lue par `GetMetricData`, et période d'agrégation des points.
const METRIC_WINDOW_SECS: i64 = 600;
const METRIC_PERIOD_SECS: u32 = 300;
/// Pages au plus par liste (alarmes, métriques, données).
const MAX_PAGES: usize = 5;
/// Instances EC2 et bases RDS lues, chacune : 2 × 100 + 3 × 100 = 500 requêtes,
/// le maximum d'un `GetMetricData`.
const MAX_INSTANCES: usize = 100;
/// Durée de vie demandée à STS, la plus courte que AWS accepte.
const ROLE_SESSION_SECS: u64 = 3600;

/// Codes d'erreur d'AWS qui disent « ces identifiants ne passent pas ».
const AUTH_CODES: &[&str] = &[
    "UnrecognizedClientException",
    "InvalidSignatureException",
    "SignatureDoesNotMatch",
    "InvalidClientTokenId",
    "ExpiredTokenException",
    "ExpiredToken",
    "AccessDeniedException",
    "AccessDenied",
    "MissingAuthenticationToken",
    "IncompleteSignature",
    "InvalidAccessKeyId",
    "UnauthorizedOperation",
    "AuthFailure",
    "InvalidIdentityToken",
    "OptInRequired",
];

#[derive(Default)]
pub struct AwsCollector {
    /// Identifiants temporaires d'un rôle endossé, gardés jusqu'à leur expiration.
    sessions: TokenCache,
}

impl AwsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Prépare un client CloudWatch signé, après avoir endossé le rôle s'il y en a un.
    async fn session(&self, target: &Target) -> Result<CloudWatch, ProbeError> {
        let region = region(target)?;
        let Credential::UsernamePassword { username, password } = &target.credential else {
            return Err(ProbeError::Config(format!(
                "AWS expects an access key ID and a secret access key, configured: {}",
                target.credential
            )));
        };
        let (access_key, secret_key) = (username.trim(), password.trim());
        if access_key.is_empty() || secret_key.is_empty() {
            return Err(ProbeError::Config(
                "AWS needs both the access key ID and the secret access key".to_string(),
            ));
        }
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "AWS CloudWatch")?;
        let mut access = Access {
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
            session_token: None,
        };
        let mut cache_key = None;
        if let Some(role) = tag(target, "role_arn") {
            if !role.starts_with("arn:") {
                return Err(ProbeError::Config(format!(
                    "Invalid role ARN \"{role}\": expected arn:aws:iam::<account>:role/<name>"
                )));
            }
            let key = format!("{}:{access_key}:{role}", target.id);
            let cached = self.sessions.get(&key).and_then(|packed| Access::unpack(&packed));
            access = match cached {
                Some(access) => access,
                None => {
                    let (assumed, lifetime) = assume_role(target, &region, &access, role).await?;
                    self.sessions.put(&key, &assumed.pack(), lifetime);
                    assumed
                }
            };
            cache_key = Some(key);
        }
        Ok(CloudWatch { client, region, access, cache_key })
    }

    /// Oublie les identifiants temporaires que AWS vient de refuser : le prochain
    /// passage en demandera de nouveaux.
    fn expire(&self, cloudwatch: &CloudWatch, error: &ProbeError) {
        if matches!(error, ProbeError::Auth(_))
            && let Some(key) = &cloudwatch.cache_key
        {
            self.sessions.forget(key);
        }
    }
}

#[async_trait]
impl Collector for AwsCollector {
    fn kind(&self) -> &'static str {
        PREFIX
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let cloudwatch = self.session(target).await?;
        let ts_ms = now_ms();
        let alarms = cloudwatch
            .alarms(100, MAX_PAGES)
            .await
            .inspect_err(|error| self.expire(&cloudwatch, error))?;
        let ec2 = soft(cloudwatch.dimension_ids("AWS/EC2", "InstanceId").await)?;
        let rds = soft(cloudwatch.dimension_ids("AWS/RDS", "DBInstanceIdentifier").await)?;
        let inventory = cloudwatch.inventory(ec2, rds, ts_ms / 1000).await?;
        Ok(samples(&alarms, &inventory, ts_ms))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let cloudwatch = self.session(target).await?;
        cloudwatch.alarms(1, 1).await.inspect_err(|error| self.expire(&cloudwatch, error))?;
        Ok(Some(PREFIX.to_string()))
    }
}

/// Un appel facultatif : un refus ou une réponse inattendue devient `None`, seule
/// une panne du réseau remonte (si CloudWatch ne répond plus, la sonde échoue).
fn soft<T>(result: Result<T, ProbeError>) -> Result<Option<T>, ProbeError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.means_down() => Err(error),
        Err(error) => {
            tracing::debug!(%error, "AWS optional call skipped");
            Ok(None)
        }
    }
}

fn region(target: &Target) -> Result<String, ProbeError> {
    let Some(region) = tag(target, "region") else {
        return Err(ProbeError::Config(
            "The Region option is required, for example eu-west-3".to_string(),
        ));
    };
    if !region.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ProbeError::Config(format!("Invalid AWS region \"{region}\"")));
    }
    Ok(region.to_ascii_lowercase())
}

/// Identifiants qui signent les requêtes : la clé d'accès, ou ceux d'un rôle endossé.
#[derive(Clone)]
struct Access {
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
}

impl Access {
    /// Forme gardée dans le cache de jetons (aucun des trois champs ne contient de saut de ligne).
    fn pack(&self) -> String {
        format!(
            "{}\n{}\n{}",
            self.access_key,
            self.secret_key,
            self.session_token.as_deref().unwrap_or_default()
        )
    }

    fn unpack(packed: &str) -> Option<Self> {
        let mut parts = packed.splitn(3, '\n');
        let (access_key, secret_key, token) = (parts.next()?, parts.next()?, parts.next()?);
        Some(Self {
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
            session_token: Some(token.to_string()).filter(|token| !token.is_empty()),
        })
    }
}

/// Hôte tel que l'enverra `reqwest` dans l'en-tête `Host` (port par défaut omis).
fn host_of(base_url: &str) -> String {
    let (scheme, rest) = base_url.split_once("://").unwrap_or(("https", base_url));
    let host = rest.split('/').next().unwrap_or(rest);
    let implicit = if scheme.eq_ignore_ascii_case("http") { ":80" } else { ":443" };
    host.strip_suffix(implicit).unwrap_or(host).to_string()
}

/// POST signé sur `/` : `scope` = (région, service).
async fn signed_post(
    client: &RestClient,
    scope: (&str, &str),
    access: &Access,
    content_type: &str,
    target: Option<&str>,
    body: String,
) -> Result<(StatusCode, String), ProbeError> {
    let amz_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let payload_hash = sha256_hex(body.as_bytes());
    let mut headers = vec![
        ("content-type".to_string(), content_type.to_string()),
        ("host".to_string(), host_of(client.base_url())),
        ("x-amz-content-sha256".to_string(), payload_hash.clone()),
        ("x-amz-date".to_string(), amz_date.clone()),
    ];
    if let Some(target) = target {
        headers.push(("x-amz-target".to_string(), target.to_string()));
    }
    if let Some(token) = &access.session_token {
        headers.push(("x-amz-security-token".to_string(), token.clone()));
    }
    let signer = Signer {
        access_key: &access.access_key,
        secret_key: &access.secret_key,
        region: scope.0,
        service: scope.1,
    };
    let authorization = signer.authorization(&amz_date, "POST", "/", "", &headers, &payload_hash);
    // `host` est posé par `reqwest` lui-même, à l'identique de ce qui a été signé.
    let mut extra: Vec<(&str, String)> = headers
        .iter()
        .filter(|(name, _)| name.as_str() != "host")
        .map(|(name, value)| (name.as_str(), value.clone()))
        .collect();
    extra.push(("authorization", authorization));
    client.post_raw("/", body, &extra).await
}

/// Code et message d'une erreur JSON d'AWS (`__type`, `message`).
fn json_error(body: &str) -> (String, String) {
    let reply: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let raw = text(&reply, "__type").or_else(|| text(&reply, "code")).unwrap_or_default();
    // `com.amazon.coral.service#AccessDeniedException` ou `AccessDenied:http://…`.
    let code = raw.rsplit('#').next().unwrap_or(raw).split(':').next().unwrap_or(raw);
    let message = text(&reply, "message").or_else(|| text(&reply, "Message")).unwrap_or_default();
    (code.to_string(), message.to_string())
}

/// Traduit une réponse d'erreur de CloudWatch.
fn cloudwatch_error(client: &RestClient, status: StatusCode, body: &str, path: &str) -> ProbeError {
    let (code, message) = json_error(body);
    let lower = message.to_ascii_lowercase();
    if lower.contains("signature expired")
        || lower.contains("too skewed")
        || matches!(code.as_str(), "RequestExpired" | "RequestTimeTooSkewed")
    {
        return ProbeError::Config(format!(
            "AWS rejected the request date ({code}): check the clock of the DumbMonit server. \
             {message}"
        ));
    }
    if AUTH_CODES.contains(&code.as_str()) {
        return ProbeError::Auth(format!(
            "AWS refused the request ({code}): {message} Check the access key and its secret, \
             that the Region option matches the address, and the IAM permissions."
        ));
    }
    client.status_error(status, body, path)
}

/// Un client CloudWatch prêt à signer.
struct CloudWatch {
    client: RestClient,
    region: String,
    access: Access,
    /// Clé du cache quand un rôle est endossé.
    cache_key: Option<String>,
}

impl CloudWatch {
    async fn call(&self, action: &str, body: &Value) -> Result<Value, ProbeError> {
        let target = format!("{CLOUDWATCH_TARGET}.{action}");
        let (status, reply) = signed_post(
            &self.client,
            (self.region.as_str(), "monitoring"),
            &self.access,
            JSON_CONTENT_TYPE,
            Some(&target),
            body.to_string(),
        )
        .await?;
        if status.is_success() {
            return self.client.decode(&reply, action);
        }
        Err(cloudwatch_error(&self.client, status, &reply, action))
    }

    /// Noms des alarmes à l'état `ALARM`, triés et sans doublon.
    async fn alarms(&self, max_records: u32, max_pages: usize) -> Result<Vec<String>, ProbeError> {
        let mut names = BTreeSet::new();
        let mut token: Option<String> = None;
        for _ in 0..max_pages {
            let mut body = json!({
                "StateValue": "ALARM",
                "AlarmTypes": ["CompositeAlarm", "MetricAlarm"],
                "MaxRecords": max_records,
            });
            if let Some(token) = &token {
                body["NextToken"] = json!(token);
            }
            let reply = self.call("DescribeAlarms", &body).await?;
            names.extend(parse_alarms(&reply));
            token = text(&reply, "NextToken").map(str::to_string);
            if token.is_none() {
                break;
            }
        }
        Ok(names.into_iter().collect())
    }

    /// Valeurs triées de la dimension `dimension` des métriques `CPUUtilization`
    /// d'un espace de noms actives depuis trois heures.
    async fn dimension_ids(
        &self,
        namespace: &str,
        dimension: &str,
    ) -> Result<Vec<String>, ProbeError> {
        let mut ids = BTreeSet::new();
        let mut token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut body = json!({
                "Namespace": namespace,
                "MetricName": "CPUUtilization",
                "RecentlyActive": "PT3H",
            });
            if let Some(token) = &token {
                body["NextToken"] = json!(token);
            }
            let reply = self.call("ListMetrics", &body).await?;
            ids.extend(parse_dimension_ids(&reply, dimension));
            token = text(&reply, "NextToken").map(str::to_string);
            if token.is_none() {
                break;
            }
        }
        Ok(ids.into_iter().collect())
    }

    /// Points `(horodatage, valeur)` de chaque requête, par identifiant de requête.
    async fn metric_data(
        &self,
        queries: &[Query],
        now_s: i64,
    ) -> Result<HashMap<String, Vec<(f64, f64)>>, ProbeError> {
        let wanted: Vec<Value> = queries.iter().map(Query::to_json).collect();
        let mut points: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
        let mut token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut body = json!({
                "StartTime": now_s - METRIC_WINDOW_SECS,
                "EndTime": now_s,
                "ScanBy": "TimestampDescending",
                "MetricDataQueries": wanted,
            });
            if let Some(token) = &token {
                body["NextToken"] = json!(token);
            }
            let reply = self.call("GetMetricData", &body).await?;
            let results = reply.get("MetricDataResults").and_then(Value::as_array);
            for result in results.into_iter().flatten() {
                let Some(id) = text(result, "Id") else { continue };
                let numbers = |key: &str| -> Vec<f64> {
                    result
                        .get(key)
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|v| number(Some(v)))
                        .collect()
                };
                let found = points.entry(id.to_string()).or_default();
                found.extend(numbers("Timestamps").into_iter().zip(numbers("Values")));
            }
            token = text(&reply, "NextToken").map(str::to_string);
            if token.is_none() {
                break;
            }
        }
        Ok(points)
    }

    /// Lit les mesures des instances et bases listées ; `None` pour une famille
    /// dont la liste ou les mesures ne sont pas lisibles.
    async fn inventory(
        &self,
        ec2: Option<Vec<String>>,
        rds: Option<Vec<String>>,
        now_s: i64,
    ) -> Result<Inventory, ProbeError> {
        let queries =
            build_queries(ec2.as_deref().unwrap_or_default(), rds.as_deref().unwrap_or_default());
        let data = if queries.is_empty() {
            Some(HashMap::new())
        } else {
            soft(self.metric_data(&queries, now_s).await)?
        };
        let Some(data) = data else { return Ok(Inventory::default()) };
        Ok(Inventory::from_data(ec2, rds, &data))
    }
}

/// Endosse un rôle (`sts:AssumeRole`) : identifiants temporaires et durée de vie.
async fn assume_role(
    target: &Target,
    region: &str,
    base: &Access,
    role_arn: &str,
) -> Result<(Access, Duration), ProbeError> {
    let endpoint = match tag(target, "endpoint_url") {
        Some(url) if url.starts_with("https://") || url.starts_with("http://") => url.to_string(),
        Some(other) => {
            return Err(ProbeError::Config(format!(
                "Invalid endpoint URL \"{other}\": expected a full URL such as \
                 https://sts.{region}.amazonaws.com"
            )));
        }
        None => format!("https://sts.{region}.amazonaws.com"),
    };
    let client = RestClient::for_base(target, &endpoint, "AWS STS")?;
    let body = format!(
        "Action=AssumeRole&Version=2011-06-15&RoleArn={}&RoleSessionName=dumbmonit\
         &DurationSeconds={ROLE_SESSION_SECS}",
        form_encode(role_arn)
    );
    let (status, reply) =
        signed_post(&client, (region, "sts"), base, FORM_CONTENT_TYPE, None, body).await?;
    if !status.is_success() {
        return Err(sts_error(&client, status, &reply));
    }
    parse_assumed(&reply)
}

/// Encodage des valeurs d'un formulaire (tout sauf les caractères non réservés).
fn form_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Texte du premier élément de ce nom (espace de noms ignoré), vide s'il manque.
fn xml_text(doc: &roxmltree::Document, name: &str) -> String {
    doc.descendants()
        .find(|node| node.is_element() && node.tag_name().name() == name)
        .and_then(|node| node.text())
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn sts_error(client: &RestClient, status: StatusCode, body: &str) -> ProbeError {
    let (code, message) = roxmltree::Document::parse(body)
        .map(|doc| (xml_text(&doc, "Code"), xml_text(&doc, "Message")))
        .unwrap_or_default();
    match code.as_str() {
        "ValidationError" | "MalformedPolicyDocument" | "InvalidParameterValue" => {
            ProbeError::Config(format!("AWS rejected the role to assume: {message}"))
        }
        "RegionDisabledException" => ProbeError::Config(format!(
            "AWS STS is disabled in the region of the Region option: {message}"
        )),
        code if AUTH_CODES.contains(&code) => ProbeError::Auth(format!(
            "AWS refused to assume the role ({code}): {message} Check the access key, the \
             role ARN and the trust policy of the role."
        )),
        _ => client.status_error(status, body, "AssumeRole"),
    }
}

/// Lit la réponse XML d'`AssumeRole`.
fn parse_assumed(body: &str) -> Result<(Access, Duration), ProbeError> {
    let unreadable = |detail: String| {
        ProbeError::Protocol(format!("AWS STS sent an unexpected answer to AssumeRole: {detail}"))
    };
    let doc = roxmltree::Document::parse(body).map_err(|error| unreadable(error.to_string()))?;
    let access_key = xml_text(&doc, "AccessKeyId");
    let secret_key = xml_text(&doc, "SecretAccessKey");
    let token = xml_text(&doc, "SessionToken");
    if access_key.is_empty() || secret_key.is_empty() || token.is_empty() {
        return Err(unreadable("no temporary credentials".to_string()));
    }
    let remaining = chrono::DateTime::parse_from_rfc3339(&xml_text(&doc, "Expiration"))
        .map(|at| at.timestamp() - chrono::Utc::now().timestamp())
        .unwrap_or(ROLE_SESSION_SECS as i64);
    let lifetime = remaining.clamp(0, ROLE_SESSION_SECS as i64) as u64;
    Ok((
        Access { access_key, secret_key, session_token: Some(token) },
        Duration::from_secs(lifetime),
    ))
}

/// Noms des alarmes d'une page de `DescribeAlarms` (métriques et composites).
fn parse_alarms(reply: &Value) -> Vec<String> {
    ["MetricAlarms", "CompositeAlarms"]
        .iter()
        .filter_map(|key| reply.get(*key).and_then(Value::as_array))
        .flatten()
        .filter_map(|alarm| text(alarm, "AlarmName"))
        .map(str::to_string)
        .collect()
}

/// Valeurs de la dimension `dimension` des métriques n'ayant que cette dimension
/// (les agrégats par groupe, par image ou par type sont écartés).
fn parse_dimension_ids(reply: &Value, dimension: &str) -> Vec<String> {
    reply
        .get("Metrics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|metric| {
            let dimensions = metric.get("Dimensions").and_then(Value::as_array)?;
            match dimensions.as_slice() {
                [only] if text(only, "Name") == Some(dimension) => text(only, "Value"),
                _ => None,
            }
        })
        .map(str::to_string)
        .collect()
}

/// Une requête de `GetMetricData`.
struct Query {
    id: String,
    namespace: &'static str,
    metric: &'static str,
    dimension: &'static str,
    value: String,
    stat: &'static str,
}

impl Query {
    fn to_json(&self) -> Value {
        json!({
            "Id": self.id,
            "MetricStat": {
                "Metric": {
                    "Namespace": self.namespace,
                    "MetricName": self.metric,
                    "Dimensions": [{ "Name": self.dimension, "Value": self.value }],
                },
                "Period": METRIC_PERIOD_SECS,
                "Stat": self.stat,
            },
            "ReturnData": true,
        })
    }
}

/// Identifiant de requête : une lettre minuscule d'abord, puis lettres et chiffres.
/// `ec` / `es` : CPU et contrôles d'état d'une instance ; `rc` / `rf` / `rn` : CPU,
/// espace libre et connexions d'une base.
fn query_id(kind: &str, index: usize) -> String {
    format!("{kind}{index}")
}

fn build_queries(ec2: &[String], rds: &[String]) -> Vec<Query> {
    let mut queries = Vec::new();
    for (i, id) in ec2.iter().take(MAX_INSTANCES).enumerate() {
        let make = |kind: &str, metric: &'static str| Query {
            id: query_id(kind, i),
            namespace: "AWS/EC2",
            metric,
            dimension: "InstanceId",
            value: id.clone(),
            stat: "Maximum",
        };
        queries.push(make("ec", "CPUUtilization"));
        queries.push(make("es", "StatusCheckFailed"));
    }
    for (i, id) in rds.iter().take(MAX_INSTANCES).enumerate() {
        let make = |kind: &str, metric: &'static str, stat: &'static str| Query {
            id: query_id(kind, i),
            namespace: "AWS/RDS",
            metric,
            dimension: "DBInstanceIdentifier",
            value: id.clone(),
            stat,
        };
        queries.push(make("rc", "CPUUtilization", "Maximum"));
        queries.push(make("rf", "FreeStorageSpace", "Average"));
        queries.push(make("rn", "DatabaseConnections", "Average"));
    }
    queries
}

/// Plus grande valeur d'une série.
fn series_max(points: &[(f64, f64)]) -> Option<f64> {
    points.iter().map(|point| point.1).reduce(f64::max)
}

/// Valeur du point le plus récent.
fn series_latest(points: &[(f64, f64)]) -> Option<f64> {
    points.iter().max_by(|a, b| a.0.total_cmp(&b.0)).map(|point| point.1)
}

struct Instance {
    id: String,
    cpu: Option<f64>,
    failed: Option<f64>,
}

struct Database {
    id: String,
    cpu: Option<f64>,
    free_bytes: Option<f64>,
    connections: Option<f64>,
}

/// Une famille d'objets : leur nombre exact, et ceux qui sont décrits (plafonnés).
struct Part<T> {
    total: usize,
    items: Vec<T>,
}

/// Ce que `ListMetrics` et `GetMetricData` ont appris ; `None` = famille illisible.
#[derive(Default)]
struct Inventory {
    ec2: Option<Part<Instance>>,
    rds: Option<Part<Database>>,
}

impl Inventory {
    fn from_data(
        ec2: Option<Vec<String>>,
        rds: Option<Vec<String>>,
        data: &HashMap<String, Vec<(f64, f64)>>,
    ) -> Self {
        let max = |id: String| data.get(&id).and_then(|points| series_max(points));
        let latest = |id: String| data.get(&id).and_then(|points| series_latest(points));
        Self {
            ec2: ec2.map(|ids| Part {
                total: ids.len(),
                items: ids
                    .into_iter()
                    .take(MAX_INSTANCES)
                    .enumerate()
                    .map(|(i, id)| Instance {
                        id,
                        cpu: max(query_id("ec", i)),
                        failed: max(query_id("es", i)),
                    })
                    .collect(),
            }),
            rds: rds.map(|ids| Part {
                total: ids.len(),
                items: ids
                    .into_iter()
                    .take(MAX_INSTANCES)
                    .enumerate()
                    .map(|(i, id)| Database {
                        id,
                        cpu: max(query_id("rc", i)),
                        free_bytes: latest(query_id("rf", i)),
                        connections: latest(query_id("rn", i)),
                    })
                    .collect(),
            }),
        }
    }
}

fn samples(alarms: &[String], inventory: &Inventory, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = vec![
        g("alarms_in_alarm", alarms.len() as f64),
        figure(PREFIX, "Alarms in alarm", "", alarms.len() as f64, ts_ms),
    ];
    for name in alarms.iter().take(MAX_NAMED) {
        out.push(g("alarm_in_alarm", 1.0).with_label("alarm", name.as_str()));
        out.push(item(PREFIX, "Alarms", name, false, "In alarm", ts_ms));
    }

    out.push(flag("ec2_readable", inventory.ec2.is_some(), ts_ms));
    if let Some(part) = &inventory.ec2 {
        out.push(g("ec2_instances", part.total as f64));
        out.push(figure(PREFIX, "EC2 instances", "", part.total as f64, ts_ms));
        let mut failed = 0;
        for instance in &part.items {
            let named = |sample: Sample| sample.with_label("instance", instance.id.as_str());
            if let Some(cpu) = instance.cpu {
                out.push(named(g("ec2_cpu_percent", cpu)));
            }
            if let Some(value) = instance.failed {
                let bad = value > 0.0;
                failed += usize::from(bad);
                out.push(named(g("ec2_status_check_failed", f64::from(bad))));
                let detail = if bad { "Status check failed" } else { "OK" };
                out.push(item(PREFIX, "EC2 instances", &instance.id, !bad, detail, ts_ms));
            }
        }
        out.push(figure(PREFIX, "EC2 status checks failed", "", failed as f64, ts_ms));
    }

    out.push(flag("rds_readable", inventory.rds.is_some(), ts_ms));
    if let Some(part) = &inventory.rds {
        out.push(g("rds_instances", part.total as f64));
        out.push(figure(PREFIX, "RDS databases", "", part.total as f64, ts_ms));
        for database in &part.items {
            let named = |sample: Sample| sample.with_label("db", database.id.as_str());
            if let Some(cpu) = database.cpu {
                out.push(named(g("rds_cpu_percent", cpu)));
            }
            if let Some(free) = database.free_bytes {
                out.push(named(g("rds_free_storage_bytes", free)));
            }
            if let Some(connections) = database.connections {
                out.push(named(g("rds_connections", connections)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    /// Réponses d'après les exemples de la documentation de l'API CloudWatch
    /// (`DescribeAlarms`, `ListMetrics`, `GetMetricData`) et de STS (`AssumeRole`).
    const ALARMS: &str = include_str!("testdata/describe_alarms.json");
    const LIST_EC2: &str = include_str!("testdata/list_metrics_ec2.json");
    const LIST_RDS: &str = include_str!("testdata/list_metrics_rds.json");
    const DATA: &str = include_str!("testdata/get_metric_data.json");
    const STS_OK: &str = include_str!("testdata/assume_role.xml");
    const STS_DENIED: &str = include_str!("testdata/sts_denied.xml");

    const ACCESS_KEY: &str = "AKIDEXAMPLE";
    const SECRET: &str = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";
    const TEMP_ACCESS_KEY: &str = "ASIAEXAMPLETEMP";
    const TEMP_SECRET: &str = "tempSecretKeyExample/abc+def";
    const ROLE: &str = "arn:aws:iam::123456789012:role/dumbmonit-read";

    const DENIED: &str = r#"{"__type":"com.amazon.coral.service#AccessDeniedException","message":"User: arn:aws:iam::123456789012:user/dumbmonit is not authorized to perform: cloudwatch:ListMetrics because no identity-based policy allows it"}"#;

    /// Comportement du faux AWS : CloudWatch et STS sur le même port.
    struct Behavior {
        alarms: (u16, String),
        list_denied: bool,
        data_denied: bool,
        sts: (u16, String),
        sts_calls: AtomicUsize,
        queries: AtomicUsize,
    }

    impl Default for Behavior {
        fn default() -> Self {
            Self {
                alarms: (200, ALARMS.to_string()),
                list_denied: false,
                data_denied: false,
                sts: (200, STS_OK.to_string()),
                sts_calls: AtomicUsize::new(0),
                queries: AtomicUsize::new(0),
            }
        }
    }

    fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
        headers.get(name).and_then(|value| value.to_str().ok())
    }

    /// Recalcule la signature à partir de ce que le faux serveur a reçu.
    fn signature_valid(
        headers: &HeaderMap,
        body: &str,
        access_key: &str,
        secret_key: &str,
    ) -> bool {
        let auth = header(headers, "authorization").unwrap_or_default();
        let field =
            |name: &str| auth.split([',', ' ']).find_map(|p| p.strip_prefix(name)).unwrap_or("");
        let scope: Vec<&str> = field("Credential=").split('/').collect();
        let [key, day, region, service, _] = scope.as_slice() else { return false };
        let signed: Vec<(String, String)> = field("SignedHeaders=")
            .split(';')
            .map(|name| (name.to_string(), header(headers, name).unwrap_or("").to_string()))
            .collect();
        let amz_date = header(headers, "x-amz-date").unwrap_or_default();
        let signer = Signer { access_key, secret_key, region, service };
        let expected =
            signer.authorization(amz_date, "POST", "/", "", &signed, &sha256_hex(body.as_bytes()));
        *key == access_key && amz_date.starts_with(*day) && expected == auth
    }

    impl Behavior {
        fn answer(&self, headers: &HeaderMap, body: &str) -> (StatusCode, String) {
            let temporary = header(headers, "x-amz-security-token") == Some("TOKEN-TEMP");
            let (access_key, secret_key) =
                if temporary { (TEMP_ACCESS_KEY, TEMP_SECRET) } else { (ACCESS_KEY, SECRET) };
            if !signature_valid(headers, body, access_key, secret_key) {
                return (
                    StatusCode::FORBIDDEN,
                    r#"{"__type":"InvalidSignatureException","message":"bad signature"}"#
                        .to_string(),
                );
            }
            let status = |code: u16| StatusCode::from_u16(code).unwrap();
            let Some(target) = header(headers, "x-amz-target") else {
                self.sts_calls.fetch_add(1, Ordering::SeqCst);
                assert!(body.contains("Action=AssumeRole"), "{body}");
                assert!(body.contains("RoleArn=arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2F"));
                return (status(self.sts.0), self.sts.1.clone());
            };
            let request: Value = serde_json::from_str(body).unwrap();
            match target.rsplit('.').next().unwrap() {
                "DescribeAlarms" => {
                    assert_eq!(request["StateValue"], "ALARM");
                    (status(self.alarms.0), self.alarms.1.clone())
                }
                "ListMetrics" if self.list_denied => (StatusCode::FORBIDDEN, DENIED.to_string()),
                "ListMetrics" => match request["Namespace"].as_str() {
                    Some("AWS/EC2") => (StatusCode::OK, LIST_EC2.to_string()),
                    _ => (StatusCode::OK, LIST_RDS.to_string()),
                },
                "GetMetricData" if self.data_denied => (StatusCode::FORBIDDEN, DENIED.to_string()),
                "GetMetricData" => {
                    let count = request["MetricDataQueries"].as_array().map_or(0, Vec::len);
                    self.queries.store(count, Ordering::SeqCst);
                    (StatusCode::OK, DATA.to_string())
                }
                other => panic!("unexpected action {other}"),
            }
        }
    }

    async fn fake_aws(behavior: Arc<Behavior>) -> String {
        let app = Router::new().route(
            "/",
            post(move |headers: HeaderMap, body: String| {
                let behavior = behavior.clone();
                async move { behavior.answer(&headers, &body) }
            }),
        );
        serve(app).await
    }

    fn target(address: &str, tags: &[(&str, &str)]) -> Target {
        let mut target = cible("aws", address, tags);
        target.credential = Credential::UsernamePassword {
            username: ACCESS_KEY.to_string(),
            password: SECRET.to_string(),
        };
        target
    }

    async fn probe(behavior: Behavior) -> Result<Vec<Sample>, ProbeError> {
        let address = fake_aws(Arc::new(behavior)).await;
        let target = target(&format!("http://{address}"), &[("region", "eu-west-3")]);
        AwsCollector::new().probe(&target).await
    }

    #[tokio::test]
    async fn alarmes_instances_et_bases() {
        let behavior = Arc::new(Behavior::default());
        let address = fake_aws(behavior.clone()).await;
        let target = target(&format!("http://{address}"), &[("region", "eu-west-3")]);
        let s = AwsCollector::new().probe(&target).await.unwrap();

        // Règle « alarme CloudWatch en ALARM ».
        assert_eq!(value(&s, "alarms_in_alarm", &[]), 3.0);
        for name in ["rds-prod-cpu-high", "web-5xx", "prod-degraded"] {
            assert_eq!(value(&s, "alarm_in_alarm", &[("alarm", name)]), 1.0, "{name}");
        }
        // Règles « contrôle d'état en échec » et « CPU EC2 élevé ».
        assert_eq!(value(&s, "ec2_readable", &[]), 1.0);
        assert_eq!(value(&s, "ec2_instances", &[]), 2.0);
        assert_eq!(value(&s, "ec2_cpu_percent", &[("instance", "i-0123456789abcdef0")]), 97.5);
        assert!(
            find(&s, "ec2_cpu_percent", &[("instance", "i-0fedcba9876543210")]).is_none(),
            "no datapoint: no series"
        );
        let status = |id: &str| value(&s, "ec2_status_check_failed", &[("instance", id)]);
        assert_eq!(status("i-0123456789abcdef0"), 0.0);
        assert_eq!(status("i-0fedcba9876543210"), 1.0);
        // Règles « CPU RDS élevé » et « stockage RDS bas ».
        assert_eq!(value(&s, "rds_readable", &[]), 1.0);
        assert_eq!(value(&s, "rds_instances", &[]), 1.0);
        assert_eq!(value(&s, "rds_cpu_percent", &[("db", "prod-db")]), 60.5);
        assert_eq!(value(&s, "rds_free_storage_bytes", &[("db", "prod-db")]), 21_474_836_480.0);
        assert_eq!(value(&s, "rds_connections", &[("db", "prod-db")]), 42.0);
        // Séries génériques de la page.
        let alarm = find(&s, "aws_item_ok", &[("group", "Alarms"), ("item", "web-5xx")]).unwrap();
        assert_eq!(alarm.value, 0.0);
        assert_eq!(alarm.labels["detail"], "In alarm");
        let bad = find(&s, "aws_item_ok", &[("item", "i-0fedcba9876543210")]).unwrap();
        assert_eq!((bad.value, bad.labels["detail"].as_str()), (0.0, "Status check failed"));
        let good = find(&s, "aws_item_ok", &[("item", "i-0123456789abcdef0")]).unwrap();
        assert_eq!(good.value, 1.0);
        assert_eq!(value(&s, "aws_figure", &[("name", "Alarms in alarm")]), 3.0);
        assert_eq!(value(&s, "aws_figure", &[("name", "EC2 status checks failed")]), 1.0);
        assert_eq!(behavior.queries.load(Ordering::SeqCst), 7, "2 x 2 EC2 and 1 x 3 RDS queries");
    }

    #[test]
    fn les_agregats_de_dimensions_sont_ecartes() {
        let ec2: Value = serde_json::from_str(LIST_EC2).unwrap();
        let ids = parse_dimension_ids(&ec2, "InstanceId");
        assert_eq!(ids, ["i-0fedcba9876543210", "i-0123456789abcdef0"]);
        let rds: Value = serde_json::from_str(LIST_RDS).unwrap();
        assert_eq!(parse_dimension_ids(&rds, "DBInstanceIdentifier"), ["prod-db"]);
    }

    #[test]
    fn le_lot_de_requetes_reste_borne() {
        let many: Vec<String> = (0..300).map(|i| format!("i-{i}")).collect();
        let queries = build_queries(&many, &many);
        assert_eq!(queries.len(), 500);
        assert!(queries.iter().all(|q| q.id.starts_with(|c: char| c.is_ascii_lowercase())));
        let ids: BTreeSet<&str> = queries.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids.len(), 500, "unique ids");
    }

    #[tokio::test]
    async fn identifiants_refuses() {
        for (status, body) in [
            (
                400,
                r#"{"__type":"com.amazon.coral.service#UnrecognizedClientException","message":"The security token included in the request is invalid."}"#,
            ),
            (403, DENIED),
            (
                400,
                r#"{"__type":"ExpiredTokenException","message":"The security token included in the request is expired"}"#,
            ),
            (401, "{}"),
        ] {
            let behavior = Behavior { alarms: (status, body.to_string()), ..Behavior::default() };
            let error = probe(behavior).await.unwrap_err();
            assert!(matches!(error, ProbeError::Auth(_)), "{status} {error}");
            assert!(!error.means_down());
        }
    }

    #[tokio::test]
    async fn une_horloge_decalee_est_une_erreur_de_configuration() {
        let body = r#"{"__type":"InvalidSignatureException","message":"Signature expired: 20260101T000000Z is now earlier than 20260101T000500Z"}"#;
        let behavior = Behavior { alarms: (403, body.to_string()), ..Behavior::default() };
        let error = probe(behavior).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn un_serveur_en_panne_est_une_panne() {
        let behavior = Behavior { alarms: (503, "{}".to_string()), ..Behavior::default() };
        let error = probe(behavior).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn une_liste_de_metriques_refusee_ne_fait_pas_echouer_la_sonde() {
        let s = probe(Behavior { list_denied: true, ..Behavior::default() }).await.unwrap();
        assert_eq!(value(&s, "alarms_in_alarm", &[]), 3.0);
        assert_eq!(value(&s, "ec2_readable", &[]), 0.0);
        assert_eq!(value(&s, "rds_readable", &[]), 0.0);
        assert!(find(&s, "ec2_instances", &[]).is_none());
        assert!(find(&s, "rds_cpu_percent", &[]).is_none());
    }

    #[tokio::test]
    async fn des_mesures_refusees_ne_font_pas_echouer_la_sonde() {
        let s = probe(Behavior { data_denied: true, ..Behavior::default() }).await.unwrap();
        assert_eq!(value(&s, "alarms_in_alarm", &[]), 3.0);
        assert_eq!(value(&s, "ec2_readable", &[]), 0.0);
        assert_eq!(value(&s, "rds_readable", &[]), 0.0);
        assert!(find(&s, "ec2_cpu_percent", &[]).is_none());
    }

    #[tokio::test]
    async fn role_endosse_puis_identifiants_temporaires_gardes() {
        let behavior = Arc::new(Behavior::default());
        let address = fake_aws(behavior.clone()).await;
        let url = format!("http://{address}");
        let target = target(
            &url,
            &[("region", "eu-west-3"), ("role_arn", ROLE), ("endpoint_url", url.as_str())],
        );
        let collector = AwsCollector::new();
        // Le faux serveur n'accepte les appels CloudWatch que signés avec les clés temporaires.
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "alarms_in_alarm", &[]), 3.0);
        collector.probe(&target).await.unwrap();
        assert_eq!(behavior.sts_calls.load(Ordering::SeqCst), 1, "temporary keys are kept");
        assert_eq!(collector.discover(&target).await.unwrap().as_deref(), Some("aws"));
    }

    #[tokio::test]
    async fn role_refuse() {
        let behavior = Behavior { sts: (403, STS_DENIED.to_string()), ..Behavior::default() };
        let address = fake_aws(Arc::new(behavior)).await;
        let url = format!("http://{address}");
        let target = target(
            &url,
            &[("region", "eu-west-3"), ("role_arn", ROLE), ("endpoint_url", url.as_str())],
        );
        let error = AwsCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(&error, ProbeError::Auth(m) if m.contains("not authorized")), "{error}");
    }

    #[test]
    fn reponse_d_assume_role() {
        let (access, lifetime) = parse_assumed(STS_OK).unwrap();
        assert_eq!(access.access_key, TEMP_ACCESS_KEY);
        assert_eq!(access.session_token.as_deref(), Some("TOKEN-TEMP"));
        assert_eq!(lifetime, Duration::from_secs(ROLE_SESSION_SECS), "capped at the session");
        let again = Access::unpack(&access.pack()).unwrap();
        assert_eq!(again.secret_key, TEMP_SECRET);
        assert!(parse_assumed("<nope/>").is_err());
        assert!(parse_assumed("not xml").is_err());
    }

    #[tokio::test]
    async fn reglages_invalides_sans_appel_reseau() {
        let collector = AwsCollector::new();
        for tags in [
            vec![],
            vec![("region", "eu west")],
            vec![("region", "eu-west-3"), ("role_arn", "dumbmonit-read")],
        ] {
            let target = target("unreachable.invalid:1", &tags);
            let error = collector.probe(&target).await.unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{tags:?}: {error}");
        }
        let mut target = target("unreachable.invalid:1", &[("region", "eu-west-3")]);
        target.credential = Credential::ApiToken { token: "x".to_string() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[test]
    fn hote_signe_comme_l_envoie_reqwest() {
        let official = "https://monitoring.eu-west-3.amazonaws.com";
        assert_eq!(host_of(official), "monitoring.eu-west-3.amazonaws.com");
        assert_eq!(host_of("https://sts.example:443"), "sts.example");
        assert_eq!(host_of("http://127.0.0.1:8080/x"), "127.0.0.1:8080");
        assert_eq!(host_of("http://127.0.0.1:443"), "127.0.0.1:443");
    }

    #[test]
    fn formulaire_encode() {
        assert_eq!(form_encode(ROLE), "arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2Fdumbmonit-read");
    }
}
