//! LUN iSCSI : ce qui occupe un volume sans y être des données.
//!
//! Un LUN en provisionnement **épais** (« thick ») réserve toute sa taille sur
//! le volume dès sa création : un volume de 10 To qui porte un LUN épais de
//! 9,8 To apparaît plein à 98 % dans DSM, et c'est **normal**. Le NAS ne voit
//! pas l'intérieur du LUN — c'est le système de fichiers de l'hôte iSCSI
//! (VMFS, NTFS, ext4) qui sait s'il est plein. Un LUN **fin** (« thin ») ne
//! consomme que ce qui a été écrit, mais peut promettre plus d'espace que le
//! volume n'en a encore de libre.
//!
//! D'où les grandeurs dérivées de ce module, par volume :
//!
//! * **données hors LUN** = utilisé − réservations épaisses − allocations fines ;
//! * **capacité effective** = taille − réservations épaisses ;
//! * **occupation effective** = (utilisé − réservations épaisses) / capacité
//!   effective — c'est elle que regardent l'alerte de remplissage et la
//!   prévision ; sans LUN, elle vaut exactement l'occupation brute ;
//! * **sur-allocation fine** = espace promis aux LUN fins et pas encore écrit,
//!   rapporté à l'espace libre du volume.
//!
//! # Sources, et ce qui reste à vérifier
//!
//! `SYNO.Core.ISCSI.LUN` (`list`, version 1) et `SYNO.Core.ISCSI.Target`
//! (`list`, version 1) sont annoncées par le catalogue réel de DSM 7.4.1
//! (`testdata/dsm74/api-info.json`, `requestFormat: JSON`). Le pilote
//! Synology d'OpenStack Cinder (`cinder/volume/drivers/synology/synology_common.py`,
//! relu le 27 octobre 2026) confirme `list` → `data.luns[]`, et les champs
//! `uuid`, `size`, `status`, `is_mapped` d'un LUN.
//!
//! **À vérifier sur un vrai NAS** — aucune capture réelle n'existe encore, les
//! fixtures de `testdata/iscsi/` sont écrites à la main :
//!
//! * `name`, `location` (`/volume1`), `type_str` (`BLUN`, `BLUN_THICK`, `FILE`,
//!   `THIN`, `ADV`…) et `allocated_size` : noms relevés dans l'interface de SAN
//!   Manager, pas dans une source écrite. La lecture essaie plusieurs noms et
//!   reste tolérante ; un LUN dont le type est illisible est `unknown` ;
//! * le paramètre `additional` : une clé refusée ferait échouer l'appel, d'où
//!   le second essai sans lui ;
//! * la forme de `SYNO.Core.ISCSI.Target` (`iqn`, `mapped_luns[].lun_uuid`,
//!   `connected_sessions[].iqn`) : facultative, son échec n'efface pas les LUN.

use dumbmonit_proto::{MetricKind, ProbeError, Sample};
use serde_json::Value;
use tracing::warn;

use super::client::DsmClient;
use super::model::Volume;

/// Liste des LUN.
pub const API_LUN: &str = "SYNO.Core.ISCSI.LUN";
/// Liste des cibles, avec les LUN qu'elles exposent et les sessions ouvertes.
pub const API_TARGET: &str = "SYNO.Core.ISCSI.Target";
const VERSION: u32 = 1;

/// Champs supplémentaires demandés à la liste des LUN (à vérifier, voir l'en-tête).
const LUN_ADDITIONAL: &str = r#"["allocated_size","is_mapped","status"]"#;
/// Champs supplémentaires demandés à la liste des cibles (à vérifier).
const TARGET_ADDITIONAL: &str = r#"["mapped_lun","connected_sessions"]"#;

/// LUN nommés individuellement, au-delà desquels on s'arrête : un NAS en a
/// rarement plus d'une poignée, et la borne protège la base de séries.
pub const MAX_LUNS: usize = 64;

/// Booléens explicites qu'un DSM pourrait renvoyer à la place du type (à vérifier).
const THIN_FLAGS: &[&str] = &["is_thin", "thin_provision", "thin_provisioning"];

