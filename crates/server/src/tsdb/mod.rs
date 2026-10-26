//! Client VictoriaMetrics : écriture des échantillons et lecture des séries.
//!
//! L'écriture passe par le format d'exposition Prometheus sur
//! `/api/v1/import/prometheus`, volontairement préféré au `remote_write` protobuf :
//! pas de dépendance à snappy ni à un schéma généré, et un flux lisible à l'œil
//! quand il faut diagnostiquer.

pub mod embedded;
pub mod rollup;
mod writer;

pub use embedded::{EmbeddedConfig, EmbeddedVm, Flavor};
pub use writer::{DEFAULT_FLUSH_SIZE, SampleSink, spawn_writer, spawn_writer_with};

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use anyhow::{Context, Result, bail};
use dumbmonit_proto::Sample;
use serde::Deserialize;

/// Préfixe appliqué à toutes nos métriques, pour cohabiter sans collision avec une
/// instance VictoriaMetrics que l'utilisateur partagerait avec d'autres outils.
pub const METRIC_PREFIX: &str = "dumbmonit_";

/// Décalage des lectures « fraîches ». VictoriaMetrics masque par défaut les
/// 30 dernières secondes ; il n'accepte pas zéro, mais une milliseconde.
const FRESH_LATENCY_OFFSET: &str = "1ms";

/// Pourquoi une lecture destinée à un collecteur externe a échoué.
///
/// Trois cas, et trois réponses différentes : ce que l'appelant peut corriger
/// (`Refused`, `TooLarge`) n'est pas ce qu'il doit attendre (`Unreachable`).
/// Les distinguer par un type plutôt que par le texte de l'erreur : « connection
/// refused » contient le mot « refused » sans être un refus de la requête.
#[derive(Debug)]
pub enum ScrapeError {
    /// VictoriaMetrics a répondu, et a refusé : sélecteur invalide, par exemple.
    Refused { status: u16, detail: String },
    /// La réponse dépasse le plafond que l'appelant s'est fixé.
    TooLarge { max_bytes: usize },
    /// Injoignable, ou réponse illisible.
    Unreachable(anyhow::Error),
}

impl std::fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused { status, detail } => write!(f, "refused ({status}): {detail}"),
            Self::TooLarge { max_bytes } => write!(
                f,
                "the selected series are too large for one response (over {} MiB)",
                max_bytes / (1024 * 1024)
            ),
            Self::Unreachable(error) => write!(f, "{error:#}"),
        }
    }
}

#[derive(Clone)]
pub struct Victoria {
    http: reqwest::Client,
    base_url: String,
    /// Clé exigée par les routes d'administration (effacement, instantanés)
    /// du VictoriaMetrics embarqué. Tirée au hasard à chaque démarrage et
    /// jamais exposée : seul ce client la connaît.
    admin_key: Option<String>,
    /// Aiguillage vers l'historique long terme, partagé par tous les clones :
    /// posé au démarrage, et ses réglages suivent ceux de l'interface.
    long: Arc<std::sync::RwLock<Option<Arc<LongTerm>>>>,
}

/// L'instance des agrégats cinq minutes, et ce qu'il faut savoir pour décider
/// d'y lire (voir [`rollup`]).
pub struct LongTerm {
    pub victoria: Victoria,
    /// Rétention brute, en secondes.
    raw_retention_secs: AtomicI64,
    /// Plus ancienne fin de fenêtre agrégée (secondes), `i64::MAX` sans agrégat.
    coverage_start_secs: AtomicI64,
    /// Coupé depuis l'interface : plus aucune lecture n'y va.
    enabled: AtomicBool,
}

impl LongTerm {
    pub fn new(victoria: Victoria, raw_retention_secs: i64) -> Self {
        Self {
            victoria,
            raw_retention_secs: AtomicI64::new(raw_retention_secs),
            coverage_start_secs: AtomicI64::new(i64::MAX),
            enabled: AtomicBool::new(true),
        }
    }

