//! Mesures et vues que déclare chaque type, pour la page de détail.
//!
//! Chaque intégration dit ici quelles familles elle émet (unité, nature, sens
//! d'un bon chiffre) et lesquelles montrer : une rangée de chiffres (`stat`) puis
//! zéro à quatre graphiques. Ce que les vues ne citent pas reste accessible dans
//! « Toutes les mesures » ; un type sans vues garde le rendu générique.
//!
//! Un fichier par type dans `crates/collectors/catalog/<kind>.yaml` :
//!
//! ```yaml
//! source: adguard            # répertoire du collecteur sous src/ (tests)
//! prefixes: [adguard_]       # préfixes ajoutés par les assistants d'émission (tests)
//! metrics:
//!   - { name: adguard_avg_processing_seconds, label: "Processing time", unit: "s" }
//! views:
//!   - view: timeseries
//!     title: "DNS latency"
//!     metrics: [adguard_avg_processing_seconds]
//! ```
//!
//! Les noms sont ceux qu'émet le collecteur, sans le préfixe `dumbmonit_` que le
//! serveur ajoute à l'écriture. Les profils SNMP portent leurs vues eux-mêmes
//! (`Catalog::measures`) et l'agent est décrit par le serveur.

#[cfg(feature = "catalog")]
use std::collections::BTreeMap;
use std::collections::HashSet;
#[cfg(feature = "catalog")]
use std::sync::OnceLock;

use dumbmonit_proto::{Measures, MetricDescription, ViewDescription};
use serde::Deserialize;
#[cfg(feature = "catalog")]
use tracing::error;

/// Vues que l'interface sait rendre. Une autre est refusée à la lecture : une
/// faute de frappe ne doit pas faire disparaître un graphique en silence.
pub const VIEWS: &[&str] = &["stat", "timeseries", "topn", "heatmap", "gauge", "table"];

/// Natures de mesure reconnues (`MetricDescription::kind`).
pub const KINDS: &[&str] = &["gauge", "counter", "info", "state"];

/// Graphiques au plus par type, hors rangée de chiffres : au-delà, la page
/// redevient la liste de courbes que le catalogue sert à éviter.
pub const MAX_CHARTS: usize = 4;

/// Un fichier `catalog/<kind>.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogFile {
    /// Répertoire des sources du collecteur, sous `crates/collectors/src/` : les
    /// tests y cherchent chaque nom déclaré.
    #[serde(default)]
    pub source: String,
    /// Préfixes que les assistants d'émission du collecteur ajoutent aux noms
    /// (`gauge("node_cpu_percent")` émet `proxmox_node_cpu_percent`).
    #[serde(default)]
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub metrics: Vec<MetricDescription>,
    #[serde(default)]
    pub views: Vec<ViewDescription>,
}

impl CatalogFile {
    /// Lit un fichier et vérifie sa cohérence.
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: Self = crate::yaml::from_str(text).map_err(|error| error.to_string())?;
        file.validate()?;
        Ok(file)
    }

    fn validate(&self) -> Result<(), String> {
        let mut names: HashSet<&str> = HashSet::new();
        for metric in &self.metrics {
            if !names.insert(&metric.name) {
                return Err(format!("metric \"{}\" is declared twice", metric.name));
            }
            if !KINDS.contains(&&*metric.kind) {
                let (name, kind) = (&metric.name, &metric.kind);
                return Err(format!("metric \"{name}\": unknown kind \"{kind}\""));
            }
        }
        let mut charts = 0;
        for view in &self.views {
            if !VIEWS.contains(&&*view.view) {
                let (title, kind) = (&view.title, &view.view);
                return Err(format!("view \"{title}\": unknown view \"{kind}\""));
            }
            if view.metrics.is_empty() {
                return Err(format!("view \"{}\" shows no metric", view.title));
            }
            if let Some(name) = view.metrics.iter().find(|name| !names.contains(&***name)) {
                let title = &view.title;
                return Err(format!("view \"{title}\": metric \"{name}\" is not declared"));
            }
            if view.view != "stat" {
                charts += 1;
            }
        }
        if charts > MAX_CHARTS {
            return Err(format!("{charts} charts, at most {MAX_CHARTS}"));
        }
        Ok(())
    }

    /// La part servie par l'API.
    pub fn measures(&self) -> Measures {
        Measures { metrics: self.metrics.clone(), views: self.views.clone() }
    }

    /// Les noms déclarés que `source` (le texte des sources du collecteur) ne
    /// contient pas : ni le nom complet comme mot, ni, privé d'un des préfixes,
    /// comme littéral (`"node_cpu_percent"`) ou fin de littéral formaté
    /// (`"{p}_node_cpu_percent"`).
    pub fn unemitted<'a>(&'a self, source: &str) -> Vec<&'a str> {
        let emitted = |name: &str| {
            contains_word(source, name)
                || self.prefixes.iter().filter_map(|prefix| name.strip_prefix(prefix.as_str())).any(
                    |rest| {
                        source.contains(&format!("\"{rest}\""))
                            || source.contains(&format!("}}_{rest}\""))
                    },
                )
        };
        self.metrics.iter().map(|metric| &*metric.name).filter(|&name| !emitted(name)).collect()
    }
}

