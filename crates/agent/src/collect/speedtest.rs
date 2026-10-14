//! Test de débit Internet : téléchargement, envoi, latence et instabilité
//! contre un point de contrôle choisi par l'utilisateur.
//!
//! Désactivé par défaut : contrairement au reste de l'agent, ce test consomme
//! de la bande passante et, sur une ligne limitée ou facturée au volume, de
//! l'argent. `enabled: true` et une adresse de téléchargement le réveillent.
//!
//! Aucun protocole propriétaire : une simple requête HTTP GET sur l'adresse de
//! téléchargement, lue au plus jusqu'au plafond d'octets configuré, et un POST
//! sur l'adresse d'envoi. Ça fonctionne contre un LibreSpeed auto-hébergé
//! (`backend/garbage.php`, `backend/empty.php`), contre le point de test de
//! Cloudflare (`https://speed.cloudflare.com/__down?bytes=N`,
//! `https://speed.cloudflare.com/__up`) ou contre n'importe quel fichier et
//! point de dépôt HTTP, sans bibliothèque dédiée. La latence et l'instabilité
//! viennent de quelques requêtes HEAD sur l'adresse de téléchargement ; un
//! serveur qui les refuse laisse simplement ces deux séries absentes, le reste
//! du test n'en dépend pas.
//!
//! Comme pour les mises à jour d'images Docker, la mesure tourne dans une
//! tâche de fond, au plus une fois par période (six heures par défaut) : le
//! cycle de collecte ne fait que lire le dernier résultat connu.

use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};
use tokio::task::JoinHandle;
use tracing::{debug, warn};

/// Période par défaut entre deux tests. Six heures : un débit Internet ne
/// varie pas à la minute, et chaque test consomme de la bande passante.
pub const DEFAULT_INTERVAL_SECS: u64 = 6 * 3600;

/// En dessous, l'agent passerait son temps à saturer la ligne qu'il surveille.
pub const MIN_INTERVAL_SECS: u64 = 300;

/// Plafond d'octets téléchargés par défaut : cinquante mégaoctets suffisent à
/// un débit stable sans peser sur une ligne limitée.
pub const DEFAULT_DOWNLOAD_BYTES_CAP: u64 = 50_000_000;

/// Volume envoyé par défaut lors du test d'envoi.
pub const DEFAULT_UPLOAD_BYTES: u64 = 10_000_000;

/// Délai de l'ensemble d'un test. Généreux : une ligne lente ne doit pas être
/// jugée en panne simplement parce qu'elle est lente.
const TEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Délai de chaque mesure de latence, bien plus court : c'est une requête vide.
const PING_TIMEOUT: Duration = Duration::from_secs(5);

/// Nombre de mesures de latence par test, pour calculer l'instabilité.
const PING_COUNT: usize = 4;

/// Attente au tout premier cycle, pour qu'un `--dry-run` montre quelque chose
/// sans attendre six heures.
const FIRST_CYCLE_WAIT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, PartialEq)]
pub struct SpeedtestConfig {
    /// Faux par défaut : ce test consomme de la bande passante, il ne se
    /// déclenche jamais tout seul.
    pub enabled: bool,
    /// Adresse téléchargée pour mesurer le débit descendant. Sans elle, rien
    /// n'est testé même si `enabled` est vrai.
    pub download_url: Option<String>,
    /// Adresse à laquelle envoyer des données pour mesurer le débit montant.
    /// Sans elle, seuls le téléchargement et la latence sont mesurés.
    pub upload_url: Option<String>,
    /// Au-delà, la lecture du téléchargement s'arrête : c'est le débit mesuré
    /// jusque-là qui est publié, pas un débit tronqué qui semblerait en panne.
    pub download_bytes_cap: u64,
    /// Volume envoyé lors du test d'envoi.
    pub upload_bytes: u64,
    pub interval: Duration,
    /// Seuils d'alerte, publiés comme séries jumelles pour que les règles
    /// livrées les comparent à la mesure — absent, aucune règle ne peut se
    /// déclencher : c'est une option, pas un défaut imposé.
    pub min_download_bytes_per_second: Option<f64>,
    pub min_upload_bytes_per_second: Option<f64>,
    pub max_latency_ms: Option<f64>,
}