/// Une capacité effective plus petite que cela (et au moins 1 Gio) : le volume
/// est entièrement réservé par ses LUN épais, il n'y a rien d'autre à remplir.
const FULLY_RESERVED_REL: f64 = 0.005;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
/// Plafond de la sur-allocation publiée : au-delà, le chiffre ne dit plus rien
/// de plus qu'« énormément ».
const OVERCOMMIT_CAP: f64 = 1000.0;

/// Mode de provisionnement d'un LUN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provisioning {
    /// Toute la taille est réservée sur le volume dès la création.
    Thick,
    /// Seul ce qui est écrit occupe le volume.
    Thin,
    /// Type illisible : traité comme fin s'il annonce une allocation, comme
    /// épais sinon (prudent : on ne compte pas sa taille comme des données).
    #[default]
    Unknown,
}

impl Provisioning {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Thick => "thick",
            Self::Thin => "thin",
            Self::Unknown => "unknown",
        }
    }

    /// Lit le type de DSM : `BLUN_THICK`, `FILE` et `BLOCK` réservent tout ;
    /// `BLUN`, `THIN`, `ADV` et les LUN Cinder sont fins.
    pub fn classify(lun_type: &str) -> Self {
        let upper = lun_type.trim().to_ascii_uppercase();
        if upper.is_empty() {
            return Self::Unknown;
        }
        if upper.contains("THICK") || upper == "FILE" || upper == "BLOCK" {
            return Self::Thick;
        }
        if upper.contains("THIN")
            || upper.contains("ADV")
            || upper.starts_with("BLUN")
            || upper.contains("CINDER")
        {
            return Self::Thin;
        }
        Self::Unknown
    }
}

/// Une cible iSCSI qui expose un LUN.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LunTarget {
    pub iqn: String,
    pub name: String,
    /// Sessions ouvertes sur la cible.
    pub sessions: usize,
    /// IQN des initiateurs connectés (hôtes ESXi, Proxmox, Windows…).
    pub initiators: Vec<String>,
}

/// Un LUN tel que DSM le décrit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lun {
    /// Identifiant stable : `uuid`, sinon `lun_id`, sinon le nom.
    pub key: String,
    pub name: String,
    /// Volume hôte tel que DSM l'écrit : `/volume1`.
    pub location: String,
    /// Le mot de DSM : `BLUN_THICK`, `BLUN`, `FILE`…
    pub lun_type: String,
    pub provisioning: Provisioning,
    pub size: Option<f64>,
    /// Espace réellement écrit, pour un LUN fin.
    pub allocated: Option<f64>,
    pub mapped: Option<bool>,
    pub status: String,
    /// `None` : la liste des cibles n'a pas pu être lue.
    pub targets: Option<Vec<LunTarget>>,
}

impl Lun {
    /// Fin pour le calcul : déclaré fin, ou de type inconnu mais avec une
    /// allocation annoncée.
    fn counts_as_thin(&self) -> bool {
        match self.provisioning {
            Provisioning::Thin => true,
            Provisioning::Thick => false,
            Provisioning::Unknown => self.allocated.is_some(),
        }
    }

    /// Ce que le LUN occupe réellement sur son volume.
    pub fn reserved(&self) -> Option<f64> {
        if self.counts_as_thin() { self.allocated } else { self.size }
    }
}

// ------------------------------------------------------------------ lecture

/// Interroge la liste des LUN, puis, si l'API existe, celle des cibles.
pub async fn collect(dsm: &DsmClient) -> Result<Vec<Lun>, ProbeError> {
    let params = [("additional", LUN_ADDITIONAL.to_string())];
    let data: Value = match dsm.call(API_LUN, VERSION, "list", &params).await {
        Ok(data) => data,
        // Un NAS muet ne répondra pas mieux au second essai.
        Err(error) if error.means_down() => return Err(error),
        Err(error) => {
            warn!(%error, "liste des LUN refusée avec `additional`, second essai sans");
            dsm.call(API_LUN, VERSION, "list", &[]).await?
        }
    };
    let mut luns = parse_luns(&data);
    if dsm.supports(API_TARGET) {
        let params = [("additional", TARGET_ADDITIONAL.to_string())];
        let targets = match dsm.call::<Value>(API_TARGET, VERSION, "list", &params).await {
            Ok(data) => Ok(data),
            Err(error) if error.means_down() => Err(error),
            Err(_) => dsm.call::<Value>(API_TARGET, VERSION, "list", &[]).await,
        };
        match targets {
            Ok(data) => attach_targets(&mut luns, &parse_targets(&data)),
            Err(error) => warn!(%error, "cibles iSCSI illisibles : LUN sans leurs connexions"),
        }
    }
    Ok(luns)
}

