//! Le panneau des proxys inverses (Traefik, Caddy, Nginx Proxy Manager) et de
//! l'enregistrement d'un nom de domaine (`collectors/{traefik,caddy,npm,domain}`).
//!
//! `GET /targets/{id}/proxy` reconstruit la vue à partir des séries de la cible
//! dans VictoriaMetrics, sans jamais réinterroger le proxy ni le registre : même
//! forme que celle des serveurs de journaux et des services d'arrière-plan
//! (`observability.rs`, `backends.rs`), affichée par le même panneau.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_proto::TargetId;

use crate::api::backends::{Series, check, figure, label_of, names, plural, row};
use crate::api::observability::{Breakdown, ObservabilityView, Row};
use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;

pub const KINDS: [&str; 4] = ["traefik", "caddy", "npm", "domain"];

/// Un certificat renouvelé automatiquement l'est trente jours avant son
/// terme : à quatorze, le renouvellement a échoué (même seuil que la règle).
const CERT_WARNING_DAYS: f64 = 14.0;
/// Au-delà de 5 % de réponses 5xx sur dix minutes, le panneau le signale.
const ERROR_SHARE_PERCENT: f64 = 5.0;
const DAY: f64 = 86_400.0;
/// Lignes montrées par tableau.
const ROWS_SHOWN: usize = 12;

pub fn routes() -> Router<AppState> {
    Router::new().route("/targets/{id}/proxy", get(overview))
}

