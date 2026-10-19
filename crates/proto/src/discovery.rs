//! Découverte réseau : empreintes déclarées par les collecteurs, résultats d'un
//! balayage, et commande confiée à un agent relais.
//!
//! Le moteur (caisse `dumbmonit-collectors`, module `netscan`) ne connaît aucun
//! produit : chaque collecteur déclare ses empreintes
//! ([`Collector::fingerprints`](crate::Collector::fingerprints)), le registre
//! les rassemble, et le moteur se contente de les évaluer. Ajouter la
//! reconnaissance d'un produit se fait donc à côté de son collecteur.
//!
//! Une empreinte n'est jamais une tentative d'authentification : une requête
//! anonyme (`GET`), une bannière lue (ou provoquée par quelques octets
//! inoffensifs), un port ouvert, ou les scalaires d'identification SNMP.

use serde::{Deserialize, Serialize};

use crate::command::AgentCommand;

/// Plafond d'adresses d'un balayage : un `/22`. Au-delà, un réseau saisi par
/// mégarde coûterait des dizaines de milliers de connexions.
pub const MAX_DISCOVERY_HOSTS: usize = 1024;

/// Balayage confié à un agent relais. Arguments : un [`DiscoveryScanJob`].
pub const CMD_DISCOVERY_SCAN: &str = "discovery.scan";

/// Route de compte rendu progressif d'un balayage relayé :
/// `POST {DISCOVERY_REPORT_PATH}/{id}?key=…`, corps [`DiscoveryReport`].
/// `204` : continuer ; `410` : le balayage a été annulé (ou n'existe plus),
/// l'agent s'arrête.
pub const DISCOVERY_REPORT_PATH: &str = "/api/agent/relay/discovery";

// ------------------------------------------------------------- empreintes

/// Degré de certitude d'une reconnaissance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Une condition sur une réponse HTTP. Toutes les comparaisons de texte sont
/// des « contient », insensibles à la casse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "on")]
pub enum HttpMatch {
    /// Code de statut exact.
    Status {
        code: u16,
    },
    /// En-tête présent et contenant le texte (vide : simple présence).
    Header {
        name: String,
        contains: String,
    },
    Body {
        contains: String,
    },
    /// Contenu de la balise `<title>`.
    Title {
        contains: String,
    },
    /// Sujet, émetteur ou noms alternatifs du certificat TLS présenté.
    Cert {
        contains: String,
    },
}

/// Où lire la version d'un produit reconnu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "from")]
pub enum VersionRule {
    /// Texte qui suit `after` dans l'en-tête `name`, jusqu'à une espace, `;`, `)` ou `,`.
    Header { name: String, after: String },
    /// Texte qui suit `after` dans le corps, jusqu'à un guillemet, `<`, une espace ou `,`.
    Body { after: String },
    /// Valeur d'un corps JSON, par pointeur (RFC 6901) : `/version`.
    Json { pointer: String },
    /// Texte qui suit `after` dans la bannière, jusqu'à une espace ou une fin de ligne.
    Banner { after: String },
}

/// La sonde à mener sur un port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FingerprintProbe {
    /// `GET path`, TLS essayé d'abord (certificat jamais vérifié), sans suivre
    /// de redirection. Toutes les conditions doivent être vraies.
    Http { path: String, all: Vec<HttpMatch> },
    /// Ce que le service écrit (en premier, ou après `send`, quelques octets
    /// inoffensifs). Toutes les sous-chaînes doivent y figurer, casse ignorée.
    Banner {
        #[serde(default)]
        send: Option<Vec<u8>>,
        all: Vec<String>,
    },
    /// Le port ouvert suffit (et rien n'y est envoyé).
    Port,
    /// Identification SNMP : préfixe de `sysObjectID` et/ou sous-chaîne de `sysDescr`.
    Snmp {
        #[serde(default)]
        sysobjectid_prefix: Option<String>,
        #[serde(default)]
        sysdescr_contains: Option<String>,
    },
}

