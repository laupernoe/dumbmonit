//! Moniteurs en poussée (heartbeat) : l'URL que le travail surveillé appelle, et
//! ce que l'interface en montre.
//!
//! Deux façades :
//!
//! - [`public_routes`] : `GET|POST /api/push/{token}` (et `/start`, `/success`,
//!   `/fail`, `/<code de sortie>` à la suite), appelée par le cron, le script ou
//!   l'automatisation. Sans session ni en-tête anti-CSRF, par conception —
//!   c'est une ligne `curl` dans une crontab, pas un navigateur. Le jeton est la
//!   seule authentification, et il ne permet que de dire « j'ai tourné ». À
//!   merger dans le routeur public, à côté de `/api/ingest`. Le corps d'un POST
//!   est la sortie du travail, tronquée à 10 Ko.
//! - [`ui_routes`] : lecture du moniteur d'une cible (jeton compris, pour
//!   l'afficher avec un bouton « copier »), historique de ses exécutions et
//!   régénération du jeton. Sous le garde de session comme le reste de
//!   l'administration.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::Identity;
use crate::collectors::push::{self, Run, Settings, Status, Verdict, store, token};
use crate::db;
use crate::state::AppState;

/// Corps accepté par appel : la sortie d'un travail bavard, avant d'être
/// tronquée à [`store::MAX_OUTPUT_BYTES`]. Au-delà, 413.
const MAX_BODY_BYTES: usize = 256 * 1024;

pub fn public_routes() -> Router<AppState> {
    // `get` répond aussi à HEAD ; POST pour les clients qui préfèrent.
    Router::new()
        .route("/push/{token}", get(receive).post(receive))
        .route("/push/{token}/{action}", get(receive_action).post(receive_action))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}

pub fn ui_routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/push", get(get_monitor))
        .route("/targets/{id}/push/runs", get(list_runs))
        .route("/targets/{id}/push/regenerate", post(regenerate))
}

// ----------------------------------------------------------- appel public

/// Ce qu'un appel peut dire de lui-même, dans la chaîne de requête. Les noms
/// sont ceux du moniteur push d'Uptime Kuma, pour qu'un script écrit pour lui
/// fonctionne tel quel.
#[derive(Debug, Default, Deserialize)]
pub struct PingQuery {
    /// `up` (défaut) ou `down` : le script signale lui-même un échec.
    #[serde(default)]
    status: Option<String>,
    /// Mot d'explication libre, conservé tronqué.
    #[serde(default)]
    msg: Option<String>,
    /// Code de sortie du travail : différent de zéro, c'est un échec.
    #[serde(default)]
    exit_code: Option<String>,
}

/// Ce que l'appel annonce, d'après le dernier segment du chemin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// `/api/push/{token}` : un signe de vie, un succès sauf `status=down`.
    Ping,
    Start,
    Success,
    Fail,
    /// `/api/push/{token}/<code>` : la fin du travail avec son code de sortie.
    Code(i64),
}

impl Action {
    fn parse(segment: &str) -> Option<Self> {
        match segment {
            "start" => Some(Self::Start),
            "success" => Some(Self::Success),
            "fail" => Some(Self::Fail),
            digits => {
                let numeric = !digits.is_empty()
                    && digits.len() <= 9
                    && digits.bytes().all(|b| b.is_ascii_digit());
                if numeric { digits.parse().ok().map(Self::Code) } else { None }
            }
        }
    }
}

fn bad_request(message: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": message }))).into_response()
}

/// Appels tolérés par jeton et par fenêtre glissante. Un travail planifié
/// appelle une fois par exécution ; soixante par minute couvre le script qui
/// signale son début et sa fin, et arrête une boucle folle.
const LIMIT_PER_WINDOW: u32 = 60;
const WINDOW: Duration = Duration::from_secs(60);
/// Au-delà, les compteurs inactifs sont purgés à chaque écriture.
const PURGE_ABOVE: usize = 4_096;

/// Compteurs par empreinte de jeton, en mémoire : une instance est un processus
/// unique, et un compteur qui survit à un redémarrage n'apporterait rien.
static LIMITER: LazyLock<Mutex<HashMap<String, (Instant, u32)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Autorise l'appel, ou rend le nombre de secondes à attendre.
fn check_rate(token_hash: &str, now: Instant) -> Result<(), u64> {
    let mut buckets = LIMITER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if buckets.len() > PURGE_ABOVE {
        buckets.retain(|_, (started, _)| now.duration_since(*started) < WINDOW);
    }
    let (started, count) = buckets.entry(token_hash.to_string()).or_insert((now, 0));
    if now.duration_since(*started) >= WINDOW {
        *started = now;
        *count = 0;
    }
    if *count >= LIMIT_PER_WINDOW {
        return Err((WINDOW - now.duration_since(*started)).as_secs() + 1);
    }
    *count += 1;
    Ok(())
}