fn field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key).filter(|v| !v.is_null()))
}

fn num(value: &Value, keys: &[&str]) -> Option<f64> {
    let parsed: Option<f64> = match field(value, keys)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    };
    parsed.filter(|v| v.is_finite() && *v >= 0.0)
}

fn text(value: &Value, keys: &[&str]) -> Option<String> {
    match field(value, keys)? {
        Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn flag(value: &Value, keys: &[&str]) -> Option<bool> {
    match field(value, keys)? {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_f64().map(|v| v != 0.0),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Le tableau d'objets d'une liste : `data.<clé>`, ou `data` s'il en est un.
fn items<'a>(data: &'a Value, key: &str) -> &'a [Value] {
    let list = data.get(key).and_then(Value::as_array).or_else(|| data.as_array());
    list.map_or(&[][..], Vec::as_slice)
}

/// Traduit `data` de `SYNO.Core.ISCSI.LUN&method=list`.
pub fn parse_luns(data: &Value) -> Vec<Lun> {
    items(data, "luns")
        .iter()
        .filter_map(|raw| {
            let key = text(raw, &["uuid", "lun_uuid", "lun_id", "id", "name"])?;
            let lun_type = text(raw, &["type_str", "lun_type", "type"]).unwrap_or_default();
            // Un booléen explicite, s'il existe, l'emporte sur le type.
            let provisioning = match flag(raw, THIN_FLAGS) {
                Some(true) => Provisioning::Thin,
                Some(false) => Provisioning::Thick,
                None => Provisioning::classify(&lun_type),
            };
            Some(Lun {
                name: text(raw, &["name"]).unwrap_or_else(|| key.clone()),
                location: text(raw, &["location", "vol_path", "volume"]).unwrap_or_default(),
                lun_type,
                provisioning,
                size: num(raw, &["size", "lun_size", "total_size"]),
                allocated: num(raw, &["allocated_size", "used_size", "allocated"]),
                mapped: flag(raw, &["is_mapped", "mapped"]),
                status: text(raw, &["status"]).unwrap_or_default(),
                targets: None,
                key,
            })
        })
        .take(MAX_LUNS)
        .collect()
}

/// Une cible et les clés des LUN qu'elle expose.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TargetEntry {
    pub target: LunTarget,
    pub luns: Vec<String>,
}

/// Traduit `data` de `SYNO.Core.ISCSI.Target&method=list`.
pub fn parse_targets(data: &Value) -> Vec<TargetEntry> {
    items(data, "targets")
        .iter()
        .map(|raw| {
            let mapped = field(raw, &["mapped_luns", "mapped_lun", "luns"])
                .and_then(Value::as_array)
                .map_or(&[][..], Vec::as_slice);
            let luns = mapped
                .iter()
                .filter_map(|lun| match lun {
                    Value::String(s) => Some(s.trim().to_string()),
                    Value::Number(n) => Some(n.to_string()),
                    other => text(other, &["lun_uuid", "uuid", "lun_id", "id"]),
                })
                .collect();
            let sessions = field(raw, &["connected_sessions", "sessions"])
                .and_then(Value::as_array)
                .map_or(&[][..], Vec::as_slice);
            let mut initiators: Vec<String> = sessions
                .iter()
                .filter_map(|s| text(s, &["iqn", "initiator_iqn", "initiator", "name"]))
                .collect();
            initiators.sort();
            initiators.dedup();
            let count = if sessions.is_empty() {
                num(raw, &["connected_session_count", "session_count"]).unwrap_or(0.0) as usize
            } else {
                sessions.len()
            };
            TargetEntry {
                target: LunTarget {
                    iqn: text(raw, &["iqn"]).unwrap_or_default(),
                    name: text(raw, &["name"]).unwrap_or_default(),
                    sessions: count,
                    initiators,
                },
                luns,
            }
        })
        .collect()
}

/// Rattache les cibles à leurs LUN : toute LUN reçoit alors une liste, vide
/// si aucune cible ne l'expose.
pub fn attach_targets(luns: &mut [Lun], targets: &[TargetEntry]) {
    for lun in luns.iter_mut() {
        let exposed = targets
            .iter()
            .filter(|entry| entry.luns.iter().any(|key| *key == lun.key || *key == lun.name))
            .map(|entry| entry.target.clone())
            .collect();
        lun.targets = Some(exposed);
    }
}

// ------------------------------------------------------------------- calcul

/// Ce que les LUN d'un volume y occupent, et ce qui reste vraiment.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VolumeLunUsage {
    pub luns: usize,
    pub thin_luns: usize,
    /// Somme des tailles des LUN épais.
    pub thick_reserved: f64,
    /// Somme des allocations des LUN fins.
    pub thin_allocated: f64,
    /// Somme des tailles des LUN fins.
    pub thin_provisioned: f64,
    /// Espace promis aux LUN fins et pas encore écrit.
    pub thin_promised: f64,
    /// Données hors LUN.
    pub data: f64,
    pub effective_total: f64,
    pub effective_used: f64,
    /// Occupation effective, en pourcentage ; `None` sans taille.
    pub effective_percent: Option<f64>,
    /// Espace libre du volume.
    pub free: f64,
    /// Espace promis rapporté à l'espace libre, en pourcentage ; `None` sans
    /// LUN fin.
    pub overcommit_percent: Option<f64>,
    /// Les LUN épais prennent tout : il n'y a rien d'autre à remplir.
    pub fully_reserved: bool,
}