async fn overview(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<ObservabilityView>> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device {id} not found.")))?;
    let kind = target.kind.as_str();
    if !KINDS.contains(&kind) {
        return Err(ApiError::NotFound(format!(
            "Device {id} is not a Traefik, Caddy or Nginx Proxy Manager server, nor a domain."
        )));
    }
    // Un domaine se vérifie parfois une fois par jour : sa dernière valeur
    // reste lisible un jour durant.
    let lookback = if kind == "domain" { "25h" } else { "1h" };
    let selector = format!(r#"{{__name__=~"dumbmonit_{kind}_.*", target="{id}"}}"#);
    let counters = format!(r#"{{__name__=~"dumbmonit_{kind}_.*_total", target="{id}"}}"#);
    let last = format!("last_over_time({selector}[{lookback}]) keep_metric_names");
    let rates = format!("rate({counters}[10m]) keep_metric_names");
    let day = format!("increase_prometheus({counters}[24h]) keep_metric_names");
    let (last, rates, day) = tokio::try_join!(
        state.victoria.query(&last),
        state.victoria.query(&rates),
        state.victoria.query(&day),
    )?;
    let now = chrono::Utc::now().timestamp() as f64;
    Ok(Json(build_view(kind, &Series::new(kind, &last, &rates, &day, &[]), now)))
}

pub fn build_view(kind: &str, s: &Series, now: f64) -> ObservabilityView {
    let mut view = ObservabilityView {
        kind: kind.to_string(),
        version: s.label("version_info", "version"),
        sampled_at: s.sampled_at(),
        ..Default::default()
    };
    if view.sampled_at.is_none() {
        view.state = "ok".to_string();
        return view;
    }
    match kind {
        "traefik" => traefik(&mut view, s),
        "caddy" => caddy(&mut view, s),
        "npm" => npm(&mut view, s),
        _ => domain(&mut view, s, now),
    }
    view.state = if view.checks.iter().any(|c| c.state == "warning") {
        "warning"
    } else if view.checks.iter().any(|c| c.state == "advisory") {
        "advisory"
    } else {
        "ok"
    }
    .to_string();
    view
}

/// La part de 5xx sur dix minutes, en pour cent, s'il y a eu du trafic.
fn error_share(s: &Series, total: &str, errors: &str) -> Option<f64> {
    let total = s.rate(total)?;
    (total > 0.0).then(|| s.rate(errors).unwrap_or(0.0) / total * 100.0)
}

fn error_check(view: &mut ObservabilityView, share: Option<f64>) {
    if let Some(share) = share.filter(|p| *p >= ERROR_SHARE_PERCENT) {
        view.checks.push(check(
            "Errors",
            "advisory",
            format!("{share:.0}% of the answers of the last ten minutes were 5xx errors."),
        ));
    }
}

/// Le tableau et la vérification des certificats, communs à Traefik et NPM :
/// `metric` porte les jours restants, `key` le nom du certificat.
fn certificates(view: &mut ObservabilityView, s: &Series, metric: &str, key: &str) {
    let mut rows: Vec<Row> = s
        .all(metric)
        .map(|e| Row {
            label: label_of(e, key),
            value: Some(e.value * DAY),
            unit: "seconds",
            state: Some(if e.value < CERT_WARNING_DAYS { "warning" } else { "ok" }),
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    rows.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));
    let soon: Vec<String> =
        s.all(metric).filter(|e| e.value < CERT_WARNING_DAYS).map(|e| label_of(e, key)).collect();
    let next = s.all(metric).map(|e| e.value).reduce(f64::min).unwrap_or_default();
    if soon.is_empty() {
        view.checks.push(check(
            "Certificates",
            "ok",
            format!(
                "{}; the next one expires in {}.",
                plural(rows.len() as f64, "certificate", "certificates"),
                plural(next.floor(), "day", "days")
            ),
        ));
    } else {
        view.checks.push(check(
            "Certificates",
            "warning",
            format!(
                "Expiring in less than {CERT_WARNING_DAYS:.0} days, so not renewed: {}.",
                names(&soon)
            ),
        ));
    }
    view.figures.push(figure("Next certificate expiry", Some(next * DAY), "seconds"));
    rows.truncate(ROWS_SHOWN);
    view.breakdowns.push(Breakdown {
        title: "Certificates".to_string(),
        note: "Time left before each certificate expires, soonest first.".to_string(),
        rows,
    });
}

// ---------------------------------------------------------------- Traefik

fn traefik(view: &mut ObservabilityView, s: &Series) {
    let disabled: Vec<String> = s
        .all("router_status")
        .filter(|e| e.value >= 2.0)
        .map(|e| {
            let router = label_of(e, "router");
            let error = s
                .all("router_error_info")
                .find(|i| label_of(i, "router") == router)
                .map(|i| format!(" ({})", label_of(i, "error")))
                .unwrap_or_default();
            format!("{router}{error}")
        })
        .collect();
    let warned = s.all("router_status").filter(|e| (1.0..2.0).contains(&e.value)).count();
    let routers = s.get("routers").unwrap_or(0.0);
    if !disabled.is_empty() {
        view.checks.push(check(
            "Routers",
            "warning",
            format!(
                "Disabled by a configuration error, their sites are down: {}.",
                names(&disabled)
            ),
        ));
    } else if warned > 0 {
        view.checks.push(check(
            "Routers",
            "advisory",
            format!(
                "{} with a warning in Traefik's dashboard.",
                plural(warned as f64, "router", "routers")
            ),
        ));
    } else {
        view.checks.push(check(
            "Routers",
            "ok",
            format!("{}, all enabled.", plural(routers, "router", "routers")),
        ));
    }

    // Serveurs d'arrière-plan, par service.
    let mut dead = Vec::new();
    let mut partial = Vec::new();
    let mut rows = Vec::new();
    for entry in s.all("service_servers") {
        let service = label_of(entry, "service");
        let up = s.sibling("service_servers_up", entry, &["service"]).unwrap_or(0.0);
        let state = if up < 1.0 && entry.value > 0.0 {
            dead.push(service.clone());
            "warning"
        } else if up < entry.value {
            partial.push(service.clone());
            "advisory"
        } else {
            "ok"
        };
        rows.push(Row { label: service, value: Some(up), unit: "count", state: Some(state) });
    }
    if !dead.is_empty() {
        view.checks.push(check(
            "Backend servers",
            "warning",
            format!("No server answers its health check, the sites get 503: {}.", names(&dead)),
        ));
    } else if !partial.is_empty() {
        view.checks.push(check(
            "Backend servers",
            "advisory",
            format!("Some servers fail their health check: {}.", names(&partial)),
        ));
    } else if !rows.is_empty() {
        view.checks.push(check("Backend servers", "ok", "Every server answers."));
    }

    let waiting: Vec<String> = s
        .all("resolver_routers_uncovered")
        .filter(|e| e.value >= 1.0)
        .map(|e| label_of(e, "resolver"))
        .collect();
    if !waiting.is_empty() {
        view.checks.push(check(
            "Certificate resolvers",
            "warning",
            format!(
                "Routers still wait for a certificate from {}: Traefik serves its default \
                 self-signed certificate instead.",
                names(&waiting)
            ),
        ));
    }
    certificates(view, s, "cert_expiry_days", "cert");
    let share = error_share(s, "requests_total", "requests_5xx_total");
    error_check(view, share);

    view.figures.insert(0, figure("Routers", s.get("routers"), "count"));
    view.figures.insert(1, figure("Services", s.get("services"), "count"));
    view.figures.push(figure("Requests", s.rate("requests_total"), "per_second"));
    view.figures.push(figure("5xx share", share, "percent"));

    rows.sort_by(|a, b| a.label.cmp(&b.label));
    rows.truncate(ROWS_SHOWN);
    if !rows.is_empty() {
        view.breakdowns.insert(
            0,
            Breakdown {
                title: "Services".to_string(),
                note: "Servers answering their health check, per service.".to_string(),
                rows,
            },
        );
    }
    let mut errors: Vec<Row> = s
        .all_day("router_requests_5xx_total")
        .filter(|e| e.value >= 0.5)
        .map(|e| row(label_of(e, "router"), Some(e.value), "count"))
        .collect();
    errors.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
    errors.truncate(ROWS_SHOWN);
    if !errors.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "5xx answers in 24 h".to_string(),
            note: "Per router, most first.".to_string(),
            rows: errors,
        });
    }
}

