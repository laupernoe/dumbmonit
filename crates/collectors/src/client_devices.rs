//! Appareils clients : le même manque revient pour chaque produit qui garde
//! trace de postes qui se connectent et sauvegardent — Immich (le premier
//! branché ici), Proxmox Backup Server (un appareil par groupe de sauvegarde,
//! voir `pbs::backup::client_devices`) et Veeam (un appareil par objet
//! protégé, lu sur `/api/v1/restorePoints`, voir `veeam::protected_devices`),
//! puis à terme les clients Tailscale, UniFi, l'app compagnon Home Assistant,
//! les clients Nextcloud… Dans tous les cas la question posée par
//! l'utilisateur est la même : « cet appareil s'est-il connecté récemment, et
//! a-t-il bien sauvegardé ? ». Ce module factorise la métrique, le seuil et
//! l'étiquetage pour qu'une seule règle d'alerte et un seul composant
//! d'interface ([`ClientDevicesTable.svelte`](
//! ../../../../web/src/lib/components/devices/ClientDevicesTable.svelte))
//! servent tous les produits, présents et futurs.
//!
//! Synology Active Backup for Business n'est volontairement **pas** branché
//! ici : il suit déjà ses appareils par un mécanisme propre et plus riche —
//! rythme appris par appareil, tolérance adaptée aux jours de repos, voir
//! `synology::rhythm` et `synology::devices` — construit avant ce module
//! générique. Le dupliquer ferait cohabiter deux verdicts différents pour les
//! mêmes machines.
//!
//! # Comment brancher un nouveau produit
//!
//! 1. Lire chaque appareil connu par l'API du produit dans un [`Device`] :
//!    `name` (le plus parlant que l'API donne — à défaut un identifiant
//!    court, voir `immich.rs` qui compose `<type>-<8 caractères de l'id>` en
//!    l'absence de tout nom), `device_type`, `os`, `user`, et les deux
//!    instants optionnels `last_seen` / `last_backup` en secondes Unix.
//!    Mettre `None` quand le produit ne sait pas répondre pour cet appareil
//!    précis — voir Immich, qui ne sait pas quel appareil a téléversé quoi
//!    (voir le commentaire de `immich::device_samples`).
//! 2. Lire le seuil propre à la cible avec [`stale_days`].
//! 3. Appeler [`samples`] avec la liste, le seuil, le nom du produit
//!    (`kind`, tel que stocké dans `Target::kind`) et l'horodate de la sonde,
//!    et étendre les échantillons renvoyés par la sonde avec son résultat.
//!
//! Le reste — la règle livrée `client_device_stale` et le tableau générique
//! de l'interface — fonctionne sans rien connaître du produit : il ne lit que
//! les métriques ci-dessous.

use dumbmonit_proto::{MetricKind, ProbeError, Sample, Target};

/// Défaut produit : trois jours sans connexion ni sauvegarde, c'est déjà le
/// signe qu'un appareil a été oublié (chargeur débranché, app désinstallée,
/// Wi-Fi invité qui bloque l'upload).
pub const DEFAULT_STALE_DAYS: f64 = 3.0;

/// Au-delà, seul leur nombre compte (même borne que les files ou tâches
/// nommées des autres familles de collecteurs).
pub const MAX_DEVICES: usize = 32;

/// Un appareil client, tel que lu par le produit qui l'héberge.
#[derive(Debug, Clone)]
pub struct Device {
    /// Le plus parlant que l'API du produit fournisse : un nom d'appareil,
    /// ou à défaut un identifiant court mais stable.
    pub name: String,
    /// « Mobile », « Desktop », « Web »… libre au produit, affiché tel quel.
    pub device_type: String,
    /// « iOS 17 », « Android 14 », « Windows »… libre au produit.
    pub os: String,
    /// Le compte auquel l'appareil est rattaché.
    pub user: String,
    /// Dernière connexion, en secondes Unix. `None` si le produit ne la
    /// connaît pas pour cet appareil.
    pub last_seen: Option<i64>,
    /// Dernière sauvegarde (téléversement) réussie, en secondes Unix.
    /// `None` si le produit ne sait pas l'attribuer à cet appareil précis.
    pub last_backup: Option<i64>,
}

