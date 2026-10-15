//! Réglages de sécurité de DSM, lus lentement et facultativement.
//!
//! Quatre lectures nourrissent le « score de sécurité » côté serveur :
//!
//! | Métrique | API (méthode, version) | Champ lu |
//! |---|---|---|
//! | `synology_update_available` | `SYNO.Core.Upgrade.Server` (`check`, 1) | `update.available` |
//! | `synology_admin_account_enabled` | `SYNO.Core.User` (`get`, 1, `name=admin`) | `users[0].expired` |
//! | `synology_autoblock_enabled` | `SYNO.Core.Security.AutoBlock` (`get`, 1) | `enable` |
//! | `synology_firewall_enabled` | `SYNO.Core.Security.Firewall` (`get`, 1) | `enable_firewall` |
//!
//! Le 2FA obligatoire (`SYNO.Core.OTP.EnforcePolicy`) n'est pas lu : ses champs ne
//! sont pas assez sûrs pour publier un 0/1 qui pèserait sur une note.
//!
//! Règles de ce module :
//!
//! * **Tout échec est silencieux.** API absente du catalogue, refus (compte non
//!   administrateur : 105, 119…), réponse inattendue : pas de série, un `debug!` au
//!   plus, jamais d'erreur comptée dans `synology_scrape_errors`. Un collecteur qui
//!   marche sans ces lectures continue de marcher.
//! * **Cadence lente.** Le résultat est gardé [`REFRESH`] ; chaque interrogation le
//!   réémet avec son propre horodatage. Une lecture en échec garde sa valeur
//!   précédente jusqu'au tour suivant.

use std::time::{Duration, Instant};

use dumbmonit_proto::{MetricKind, Sample};
use serde_json::Value;
use tracing::debug;

use super::client::DsmClient;

pub const API_UPGRADE: &str = "SYNO.Core.Upgrade.Server";
pub const API_USER: &str = "SYNO.Core.User";
pub const API_AUTOBLOCK: &str = "SYNO.Core.Security.AutoBlock";
pub const API_FIREWALL: &str = "SYNO.Core.Security.Firewall";

/// Intervalle entre deux lectures réelles.
pub const REFRESH: Duration = Duration::from_secs(3600);

/// Budget d'un tour complet : `check` peut contacter Synology et ne doit pas
/// allonger durablement l'interrogation.
const BUDGET: Duration = Duration::from_secs(20);

/// Dernières valeurs lues pour une cible.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Instant de la dernière tentative, réussie ou non.
    pub at: Option<Instant>,
    /// Couples (métrique, valeur). Une métrique jamais lue est absente.
    pub values: Vec<(&'static str, f64)>,
}

impl Snapshot {
    pub fn is_fresh(&self, now: Instant) -> bool {
        self.at.is_some_and(|at| now.duration_since(at) < REFRESH)
    }

    fn set(&mut self, metric: &'static str, value: f64) {
        match self.values.iter_mut().find(|(name, _)| *name == metric) {
            Some(slot) => slot.1 = value,
            None => self.values.push((metric, value)),
        }
    }

    pub fn samples(&self, ts_ms: i64) -> Vec<Sample> {
        self.values
            .iter()
            .map(|(name, value)| Sample::new(*name, *value, MetricKind::Gauge, ts_ms))
            .collect()
    }
}

/// Relit les quatre réglages et fusionne ce qui a pu être lu dans `snapshot`.
pub async fn refresh(dsm: &DsmClient, mut snapshot: Snapshot) -> Snapshot {
    let read = read_all(dsm, &mut snapshot);
    if tokio::time::timeout(BUDGET, read).await.is_err() {
        debug!("réglages de sécurité DSM : délai dépassé, valeurs précédentes conservées");
    }
    snapshot.at = Some(Instant::now());
    snapshot
}

async fn read_all(dsm: &DsmClient, snapshot: &mut Snapshot) {
    if let Some(value) = step(dsm, API_AUTOBLOCK, "get", &[], autoblock_enabled).await {
        snapshot.set("synology_autoblock_enabled", value);
    }
    if let Some(value) = step(dsm, API_FIREWALL, "get", &[], firewall_enabled).await {
        snapshot.set("synology_firewall_enabled", value);
    }
    let user = [("name", "admin".to_string()), ("additional", "[\"expired\"]".to_string())];
    if let Some(value) = step(dsm, API_USER, "get", &user, admin_enabled).await {
        snapshot.set("synology_admin_account_enabled", value);
    }
    if let Some(value) = step(dsm, API_UPGRADE, "check", &[], update_available).await {
        snapshot.set("synology_update_available", value);
    }
}