// ------------------------------------------------------------------ Caddy

fn caddy(view: &mut ObservabilityView, s: &Series) {
    let servers = s.get("servers").unwrap_or(0.0);
    let routes = s.get("routes").unwrap_or(0.0);
    if s.get("config_loaded") == Some(0.0) {
        view.checks.push(check(
            "Configuration",
            "warning",
            "Caddy runs without any HTTP server configured.",
        ));
    } else if s.get("config_last_reload_ok") == Some(0.0) {
        view.checks.push(check(
            "Configuration",
            "warning",
            "The last reload failed: Caddy still runs the previous configuration.",
        ));
    } else {
        view.checks.push(check(
            "Configuration",
            "ok",
            format!(
                "{} and {} loaded.",
                plural(servers, "server", "servers"),
                plural(routes, "route", "routes")
            ),
        ));
    }

    let unhealthy: Vec<String> = s
        .all("upstream_healthy")
        .filter(|e| e.value < 1.0)
        .map(|e| label_of(e, "upstream"))
        .collect();
    let upstreams = s.get("upstreams").unwrap_or(0.0);
    if !unhealthy.is_empty() {
        view.checks.push(check(
            "Upstreams",
            "warning",
            format!("Failing their health checks: {}.", names(&unhealthy)),
        ));
    } else if s.all("upstream_healthy").next().is_some() {
        view.checks.push(check(
            "Upstreams",
            "ok",
            format!("{}, all healthy.", plural(upstreams, "upstream", "upstreams")),
        ));
    }
    let share = error_share(s, "requests_total", "requests_5xx_total");
    error_check(view, share);

    view.figures.push(figure("Servers", Some(servers), "count"));
    view.figures.push(figure("Routes", Some(routes), "count"));
    view.figures.push(figure("Upstreams", Some(upstreams), "count"));
    view.figures.push(figure("Requests", s.rate("requests_total"), "per_second"));
    view.figures.push(figure("5xx share", share, "percent"));
    view.figures.push(figure("Handler errors in 24 h", s.day("request_errors_total"), "count"));

    let mut rows: Vec<Row> = s
        .all("upstream_fails")
        .map(|e| Row {
            label: format!("{}, recent failures", label_of(e, "upstream")),
            value: Some(e.value),
            unit: "count",
            state: s
                .sibling("upstream_healthy", e, &["upstream"])
                .map(|healthy| if healthy >= 1.0 { "ok" } else { "warning" }),
        })
        .collect();
    rows.sort_by(|a, b| a.label.cmp(&b.label));
    rows.truncate(ROWS_SHOWN);
    if !rows.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Upstreams".to_string(),
            note: "Failures remembered by the passive health check.".to_string(),
            rows,
        });
    }
}