/// Lit `device_stale_days` sur la cible : au-delà de ce nombre de jours sans
/// connexion ni sauvegarde, un appareil est signalé. De 1 à 90 jours.
pub fn stale_days(target: &Target) -> Result<f64, ProbeError> {
    let Some(raw) = tag(target, "device_stale_days") else { return Ok(DEFAULT_STALE_DAYS) };
    let days: f64 = raw
        .parse()
        .ok()
        .filter(|days: &f64| days.is_finite())
        .ok_or_else(|| ProbeError::Config(format!("Invalid device staleness: \"{raw}\"")))?;
    if !(1.0..=90.0).contains(&days) {
        return Err(ProbeError::Config(
            "Device staleness must be between 1 and 90 days".to_string(),
        ));
    }
    Ok(days)
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

/// Construit les métriques communes à tous les produits pour la liste
/// d'appareils donnée : l'instant de dernière connexion et de dernière
/// sauvegarde quand ils sont connus, et l'écart au seuil configuré qui fait
/// à lui seul tourner la règle livrée `client_device_stale` — quel que soit
/// le produit.
pub fn samples(kind: &str, devices: &[Device], stale_days: f64, ts_ms: i64) -> Vec<Sample> {
    let now_s = (ts_ms / 1000) as f64;
    let threshold = stale_days * 86_400.0;
    let mut out = Vec::new();
    for device in devices.iter().take(MAX_DEVICES) {
        let label = |sample: Sample| -> Sample {
            sample
                .with_label("kind", kind)
                .with_label("device", &device.name)
                .with_label("type", &device.device_type)
                .with_label("os", &device.os)
                .with_label("user", &device.user)
        };
        if let Some(last_seen) = device.last_seen {
            out.push(label(gauge(
                "client_device_last_seen_timestamp_seconds",
                last_seen as f64,
                ts_ms,
            )));
            out.push(
                label(gauge(
                    "client_device_stale_seconds",
                    (now_s - last_seen as f64) - threshold,
                    ts_ms,
                ))
                .with_label("signal", "connection"),
            );
        }
        if let Some(last_backup) = device.last_backup {
            out.push(label(gauge(
                "client_device_last_backup_timestamp_seconds",
                last_backup as f64,
                ts_ms,
            )));
            out.push(
                label(gauge(
                    "client_device_stale_seconds",
                    (now_s - last_backup as f64) - threshold,
                    ts_ms,
                ))
                .with_label("signal", "backup"),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use dumbmonit_proto::Credential;

    use super::*;

    fn target(tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "immich".into(),
            address: "immich.lan".into(),
            kind: "immich".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential: Credential::None,
        }
    }

    fn value(samples: &[Sample], name: &str, signal: Option<&str>) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == name
                    && signal
                        .is_none_or(|sig| s.labels.get("signal").map(String::as_str) == Some(sig))
            })
            .map(|s| s.value)
    }

    #[test]
    fn le_seuil_par_defaut_est_trois_jours() {
        assert_eq!(stale_days(&target(&[])).unwrap(), 3.0);
        assert_eq!(stale_days(&target(&[("device_stale_days", "7")])).unwrap(), 7.0);
        assert!(stale_days(&target(&[("device_stale_days", "0")])).is_err());
        assert!(stale_days(&target(&[("device_stale_days", "quatre")])).is_err());
    }

    #[test]
    fn un_appareil_frais_a_un_ecart_negatif() {
        let now_ms = 1_000_000_000_000i64;
        let now_s = now_ms / 1000;
        let devices = vec![Device {
            name: "Mobile-abcd1234".into(),
            device_type: "Mobile".into(),
            os: "Android 14".into(),
            user: "noe".into(),
            last_seen: Some(now_s - 3600), // une heure
            last_backup: None,
        }];
        let samples = samples("immich", &devices, 3.0, now_ms);
        let age = value(&samples, "client_device_stale_seconds", Some("connection")).unwrap();
        assert!(
            age < 0.0,
            "un appareil vu il y a une heure n'est pas périmé à trois jours : {age}"
        );
        assert!(value(&samples, "client_device_stale_seconds", Some("backup")).is_none());
        assert_eq!(
            value(&samples, "client_device_last_seen_timestamp_seconds", None),
            Some((now_s - 3600) as f64)
        );
    }

    #[test]
    fn un_appareil_muet_depuis_dix_jours_depasse_le_seuil_de_trois() {
        let now_ms = 1_000_000_000_000i64;
        let now_s = now_ms / 1000;
        let devices = vec![Device {
            name: "Desktop-11112222".into(),
            device_type: "Desktop".into(),
            os: "Linux".into(),
            user: "noe".into(),
            last_seen: Some(now_s - 10 * 86_400),
            last_backup: Some(now_s - 10 * 86_400),
        }];
        let samples = samples("immich", &devices, 3.0, now_ms);
        assert!(value(&samples, "client_device_stale_seconds", Some("connection")).unwrap() > 0.0);
        assert!(value(&samples, "client_device_stale_seconds", Some("backup")).unwrap() > 0.0);
    }

    #[test]
    fn au_dela_de_la_borne_seuls_les_premiers_appareils_comptent() {
        let now_ms = 2_000_000_000_000i64;
        let devices: Vec<Device> = (0..40)
            .map(|i| Device {
                name: format!("device-{i}"),
                device_type: "Mobile".into(),
                os: "iOS".into(),
                user: "noe".into(),
                last_seen: Some(now_ms / 1000),
                last_backup: None,
            })
            .collect();
        let samples = samples("immich", &devices, 3.0, now_ms);
        let distinct: std::collections::HashSet<_> =
            samples.iter().filter_map(|s| s.labels.get("device")).collect();
        assert_eq!(distinct.len(), MAX_DEVICES);
    }
}