impl Default for SpeedtestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            download_url: None,
            upload_url: None,
            download_bytes_cap: DEFAULT_DOWNLOAD_BYTES_CAP,
            upload_bytes: DEFAULT_UPLOAD_BYTES,
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECS),
            min_download_bytes_per_second: None,
            min_upload_bytes_per_second: None,
            max_latency_ms: None,
        }
    }
}

/// Résultat d'un test, qu'il ait réussi ou non.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpeedtestReport {
    /// `None` : pas de requête réussie suffisamment longue pour mesurer, ou
    /// pas d'adresse de téléchargement configurée.
    pub download_bytes_per_second: Option<f64>,
    pub upload_bytes_per_second: Option<f64>,
    /// Plus petite des mesures : la plus proche du temps de trajet réel, les
    /// autres étant allongées par une éventuelle charge du serveur distant.
    pub latency_ms: Option<f64>,
    /// Écart entre la plus grande et la plus petite mesure.
    pub jitter_ms: Option<f64>,
    /// Faux si le téléchargement lui-même a échoué (le test n'a rien mesuré).
    pub ok: bool,
    /// Âge du test, en secondes.
    pub age_seconds: i64,
}

/// Traduit le rapport en échantillons. Les seuils ne sont publiés que s'ils
/// sont configurés : leur présence est ce qui arme les règles livrées.
pub fn samples(report: &SpeedtestReport, now_ms: i64) -> Vec<Sample> {
    let gauge = |metric: &str, value: f64| Sample::new(metric, value, MetricKind::Gauge, now_ms);
    let mut samples = vec![
        gauge("agent_speedtest_ok", f64::from(u8::from(report.ok))),
        gauge("agent_speedtest_age_seconds", report.age_seconds as f64),
    ];
    if let Some(value) = report.download_bytes_per_second {
        samples.push(gauge("agent_speedtest_download_bytes_per_second", value));
    }
    if let Some(value) = report.upload_bytes_per_second {
        samples.push(gauge("agent_speedtest_upload_bytes_per_second", value));
    }
    if let Some(value) = report.latency_ms {
        samples.push(gauge("agent_speedtest_latency_ms", value));
    }
    if let Some(value) = report.jitter_ms {
        samples.push(gauge("agent_speedtest_jitter_ms", value));
    }
    samples
}

/// Les seuils configurés, en séries jumelles des mesures qu'ils bornent.
pub fn threshold_samples(config: &SpeedtestConfig, now_ms: i64) -> Vec<Sample> {
    let gauge = |metric: &str, value: f64| Sample::new(metric, value, MetricKind::Gauge, now_ms);
    let mut samples = Vec::with_capacity(3);
    if let Some(value) = config.min_download_bytes_per_second {
        samples.push(gauge("agent_speedtest_download_min_bytes_per_second", value));
    }
    if let Some(value) = config.min_upload_bytes_per_second {
        samples.push(gauge("agent_speedtest_upload_min_bytes_per_second", value));
    }
    if let Some(value) = config.max_latency_ms {
        samples.push(gauge("agent_speedtest_max_latency_ms", value));
    }
    samples
}