/// `GET|POST /api/push/{token}` : le travail surveillé signale qu'il a tourné.
async fn receive(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<PingQuery>,
    body: Bytes,
) -> Response {
    handle(&state, &token, Action::Ping, query, &body).await
}

/// `GET|POST /api/push/{token}/{action}` : `start`, `success`, `fail` ou un code
/// de sortie. Un segment inconnu répond 404, comme un jeton inconnu.
async fn receive_action(
    State(state): State<AppState>,
    Path((token, action)): Path<(String, String)>,
    Query(query): Query<PingQuery>,
    body: Bytes,
) -> Response {
    match Action::parse(&action) {
        Some(action) => handle(&state, &token, action, query, &body).await,
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Le coeur des points d'entrée.
///
/// 204 quand l'appel est enregistré, 404 pour un jeton inconnu — la même réponse
/// qu'un jeton mal formé, pour ne rien apprendre à qui cherche —, 429 au-delà
/// de la cadence tolérée, 400 pour un `status` qui n'est ni `up` ni `down` ou un
/// `exit_code` qui n'est pas un entier. Les corps de réponse sont vides :
/// `curl -fsS` dans une crontab n'a rien à lire.
///
/// Un code de sortie non nul, `/fail` et `?status=down` sont des échecs ; le
/// corps de la requête (ou à défaut `msg`) est la sortie conservée.
async fn handle(
    state: &AppState,
    token: &str,
    action: Action,
    query: PingQuery,
    body: &[u8],
) -> Response {
    if !token::is_well_formed(token) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let status = match query.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Status::Up,
        Some(raw) if raw.eq_ignore_ascii_case("up") => Status::Up,
        Some(raw) if raw.eq_ignore_ascii_case("down") => Status::Down,
        Some(_) => return bad_request("\"status\" must be \"up\" or \"down\"."),
    };
    let query_code = match query.exit_code.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(raw) => match raw.parse::<i32>() {
            Ok(code) if code >= 0 => Some(i64::from(code)),
            _ => return bad_request("\"exit_code\" must be a non-negative integer."),
        },
    };

    let hash = token::fingerprint(token);
    if let Err(wait) = check_rate(&hash, Instant::now()) {
        let mut headers = HeaderMap::new();
        headers.insert(header::RETRY_AFTER, HeaderValue::from(wait));
        return (StatusCode::TOO_MANY_REQUESTS, headers).into_response();
    }

    let exit_code = match action {
        Action::Code(code) => Some(code),
        _ => query_code,
    };
    // `status=down` ne vaut que pour l'appel simple : c'est la forme d'Uptime Kuma.
    let declared = match action {
        Action::Ping | Action::Success => status,
        _ => Status::Up,
    };
    let failed = match action {
        Action::Start => false,
        Action::Fail => true,
        _ => declared == Status::Down || exit_code.is_some_and(|code| code != 0),
    };

    let now_ms = chrono::Utc::now().timestamp_millis();
    let message = query.msg.as_deref().unwrap_or_default().trim();
    let target_id = match store::record_ping(&state.pool, &hash, now_ms, declared, message).await {
        Ok(Some(id)) => id,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return ApiError::Internal(error).into_response(),
    };
    debug!(target = target_id, ?action, failed, "heartbeat received");

    let recorded = if action == Action::Start {
        store::start_run(&state.pool, target_id, now_ms).await
    } else {
        let raw = if body.is_empty() { message.as_bytes() } else { body };
        let output = store::truncate_output(raw);
        store::finish_run(&state.pool, target_id, now_ms, failed, exit_code, &output).await
    };
    if let Err(error) = recorded {
        return ApiError::Internal(error).into_response();
    }

    // Le verdict est réécrit tout de suite plutôt qu'au prochain passage du
    // planificateur : l'alerte « heartbeat manqué » se résout dès que le travail
    // reprend, sans attendre jusqu'à une période de plus.
    refresh(state, target_id).await;

    StatusCode::NO_CONTENT.into_response()
}

/// Rejoue le contrôle de fraîcheur d'une cible et range son résultat, comme le
/// ferait le planificateur. Silencieux en cas d'échec : l'appel est déjà
/// enregistré, le planificateur repassera.
async fn refresh(state: &AppState, target_id: TargetId) {
    let target = match db::targets::get(&state.pool, &state.cipher, target_id).await {
        Ok(Some(target)) if target.enabled && target.kind == push::KIND => target,
        Ok(_) => return,
        Err(error) => {
            warn!(target = target_id, ?error, "cannot reload the heartbeat target");
            return;
        }
    };
    let outcome = state.collectors.probe(&target, state.config.probe_timeout).await;
    let error = match outcome {
        Ok(samples) => {
            state.sink.send(samples).await;
            None
        }
        Err(error) => Some(error.to_string()),
    };
    if let Err(error) = db::targets::record_probe(&state.pool, target_id, error.as_deref()).await {
        warn!(target = target_id, ?error, "cannot record the heartbeat verdict");
    }
}