/// `word` figure dans `text` sans être collé à une lettre, un chiffre ou un `_`.
fn contains_word(text: &str, word: &str) -> bool {
    let part = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(part) && !after.is_some_and(part)
    })
}

/// Les fichiers du catalogue, incorporés au binaire comme les profils SNMP.
/// Seulement avec la fonctionnalité `catalog` (le serveur) : l'agent relais n'en
/// a pas l'usage.
#[cfg(feature = "catalog")]
const EMBEDDED: &[(&str, &str)] = &[
    ("activedirectory", include_str!("../catalog/activedirectory.yaml")),
    ("adguard", include_str!("../catalog/adguard.yaml")),
    ("apache", include_str!("../catalog/apache.yaml")),
    ("arubacentral", include_str!("../catalog/arubacentral.yaml")),
    ("authentik", include_str!("../catalog/authentik.yaml")),
    ("aws", include_str!("../catalog/aws.yaml")),
    ("azure", include_str!("../catalog/azure.yaml")),
    ("caddy", include_str!("../catalog/caddy.yaml")),
    ("ceph", include_str!("../catalog/ceph.yaml")),
    ("cloudflare", include_str!("../catalog/cloudflare.yaml")),
    ("crowdsec", include_str!("../catalog/crowdsec.yaml")),
    ("dns", include_str!("../catalog/dns.yaml")),
    ("domain", include_str!("../catalog/domain.yaml")),
    ("duplicati", include_str!("../catalog/duplicati.yaml")),
    ("elasticsearch", include_str!("../catalog/elasticsearch.yaml")),
    ("entra", include_str!("../catalog/entra.yaml")),
    ("forgejo", include_str!("../catalog/forgejo.yaml")),
    ("fortigate", include_str!("../catalog/fortigate.yaml")),
    ("frigate", include_str!("../catalog/frigate.yaml")),
    ("gitlab", include_str!("../catalog/gitlab.yaml")),
    ("graylog", include_str!("../catalog/graylog.yaml")),
    ("grpc", include_str!("../catalog/grpc.yaml")),
    ("hetzner", include_str!("../catalog/hetzner.yaml")),
    ("homeassistant", include_str!("../catalog/homeassistant.yaml")),
    ("http", include_str!("../catalog/http.yaml")),
    ("imap", include_str!("../catalog/imap.yaml")),
    ("immich", include_str!("../catalog/immich.yaml")),
    ("influxdb", include_str!("../catalog/influxdb.yaml")),
    ("infomaniak", include_str!("../catalog/infomaniak.yaml")),
    ("ip_blacklist", include_str!("../catalog/ip_blacklist.yaml")),
    ("jellyfin", include_str!("../catalog/jellyfin.yaml")),
    ("kafka", include_str!("../catalog/kafka.yaml")),
    ("keycloak", include_str!("../catalog/keycloak.yaml")),
    ("kopia", include_str!("../catalog/kopia.yaml")),
    ("kubernetes", include_str!("../catalog/kubernetes.yaml")),
    ("lidarr", include_str!("../catalog/lidarr.yaml")),
    ("loki", include_str!("../catalog/loki.yaml")),
    ("m365", include_str!("../catalog/m365.yaml")),
    ("mail_blacklist", include_str!("../catalog/mail_blacklist.yaml")),
    ("mailcow", include_str!("../catalog/mailcow.yaml")),
    ("mdaemon", include_str!("../catalog/mdaemon.yaml")),
    ("meraki", include_str!("../catalog/meraki.yaml")),
    ("mikrotik", include_str!("../catalog/mikrotik.yaml")),
    ("minio", include_str!("../catalog/minio.yaml")),
    ("mongodb", include_str!("../catalog/mongodb.yaml")),
    ("mqtt", include_str!("../catalog/mqtt.yaml")),
    ("mssql", include_str!("../catalog/mssql.yaml")),
    ("mysql", include_str!("../catalog/mysql.yaml")),
    ("nextcloud", include_str!("../catalog/nextcloud.yaml")),
    ("nginx", include_str!("../catalog/nginx.yaml")),
    ("npm", include_str!("../catalog/npm.yaml")),
    ("ntp", include_str!("../catalog/ntp.yaml")),
    ("nutanix", include_str!("../catalog/nutanix.yaml")),
    ("nut", include_str!("../catalog/nut.yaml")),
    ("omada", include_str!("../catalog/omada.yaml")),
    ("openvpn", include_str!("../catalog/openvpn.yaml")),
    ("opnsense", include_str!("../catalog/opnsense.yaml")),
    ("oracle", include_str!("../catalog/oracle.yaml")),
    ("ovh", include_str!("../catalog/ovh.yaml")),
    ("paloalto", include_str!("../catalog/paloalto.yaml")),
    ("paperless", include_str!("../catalog/paperless.yaml")),
    ("pbs", include_str!("../catalog/pbs.yaml")),
    ("pdm", include_str!("../catalog/pdm.yaml")),
    ("pfsense", include_str!("../catalog/pfsense.yaml")),
    ("pihole", include_str!("../catalog/pihole.yaml")),
    ("ping", include_str!("../catalog/ping.yaml")),
    ("plex", include_str!("../catalog/plex.yaml")),
    ("pmg", include_str!("../catalog/pmg.yaml")),
    ("pop3", include_str!("../catalog/pop3.yaml")),
    ("postgres", include_str!("../catalog/postgres.yaml")),
    ("provider", include_str!("../catalog/provider.yaml")),
    ("prowlarr", include_str!("../catalog/prowlarr.yaml")),
    ("proxmox", include_str!("../catalog/proxmox.yaml")),
    ("qbittorrent", include_str!("../catalog/qbittorrent.yaml")),
    ("qnap", include_str!("../catalog/qnap.yaml")),
    ("rabbitmq", include_str!("../catalog/rabbitmq.yaml")),
    ("radarr", include_str!("../catalog/radarr.yaml")),
    ("radius", include_str!("../catalog/radius.yaml")),
    ("redfish", include_str!("../catalog/redfish.yaml")),
    ("redis", include_str!("../catalog/redis.yaml")),
    ("securitygateway", include_str!("../catalog/securitygateway.yaml")),
    ("smtp", include_str!("../catalog/smtp.yaml")),
    ("snmp_oid", include_str!("../catalog/snmp_oid.yaml")),
    ("sonarr", include_str!("../catalog/sonarr.yaml")),
    ("sonicwall", include_str!("../catalog/sonicwall.yaml")),
    ("sophos", include_str!("../catalog/sophos.yaml")),
    ("ssh", include_str!("../catalog/ssh.yaml")),
    ("synology", include_str!("../catalog/synology.yaml")),
    ("tailscale", include_str!("../catalog/tailscale.yaml")),
    ("tcp", include_str!("../catalog/tcp.yaml")),
    ("tls", include_str!("../catalog/tls.yaml")),
    ("traefik", include_str!("../catalog/traefik.yaml")),
    ("truenas", include_str!("../catalog/truenas.yaml")),
    ("udp", include_str!("../catalog/udp.yaml")),
    ("unifiprotect", include_str!("../catalog/unifiprotect.yaml")),
    ("unifi", include_str!("../catalog/unifi.yaml")),
    ("unraid", include_str!("../catalog/unraid.yaml")),
    ("vaultwarden", include_str!("../catalog/vaultwarden.yaml")),
    ("veeam", include_str!("../catalog/veeam.yaml")),
    ("victorialogs", include_str!("../catalog/victorialogs.yaml")),
    ("victoriametrics", include_str!("../catalog/victoriametrics.yaml")),
    ("vsphere", include_str!("../catalog/vsphere.yaml")),
    ("websocket", include_str!("../catalog/websocket.yaml")),
    ("winrm", include_str!("../catalog/winrm.yaml")),
    ("xcpng", include_str!("../catalog/xcpng.yaml")),
];