    pub fn set_raw_retention_secs(&self, secs: i64) {
        self.raw_retention_secs.store(secs, Ordering::Relaxed);
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Les agrégats remontent désormais jusqu'à `secs` (s'il est plus ancien).
    pub fn extend_coverage(&self, secs: i64) {
        self.coverage_start_secs.fetch_min(secs, Ordering::Relaxed);
    }

    pub fn coverage_start_secs(&self) -> Option<i64> {
        let value = self.coverage_start_secs.load(Ordering::Relaxed);
        (value != i64::MAX).then_some(value)
    }

    fn horizon(&self) -> rollup::Horizon {
        rollup::Horizon {
            now_secs: chrono::Utc::now().timestamp(),
            raw_retention_secs: self.raw_retention_secs.load(Ordering::Relaxed),
            coverage_start_secs: self.coverage_start_secs(),
        }
    }
}

impl Victoria {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("construction du client HTTP VictoriaMetrics")?;
        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            admin_key: None,
            long: Arc::default(),
        })
    }

    /// Branche l'historique long terme : à partir de là, les lectures qui
    /// remontent au-delà de la rétention brute y sont aiguillées.
    pub fn set_long_term(&self, long: Option<Arc<LongTerm>>) {
        *self.long.write().unwrap_or_else(std::sync::PoisonError::into_inner) = long;
    }

    /// L'historique long terme branché, s'il y en a un.
    pub fn long_term(&self) -> Option<Arc<LongTerm>> {
        self.long.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    fn active_long_term(&self) -> Option<Arc<LongTerm>> {
        self.long_term().filter(|long| long.enabled())
    }

    /// Écrit des échantillons tels quels, sans passer par le tampon : c'est
    /// l'agrégateur qui l'appelle, avec des lots déjà constitués.
    pub async fn write_now(&self, samples: &[Sample]) -> Result<()> {
        self.write(samples).await
    }

    /// Fixe la clé d'administration (`-deleteAuthKey`) que le processus
    /// embarqué exige ; voir [`embedded::EmbeddedConfig::admin_key`].
    pub fn with_admin_key(mut self, key: impl Into<String>) -> Self {
        self.admin_key = Some(key.into());
        self
    }

    /// URL de base, telle que normalisée à la construction.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Vérifie que VictoriaMetrics répond. Appelée au démarrage et par `/api/health`.
    pub async fn health(&self) -> Result<()> {
        let response = self
            .http
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .context("VictoriaMetrics injoignable")?;
        if !response.status().is_success() {
            bail!("VictoriaMetrics answered {}", response.status());
        }
        Ok(())
    }

    pub async fn write(&self, samples: &[Sample]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }
        let (body, rejected) = encode_prometheus(samples);
        if rejected > 0 {
            REJECTED_SAMPLES.fetch_add(rejected as u64, std::sync::atomic::Ordering::Relaxed);
            tracing::warn!(
                rejected,
                "samples with an invalid metric or label name were dropped before writing"
            );
        }
        if body.is_empty() {
            return Ok(());
        }
        let response = self
            .http
            .post(format!("{}/api/v1/import/prometheus", self.base_url))
            .header("Content-Type", "text/plain")
            .body(body)
            .send()
            .await
            .context("sending samples to VictoriaMetrics")?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            bail!("write refused ({status}): {}", detail.trim());
        }
        Ok(())
    }

    /// Exécute une requête MetricsQL sur une plage de temps.
    ///
    /// Une plage qui remonte au-delà de la rétention brute est servie par les
    /// agrégats pour sa partie ancienne (voir [`rollup::plan_range`]).
    pub async fn query_range(
        &self,
        query: &str,
        start_ms: i64,
        end_ms: i64,
        step_secs: u64,
    ) -> Result<Vec<RangeSeries>> {
        self.query_range_filtered(query, start_ms, end_ms, step_secs, None).await
    }

    /// [`Self::query_range`], restreinte aux séries qui correspondent à
    /// `filter` (`extra_filters[]` de VictoriaMetrics : appliqué à chaque
    /// sélecteur de la requête, quelle que soit l'expression). Le filtre suit
    /// la requête jusqu'aux agrégats.
    pub async fn query_range_filtered(
        &self,
        query: &str,
        start_ms: i64,
        end_ms: i64,
        step_secs: u64,
        filter: Option<&str>,
    ) -> Result<Vec<RangeSeries>> {
        if let Some(long) = self.active_long_term() {
            let step = step_secs.max(1) as i64;
            let (start, end) = (start_ms / 1000, end_ms / 1000);
            match rollup::plan_range(query, start, end, step, &long.horizon()) {
                rollup::RangePlan::Raw => {}
                rollup::RangePlan::Long { query: rewritten } => {
                    match long
                        .victoria
                        .query_range_raw_filtered(&rewritten, start_ms, end_ms, step_secs, filter)
                        .await
                    {
                        Ok(series) => return Ok(series),
                        Err(error) => log_query_failure(&error, "long-term read failed, raw used"),
                    }
                }
                rollup::RangePlan::Split { query: rewritten, split_secs } => {
                    let old_end_ms = (split_secs - step) * 1000;
                    let (old, recent) = tokio::join!(
                        long.victoria.query_range_raw_filtered(
                            &rewritten,
                            start_ms,
                            old_end_ms,
                            step_secs,
                            filter,
                        ),
                        self.query_range_raw_filtered(
                            query,
                            split_secs * 1000,
                            end_ms,
                            step_secs,
                            filter,
                        ),
                    );
                    match old {
                        Ok(old) => return Ok(rollup::stitch(old, recent?)),
                        Err(error) => log_query_failure(&error, "long-term read failed, raw used"),
                    }
                }
            }
        }
        self.query_range_raw_filtered(query, start_ms, end_ms, step_secs, filter).await
    }

    /// `query_range` sur cette instance seulement, sans aiguillage.
    pub async fn query_range_raw(
        &self,
        query: &str,
        start_ms: i64,
        end_ms: i64,
        step_secs: u64,
    ) -> Result<Vec<RangeSeries>> {
        self.query_range_raw_filtered(query, start_ms, end_ms, step_secs, None).await
    }

    /// [`Self::query_range_raw`], restreinte par `filter` (voir
    /// [`Self::query_range_filtered`]).
    async fn query_range_raw_filtered(
        &self,
        query: &str,
        start_ms: i64,
        end_ms: i64,
        step_secs: u64,
        filter: Option<&str>,
    ) -> Result<Vec<RangeSeries>> {
        let extra: Vec<(&str, &str)> =
            filter.map(|filter| ("extra_filters[]", filter)).into_iter().collect();
        let response = self
            .http
            .get(format!("{}/api/v1/query_range", self.base_url))
            .query(&[
                ("query", query),
                ("start", &(start_ms / 1000).to_string()),
                ("end", &(end_ms / 1000).to_string()),
                ("step", &step_secs.to_string()),
            ])
            .query(&extra)
            .send()
            .await
            .context("interrogation de VictoriaMetrics")?;

        if !response.status().is_success() {
            return Err(refusal(response).await);
        }

        let parsed: QueryResponse =
            response.json().await.context("unreadable VictoriaMetrics response")?;
        Ok(parsed.data.result)
    }

    /// Efface toutes les séries d'une cible.
    ///
    /// Appelé quand l'équipement est supprimé : sans cela, ses séries orphelines
    /// resteraient interrogeables pendant toute la rétention, et « injoignable »
    /// pendant la fenêtre de sept jours de la règle. VictoriaMetrics expose cette
    /// route sans option particulière en mono-nœud ; l'appelant traite un échec
    /// comme un simple avertissement.
    pub async fn delete_target_series(&self, target_id: i64) -> Result<()> {
        self.delete_target_series_here(target_id).await?;
        // Ses agrégats aussi : sans quoi ils resteraient deux ans.
        if let Some(long) = self.long_term()
            && let Err(error) = long.victoria.delete_target_series_here(target_id).await
        {
            tracing::warn!(error = %format!("{error:#}"), target_id, "long-term series not deleted");
        }
        Ok(())
    }

    async fn delete_target_series_here(&self, target_id: i64) -> Result<()> {
        let matcher = format!("{{target=\"{target_id}\"}}");
        let mut request = self
            .http
            .post(format!("{}/api/v1/admin/tsdb/delete_series", self.base_url))
            .query(&[("match[]", matcher.as_str())]);
        if let Some(key) = &self.admin_key {
            request = request.query(&[("authKey", key.as_str())]);
        }
        let response =
            request.send().await.context("effacement des séries dans VictoriaMetrics")?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            bail!("delete refused ({status}): {}", detail.trim());
        }
        Ok(())
    }

    /// `{base}/api/v1/<segments…>?<query>`, chaque segment encodé pour ce qu'il
    /// est : un segment, et rien d'autre.
    pub(crate) fn api_url(&self, route: &[&str], query: Option<&str>) -> Result<reqwest::Url> {
        let mut url = reqwest::Url::parse(&self.base_url).context("VictoriaMetrics URL")?;
        url.path_segments_mut()
            .map_err(|()| anyhow::anyhow!("VictoriaMetrics URL cannot carry a path"))?
            .pop_if_empty()
            .extend(["api", "v1"])
            .extend(route);
        url.set_query(query.filter(|q| !q.is_empty()));
        Ok(url)
    }

    /// Fédère les séries désignées, au format d'exposition Prometheus.
    ///
    /// C'est la route que Prometheus appelle pour lire un autre Prometheus :
    /// elle rend le dernier point de chaque série retenue par les sélecteurs,
    /// horodaté, et rien d'autre. La réponse est lue avec un plafond plutôt
    /// qu'en entier : un sélecteur trop large doit se solder par un refus
    /// immédiat, pas par la mémoire du serveur.
    pub async fn federate(
        &self,
        selectors: &[String],
        max_lookback: Option<&str>,
        max_bytes: usize,
    ) -> std::result::Result<String, ScrapeError> {
        let mut query: Vec<(&str, &str)> =
            selectors.iter().map(|selector| ("match[]", selector.as_str())).collect();
        if let Some(lookback) = max_lookback {
            query.push(("max_lookback", lookback));
        }

        let response = self
            .http
            .get(format!("{}/federate", self.base_url))
            .query(&query)
            .send()
            .await
            .context("federating from VictoriaMetrics")
            .map_err(ScrapeError::Unreachable)?;

        read_capped(response, max_bytes).await
    }

    /// Relaie une route de lecture de l'API Prometheus, telle quelle.
    ///
    /// C'est ce qui permet à un Grafana de prendre le serveur pour une source
    /// Prometheus ordinaire : il interroge `/api/v1/query`, `/api/v1/series` et
    /// consorts, et reçoit la réponse de VictoriaMetrics sans traduction. La
    /// liste des routes relayées est tenue par l'appelant ; ici, rien n'est
    /// interprété — sauf la taille, qui reste bornée.
    ///
    /// L'URL est construite segment par segment, jamais en collant le chemin
    /// décodé dans une chaîne : un `%2e%2e` ou un `?` venu du client resterait
    /// sinon interprété par le parseur d'URL, et sortirait de `/api/v1/`.
    pub async fn proxy_read(
        &self,
        route: &[&str],
        query: Option<&str>,
        form: Option<String>,
        max_bytes: usize,
    ) -> std::result::Result<(u16, String), ScrapeError> {
        let url = self.api_url(route, query).map_err(ScrapeError::Unreachable)?;
        let request = match form {
            Some(body) => self
                .http
                .post(url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body),
            None => self.http.get(url),
        };

        let response = request
            .send()
            .await
            .context("querying VictoriaMetrics")
            .map_err(ScrapeError::Unreachable)?;
        // Le statut est relayé tel quel : une requête PromQL invalide doit
        // revenir à Grafana avec le message de VictoriaMetrics, qui nomme
        // l'erreur de syntaxe, et non sous une forme réécrite ici. Un refus
        // n'est donc pas une erreur pour cet appelant, contrairement à la
        // fédération.
        let status = response.status().as_u16();
        match read_capped(response, max_bytes).await {
            Ok(body) => Ok((status, body)),
            Err(ScrapeError::Refused { status, detail }) => Ok((status, detail)),
            Err(error) => Err(error),
        }
    }

    /// Étiquettes (nom compris, sous `__name__`) des séries qu'a écrites une
    /// cible depuis `since_secs` (secondes Unix).
    pub async fn target_series(
        &self,
        target_id: i64,
        since_secs: i64,
    ) -> Result<Vec<BTreeMap<String, String>>> {
        let response = self
            .http
            .get(format!("{}/api/v1/series", self.base_url))
            .query(&[
                ("match[]", format!("{{target=\"{target_id}\"}}")),
                ("start", since_secs.to_string()),
            ])
            .send()
            .await
            .context("interrogation de VictoriaMetrics")?;

        if !response.status().is_success() {
            return Err(refusal(response).await);
        }

        let parsed: SeriesResponse =
            response.json().await.context("unreadable VictoriaMetrics response")?;
        Ok(parsed.data)
    }

    /// Exécute une requête MetricsQL instantanée.
    pub async fn query(&self, query: &str) -> Result<Vec<InstantSeries>> {
        if let Some(answer) = self.query_long(query).await {
            return Ok(answer);
        }
        self.query_with(query, &[]).await
    }

    /// Une requête instantanée dont la fenêtre dépasse la rétention brute est
    /// lue sur les agrégats ; `None` quand elle doit rester sur le brut, ou si
    /// l'historique long terme ne répond pas.
    async fn query_long(&self, query: &str) -> Option<Vec<InstantSeries>> {
        let long = self.active_long_term()?;
        let rewritten = rollup::plan_instant(query, &long.horizon())?;
        match long.victoria.query_with(&rewritten, &[]).await {
            Ok(series) => Some(series),
            Err(error) => {
                log_query_failure(&error, "long-term read failed, raw used");
                None
            }
        }
    }

    /// Requête instantanée qui voit aussi les échantillons des dernières
    /// secondes.
    ///
    /// VictoriaMetrics masque par défaut les points de moins de 30 s
    /// (`-search.latencyOffset`), prévu pour des scrapes encore incomplets. Nos
    /// lots sont écrits d'un seul tenant : l'interface peut donc lire le
    /// résultat d'une sonde sitôt écrit, en particulier après « Probe now ».
    pub async fn query_fresh(&self, query: &str) -> Result<Vec<InstantSeries>> {
        // `1ms` et non `0s` : VictoriaMetrics refuse zéro (« latency_offset=0ms
        // is out of allowed range [1ms ...] »), et chaque lecture instantanée de
        // l'interface échouait alors en 400.
        if let Some(answer) = self.query_long(query).await {
            return Ok(answer);
        }
        self.query_with(query, &[("latency_offset", FRESH_LATENCY_OFFSET)]).await
    }

    /// [`Self::query_fresh`], restreinte aux séries qui correspondent à
    /// `filter` (voir [`Self::query_range_filtered`]).
    pub async fn query_fresh_filtered(
        &self,
        query: &str,
        filter: Option<&str>,
    ) -> Result<Vec<InstantSeries>> {
        match filter {
            None => self.query_fresh(query).await,
            Some(filter) => {
                self.query_with(
                    query,
                    &[("latency_offset", FRESH_LATENCY_OFFSET), ("extra_filters[]", filter)],
                )
                .await
            }
        }
    }

    async fn query_with(&self, query: &str, extra: &[(&str, &str)]) -> Result<Vec<InstantSeries>> {
        let response = self
            .http
            .get(format!("{}/api/v1/query", self.base_url))
            .query(&[("query", query)])
            .query(extra)
            .send()
            .await
            .context("interrogation de VictoriaMetrics")?;

        if !response.status().is_success() {
            return Err(refusal(response).await);
        }

        let parsed: InstantResponse =
            response.json().await.context("unreadable VictoriaMetrics response")?;
        Ok(parsed.data.result)
    }
}