/// Le calcul, pur. Sans LUN, l'occupation effective vaut l'occupation brute.
pub fn volume_usage(total: f64, used: f64, luns: &[&Lun]) -> VolumeLunUsage {
    let mut usage = VolumeLunUsage { luns: luns.len(), ..Default::default() };
    for lun in luns {
        let size = lun.size.unwrap_or(0.0);
        if lun.counts_as_thin() {
            // Une allocation ne dépasse pas la taille, sauf taille inconnue.
            let allocated = lun.allocated.unwrap_or(0.0);
            let allocated = if size > 0.0 { allocated.min(size) } else { allocated };
            usage.thin_luns += 1;
            usage.thin_allocated += allocated;
            usage.thin_provisioned += size;
            usage.thin_promised += (size - allocated).max(0.0);
        } else {
            usage.thick_reserved += size;
        }
    }
    let total = total.max(0.0);
    let used = used.max(0.0);
    usage.free = (total - used).max(0.0);
    usage.effective_total = (total - usage.thick_reserved).max(0.0);
    usage.effective_used = (used - usage.thick_reserved).max(0.0).min(usage.effective_total);
    usage.data = (used - usage.thick_reserved - usage.thin_allocated).max(0.0);
    usage.fully_reserved = usage.thick_reserved > 0.0
        && usage.effective_total <= (total * FULLY_RESERVED_REL).max(GIB);
    usage.effective_percent = if total <= 0.0 {
        None
    } else if usage.fully_reserved {
        Some(0.0)
    } else {
        Some(usage.effective_used / usage.effective_total * 100.0)
    };
    if usage.thin_luns > 0 {
        usage.overcommit_percent = Some(if usage.thin_promised <= 0.0 {
            0.0
        } else {
            (usage.thin_promised / usage.free.max(1.0) * 100.0).min(OVERCOMMIT_CAP)
        });
    }
    usage
}

/// Vrai si le LUN est posé sur ce volume (`/volume1` ou `volume_1`).
fn hosted_on(volume: &Volume, lun: &Lun) -> bool {
    let norm = |s: &str| s.trim().trim_matches('/').to_ascii_lowercase();
    let location = norm(&lun.location);
    !location.is_empty()
        && (volume.vol_path.as_deref().is_some_and(|path| norm(path) == location)
            || norm(&volume.id) == location)
}