impl FingerprintProbe {
    /// Vrai si la sonde parle HTTP sur le port.
    pub fn is_http(&self) -> bool {
        matches!(self, Self::Http { .. })
    }
}

/// Forme de l'adresse pré-remplie, telle que le collecteur du type l'attend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "form")]
pub enum AddressForm {
    /// `192.168.1.20`
    Host,
    /// `192.168.1.20:22`
    HostPort,
    /// `https://192.168.1.20:8443/path`
    Url { path: String },
}

/// Valeur d'une option pré-remplie (`Target::tags`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "value")]
pub enum PrefillValue {
    /// Le port où le produit a été vu.
    Port,
    /// `https` si le port parle TLS, `http` sinon.
    Scheme,
    /// `true` si le port parle TLS (certificat souvent auto-signé en réseau local).
    InsecureTls,
    /// Valeur fixe.
    Text { text: String },
}

/// Ce que le formulaire de cible reçoit pour le type reconnu. Jamais de secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefillSpec {
    pub address: AddressForm,
    /// Options du type (clés de `Target::tags`, telles que `GET /api/collectors`
    /// les décrit).
    #[serde(default)]
    pub tags: Vec<(String, PrefillValue)>,
}

impl PrefillSpec {
    pub fn host() -> Self {
        Self { address: AddressForm::Host, tags: Vec::new() }
    }

    pub fn host_port() -> Self {
        Self { address: AddressForm::HostPort, tags: Vec::new() }
    }

    pub fn url(path: &str) -> Self {
        Self { address: AddressForm::Url { path: path.to_string() }, tags: Vec::new() }
    }

    pub fn tag(mut self, key: &str, value: PrefillValue) -> Self {
        self.tags.push((key.to_string(), value));
        self
    }

    /// Raccourci le plus courant : `port`, et `insecure_tls` si TLS.
    pub fn port_and_tls(self) -> Self {
        self.tag("port", PrefillValue::Port).tag("insecure_tls", PrefillValue::InsecureTls)
    }
}

/// Une empreinte : comment reconnaître un produit sur un port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    /// Identifiant stable (`proxmox.web`), pour les tests et les journaux.
    pub id: String,
    /// Libellé lisible : « Proxmox VE ».
    pub product: String,
    /// Type de collecteur DumbMonit correspondant ; `None` si aucun.
    pub kind: Option<String>,
    pub confidence: Confidence,
    /// Ports TCP où la sonde s'applique. Vide : tout port qui parle HTTP
    /// (empreintes HTTP seulement).
    pub ports: Vec<u16>,
    pub probe: FingerprintProbe,
    pub version: Option<VersionRule>,
    pub prefill: Option<PrefillSpec>,
    /// Empreinte générique (SSH, HTTP, SNMP) : retenue seulement si rien de plus
    /// précis n'a reconnu le port.
    pub generic: bool,
}

impl Fingerprint {
    fn new(id: &str, product: &str, ports: &[u16], probe: FingerprintProbe) -> Self {
        Self {
            id: id.to_string(),
            product: product.to_string(),
            kind: None,
            confidence: Confidence::High,
            ports: ports.to_vec(),
            probe,
            version: None,
            prefill: None,
            generic: false,
        }
    }

    /// `GET path` sur les ports donnés.
    pub fn http(id: &str, product: &str, ports: &[u16], path: &str) -> Self {
        Self::new(
            id,
            product,
            ports,
            FingerprintProbe::Http { path: path.to_string(), all: Vec::new() },
        )
    }

    /// Bannière lue sans rien envoyer.
    pub fn banner(id: &str, product: &str, ports: &[u16], contains: &[&str]) -> Self {
        Self::new(
            id,
            product,
            ports,
            FingerprintProbe::Banner {
                send: None,
                all: contains.iter().map(|s| s.to_string()).collect(),
            },
        )
    }