/// Un test complet : téléchargement, envoi s'il est configuré, puis latence.
/// Aucune étape n'abandonne les autres : un envoi qui échoue laisse quand même
/// le téléchargement et la latence publiés.
async fn run_test(config: &SpeedtestConfig) -> SpeedtestReport {
    let http = match reqwest::Client::builder()
        .timeout(TEST_TIMEOUT)
        .user_agent(concat!("dumbmonit-agent/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(http) => http,
        Err(error) => {
            warn!(%error, "speedtest: cannot build the HTTP client");
            return SpeedtestReport { ok: false, ..SpeedtestReport::default() };
        }
    };

    let download = match &config.download_url {
        Some(url) => download(&http, url, config.download_bytes_cap).await,
        None => None,
    };
    let upload = match &config.upload_url {
        Some(url) => upload(&http, url, config.upload_bytes).await,
        None => None,
    };
    let (latency, jitter) = match &config.download_url {
        Some(url) => ping(&http, url).await,
        None => (None, None),
    };

    SpeedtestReport {
        download_bytes_per_second: download,
        upload_bytes_per_second: upload,
        latency_ms: latency,
        jitter_ms: jitter,
        // Le téléchargement est la mesure de référence : sans adresse d'envoi
        // configurée, son seul échec compte comme « test en échec ».
        ok: config.download_url.is_none() || download.is_some(),
        age_seconds: 0,
    }
}

/// Télécharge jusqu'au plafond (ou jusqu'à la fin du corps), et rend le débit
/// en octets par seconde. `None` : la requête elle-même a échoué.
async fn download(http: &reqwest::Client, url: &str, cap: u64) -> Option<f64> {
    let started = tokio::time::Instant::now();
    let mut response = match http.get(url).send().await {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            debug!(url, status = %response.status(), "speedtest: download answered an error");
            return None;
        }
        Err(error) => {
            debug!(url, %error, "speedtest: download request failed");
            return None;
        }
    };
    let mut received: u64 = 0;
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                received += chunk.len() as u64;
                if received >= cap {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                debug!(url, %error, "speedtest: download interrupted mid-stream");
                break;
            }
        }
    }
    rate(received, started.elapsed())
}

/// Envoie un volume fixe d'octets et rend le débit en octets par seconde.
async fn upload(http: &reqwest::Client, url: &str, bytes: u64) -> Option<f64> {
    let payload = vec![0u8; bytes as usize];
    let started = tokio::time::Instant::now();
    match http.post(url).body(payload).send().await {
        Ok(response) if response.status().is_success() => rate(bytes, started.elapsed()),
        Ok(response) => {
            debug!(url, status = %response.status(), "speedtest: upload answered an error");
            None
        }
        Err(error) => {
            debug!(url, %error, "speedtest: upload request failed");
            None
        }
    }
}

