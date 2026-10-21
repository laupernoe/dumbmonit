//! `/robots.txt` et `/sitemap.xml`, publics.
//!
//! Sur l'hôte principal, tout reste fermé sauf les pages de statut dont
//! l'administrateur a coché l'indexation. Sur le domaine d'une page, seul ce
//! qui la concerne répond (voir `crate::status_host`).

use axum::extract::{Request, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::db::status_pages::StatusPage;
use crate::state::AppState;
use crate::status_host::StatusHost;
use crate::status_seo;

fn text(content_type: &'static str, body: String) -> Response {
    ([(header::CONTENT_TYPE, content_type), (header::CACHE_CONTROL, "public, max-age=300")], body)
        .into_response()
}

/// Les pages à considérer : celle du domaine, ou toutes sur l'hôte principal.
async fn pages(state: &AppState, host: Option<&StatusHost>) -> Vec<StatusPage> {
    let all = crate::db::status_pages::list_pages(&state.pool).await.unwrap_or_default();
    match host {
        Some(host) => all.into_iter().filter(|page| page.slug == host.slug).collect(),
        None => all.into_iter().filter(|page| page.domain.is_none()).collect(),
    }
}

pub async fn robots(State(state): State<AppState>, request: Request) -> Response {
    let host = request.extensions().get::<StatusHost>().cloned();
    let pages = pages(&state, host.as_ref()).await;
    let body = match host {
        Some(_) => status_seo::robots_domain(pages.first()),
        None => status_seo::robots_main(&pages),
    };
    text("text/plain; charset=utf-8", body)
}

pub async fn sitemap(State(state): State<AppState>, request: Request) -> Response {
    let host = request.extensions().get::<StatusHost>().cloned();
    let pages = pages(&state, host.as_ref()).await;
    let refs: Vec<&StatusPage> = pages.iter().collect();
    text("application/xml; charset=utf-8", status_seo::sitemap(&refs))
}