    /// Bannière provoquée par `send`.
    pub fn exchange(
        id: &str,
        product: &str,
        ports: &[u16],
        send: &[u8],
        contains: &[&str],
    ) -> Self {
        Self::new(
            id,
            product,
            ports,
            FingerprintProbe::Banner {
                send: Some(send.to_vec()),
                all: contains.iter().map(|s| s.to_string()).collect(),
            },
        )
        .confidence(Confidence::Medium)
    }

    /// Port ouvert seul : confiance basse par défaut.
    pub fn port(id: &str, product: &str, ports: &[u16]) -> Self {
        Self::new(id, product, ports, FingerprintProbe::Port).confidence(Confidence::Low)
    }

    /// Identification SNMP (UDP 161).
    pub fn snmp(
        id: &str,
        product: &str,
        sysobjectid_prefix: Option<&str>,
        sysdescr_contains: Option<&str>,
    ) -> Self {
        Self::new(
            id,
            product,
            &[],
            FingerprintProbe::Snmp {
                sysobjectid_prefix: sysobjectid_prefix.map(str::to_string),
                sysdescr_contains: sysdescr_contains.map(str::to_string),
            },
        )
    }

    pub fn kind(mut self, kind: &str) -> Self {
        self.kind = Some(kind.to_string());
        self
    }

    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn generic(mut self) -> Self {
        self.generic = true;
        self
    }

    pub fn version(mut self, rule: VersionRule) -> Self {
        self.version = Some(rule);
        self
    }

    pub fn prefill(mut self, prefill: PrefillSpec) -> Self {
        self.prefill = Some(prefill);
        self
    }

    fn push_match(mut self, condition: HttpMatch) -> Self {
        if let FingerprintProbe::Http { all, .. } = &mut self.probe {
            all.push(condition);
        }
        self
    }

    pub fn status(self, code: u16) -> Self {
        self.push_match(HttpMatch::Status { code })
    }

    pub fn header(self, name: &str, contains: &str) -> Self {
        self.push_match(HttpMatch::Header {
            name: name.to_ascii_lowercase(),
            contains: contains.to_string(),
        })
    }

    pub fn body(self, contains: &str) -> Self {
        self.push_match(HttpMatch::Body { contains: contains.to_string() })
    }

    pub fn title(self, contains: &str) -> Self {
        self.push_match(HttpMatch::Title { contains: contains.to_string() })
    }

    pub fn cert(self, contains: &str) -> Self {
        self.push_match(HttpMatch::Cert { contains: contains.to_string() })
    }

    pub fn version_header(self, name: &str, after: &str) -> Self {
        self.version(VersionRule::Header {
            name: name.to_ascii_lowercase(),
            after: after.to_string(),
        })
    }

    pub fn version_body(self, after: &str) -> Self {
        self.version(VersionRule::Body { after: after.to_string() })
    }

    pub fn version_json(self, pointer: &str) -> Self {
        self.version(VersionRule::Json { pointer: pointer.to_string() })
    }

    pub fn version_banner(self, after: &str) -> Self {
        self.version(VersionRule::Banner { after: after.to_string() })
    }
}

// ------------------------------------------------------------- résultats

/// Profondeur du balayage : la liste de ports essayés.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanDepth {
    /// Une trentaine de ports courants.
    #[default]
    Standard,
    /// Les mêmes, plus tous ceux que déclarent les empreintes et quelques autres.
    Extended,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanPhase {
    #[default]
    Sweep,
    Fingerprint,
    Done,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanProgress {
    pub phase: ScanPhase,
    /// Adresses entièrement traitées.
    pub probed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Tcp,
    Udp,
}

/// Un service reconnu sur un hôte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub port: u16,
    pub protocol: Transport,
    pub product: String,
    pub kind: Option<String>,
    pub version: Option<String>,
    pub confidence: Confidence,
    pub tls: bool,
    /// Titre HTML, bannière, `sysDescr` tronqué…
    pub detail: Option<String>,
    /// Cible existante qui pointe déjà sur cette adresse (et ce port) : rempli
    /// par le serveur, jamais par le moteur.
    #[serde(default)]
    pub monitored_target_id: Option<String>,
    /// Champs du formulaire de cible pour `kind` : `name`, `address`, `tags`
    /// (et `profile_id`, `via_agent`). Jamais de secret.
    pub prefill: Option<serde_json::Value>,
}