// ------------------------------------------------------ Nginx Proxy Manager

fn npm(view: &mut ObservabilityView, s: &Series) {
    let offline: Vec<String> = s
        .all("host_online")
        .filter(|e| e.value < 1.0)
        .map(|e| {
            let host = label_of(e, "host");
            let error = s
                .all("host_error_info")
                .find(|i| label_of(i, "host") == host)
                .map(|i| format!(" ({})", label_of(i, "error")))
                .unwrap_or_default();
            format!("{host}{error}")
        })
        .collect();
    let served = s.all("host_online").filter(|e| e.value >= 1.0).count();
    let disabled = s.get("hosts_disabled").unwrap_or(0.0);
    if !offline.is_empty() {
        view.checks.push(check(
            "Hosts",
            "warning",
            format!("Not served, nginx refused their configuration: {}.", names(&offline)),
        ));
    } else {
        let mut detail = format!("{} served", plural(served as f64, "host", "hosts"));
        if disabled >= 1.0 {
            detail.push_str(&format!(", {} disabled", plural(disabled, "host", "hosts")));
        }
        detail.push('.');
        view.checks.push(check("Hosts", "ok", detail));
    }
    certificates(view, s, "cert_expiry_days", "host");
    if let (Some(checked), Some(read)) = (s.get("certificates_checked"), s.get("certificates_read"))
        && read < checked
    {
        view.checks.push(check(
            "Certificate check",
            "advisory",
            format!(
                "{} of {} could not be read on the HTTPS port: check TLS port.",
                plural(checked - read, "certificate", "certificates"),
                checked
            ),
        ));
    }

    let of_type = |kind: &str| {
        s.all("hosts")
            .find(|e| e.labels.get("type").map(String::as_str) == Some(kind))
            .map(|e| e.value)
    };
    view.figures.insert(0, figure("Proxy hosts", of_type("proxy"), "count"));
    view.figures.insert(1, figure("Redirections", of_type("redirection"), "count"));
    view.figures.insert(2, figure("404 hosts", of_type("dead"), "count"));
    view.figures.insert(3, figure("Streams", of_type("stream"), "count"));
    view.figures.insert(4, figure("Disabled", Some(disabled), "count"));
}

// ----------------------------------------------------------------- Domaine

