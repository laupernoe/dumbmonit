//! Notices des intégrations en nuage (Microsoft 365, Entra ID, Azure, AWS,
//! Cloudflare, Hetzner Cloud, OVHcloud, Infomaniak) : un fichier par produit,
//! même forme que les tables de `api/collectors.rs`.

use super::CollectorView;

mod aws;
mod azure;
mod cloudflare;
mod entra;
mod hetzner;
mod infomaniak;
mod m365;
mod ovh;

/// La notice d'un type en nuage, `None` pour tout autre type.
pub(super) fn view(kind: &str) -> Option<CollectorView> {
    Some(match kind {
        "aws" => aws::view(),
        "azure" => azure::view(),
        "cloudflare" => cloudflare::view(),
        "entra" => entra::view(),
        "hetzner" => hetzner::view(),
        "infomaniak" => infomaniak::view(),
        "m365" => m365::view(),
        "ovh" => ovh::view(),
        _ => return None,
    })
}