/// Quelques requêtes `HEAD`, bon marché, pour approcher la latence et son
/// instabilité. Un serveur qui n'y répond pas laisse les deux à `None` plutôt
/// que de faire échouer tout le test.
async fn ping(http: &reqwest::Client, url: &str) -> (Option<f64>, Option<f64>) {
    let mut samples = Vec::with_capacity(PING_COUNT);
    for _ in 0..PING_COUNT {
        let started = tokio::time::Instant::now();
        let result = tokio::time::timeout(PING_TIMEOUT, http.head(url).send()).await;
        if matches!(result, Ok(Ok(_))) {
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    if samples.is_empty() {
        return (None, None);
    }
    let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (Some(min), Some(max - min))
}

/// Octets par seconde, sans division par zéro : une mesure plus courte qu'une
/// dizaine de millisecondes n'est pas fiable et ne doit rien publier.
fn rate(bytes: u64, elapsed: Duration) -> Option<f64> {
    let secs = elapsed.as_secs_f64();
    if secs < 0.01 || bytes == 0 {
        return None;
    }
    Some(bytes as f64 / secs)
}

/// Lecteur du test, conservé d'un cycle à l'autre pour porter le cache et la
/// tâche de fond — même mécanique que [`super::plakar::PlakarProbe`].
pub struct SpeedtestProbe {
    config: SpeedtestConfig,
    last: Option<SpeedtestReport>,
    refreshed_at: Option<tokio::time::Instant>,
    task: Option<JoinHandle<SpeedtestReport>>,
    announced: bool,
}

impl SpeedtestProbe {
    pub fn new(config: &SpeedtestConfig) -> Self {
        Self {
            config: config.clone(),
            last: None,
            refreshed_at: None,
            task: None,
            announced: false,
        }
    }

    /// Un cycle : relance le test s'il a vieilli, récolte le précédent s'il a
    /// fini, et rend le dernier résultat connu avec son âge à jour. `None` :
    /// désactivé, pas d'adresse configurée, ou premier test pas encore terminé.
    pub async fn read(&mut self) -> Option<SpeedtestReport> {
        if !self.config.enabled || self.config.download_url.is_none() {
            if self.config.enabled && !self.announced {
                debug!("speedtest enabled but no download_url configured, nothing measured");
                self.announced = true;
            }
            return None;
        }

        if self.task.is_none() {
            let stale = self.refreshed_at.is_none_or(|at| at.elapsed() >= self.config.interval);
            if stale {
                let config = self.config.clone();
                self.task = Some(tokio::spawn(async move { run_test(&config).await }));
            }
        }

        if let Some(task) = self.task.as_mut() {
            let first_cycle = self.refreshed_at.is_none();
            if (task.is_finished() || first_cycle)
                && let Ok(outcome) = tokio::time::timeout(FIRST_CYCLE_WAIT, task).await
            {
                self.task = None;
                self.refreshed_at = Some(tokio::time::Instant::now());
                match outcome {
                    Ok(report) => {
                        if !report.ok {
                            warn!(
                                url = self.config.download_url.as_deref().unwrap_or(""),
                                "speedtest failed: no throughput measured"
                            );
                        }
                        self.last = Some(report);
                    }
                    Err(error) => debug!(%error, "speedtest task aborted"),
                }
            }
        }

        self.last.map(|mut report| {
            report.age_seconds =
                self.refreshed_at.map(|at| at.elapsed().as_secs() as i64).unwrap_or(0);
            report
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_transfer_is_not_measured() {
        assert_eq!(rate(100, Duration::from_millis(1)), None);
        assert_eq!(rate(0, Duration::from_secs(1)), None);
        assert_eq!(rate(1_000_000, Duration::from_secs(1)), Some(1_000_000.0));
    }

    #[test]
    fn samples_only_carry_what_was_measured() {
        let report = SpeedtestReport {
            download_bytes_per_second: Some(12_500_000.0),
            ok: true,
            age_seconds: 10,
            ..SpeedtestReport::default()
        };
        let series = samples(&report, 1_000);
        assert!(series.iter().any(|s| s.metric == "agent_speedtest_download_bytes_per_second"));
        assert!(series.iter().all(|s| s.metric != "agent_speedtest_upload_bytes_per_second"));
        assert!(series.iter().all(|s| s.metric != "agent_speedtest_latency_ms"));
        assert_eq!(
            series.iter().find(|s| s.metric == "agent_speedtest_ok").map(|s| s.value),
            Some(1.0)
        );
    }

    #[test]
    fn thresholds_are_published_only_when_configured() {
        let config = SpeedtestConfig::default();
        assert!(threshold_samples(&config, 0).is_empty());

        let config = SpeedtestConfig {
            min_download_bytes_per_second: Some(2_500_000.0),
            max_latency_ms: Some(50.0),
            ..SpeedtestConfig::default()
        };
        let series = threshold_samples(&config, 0);
        assert_eq!(series.len(), 2);
        assert!(series.iter().any(|s| s.metric == "agent_speedtest_download_min_bytes_per_second"
            && s.value == 2_500_000.0));
    }

    #[tokio::test]
    async fn disabled_by_default_nothing_is_measured() {
        let mut probe = SpeedtestProbe::new(&SpeedtestConfig::default());
        assert!(probe.read().await.is_none());
    }

    #[tokio::test]
    async fn enabled_without_a_download_url_measures_nothing() {
        let mut probe =
            SpeedtestProbe::new(&SpeedtestConfig { enabled: true, ..SpeedtestConfig::default() });
        assert!(probe.read().await.is_none());
    }

    #[tokio::test]
    async fn a_failing_download_is_reported_as_not_ok() {
        let config = SpeedtestConfig {
            enabled: true,
            download_url: Some("http://127.0.0.1:1/nope".to_string()),
            interval: Duration::from_secs(MIN_INTERVAL_SECS),
            ..SpeedtestConfig::default()
        };
        let report = run_test(&config).await;
        assert!(!report.ok);
        assert_eq!(report.download_bytes_per_second, None);
    }
}
