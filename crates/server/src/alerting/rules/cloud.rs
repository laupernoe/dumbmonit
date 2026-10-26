//! Règles livrées des intégrations en nuage, groupées par produit : un fichier
//! par produit (`uid` préfixé du type), même forme que `builtin_rules`.

use crate::alerting::model::Rule;

mod aws;
mod azure;
mod cloudflare;
mod entra;
mod hetzner;
mod infomaniak;
mod m365;
mod ovh;

/// Les règles de tous les produits en nuage.
pub(super) fn rules() -> Vec<Rule> {
    let mut out = Vec::new();
    out.extend(aws::rules());
    out.extend(azure::rules());
    out.extend(cloudflare::rules());
    out.extend(entra::rules());
    out.extend(hetzner::rules());
    out.extend(infomaniak::rules());
    out.extend(m365::rules());
    out.extend(ovh::rules());
    out
}

/// Les métriques (préfixe `dumbmonit_` compris) que ces règles visent : le test
/// `les_regles_livrees_ne_visent_que_des_metriques_produites` les accepte.
#[cfg(test)]
pub(super) fn produced() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    out.extend_from_slice(aws::PRODUCED);
    out.extend_from_slice(azure::PRODUCED);
    out.extend_from_slice(cloudflare::PRODUCED);
    out.extend_from_slice(entra::PRODUCED);
    out.extend_from_slice(hetzner::PRODUCED);
    out.extend_from_slice(infomaniak::PRODUCED);
    out.extend_from_slice(m365::PRODUCED);
    out.extend_from_slice(ovh::PRODUCED);
    out
}
