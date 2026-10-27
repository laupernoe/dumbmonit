//! Réglages d'instance : mode produit et assistant de démarrage
//! ([`crate::instance`]).
//!
//! * `GET /instance` — public : ce dont l'interface a besoin avant même la
//!   connexion pour choisir son habillage (`product_mode`, `setup_completed`).
//!   Rien d'autre : le nom de l'instance n'a pas à être lu par un inconnu.
//! * `GET /instance/settings` — tout le réglage, pour une session ouverte.
//! * `PUT /instance/settings` — administrateur ; chaque champ absent garde sa
//!   valeur.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::instance::{self, InstanceSettings, LANGUAGES, MAX_NAME_LEN, ProductMode};
use crate::state::AppState;

/// Routes ouvertes sans session.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/instance", get(public))
}

/// Routes sous le garde de session.
pub fn routes() -> Router<AppState> {
    Router::new().route("/instance/settings", get(read).put(update))
}

/// Réponse publique, volontairement minimale.
#[derive(Debug, Serialize)]
pub struct PublicInstance {
    pub product_mode: ProductMode,
    pub setup_completed: bool,
}

/// Modification envoyée par l'interface. Chaque champ absent conserve sa valeur.
#[derive(Debug, Default, Deserialize)]
pub struct InstancePatch {
    #[serde(default)]
    pub product_mode: Option<String>,
    #[serde(default)]
    pub setup_completed: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
}

/// La démonstration publique n'a pas d'assistant : son parc est déjà là.
fn effective(state: &AppState, mut value: InstanceSettings) -> InstanceSettings {
    if state.config.demo {
        value.setup_completed = true;
    }
    value
}

async fn public(State(state): State<AppState>) -> Json<PublicInstance> {
    let value = effective(&state, instance::load(&state.pool).await);
    Json(PublicInstance {
        product_mode: value.product_mode,
        setup_completed: value.setup_completed,
    })
}

async fn read(State(state): State<AppState>) -> Json<InstanceSettings> {
    Json(effective(&state, instance::load(&state.pool).await))
}

async fn update(
    _: AdminIdentity,
    State(state): State<AppState>,
    Json(patch): Json<InstancePatch>,
) -> ApiResult<Json<InstanceSettings>> {
    let mut value = instance::load(&state.pool).await;
    apply(&mut value, patch)?;
    instance::save(&state.pool, &value).await?;
    Ok(Json(effective(&state, value)))
}

/// Valide et applique une modification. Rien n'est écrit si un champ est refusé.
fn apply(value: &mut InstanceSettings, patch: InstancePatch) -> ApiResult<()> {
    if let Some(mode) = patch.product_mode {
        value.product_mode = ProductMode::parse(&mode).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Unknown product mode \"{mode}\" (available: homelab, enterprise)"
            ))
        })?;
    }
    if let Some(name) = patch.name {
        let name = name.trim();
        if name.chars().count() > MAX_NAME_LEN {
            return Err(ApiError::BadRequest(format!(
                "The instance name is limited to {MAX_NAME_LEN} characters."
            )));
        }
        if name.chars().any(char::is_control) {
            return Err(ApiError::BadRequest(
                "The instance name cannot contain control characters.".into(),
            ));
        }
        value.name = name.to_string();
    }
    if let Some(language) = patch.language {
        let language = language.trim();
        if !language.is_empty() && !LANGUAGES.contains(&language) {
            return Err(ApiError::BadRequest(format!(
                "Unknown language \"{language}\" (available: {})",
                LANGUAGES.join(", ")
            )));
        }
        value.language = language.to_string();
    }
    if let Some(timezone) = patch.timezone {
        let timezone = timezone.trim();
        if !timezone.is_empty() && crate::reports::schedule::parse_timezone(timezone).is_none() {
            return Err(ApiError::BadRequest(format!(
                "Unknown time zone \"{timezone}\" (expected an IANA name such as Europe/Paris)"
            )));
        }
        value.timezone = timezone.to_string();
    }
    if let Some(done) = patch.setup_completed {
        // L'assistant peut être relancé depuis les réglages : remettre à faux
        // est permis, et la date suit. Répéter la même valeur ne la change pas.
        if done != value.setup_completed {
            value.setup_completed_at =
                done.then(|| Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
        }
        value.setup_completed = done;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patch_keeps_what_it_does_not_name() {
        let mut value = InstanceSettings { name: "Lab".into(), ..InstanceSettings::default() };
        let patch = InstancePatch { product_mode: Some("enterprise".into()), ..Default::default() };
        assert!(apply(&mut value, patch).is_ok());
        assert_eq!(value.product_mode, ProductMode::Enterprise);
        assert_eq!(value.name, "Lab");
    }

    #[test]
    fn bad_values_are_refused() {
        let mut value = InstanceSettings::default();
        for patch in [
            InstancePatch { product_mode: Some("pro".into()), ..Default::default() },
            InstancePatch { language: Some("klingon".into()), ..Default::default() },
            InstancePatch { timezone: Some("Mars/Olympus".into()), ..Default::default() },
            InstancePatch { name: Some("x".repeat(MAX_NAME_LEN + 1)), ..Default::default() },
        ] {
            assert!(apply(&mut value, patch).is_err());
        }
        assert_eq!(value, InstanceSettings::default());
    }

    #[test]
    fn finishing_the_wizard_stamps_a_date_and_reopening_clears_it() {
        let mut value = InstanceSettings::default();
        let done = InstancePatch { setup_completed: Some(true), ..Default::default() };
        assert!(apply(&mut value, done).is_ok());
        assert!(value.setup_completed_at.is_some());
        let reopen = InstancePatch { setup_completed: Some(false), ..Default::default() };
        assert!(apply(&mut value, reopen).is_ok());
        assert!(value.setup_completed_at.is_none());
    }
}