// ----------------------------------------------------------- interface

#[derive(Debug, Default, Deserialize)]
pub struct RunsQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    before: Option<i64>,
}

const DEFAULT_RUNS_PAGE: i64 = 50;
const MAX_RUNS_PAGE: i64 = 100;

/// Le moniteur d'une cible, jeton compris pour un administrateur.
///
/// Le jeton est bien renvoyé à un administrateur (compte `admin` ou jeton
/// `write`), contrairement aux autres secrets : l'utilisateur doit pouvoir le
/// recopier dans une crontab longtemps après l'avoir créé. Il permet pourtant
/// de déclarer « le travail a tourné » — donc de masquer une sauvegarde qui
/// échoue : un lecteur n'en reçoit ni le jeton ni le chemin (`null`).
#[derive(Debug, Serialize)]
pub struct PushMonitorView {
    pub target_id: TargetId,
    pub token: Option<String>,
    /// Chemin à appeler, relatif à l'adresse du serveur : `/api/push/<token>`.
    /// L'interface le complète avec l'origine qu'elle voit — le serveur ne
    /// connaît pas son adresse publique.
    pub path: Option<String>,
    pub last_seen_at: Option<String>,
    /// Âge du dernier appel en secondes, `None` tant qu'aucun n'a été reçu.
    pub last_seen_age_secs: Option<u64>,
    /// `up` ou `down` : ce que le dernier appel a déclaré.
    pub last_status: &'static str,
    pub last_message: String,
    pub received_total: i64,
    pub created_at: String,
    /// Période attendue et tolérance effectives, en secondes ; `None` quand les
    /// options sont illisibles (`settings_error` dit pourquoi).
    pub expected_interval_secs: Option<u64>,
    pub grace_secs: Option<u64>,
    pub settings_error: Option<String>,
    /// `waiting` (aucun appel encore), `on_time`, `missed`, `reported_down`.
    pub verdict: &'static str,
    /// Planning cron et fuseau, quand le moniteur en a un (sinon l'intervalle).
    pub schedule: Option<String>,
    pub timezone: Option<String>,
    /// Prochaine échéance du planning cron, UTC sans suffixe.
    pub next_expected_at: Option<String>,
    pub max_duration_secs: Option<u64>,
    /// Dernière exécution connue (en cours ou terminée).
    pub last_run: Option<PushRunView>,
}

/// Une exécution du travail, telle que l'interface la montre.
#[derive(Debug, Serialize)]
pub struct PushRunView {
    pub id: i64,
    /// UTC sans suffixe ; `None` quand aucun `/start` n'a précédé la fin.
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_secs: Option<f64>,
    /// `running`, `success`, `failed` ou `lost`.
    pub status: &'static str,
    pub exit_code: Option<i64>,
    /// Sortie du travail, tronquée à 10 Ko.
    pub output: String,
}

/// Une page de l'historique des exécutions.
#[derive(Debug, Serialize)]
pub struct PushRunsPage {
    pub runs: Vec<PushRunView>,
    /// Curseur à passer en `before` pour la page suivante ; `None` à la fin.
    pub next_before: Option<i64>,
}

fn utc_text(ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ms).map(|at| at.format("%Y-%m-%dT%H:%M:%S").to_string())
}

fn run_view(run: Run) -> PushRunView {
    PushRunView {
        id: run.id,
        started_at: run.started_ms.and_then(utc_text),
        finished_at: run.finished_ms.and_then(utc_text),
        duration_secs: run.duration_ms.map(|ms| ms as f64 / 1000.0),
        status: run.status.as_str(),
        exit_code: run.exit_code,
        output: run.output,
    }
}

fn verdict_label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Waiting => "waiting",
        Verdict::OnTime => "on_time",
        Verdict::Missed => "missed",
        Verdict::ReportedDown => "reported_down",
    }
}

async fn load_push_target(state: &AppState, id: TargetId) -> ApiResult<dumbmonit_proto::Target> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    if target.kind != push::KIND {
        return Err(ApiError::BadRequest(format!(
            "Monitor {id} is a \"{}\" monitor, not a heartbeat monitor.",
            target.kind
        )));
    }
    Ok(target)
}