/// Un hôte qui a répondu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredHost {
    pub ip: String,
    /// DNS inverse.
    pub hostname: Option<String>,
    /// Seulement si visible (même segment de réseau, mode réseau `host`).
    pub mac: Option<String>,
    pub vendor: Option<String>,
    pub services: Vec<DiscoveredService>,
}

// ------------------------------------------------------------- relais

/// Balayage confié à un agent relais, dans les arguments d'une commande
/// [`CMD_DISCOVERY_SCAN`].
#[derive(Clone, Serialize, Deserialize)]
pub struct DiscoveryScanJob {
    pub cidr: String,
    /// Community SNMP essayée sur chaque adresse ; `public` si absente. Voyage
    /// comme les secrets des sondes déléguées : en mémoire, jamais journalisée.
    #[serde(default)]
    pub snmp_community: Option<String>,
    #[serde(default)]
    pub depth: ScanDepth,
}

impl std::fmt::Debug for DiscoveryScanJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryScanJob")
            .field("cidr", &self.cidr)
            .field("snmp_community", &self.snmp_community.as_ref().map(|_| "<redacted>"))
            .field("depth", &self.depth)
            .finish()
    }
}

impl DiscoveryScanJob {
    pub fn into_command(self, id: i64, created_at_ms: i64) -> AgentCommand {
        AgentCommand {
            id,
            kind: CMD_DISCOVERY_SCAN.to_string(),
            args: serde_json::to_value(self).unwrap_or(serde_json::Value::Null),
            created_at_ms,
        }
    }

    pub fn from_command(command: &AgentCommand) -> Result<Self, String> {
        if command.kind != CMD_DISCOVERY_SCAN {
            return Err(format!("not a discovery command: {}", command.kind));
        }
        serde_json::from_value(command.args.clone())
            .map_err(|error| format!("unreadable discovery job: {error}"))
    }
}

/// Compte rendu progressif d'un balayage relayé.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub progress: ScanProgress,
    /// Hôtes trouvés depuis le compte rendu précédent.
    #[serde(default)]
    pub hosts: Vec<DiscoveredHost>,
    /// Dernier compte rendu : le balayage est terminé (ou a échoué).
    #[serde(default)]
    pub finished: bool,
    #[serde(default)]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_discovery_job_survives_the_command_envelope_and_hides_its_community() {
        let job = DiscoveryScanJob {
            cidr: "192.168.1.0/24".into(),
            snmp_community: Some("s3cr3t".into()),
            depth: ScanDepth::Extended,
        };
        assert!(!format!("{job:?}").contains("s3cr3t"));
        let command = job.into_command(4, 0);
        assert_eq!(command.kind, CMD_DISCOVERY_SCAN);
        let back = DiscoveryScanJob::from_command(&command).unwrap();
        assert_eq!(back.cidr, "192.168.1.0/24");
        assert_eq!(back.depth, ScanDepth::Extended);
        // Un ancien agent y verrait une commande qui n'est pas une sonde.
        assert!(crate::ProbeJob::from_command(&command).is_err());
    }

    #[test]
    fn the_builder_accumulates_http_conditions() {
        let fp = Fingerprint::http("x.web", "X", &[8006], "/")
            .header("Server", "pve")
            .title("Proxmox")
            .kind("proxmox");
        let FingerprintProbe::Http { all, .. } = &fp.probe else { panic!() };
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], HttpMatch::Header { name: "server".into(), contains: "pve".into() });
        assert_eq!(fp.kind.as_deref(), Some("proxmox"));
        let json = serde_json::to_value(ScanProgress::default()).unwrap();
        assert_eq!(json["phase"], "sweep");
    }
}