/// L'identifiant du volume hôte, sinon l'emplacement tel que DSM l'écrit.
fn host_of(volumes: &[Volume], lun: &Lun) -> String {
    volumes
        .iter()
        .find(|volume| hosted_on(volume, lun))
        .map_or_else(|| lun.location.clone(), |volume| volume.id.clone())
}

// ---------------------------------------------------------------- mesures

fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(format!("synology_{metric}"), value, MetricKind::Gauge, ts_ms)
}

/// Grandeurs effectives de chaque volume, avec les mêmes étiquettes que
/// `synology_volume_*` pour que l'interface et les règles les rapprochent.
///
/// `luns` vaut `None` quand le NAS n'a pas d'API iSCSI : l'occupation effective
/// est alors l'occupation brute. Elle est publiée pour tout volume dont la
/// taille est connue, LUN ou pas, pour que la règle et la prévision n'aient
/// qu'une série à suivre.
pub fn volume_samples(volumes: &[Volume], luns: Option<&[Lun]>, ts_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::new();
    for volume in volumes {
        let Some(size) = &volume.size else { continue };
        let (Some(total), Some(used)) = (size.total, size.used) else { continue };
        let hosted: Vec<&Lun> =
            luns.unwrap_or_default().iter().filter(|lun| hosted_on(volume, lun)).collect();
        let usage = volume_usage(total.0, used.0, &hosted);
        let labels = super::metrics::volume_labels(volume);
        let mut push = |metric: &str, value: f64| {
            let mut sample = gauge(metric, value, ts_ms);
            for (key, value) in &labels {
                sample = sample.with_label(*key, value.clone());
            }
            samples.push(sample);
        };
        push("volume_effective_total_bytes", usage.effective_total);
        push("volume_effective_used_bytes", usage.effective_used);
        if let Some(percent) = usage.effective_percent {
            push("volume_effective_used_percent", percent);
        }
        if hosted.is_empty() {
            continue;
        }
        push("volume_lun_count", usage.luns as f64);
        push("volume_lun_reserved_bytes", usage.thick_reserved);
        push("volume_lun_thin_allocated_bytes", usage.thin_allocated);
        push("volume_lun_thin_provisioned_bytes", usage.thin_provisioned);
        push("volume_data_bytes", usage.data);
        push("volume_lun_fully_reserved", if usage.fully_reserved { 1.0 } else { 0.0 });
        if let Some(overcommit) = usage.overcommit_percent {
            push("volume_thin_promised_bytes", usage.thin_promised);
            push("volume_thin_overcommit_percent", overcommit);
        }
    }
    samples
}