/// Erreur d'une réponse non 2xx de VictoriaMetrics. Seul un 4xx est un refus de
/// la requête ([`QueryRejected`]) : un 5xx (surcharge, requête trop longue côté
/// serveur, stockage indisponible) est une panne du backend, pas de l'appelant.
async fn refusal(response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let detail = response.text().await.unwrap_or_default();
    let message = format!("query refused ({status}): {}", detail.trim());
    if status.is_client_error() { QueryRejected(message).into() } else { anyhow::anyhow!(message) }
}

/// VictoriaMetrics a répondu, mais refuse la requête (MetricsQL invalide, plage
/// refusée…) : la faute est celle de l'appelant, pas du backend. Une erreur de
/// transport ou une réponse illisible n'en est pas une.
#[derive(Debug)]
pub struct QueryRejected(pub String);

impl std::fmt::Display for QueryRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for QueryRejected {}

/// Lit le corps d'une réponse en s'arrêtant net au-delà du plafond.
///
/// Le corps n'est jamais accumulé en entier avant d'être mesuré : un sélecteur
/// qui ramènerait toute la base est interrompu en cours de lecture, pas après.
async fn read_capped(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> std::result::Result<String, ScrapeError> {
    let refused = (!response.status().is_success()).then(|| response.status().as_u16());
    let mut body = Vec::with_capacity(8 * 1024);
    loop {
        let chunk = response
            .chunk()
            .await
            .context("reading the answer from VictoriaMetrics")
            .map_err(ScrapeError::Unreachable)?;
        let Some(chunk) = chunk else { break };
        body.extend_from_slice(&chunk);
        if body.len() > max_bytes {
            return Err(ScrapeError::TooLarge { max_bytes });
        }
    }
    let text = String::from_utf8(body)
        .context("the answer from VictoriaMetrics is not valid UTF-8")
        .map_err(ScrapeError::Unreachable)?;
    match refused {
        Some(status) => Err(ScrapeError::Refused { status, detail: text.trim().to_string() }),
        None => Ok(text),
    }
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    data: QueryData,
}

#[derive(Debug, Deserialize)]
struct QueryData {
    result: Vec<RangeSeries>,
}

#[derive(Debug, Deserialize)]
struct SeriesResponse {
    data: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct InstantResponse {
    data: InstantData,
}

#[derive(Debug, Deserialize)]
struct InstantData {
    result: Vec<InstantSeries>,
}

/// Une série et ses points, tels que renvoyés par `query_range`.
#[derive(Debug, Deserialize)]
pub struct RangeSeries {
    #[serde(default)]
    pub metric: std::collections::BTreeMap<String, String>,
    /// Couples `[horodatage_secondes, "valeur"]` — VictoriaMetrics encode la valeur
    /// en chaîne, conformément à l'API Prometheus.
    #[serde(default)]
    pub values: Vec<(f64, String)>,
}

#[derive(Debug, Deserialize)]
pub struct InstantSeries {
    #[serde(default)]
    pub metric: std::collections::BTreeMap<String, String>,
    pub value: (f64, String),
}

/// Échantillons écartés depuis le démarrage parce qu'un nom de métrique ou
/// d'étiquette n'était pas valide.
pub static REJECTED_SAMPLES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Vrai si la chaîne d'erreurs indique une requête annulée (« context canceled »
/// côté VictoriaMetrics, requête abandonnée côté client) : bruit normal quand un
/// visiteur quitte une page, pas une panne de la base de séries.
pub fn is_cancellation(error: &anyhow::Error) -> bool {
    let chain = format!("{error:#}").to_lowercase();
    chain.contains("canceled") || chain.contains("cancelled")
}

/// Journalise l'échec d'une lecture des séries : `debug` pour une annulation,
/// `warn` sinon, avec la chaîne complète des causes — « délai dépassé » ou
/// « connexion refusée » plutôt que le seul contexte de l'appel.
pub fn log_query_failure(error: &anyhow::Error, what: &str) {
    let cancelled = is_cancellation(error);
    let error = format!("{error:#}");
    if cancelled {
        tracing::debug!(%error, "{what} (query cancelled)");
    } else {
        tracing::warn!(%error, "{what}");
    }
}

/// `[a-zA-Z_:][a-zA-Z0-9_:]*` une fois le préfixe ajouté : le préfixe commence
/// par une lettre, le reste n'a donc qu'à rester dans l'alphabet.
fn is_valid_metric_suffix(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

/// `[a-zA-Z_][a-zA-Z0-9_]*`, la grammaire Prometheus d'un nom d'étiquette.
fn is_valid_label_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Sérialise les échantillons au format d'exposition Prometheus, et compte
/// ceux qui ont été écartés pour un nom invalide.
///
/// C'est le seul point par où passe toute écriture — collecteurs du serveur,
/// agents, relais. Les noms y sont donc vérifiés, et non seulement échappés :
/// un nom de métrique ou d'étiquette contenant `{`, `"` ou un saut de ligne
/// fabriquerait une ligne de plus dans le flux, donc une série au nom de
/// n'importe quelle autre cible.
fn encode_prometheus(samples: &[Sample]) -> (String, usize) {
    let mut out = String::with_capacity(samples.len() * 96);
    let mut rejected = 0;
    for sample in samples {
        // Les valeurs non finies ne sont pas représentables et feraient rejeter tout
        // le lot par VictoriaMetrics : on les écarte silencieusement.
        if !sample.value.is_finite() {
            continue;
        }
        if !is_valid_metric_suffix(&sample.metric)
            || !sample.labels.keys().all(|name| is_valid_label_name(name))
        {
            rejected += 1;
            continue;
        }
        out.push_str(METRIC_PREFIX);
        out.push_str(&sample.metric);
        if !sample.labels.is_empty() {
            out.push('{');
            for (i, (name, value)) in sample.labels.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(name);
                out.push_str("=\"");
                escape_label_value(value, &mut out);
                out.push('"');
            }
            out.push('}');
        }
        out.push(' ');
        out.push_str(&sample.value.to_string());
        out.push(' ');
        out.push_str(&sample.ts_ms.to_string());
        out.push('\n');
    }
    (out, rejected)
}

/// Échappe une valeur d'étiquette selon les règles du format d'exposition.
fn escape_label_value(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::MetricKind;

    use super::*;

    /// VictoriaMetrics refuse `latency_offset=0s` (plage [1ms …]) : la valeur
    /// doit rester une durée strictement positive, sans quoi chaque lecture
    /// instantanée de l'interface part en 400.
    #[test]
    fn the_fresh_latency_offset_is_accepted_by_victoriametrics() {
        assert_eq!(FRESH_LATENCY_OFFSET, "1ms");
    }

    /// Un 4xx est la faute de la requête ; un 5xx (VictoriaMetrics surchargé)
    /// est une panne du backend et ne doit pas finir en 400.
    #[tokio::test]
    async fn only_a_client_error_from_victoriametrics_is_a_rejected_query() {
        let reply = |status: u16| {
            reqwest::Response::from(
                axum::http::Response::builder().status(status).body("détail").unwrap(),
            )
        };
        let rejected = refusal(reply(422)).await;
        assert!(rejected.downcast_ref::<QueryRejected>().is_some(), "{rejected:#}");
        let overloaded = refusal(reply(503)).await;
        assert!(overloaded.downcast_ref::<QueryRejected>().is_none(), "{overloaded:#}");
    }

    /// Un segment reste un segment : ni `..`, ni `%2e%2e`, ni `?` ne sortent
    /// de `/api/v1/` une fois l'URL construite.
    #[test]
    fn proxied_routes_cannot_leave_the_read_api() {
        let victoria = Victoria::new("http://127.0.0.1:8428").unwrap();
        let url = victoria
            .api_url(&["label", "%2e%2e", "a?b=c#d", "values"], Some("match[]=up"))
            .unwrap();
        assert!(url.path().starts_with("/api/v1/label/"), "{url}");
        assert!(!url.path().contains("/admin/"), "{url}");
        assert_eq!(url.query(), Some("match[]=up"));
        assert_eq!(url.fragment(), None);

        let url = victoria.api_url(&["query"], None).unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:8428/api/v1/query");
    }

    #[test]
    fn encodes_a_sample_with_labels() {
        let sample = Sample::new("if_octets_in", 1234.0, MetricKind::Counter, 1_700_000_000_000)
            .with_label("host", "sw1")
            .with_label("ifname", "eth0");
        assert_eq!(
            encode_prometheus(&[sample]).0,
            "dumbmonit_if_octets_in{host=\"sw1\",ifname=\"eth0\"} 1234 1700000000000\n"
        );
    }

    #[test]
    fn escapes_quotes_and_backslashes_in_label_values() {
        // Un alias d'interface Windows ressemble typiquement à ceci.
        let sample = Sample::new("if_up", 1.0, MetricKind::Gauge, 0)
            .with_label("ifalias", r#"Lien "backup" \ site B"#);
        let (encoded, _) = encode_prometheus(&[sample]);
        assert_eq!(encoded, "dumbmonit_if_up{ifalias=\"Lien \\\"backup\\\" \\\\ site B\"} 1 0\n");
    }

    #[test]
    fn drops_non_finite_values_instead_of_failing_the_batch() {
        let samples = vec![
            Sample::new("bad", f64::NAN, MetricKind::Gauge, 0),
            Sample::new("also_bad", f64::INFINITY, MetricKind::Gauge, 0),
            Sample::new("good", 1.0, MetricKind::Gauge, 0),
        ];
        assert_eq!(encode_prometheus(&samples), ("dumbmonit_good 1 0\n".to_string(), 0));
    }

    #[test]
    fn an_empty_batch_encodes_to_nothing() {
        assert_eq!(encode_prometheus(&[]), (String::new(), 0));
    }

    /// Un nom de métrique ou d'étiquette ne peut pas ajouter de ligne au flux :
    /// l'échantillon est écarté, compté, et le reste du lot part.
    #[test]
    fn invalid_names_cannot_inject_series() {
        let samples = vec![
            Sample::new(
                "up{target=\"12\"} 0 1759500000000\ndumbmonit_x",
                1.0,
                MetricKind::Gauge,
                0,
            ),
            Sample::new("up", 1.0, MetricKind::Gauge, 0)
                .with_label("a=\"1\"} 0 0\ndumbmonit_up{target", "12"),
            Sample::new("up", 1.0, MetricKind::Gauge, 0).with_label("9lives", "x"),
            Sample::new("", 1.0, MetricKind::Gauge, 0),
            Sample::new("good:total", 1.0, MetricKind::Gauge, 0).with_label("_ok", "v"),
        ];
        let (encoded, rejected) = encode_prometheus(&samples);
        assert_eq!(rejected, 4);
        assert_eq!(encoded, "dumbmonit_good:total{_ok=\"v\"} 1 0\n");
    }

    #[test]
    fn carriage_returns_in_label_values_are_escaped() {
        let sample =
            Sample::new("if_up", 1.0, MetricKind::Gauge, 0).with_label("ifalias", "a\r\nb");
        let (encoded, _) = encode_prometheus(&[sample]);
        assert_eq!(encoded.lines().count(), 1, "{encoded}");
        assert_eq!(encoded, "dumbmonit_if_up{ifalias=\"a\\r\\nb\"} 1 0\n");
    }
}