fn domain(view: &mut ObservabilityView, s: &Series, now: f64) {
    let statuses: Vec<String> = s.all("status").map(|e| label_of(e, "status")).collect();
    match s.get("expiry_timestamp_seconds") {
        Some(expires) => {
            let days = (expires - now) / DAY;
            let date = chrono::DateTime::from_timestamp(expires as i64, 0)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            let (state, detail) = if days < 0.0 {
                (
                    "warning",
                    format!("Expired on {date}, {} ago.", plural((-days).floor(), "day", "days")),
                )
            } else if days < 7.0 {
                (
                    "warning",
                    format!("Expires on {date}, in {}.", plural(days.floor(), "day", "days")),
                )
            } else if days < 30.0 {
                (
                    "advisory",
                    format!(
                        "Expires on {date}, in {}: renew it now.",
                        plural(days.floor(), "day", "days")
                    ),
                )
            } else {
                ("ok", format!("Expires on {date}, in {}.", plural(days.floor(), "day", "days")))
            };
            view.checks.push(check("Expiry", state, detail));
            view.figures.push(figure("Expires in", Some(expires - now), "seconds"));
        }
        None if s.get("expiry_published") == Some(0.0) => view.checks.push(check(
            "Expiry",
            "ok",
            "The registry does not publish the expiry date of this domain.",
        )),
        None => {}
    }

    let hold: Vec<String> = statuses.iter().filter(|st| st.ends_with("Hold")).cloned().collect();
    let redemption: Vec<String> = statuses
        .iter()
        .filter(|st| matches!(st.as_str(), "redemptionPeriod" | "pendingDelete" | "pendingRestore"))
        .cloned()
        .collect();
    if s.get("on_hold") == Some(1.0) {
        view.checks.push(check(
            "Status",
            "warning",
            format!(
                "{}: the registry took the domain out of the DNS, nothing under it resolves.",
                names(&hold)
            ),
        ));
    } else if s.get("redemption") == Some(1.0) {
        view.checks.push(check(
            "Status",
            "warning",
            format!(
                "{}: the domain has expired and will be deleted unless it is restored.",
                names(&redemption)
            ),
        ));
    } else if !statuses.is_empty() {
        view.checks.push(check("Status", "ok", statuses.join(", ")));
    }

    let registrar = s.label("registrar_info", "registrar");
    let dnssec = match s.get("dnssec") {
        Some(signed) if signed >= 1.0 => Some("the delegation is signed with DNSSEC"),
        Some(_) => Some("the delegation is not signed with DNSSEC"),
        None => None,
    };
    let detail = match (registrar, dnssec) {
        (Some(r), Some(d)) => Some(format!("Registrar {r}; {d}.")),
        (Some(r), None) => Some(format!("Registrar {r}.")),
        (None, Some(d)) => Some(format!("{}.", crate::api::backends::capitalise(d))),
        (None, None) => None,
    };
    if let Some(detail) = detail {
        view.checks.push(check("Registration", "ok", detail));
    }
    if s.get("rdap_ok") == Some(0.0) {
        view.checks.push(check(
            "Registry",
            "advisory",
            "The registry did not answer the last query: the previous answer is shown.",
        ));
    }
    view.figures.push(figure("Answer age", s.get("rdap_age_seconds"), "seconds"));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::tsdb::InstantSeries;

    fn serie(kind: &str, name: &str, value: f64, labels: &[(&str, &str)]) -> InstantSeries {
        let mut metric: BTreeMap<String, String> =
            labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        metric.insert("__name__".into(), format!("dumbmonit_{kind}_{name}"));
        metric.insert("target".into(), "7".into());
        InstantSeries { metric, value: (1_790_812_800.0, value.to_string()) }
    }

    fn state<'a>(view: &'a ObservabilityView, label: &str) -> Option<&'a str> {
        view.checks.iter().find(|c| c.label == label).map(|c| c.state)
    }

    #[test]
    fn sans_mesure_la_vue_est_vide_mais_valide() {
        let view = build_view("traefik", &Series::new("traefik", &[], &[], &[], &[]), 0.0);
        assert!(view.sampled_at.is_none());
        assert!(view.checks.is_empty());
        assert_eq!(view.state, "ok");
    }

    #[test]
    fn traefik_routeur_desactive_et_service_sans_serveur() {
        let k = "traefik";
        let last = [
            serie(k, "version_info", 1.0, &[("version", "3.7.13")]),
            serie(k, "routers", 9.0, &[]),
            serie(k, "router_status", 2.0, &[("router", "wiki@file")]),
            serie(k, "router_error_info", 1.0, &[("router", "wiki@file"), ("error", "no service")]),
            serie(k, "router_status", 0.0, &[("router", "blog@file")]),
            serie(k, "service_servers", 1.0, &[("service", "legacy@file")]),
            serie(k, "service_servers_up", 0.0, &[("service", "legacy@file")]),
            serie(k, "resolver_routers_uncovered", 1.0, &[("resolver", "le")]),
            serie(k, "cert_expiry_days", 10.0, &[("cert", "shop.example.test")]),
        ];
        let rates =
            [serie(k, "requests_total", 2.0, &[]), serie(k, "requests_5xx_total", 1.0, &[])];
        let view = build_view(k, &Series::new(k, &last, &rates, &[], &[]), 0.0);
        assert_eq!(view.version.as_deref(), Some("3.7.13"));
        assert_eq!(state(&view, "Routers"), Some("warning"));
        assert!(view.checks[0].detail.contains("wiki@file (no service)"));
        assert_eq!(state(&view, "Backend servers"), Some("warning"));
        assert_eq!(state(&view, "Certificate resolvers"), Some("warning"));
        assert_eq!(state(&view, "Certificates"), Some("warning"));
        assert_eq!(state(&view, "Errors"), Some("advisory"));
        assert_eq!(view.state, "warning");
    }

    #[test]
    fn caddy_et_npm_en_bonne_sante() {
        let k = "caddy";
        let last = [
            serie(k, "config_loaded", 1.0, &[]),
            serie(k, "config_last_reload_ok", 1.0, &[]),
            serie(k, "servers", 2.0, &[]),
            serie(k, "upstreams", 1.0, &[]),
            serie(k, "upstream_healthy", 1.0, &[("upstream", "app:80")]),
            serie(k, "upstream_fails", 0.0, &[("upstream", "app:80")]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &[], &[]), 0.0);
        assert_eq!(view.state, "ok", "{:?}", view.checks);
        assert_eq!(view.breakdowns[0].rows[0].state, Some("ok"));

        let k = "npm";
        let last = [
            serie(k, "hosts", 3.0, &[("type", "proxy")]),
            serie(k, "hosts_disabled", 1.0, &[("type", "proxy")]),
            serie(k, "host_online", 0.0, &[("type", "proxy"), ("host", "broken.test")]),
            serie(k, "host_error_info", 1.0, &[("host", "broken.test"), ("error", "[emerg]")]),
            serie(k, "certificates_checked", 2.0, &[]),
            serie(k, "certificates_read", 1.0, &[]),
            serie(k, "cert_expiry_days", 40.0, &[("host", "app.test")]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &[], &[]), 0.0);
        assert_eq!(state(&view, "Hosts"), Some("warning"));
        assert!(view.checks[0].detail.contains("broken.test ([emerg])"));
        assert_eq!(state(&view, "Certificates"), Some("ok"));
        assert_eq!(state(&view, "Certificate check"), Some("advisory"));
        assert_eq!(view.figures[0].value, Some(3.0));
    }

    #[test]
    fn un_domaine_bientot_expire_et_suspendu() {
        let k = "domain";
        let now = 1_790_812_800.0;
        let last = [
            serie(k, "expiry_timestamp_seconds", now + 5.0 * DAY, &[]),
            serie(k, "on_hold", 1.0, &[]),
            serie(k, "redemption", 0.0, &[]),
            serie(k, "status", 1.0, &[("status", "clientHold")]),
            serie(k, "registrar_info", 1.0, &[("registrar", "Gandi SAS")]),
            serie(k, "dnssec", 1.0, &[]),
            serie(k, "rdap_ok", 1.0, &[]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &[], &[]), now);
        assert_eq!(state(&view, "Expiry"), Some("warning"));
        assert!(view.checks[0].detail.contains("in 5 days"), "{}", view.checks[0].detail);
        assert_eq!(state(&view, "Status"), Some("warning"));
        assert!(view.checks[1].detail.starts_with("clientHold"));
        let registration = view.checks.iter().find(|c| c.label == "Registration").unwrap();
        assert!(registration.detail.contains("Gandi SAS"));

        let last = [serie(k, "expiry_published", 0.0, &[]), serie(k, "on_hold", 0.0, &[])];
        let view = build_view(k, &Series::new(k, &last, &[], &[], &[]), now);
        assert_eq!(state(&view, "Expiry"), Some("ok"));
        assert_eq!(view.state, "ok");
    }
}
