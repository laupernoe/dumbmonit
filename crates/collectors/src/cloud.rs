//! Briques communes aux intégrations de services en nuage (Microsoft 365,
//! Entra ID, Azure, Cloudflare, Hetzner Cloud, OVHcloud, Infomaniak, AWS).
//!
//! Ces produits n'ont pas d'adresse sur le réseau local : la cible porte l'hôte
//! de leur API (`graph.microsoft.com`, `api.hetzner.cloud`…), comme Tailscale, et
//! le garde-fou des adresses privées ne les concerne pas. Ce module ne contient
//! que ce qui est partagé :
//!
//! * l'échange d'un secret d'application Microsoft (flux « client credentials »)
//!   contre un jeton d'accès, commun à Microsoft 365, Entra ID et Azure ;
//! * deux familles de séries génériques que la page de l'équipement sait
//!   afficher sans code propre au produit (`item_ok` et `figure`, voir ci-dessous).
//!
//! # Séries génériques de la page de l'équipement
//!
//! * `<préfixe>_item_ok{group, item, detail}` : 1 quand l'élément va bien, 0
//!   sinon. `group` est le titre de la liste (« Services », « Licences »…),
//!   `item` le nom affiché, `detail` un état court et **stable** (« Degraded »,
//!   « Expires in 12 days ») : un détail qui change à chaque passage crée une
//!   nouvelle série à chaque passage.
//! * `<préfixe>_figure{name, unit}` : un chiffre clé de l'en-tête de la page.
//!   `unit` vaut `""` (entier), `percent`, `bytes` ou `days`.
//!
//! Ces deux familles servent à l'affichage ; les règles d'alerte visent des
//! séries dédiées, aux noms et aux unités précis.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Sample};
use serde_json::Value;

use crate::rest::{RestClient, gauge, text};

/// Un élément d'une liste de la page (`<préfixe>_item_ok`).
pub fn item(prefix: &str, group: &str, name: &str, ok: bool, detail: &str, ts_ms: i64) -> Sample {
    gauge(&format!("{prefix}_item_ok"), if ok { 1.0 } else { 0.0 }, ts_ms)
        .with_label("group", group)
        .with_label("item", name)
        .with_label("detail", detail)
}

/// Un chiffre clé de l'en-tête de la page (`<préfixe>_figure`).
pub fn figure(prefix: &str, name: &str, unit: &str, value: f64, ts_ms: i64) -> Sample {
    gauge(&format!("{prefix}_figure"), value, ts_ms)
        .with_label("name", name)
        .with_label("unit", unit)
}

/// Refuse un identifiant qui sortirait de son segment d'URL (locataire, abonnement…).
pub fn url_segment(name: &str, value: &str) -> Result<String, ProbeError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ProbeError::Config(format!("The {name} option is required")));
    }
    if value.contains(['/', '?', '#', ' ', '\\']) {
        return Err(ProbeError::Config(format!("Invalid {name} \"{value}\"")));
    }
    Ok(value.to_string())
}

/// Échange l'identifiant et le secret d'une application Microsoft Entra contre
/// un jeton d'accès (`POST {login}/{tenant}/oauth2/v2.0/token`).
///
/// `login` est un client déjà posé sur le service de connexion
/// (`https://login.microsoftonline.com`, ou celui d'un cloud souverain).
/// Renvoie le jeton et sa durée de vie. Un refus d'identifiants est une erreur
/// d'authentification, un locataire inconnu une erreur de configuration, un
/// service qui ne répond pas une panne : seule celle-là doit alerter.
pub async fn microsoft_token(
    login: &RestClient,
    tenant: &str,
    client_id: &str,
    secret: &str,
    scope: &str,
) -> Result<(String, Duration), ProbeError> {
    let path = format!("/{tenant}/oauth2/v2.0/token");
    let form = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", secret),
        ("scope", scope),
    ];
    let (status, body) = login.post_form(&path, &form).await?;
    if status.is_success() {
        let reply: Value = login.decode(&body, &path)?;
        let token = text(&reply, "access_token").ok_or_else(|| {
            ProbeError::Protocol("Microsoft returned no access_token for the application".into())
        })?;
        let lifetime = match reply.get("expires_in") {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(3600),
            Some(Value::String(s)) => s.parse().unwrap_or(3600),
            _ => 3600,
        };
        return Ok((token.to_string(), Duration::from_secs(lifetime)));
    }
    let reply: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let code = text(&reply, "error").unwrap_or_default();
    let description = text(&reply, "error_description").unwrap_or_default();
    // AADSTS90002 : locataire introuvable ; AADSTS700016 : application inconnue.
    if description.contains("AADSTS90002") || description.contains("AADSTS900023") {
        return Err(ProbeError::Config(format!(
            "Microsoft knows no tenant \"{tenant}\": copy the Directory (tenant) ID of the \
             application registration."
        )));
    }
    if matches!(status.as_u16(), 400 | 401 | 403) || code == "invalid_client" {
        let first = description.lines().next().unwrap_or_default();
        return Err(ProbeError::Auth(format!(
            "Microsoft refused the application ({status}, {code}): check the client ID and \
             that the client secret has not expired. {first}"
        )));
    }
    Err(login.status_error(status, &body, &path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_series_generiques_portent_leurs_etiquettes() {
        let s = item("m365", "Services", "Exchange Online", false, "Degraded", 1);
        assert_eq!(s.metric, "m365_item_ok");
        assert_eq!(s.value, 0.0);
        assert_eq!(s.labels["group"], "Services");
        assert_eq!(s.labels["detail"], "Degraded");
        let f = figure("m365", "Licenses", "", 12.0, 1);
        assert_eq!(f.metric, "m365_figure");
        assert_eq!(f.labels["unit"], "");
    }

    #[test]
    fn un_identifiant_ne_sort_pas_de_son_segment() {
        assert!(url_segment("tenant", "contoso.onmicrosoft.com").is_ok());
        assert!(url_segment("tenant", "").is_err());
        assert!(url_segment("tenant", "a/b").is_err());
        assert!(url_segment("tenant", "a?b").is_err());
    }
}