/// Une série par LUN et par grandeur, plus le nombre total.
///
/// Le volume hôte est porté par `host_volume`, pas `volume` : la prévision
/// nomme une entité par l'étiquette la plus parlante, et c'est le nom du LUN
/// qu'on veut lire, pas celui de son volume.
pub fn lun_samples(luns: &[Lun], volumes: &[Volume], ts_ms: i64) -> Vec<Sample> {
    let mut samples = vec![gauge("lun_count", luns.len() as f64, ts_ms)];
    for lun in luns {
        let host = host_of(volumes, lun);
        let base = |metric: &str, value: f64| {
            gauge(metric, value, ts_ms)
                .with_label("lun", lun.key.clone())
                .with_label("name", lun.name.clone())
                .with_label("host_volume", host.clone())
        };
        samples.push(
            base("lun_info", 1.0)
                .with_label("lun_type", lun.lun_type.clone())
                .with_label("provisioning", lun.provisioning.as_str())
                .with_label("status", lun.status.clone()),
        );
        if let Some(size) = lun.size {
            samples.push(base("lun_size_bytes", size));
        }
        if let Some(reserved) = lun.reserved() {
            samples.push(base("lun_reserved_bytes", reserved));
        }
        if lun.counts_as_thin()
            && let Some(allocated) = lun.allocated
        {
            samples.push(base("lun_allocated_bytes", allocated));
            if let Some(size) = lun.size.filter(|s| *s > 0.0) {
                samples.push(base("lun_used_percent", (allocated / size * 100.0).min(100.0)));
            }
        }
        if let Some(mapped) = lun.mapped {
            samples.push(base("lun_mapped", if mapped { 1.0 } else { 0.0 }));
        }
        if let Some(targets) = &lun.targets {
            let sessions: usize = targets.iter().map(|t| t.sessions).sum();
            samples.push(base("lun_sessions", sessions as f64));
            for target in targets {
                samples.push(
                    base("lun_target_sessions", target.sessions as f64)
                        .with_label("target_iqn", target.iqn.clone())
                        .with_label("target_name", target.name.clone())
                        .with_label("initiators", target.initiators.join(",")),
                );
            }
        }
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::super::model::{Envelope, Num, VolumeSize};
    use super::*;

    const LUN_LIST: &str = include_str!("testdata/iscsi/lun-list.json");
    const TARGET_LIST: &str = include_str!("testdata/iscsi/target-list.json");
    const TB: f64 = 1_000_000_000_000.0;

    fn data(json: &str) -> Value {
        serde_json::from_str::<Envelope<Value>>(json).unwrap().data.unwrap()
    }

    fn lun(name: &str, provisioning: Provisioning, size: f64, allocated: Option<f64>) -> Lun {
        Lun {
            key: name.to_string(),
            name: name.to_string(),
            location: "/volume1".to_string(),
            provisioning,
            size: Some(size),
            allocated,
            ..Default::default()
        }
    }

    fn volume(id: &str, path: &str, total: f64, used: f64) -> Volume {
        Volume {
            id: id.to_string(),
            vol_path: Some(path.to_string()),
            fs_type: Some("btrfs".to_string()),
            device_type: Some("shr_1".to_string()),
            status: Some("normal".to_string()),
            size: Some(VolumeSize { total: Some(Num(total)), used: Some(Num(used)) }),
            ..Default::default()
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6 * b.abs().max(1.0)
    }

    #[test]
    fn le_type_de_dsm_dit_le_provisionnement() {
        assert_eq!(Provisioning::classify("BLUN_THICK"), Provisioning::Thick);
        assert_eq!(Provisioning::classify("FILE"), Provisioning::Thick);
        assert_eq!(Provisioning::classify("block"), Provisioning::Thick);
        assert_eq!(Provisioning::classify("BLUN"), Provisioning::Thin);
        assert_eq!(Provisioning::classify("THIN"), Provisioning::Thin);
        assert_eq!(Provisioning::classify("ADV"), Provisioning::Thin);
        assert_eq!(Provisioning::classify("263"), Provisioning::Unknown);
        assert_eq!(Provisioning::classify(""), Provisioning::Unknown);
    }

    #[test]
    fn sans_lun_l_occupation_effective_est_l_occupation_brute() {
        let usage = volume_usage(10.0 * TB, 9.5 * TB, &[]);
        assert!(close(usage.effective_total, 10.0 * TB));
        assert!(close(usage.effective_used, 9.5 * TB));
        assert!(close(usage.effective_percent.unwrap(), 95.0));
        assert!(close(usage.data, 9.5 * TB));
        assert_eq!(usage.overcommit_percent, None);
        assert!(!usage.fully_reserved);
    }

    #[test]
    fn un_lun_epais_qui_remplit_le_volume_n_est_pas_un_volume_plein() {
        // 10 To, un LUN épais de 9 To, 0,3 To de partages : DSM affiche 93 %.
        let thick = lun("vmfs", Provisioning::Thick, 9.0 * TB, None);
        let usage = volume_usage(10.0 * TB, 9.3 * TB, &[&thick]);
        assert!(close(usage.thick_reserved, 9.0 * TB));
        assert!(close(usage.effective_total, 1.0 * TB));
        assert!(close(usage.data, 0.3 * TB));
        assert!(close(usage.effective_percent.unwrap(), 30.0), "{usage:?}");
        assert_eq!(usage.overcommit_percent, None, "aucun LUN fin");
        // Les données hors LUN, elles, peuvent remplir le reste.
        let full = volume_usage(10.0 * TB, 9.95 * TB, &[&thick]);
        assert!(close(full.effective_percent.unwrap(), 95.0), "{full:?}");
    }

    #[test]
    fn un_volume_entierement_reserve_ne_se_remplit_plus() {
        let thick = lun("vmfs", Provisioning::Thick, 9.99 * TB, None);
        let usage = volume_usage(10.0 * TB, 9.999 * TB, &[&thick]);
        assert!(usage.fully_reserved, "{usage:?}");
        assert_eq!(usage.effective_percent, Some(0.0));
    }

    #[test]
    fn des_lun_fins_sur_alloues_sont_signales() {
        // 10 To, deux LUN fins de 8 To chacun, 2 To et 3 To écrits : 5 To utilisés,
        // 5 To libres, 11 To promis et pas encore écrits.
        let a = lun("a", Provisioning::Thin, 8.0 * TB, Some(2.0 * TB));
        let b = lun("b", Provisioning::Thin, 8.0 * TB, Some(3.0 * TB));
        let usage = volume_usage(10.0 * TB, 5.0 * TB, &[&a, &b]);
        assert!(close(usage.thin_provisioned, 16.0 * TB));
        assert!(close(usage.thin_allocated, 5.0 * TB));
        assert!(close(usage.thin_promised, 11.0 * TB));
        assert!(close(usage.free, 5.0 * TB));
        assert!(close(usage.overcommit_percent.unwrap(), 220.0), "{usage:?}");
        // Un LUN fin occupe vraiment ce qu'il a écrit : rien n'est retiré de la capacité.
        assert!(close(usage.effective_total, 10.0 * TB));
        assert!(close(usage.effective_percent.unwrap(), 50.0));
        assert!(close(usage.data, 0.0));
    }

    #[test]
    fn epais_et_fins_melanges() {
        // 20 To : un épais de 10 To, un fin de 6 To dont 1 To écrit, 4 To de partages.
        let thick = lun("thick", Provisioning::Thick, 10.0 * TB, None);
        let thin = lun("thin", Provisioning::Thin, 6.0 * TB, Some(1.0 * TB));
        let usage = volume_usage(20.0 * TB, 15.0 * TB, &[&thick, &thin]);
        assert!(close(usage.data, 4.0 * TB));
        assert!(close(usage.effective_total, 10.0 * TB));
        assert!(close(usage.effective_used, 5.0 * TB));
        assert!(close(usage.effective_percent.unwrap(), 50.0));
        // 5 To promis pour 5 To libres : tout juste couvert.
        assert!(close(usage.overcommit_percent.unwrap(), 100.0), "{usage:?}");
    }

    #[test]
    fn un_lun_de_type_inconnu_avec_allocation_compte_comme_fin() {
        let unknown = lun("x", Provisioning::Unknown, 4.0 * TB, Some(1.0 * TB));
        assert_eq!(unknown.reserved(), Some(1.0 * TB));
        let blind = lun("y", Provisioning::Unknown, 4.0 * TB, None);
        assert_eq!(blind.reserved(), Some(4.0 * TB), "sans allocation, prudence : épais");
    }

    #[test]
    fn la_liste_des_lun_et_des_cibles_est_relue() {
        let mut luns = parse_luns(&data(LUN_LIST));
        assert_eq!(luns.len(), 3);
        let prod = &luns[0];
        assert_eq!(prod.name, "LUN-vmfs-prod");
        assert_eq!(prod.provisioning, Provisioning::Thick);
        assert_eq!(prod.location, "/volume1");
        assert_eq!(prod.size, Some(9_895_604_649_984.0));
        assert_eq!(prod.mapped, Some(true));
        let backup = &luns[1];
        assert_eq!(backup.provisioning, Provisioning::Thin);
        assert_eq!(backup.allocated, Some(1_649_267_441_664.0), "taille en chaîne");

        attach_targets(&mut luns, &parse_targets(&data(TARGET_LIST)));
        let targets = luns[0].targets.as_ref().unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].sessions, 2);
        assert_eq!(targets[0].initiators[0], "iqn.1998-01.com.vmware:esxi-01-4f2a");
        assert_eq!(luns[1].targets.as_ref().unwrap()[0].sessions, 0);
        assert_eq!(luns[2].targets.as_deref(), Some(&[][..]), "exposé par aucune cible");
    }

    #[test]
    fn les_series_effectives_portent_les_etiquettes_du_volume() {
        let luns = parse_luns(&data(LUN_LIST));
        let volumes = [
            volume("volume_1", "/volume1", 12.0 * TB, 10.4 * TB),
            volume("volume_2", "/volume2", 8.0 * TB, 3.0 * TB),
            volume("volume_3", "/volume3", 4.0 * TB, 1.0 * TB),
        ];
        let samples = volume_samples(&volumes, Some(&luns), 1000);
        let find = |metric: &str, id: &str| {
            samples
                .iter()
                .find(|s| s.metric == metric && s.labels["volume"] == id)
                .map(|s| s.value)
        };
        let percent = find("synology_volume_effective_used_percent", "volume_1").unwrap();
        // 12 To, LUN épais de 9 Tio (9,9 To), 10,4 To utilisés : 0,5 To sur 2,1 To.
        assert!(percent > 20.0 && percent < 30.0, "le LUN épais n'est pas compté : {percent}");
        let sample = samples
            .iter()
            .find(|s| s.metric == "synology_volume_effective_used_percent")
            .unwrap();
        for key in ["volume", "name", "fs_type", "raid_type"] {
            assert!(sample.labels.contains_key(key), "étiquette {key} manquante");
        }
        // Volume 3, sans LUN : occupation brute, et aucune série de LUN.
        assert_eq!(find("synology_volume_effective_used_percent", "volume_3"), Some(25.0));
        assert_eq!(find("synology_volume_lun_count", "volume_3"), None);
        // Volume 2 : deux LUN fins (4 Tio et 2 Tio), en partie écrits.
        assert_eq!(find("synology_volume_lun_count", "volume_2"), Some(2.0));
        assert!(find("synology_volume_thin_overcommit_percent", "volume_2").is_some());
        assert_eq!(find("synology_volume_thin_overcommit_percent", "volume_1"), None);

        // Sans API iSCSI : la même série, égale à la brute.
        let bare = volume_samples(&volumes, None, 1000);
        let raw = bare
            .iter()
            .find(|s| {
                s.metric == "synology_volume_effective_used_percent"
                    && s.labels["volume"] == "volume_1"
            })
            .unwrap();
        assert!(close(raw.value, 1040.0 / 12.0));
    }

    #[test]
    fn chaque_lun_est_publie_avec_son_volume_hote() {
        let mut luns = parse_luns(&data(LUN_LIST));
        attach_targets(&mut luns, &parse_targets(&data(TARGET_LIST)));
        let volumes = [volume("volume_1", "/volume1", 10.0 * TB, 9.95 * TB)];
        let samples = lun_samples(&luns, &volumes, 1000);
        let info = samples
            .iter()
            .find(|s| s.metric == "synology_lun_info" && s.labels["name"] == "LUN-vmfs-prod")
            .unwrap();
        assert_eq!(info.labels["host_volume"], "volume_1");
        assert_eq!(info.labels["provisioning"], "thick");
        assert!(!info.labels.contains_key("volume"), "pas d'étiquette `volume` sur un LUN");
        // Un LUN épais n'a ni allocation ni remplissage : le NAS ne voit pas dedans.
        assert!(
            !samples
                .iter()
                .any(|s| s.metric == "synology_lun_used_percent"
                    && s.labels["name"] == "LUN-vmfs-prod")
        );
        let thin = samples
            .iter()
            .find(|s| s.metric == "synology_lun_used_percent" && s.labels["name"] == "LUN-backup")
            .unwrap();
        assert!(close(thin.value, 37.5));
        // Volume inconnu : l'emplacement de DSM tel quel.
        let other = samples
            .iter()
            .find(|s| s.metric == "synology_lun_info" && s.labels["name"] == "LUN-backup")
            .unwrap();
        assert_eq!(other.labels["host_volume"], "/volume2");
        let sessions = samples
            .iter()
            .find(|s| s.metric == "synology_lun_sessions" && s.labels["name"] == "LUN-vmfs-prod")
            .unwrap();
        assert_eq!(sessions.value, 2.0);
    }
}