/// Mesures et vues du type `kind` d'après le catalogue embarqué ; `None` pour un
/// type qui n'y figure pas (rendu générique).
#[cfg(feature = "catalog")]
pub fn embedded(kind: &str) -> Option<Measures> {
    static TABLE: OnceLock<BTreeMap<&'static str, Measures>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            let mut table = BTreeMap::new();
            for (kind, text) in EMBEDDED {
                match CatalogFile::parse(text) {
                    Ok(file) => {
                        table.insert(*kind, file.measures());
                    }
                    // Les tests garantissent la lecture ; un fichier fautif ne doit
                    // pas pour autant priver les autres types de leurs vues.
                    Err(error) => error!(kind, %error, "catalogue des mesures ignoré"),
                }
            }
            table
        })
        .get(kind)
        .cloned()
}

/// Les types que décrit le catalogue embarqué.
#[cfg(feature = "catalog")]
pub fn embedded_kinds() -> impl Iterator<Item = &'static str> {
    EMBEDDED.iter().map(|(kind, _)| *kind)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "catalog")]
    use std::path::Path;
    #[cfg(feature = "catalog")]
    use std::time::Duration;

    use super::*;

    /// Le texte de tous les fichiers `.rs` sous `dir`.
    #[cfg(feature = "catalog")]
    fn sources(dir: &Path) -> String {
        let mut text = String::new();
        let mut pending = vec![dir.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{dir:?} : {e}")) {
                let path = entry.expect("entrée de répertoire").path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    text.push_str(&std::fs::read_to_string(&path).expect("source lisible"));
                }
            }
        }
        text
    }

    #[cfg(feature = "catalog")]
    #[test]
    fn chaque_fichier_du_catalogue_se_lit() {
        for (kind, text) in EMBEDDED {
            if let Err(error) = CatalogFile::parse(text) {
                panic!("catalog/{kind}.yaml : {error}");
            }
            assert!(embedded(kind).is_some(), "« {kind} » absent de la table");
        }
    }

    #[cfg(feature = "catalog")]
    #[test]
    fn chaque_mesure_declaree_est_emise_par_son_collecteur() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for (kind, text) in EMBEDDED {
            let file = CatalogFile::parse(text).expect("catalogue lisible");
            assert!(!file.source.is_empty(), "catalog/{kind}.yaml : « source » manquant");
            let source = sources(&root.join(&file.source));
            let missing = file.unemitted(&source);
            let dir = &file.source;
            assert!(
                missing.is_empty(),
                "catalog/{kind}.yaml : jamais émis par src/{dir} : {missing:?}"
            );
        }
    }

    #[cfg(feature = "catalog")]
    #[tokio::test]
    async fn le_catalogue_ne_decrit_que_des_types_enregistres() {
        let registry = crate::Registry::remote(Duration::from_secs(5));
        let kinds = registry.kinds();
        for kind in embedded_kinds() {
            assert!(kinds.iter().any(|known| &**known == kind), "« {kind} » n'est pas enregistré");
        }
    }

    #[test]
    fn une_vue_ne_cite_que_des_familles_declarees() {
        let text = "metrics:\n  - { name: a, kind: counter }\n\
                    views:\n  - { view: timeseries, metrics: [b] }\n";
        let error = CatalogFile::parse(text).unwrap_err();
        assert!(error.contains("\"b\" is not declared"), "{error}");
    }

    #[test]
    fn une_vue_inconnue_est_refusee() {
        let text = "metrics:\n  - { name: a }\nviews:\n  - { view: pie, metrics: [a] }\n";
        assert!(CatalogFile::parse(text).unwrap_err().contains("unknown view"));
    }

    #[test]
    fn cinq_graphiques_sont_refuses() {
        let views: String = (0..5).map(|_| "  - { view: timeseries, metrics: [a] }\n").collect();
        let text = format!("metrics:\n  - {{ name: a }}\nviews:\n{views}");
        assert!(CatalogFile::parse(&text).unwrap_err().contains("at most 4"));
    }

    #[test]
    fn la_nature_par_defaut_est_une_jauge() {
        let file = CatalogFile::parse("metrics:\n  - { name: a, unit: \"%\" }\n").unwrap();
        assert_eq!(file.metrics[0].kind, "gauge");
        assert_eq!(file.metrics[0].unit, "%");
    }

    #[test]
    fn un_nom_prefixe_par_un_assistant_est_reconnu() {
        let file = CatalogFile::parse(
            "prefixes: [proxmox_]\n\
             metrics:\n  - { name: proxmox_node_cpu_percent }\n  - { name: proxmox_absent }\n",
        )
        .unwrap();
        let source = r#"push(gauge("node_cpu_percent", value, ts_ms));"#;
        assert_eq!(file.unemitted(source), ["proxmox_absent"]);
        let formatted = r#"gauge(&format!("{p}_node_cpu_percent"), 1.0, ts_ms)"#;
        assert_eq!(file.unemitted(formatted), ["proxmox_absent"]);
        assert!(!contains_word("proxmox_node_cpu_percent_max", "proxmox_node_cpu_percent"));
    }
}
