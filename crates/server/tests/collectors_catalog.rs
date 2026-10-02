//! Instantané de `GET /api/collectors`, octet pour octet.
//!
//! Le catalogue des types est un contrat avec l'interface : il ne doit changer
//! qu'avec une modification voulue de ses tables (`api/collectors.rs`), jamais en
//! passant, à l'occasion d'un remaniement de la façon dont il est construit.
//!
//! Le registre reproduit celui de `main.rs` (les collecteurs réseau de
//! `Registry::remote`, plus l'agent, les heartbeats et la démonstration), et y
//! ajoute un type qu'aucune table ne connaît, pour figer aussi la description
//! générique.
//!
//! `DUMBMONIT_BLESS=1` réécrit l'instantané après un changement voulu.

mod common;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use dumbmonit_server::{collectors, db};
use http_body_util::BodyExt;
use tower::ServiceExt;

const FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/collectors_catalog.json");

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Un type que les tables du serveur ignorent.
struct Inconnu;

#[async_trait]
impl Collector for Inconnu {
    fn kind(&self) -> &'static str {
        "zz-inconnu"
    }

    async fn probe(&self, _target: &Target) -> Result<Vec<Sample>, ProbeError> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn le_catalogue_des_types_reste_identique_a_linstantane() {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let mut registry = collectors::Registry::remote(Duration::from_secs(5));
    registry.register(Arc::new(collectors::AgentCollector::new(pool.clone())));
    registry.register(Arc::new(collectors::PushCollector::new(pool.clone())));
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(Inconnu));
    let app = common::build_with_registry(dir, config, pool, registry).await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;

    let request =
        Request::get("/api/collectors").header(header::COOKIE, cookie).body(Body::empty()).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    let body = std::str::from_utf8(&bytes).expect("JSON en UTF-8");
    // La version du serveur (User-Agent par défaut) change à chaque publication :
    // l'instantané la remplace par un repère pour ne pas avoir à le réécrire.
    let body = &body.replace(VERSION, "<version>");

    if std::env::var_os("DUMBMONIT_BLESS").is_some() {
        std::fs::write(FIXTURE, pretty(body)).expect("écriture de l'instantané");
    }
    let expected = compact(&std::fs::read_to_string(FIXTURE).expect("instantané du catalogue"));
    if let Some(at) = expected.bytes().zip(body.bytes()).position(|(a, b)| a != b) {
        let from = at.saturating_sub(120);
        panic!(
            "GET /api/collectors a changé (DUMBMONIT_BLESS=1 pour l'accepter)\n\
             attendu : …{}…\nobtenu  : …{}…",
            excerpt(&expected, from),
            excerpt(body, from),
        );
    }
    assert_eq!(expected.len(), body.len(), "GET /api/collectors a changé de longueur");
}

/// Environ 240 octets à partir de `from`, sans couper un caractère.
fn excerpt(text: &str, from: usize) -> &str {
    let start = (0..=from).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
    let end = (start + 240).min(text.len());
    let end = (end..=text.len()).find(|&i| text.is_char_boundary(i)).unwrap_or(text.len());
    &text[start..end]
}

/// Met en forme un JSON compact comme `serde_json::to_string_pretty`, sans rien
/// réordonner (passer par `serde_json::Value` trierait les clés) : l'instantané
/// reste lisible, et [`compact`] en redonne exactement la réponse.
fn pretty(compact: &str) -> String {
    let mut out = String::with_capacity(compact.len() * 2);
    let mut depth = 0usize;
    let (mut in_string, mut escaped) = (false, false);
    let newline = |out: &mut String, depth: usize| {
        out.push('\n');
        out.push_str(&"  ".repeat(depth));
    };
    let mut chars = compact.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' => {
                out.push(c);
                if let Some(close) = chars.next_if(|next| matches!(next, '}' | ']')) {
                    out.push(close);
                } else {
                    depth += 1;
                    newline(&mut out, depth);
                }
            }
            '}' | ']' => {
                depth -= 1;
                newline(&mut out, depth);
                out.push(c);
            }
            ',' => {
                out.push(c);
                newline(&mut out, depth);
            }
            ':' => out.push_str(": "),
            _ => out.push(c),
        }
    }
    out.push('\n');
    out
}

/// L'inverse de [`pretty`] : retire les blancs hors des chaînes.
fn compact(pretty: &str) -> String {
    let mut out = String::with_capacity(pretty.len());
    let (mut in_string, mut escaped) = (false, false);
    for c in pretty.chars() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
        } else if c.is_ascii_whitespace() {
            continue;
        } else if c == '"' {
            in_string = true;
        }
        out.push(c);
    }
    out
}