fn view(
    target: &dumbmonit_proto::Target,
    monitor: store::Monitor,
    last_run: Option<Run>,
    reveal_token: bool,
) -> PushMonitorView {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let (settings, settings_error) = match Settings::from_target(target) {
        Ok(settings) => (Some(settings), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let verdict = match &settings {
        Some(settings) => {
            push::evaluate(monitor.last_seen_ms, monitor.last_status, now_ms, settings)
        }
        None if monitor.last_seen_ms.is_none() => Verdict::Waiting,
        None => Verdict::OnTime,
    };
    let schedule = settings.as_ref().and_then(|s| s.schedule.as_ref());
    PushMonitorView {
        target_id: monitor.target_id,
        path: reveal_token.then(|| format!("/api/push/{}", monitor.token)),
        token: reveal_token.then_some(monitor.token),
        last_seen_at: monitor.last_seen_at,
        last_seen_age_secs: monitor.last_seen_ms.map(|then| push::age(then, now_ms).as_secs()),
        last_status: monitor.last_status.as_str(),
        last_message: monitor.last_message,
        received_total: monitor.received_total,
        created_at: monitor.created_at,
        expected_interval_secs: settings.as_ref().map(|s| s.expected.as_secs()),
        grace_secs: settings.as_ref().map(|s| s.grace_at(now_ms).as_secs()),
        settings_error,
        verdict: verdict_label(verdict),
        schedule: schedule.map(|s| s.expression().to_string()),
        timezone: schedule.map(|s| s.timezone_name().to_string()),
        next_expected_at: schedule.and_then(|s| s.next_after(now_ms)).and_then(utc_text),
        max_duration_secs: settings.as_ref().and_then(|s| s.max_duration).map(|d| d.as_secs()),
        last_run: last_run.map(run_view),
    }
}

/// `GET /api/targets/{id}/push` : le moniteur. Pour un administrateur, il est
/// créé au passage s'il n'existait pas encore (cible créée par un client qui
/// ignore ce type, ou changée de type) ; un lecteur ne crée rien, et ne voit
/// pas le jeton.
async fn get_monitor(
    State(state): State<AppState>,
    Identity(principal): Identity,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<PushMonitorView>> {
    let target = load_push_target(&state, id).await?;
    let admin = principal.is_admin();
    let monitor = if admin {
        store::ensure(&state.pool, &state.cipher, id).await?
    } else {
        match store::get(&state.pool, &state.cipher, id).await? {
            Some(monitor) => monitor,
            None => store::Monitor {
                target_id: id,
                token: String::new(),
                last_seen_ms: None,
                last_seen_at: None,
                last_status: store::Status::Up,
                last_message: String::new(),
                received_total: 0,
                created_at: String::new(),
            },
        }
    };
    let last_run = latest_run(&state, id).await?;
    Ok(Json(view(&target, monitor, last_run, admin)))
}

/// La dernière exécution connue : celle qui tourne, sinon la dernière terminée.
async fn latest_run(state: &AppState, id: TargetId) -> ApiResult<Option<Run>> {
    let runs = store::list_runs(&state.pool, id, None, 1).await?;
    Ok(runs.into_iter().next())
}

/// `GET /api/targets/{id}/push/runs` : l'historique des exécutions, les plus
/// récentes d'abord, par pages (`limit`, 50 par défaut et 100 au plus ; `before`
/// est le `next_before` de la page précédente).
async fn list_runs(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Query(query): Query<RunsQuery>,
) -> ApiResult<Json<PushRunsPage>> {
    load_push_target(&state, id).await?;
    let limit = query.limit.unwrap_or(DEFAULT_RUNS_PAGE).clamp(1, MAX_RUNS_PAGE);
    // Une de plus que demandé, pour savoir s'il reste une page.
    let mut runs = store::list_runs(&state.pool, id, query.before, limit + 1).await?;
    let more = runs.len() as i64 > limit;
    runs.truncate(limit as usize);
    let next_before = if more { runs.last().map(|run| run.id) } else { None };
    Ok(Json(PushRunsPage { runs: runs.into_iter().map(run_view).collect(), next_before }))
}

/// `POST /api/targets/{id}/push/regenerate` : nouveau jeton, l'ancienne URL
/// répond 404 dès maintenant.
async fn regenerate(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<PushMonitorView>> {
    let target = load_push_target(&state, id).await?;
    let monitor = store::regenerate(&state.pool, &state.cipher, id).await?;
    let last_run = latest_run(&state, id).await?;
    Ok(Json(view(&target, monitor, last_run, true)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cadence_est_bornee_par_jeton_et_par_fenetre() {
        let now = Instant::now();
        for _ in 0..LIMIT_PER_WINDOW {
            assert!(check_rate("jeton-a", now).is_ok());
        }
        let wait = check_rate("jeton-a", now).unwrap_err();
        assert!((1..=WINDOW.as_secs() + 1).contains(&wait), "attente : {wait}");
        // Un autre jeton n'est pas pénalisé.
        assert!(check_rate("jeton-b", now).is_ok());
        // La fenêtre passée, le compteur repart.
        assert!(check_rate("jeton-a", now + WINDOW).is_ok());
    }
}