/// Un appel tolérant : renvoie la valeur 0/1 extraite, ou `None` sur tout échec.
async fn step(
    dsm: &DsmClient,
    api: &str,
    method: &str,
    extra: &[(&str, String)],
    parse: fn(&Value) -> Option<bool>,
) -> Option<f64> {
    if !dsm.supports(api) {
        debug!(api, "réglage de sécurité DSM : API absente du catalogue");
        return None;
    }
    match dsm.call::<Value>(api, 1, method, extra).await {
        Ok(data) => {
            let flag = parse(&data);
            if flag.is_none() {
                debug!(api, "réglage de sécurité DSM : réponse inattendue");
            }
            flag.map(|on| if on { 1.0 } else { 0.0 })
        }
        Err(error) => {
            debug!(api, %error, "réglage de sécurité DSM illisible (compte non administrateur ?)");
            None
        }
    }
}

/// Booléen DSM : `true`/`false`, ou `0`/`1`.
fn flag(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|n| n != 0),
        _ => None,
    }
}

fn autoblock_enabled(data: &Value) -> Option<bool> {
    flag(data.get("enable")?)
}

fn firewall_enabled(data: &Value) -> Option<bool> {
    flag(data.get("enable_firewall")?)
}

fn update_available(data: &Value) -> Option<bool> {
    flag(data.get("update")?.get("available")?)
}

/// `expired` vaut `"now"` pour un compte désactivé, `"normal"` pour un compte
/// actif, ou une date d'expiration (compte encore actif jusque-là). Un utilisateur
/// absent (`users` vide) ne donne rien.
fn admin_enabled(data: &Value) -> Option<bool> {
    let user = data.get("users")?.as_array()?.first()?;
    let expired = user.get("expired")?.as_str()?;
    Some(expired != "now")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn autoblock_lit_le_champ_enable() {
        let on = json!({"enable": true, "attempts": 10, "within_mins": 5, "expire_day": 0});
        assert_eq!(autoblock_enabled(&on), Some(true));
        assert_eq!(autoblock_enabled(&json!({"enable": false})), Some(false));
        assert_eq!(autoblock_enabled(&json!({"enable": 0})), Some(false));
        assert_eq!(autoblock_enabled(&json!({"attempts": 10})), None);
        assert_eq!(autoblock_enabled(&json!({"enable": "oui"})), None);
    }

    #[test]
    fn pare_feu_lit_enable_firewall() {
        let data = json!({"enable_firewall": true, "profile_name": "default"});
        assert_eq!(firewall_enabled(&data), Some(true));
        assert_eq!(firewall_enabled(&json!({"enable_firewall": false})), Some(false));
        assert_eq!(firewall_enabled(&json!({})), None);
    }

    #[test]
    fn mise_a_jour_lit_update_available() {
        let some = json!({"update": {"available": true, "version": "DSM 7.2.2-72806"}});
        assert_eq!(update_available(&some), Some(true));
        let none = json!({"update": {"available": false}});
        assert_eq!(update_available(&none), Some(false));
        assert_eq!(update_available(&json!({"update": {}})), None);
        assert_eq!(update_available(&json!([])), None);
    }

    #[test]
    fn compte_admin_actif_ou_desactive() {
        let actif = json!({"users": [{"name": "admin", "expired": "normal"}]});
        assert_eq!(admin_enabled(&actif), Some(true));
        let desactive = json!({"users": [{"name": "admin", "expired": "now"}]});
        assert_eq!(admin_enabled(&desactive), Some(false));
        let date = json!({"users": [{"name": "admin", "expired": "2030/01/01"}]});
        assert_eq!(admin_enabled(&date), Some(true));
    }

    #[test]
    fn compte_admin_absent_ou_format_inattendu() {
        assert_eq!(admin_enabled(&json!({"users": []})), None);
        assert_eq!(admin_enabled(&json!({"users": [{"name": "admin"}]})), None);
        assert_eq!(admin_enabled(&json!({"users": "x"})), None);
        assert_eq!(admin_enabled(&json!(null)), None);
    }

    #[test]
    fn le_cliche_se_reemet_et_fusionne() {
        let mut snap = Snapshot::default();
        assert!(!snap.is_fresh(Instant::now()));
        snap.set("synology_autoblock_enabled", 1.0);
        snap.set("synology_autoblock_enabled", 0.0);
        snap.set("synology_firewall_enabled", 1.0);
        snap.at = Some(Instant::now());
        assert!(snap.is_fresh(Instant::now()));
        let samples = snap.samples(42);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].metric, "synology_autoblock_enabled");
        assert_eq!(samples[0].value, 0.0);
    }
}
