//! Confrontation des empreintes aux indices récoltés sur un port.
//!
//! Le moteur ne connaît aucun produit : il applique les conditions déclarées
//! par les empreintes ([`Fingerprint`]). Une reconnaissance plus précise
//! (confiance plus haute, non générique) l'emporte sur une générique.

use dumbmonit_proto::{
    Confidence, DiscoveredService, Fingerprint, FingerprintProbe, HttpMatch, PrefillSpec,
    PrefillValue, Transport, VersionRule,
};

use super::probe::{HttpResponse, PortEvidence};
use super::snmp_probe::SnmpIdentity;

/// Teste une empreinte HTTP/bannière/port contre les indices d'un port.
/// `None` si elle ne correspond pas.
pub fn eval_tcp(
    fp: &Fingerprint,
    ip: &str,
    port: u16,
    evidence: &PortEvidence,
) -> Option<DiscoveredService> {
    match &fp.probe {
        FingerprintProbe::Http { path, all } => {
            let http = evidence.response(path)?;
            if !all.iter().all(|c| http_condition(c, http, evidence)) {
                return None;
            }
            let version = fp.version.as_ref().and_then(|rule| version_from_http(rule, http));
            let detail = http.title.clone().or_else(|| http.header("server").map(str::to_string));
            Some(service(fp, ip, port, Transport::Tcp, evidence.tls, version, detail))
        }
        FingerprintProbe::Banner { send, all } => {
            // Les empreintes à `send` sont évaluées séparément (connexion dédiée) ;
            // ici on ne traite que la bannière spontanée.
            if send.is_some() {
                return None;
            }
            let banner = evidence.banner.as_ref()?;
            let haystack = banner.to_ascii_lowercase();
            if !all.iter().all(|needle| haystack.contains(&needle.to_ascii_lowercase())) {
                return None;
            }
            let version = fp.version.as_ref().and_then(|rule| version_from_banner(rule, banner));
            Some(service(fp, ip, port, Transport::Tcp, evidence.tls, version, Some(trim(banner))))
        }
        FingerprintProbe::Port => {
            Some(service(fp, ip, port, Transport::Tcp, evidence.tls, None, None))
        }
        FingerprintProbe::Snmp { .. } => None,
    }
}

/// Teste une empreinte à bannière provoquée contre la réponse obtenue.
pub fn eval_exchange(
    fp: &Fingerprint,
    ip: &str,
    port: u16,
    response: &str,
) -> Option<DiscoveredService> {
    let FingerprintProbe::Banner { all, .. } = &fp.probe else { return None };
    let haystack = response.to_ascii_lowercase();
    if !all.iter().all(|needle| haystack.contains(&needle.to_ascii_lowercase())) {
        return None;
    }
    let version = fp.version.as_ref().and_then(|rule| version_from_banner(rule, response));
    Some(service(fp, ip, port, Transport::Tcp, false, version, Some(trim(response))))
}

/// Teste une empreinte SNMP contre l'identité relevée sur UDP 161.
pub fn eval_snmp(fp: &Fingerprint, ip: &str, identity: &SnmpIdentity) -> Option<DiscoveredService> {
    let FingerprintProbe::Snmp { sysobjectid_prefix, sysdescr_contains } = &fp.probe else {
        return None;
    };
    if let Some(prefix) = sysobjectid_prefix {
        let got = identity.sysobjectid.as_deref().unwrap_or_default();
        if !got.starts_with(prefix.as_str()) {
            return None;
        }
    }
    if let Some(needle) = sysdescr_contains {
        let got = identity.sysdescr.as_deref().unwrap_or_default().to_ascii_lowercase();
        if !got.contains(&needle.to_ascii_lowercase()) {
            return None;
        }
    }
    let detail = identity.sysdescr.clone().map(|d| trim(&d));
    let mut found = service(fp, ip, 161, Transport::Udp, false, None, detail);
    // Le profil de collecte que l'auto-détection choisirait part dans le prefill.
    if let (Some(profile), Some(prefill)) =
        (&identity.profile_id, found.prefill.as_mut().and_then(|v| v.as_object_mut()))
    {
        prefill.insert("profile_id".into(), serde_json::Value::String(profile.clone()));
    }
    Some(found)
}

fn http_condition(condition: &HttpMatch, http: &HttpResponse, evidence: &PortEvidence) -> bool {
    match condition {
        HttpMatch::Status { code } => http.status == *code,
        HttpMatch::Header { name, contains } => http.header(name).is_some_and(|value| {
            value.to_ascii_lowercase().contains(&contains.to_ascii_lowercase())
        }),
        HttpMatch::Body { contains } => {
            http.body.to_ascii_lowercase().contains(&contains.to_ascii_lowercase())
        }
        HttpMatch::Title { contains } => http
            .title
            .as_deref()
            .is_some_and(|t| t.to_ascii_lowercase().contains(&contains.to_ascii_lowercase())),
        HttpMatch::Cert { contains } => evidence
            .cert
            .as_deref()
            .is_some_and(|c| c.to_ascii_lowercase().contains(&contains.to_ascii_lowercase())),
    }
}

fn version_from_http(rule: &VersionRule, http: &HttpResponse) -> Option<String> {
    match rule {
        VersionRule::Header { name, after } => cut(http.header(name)?, after),
        VersionRule::Body { after } => cut(&http.body, after),
        VersionRule::Json { pointer } => serde_json::from_str::<serde_json::Value>(&http.body)
            .ok()?
            .pointer(pointer)
            .and_then(|v| v.as_str().map(str::to_string).or_else(|| Some(v.to_string()))),
        VersionRule::Banner { .. } => None,
    }
}

fn version_from_banner(rule: &VersionRule, banner: &str) -> Option<String> {
    match rule {
        VersionRule::Banner { after } => cut(banner, after),
        _ => None,
    }
}

/// Texte qui suit `after`, jusqu'au premier séparateur courant.
fn cut(haystack: &str, after: &str) -> Option<String> {
    let start = if after.is_empty() {
        0
    } else {
        haystack.to_ascii_lowercase().find(&after.to_ascii_lowercase())? + after.len()
    };
    let rest = &haystack[start..];
    let end = rest.find([' ', ';', ')', ',', '"', '<', '\r', '\n', '\t']).unwrap_or(rest.len());
    let value = rest[..end].trim().trim_matches('/').to_string();
    (!value.is_empty()).then_some(value)
}

fn trim(text: &str) -> String {
    let clean: String = text.chars().take(200).collect();
    clean.trim().to_string()
}

fn service(
    fp: &Fingerprint,
    ip: &str,
    port: u16,
    protocol: Transport,
    tls: bool,
    version: Option<String>,
    detail: Option<String>,
) -> DiscoveredService {
    DiscoveredService {
        port,
        protocol,
        product: fp.product.clone(),
        kind: fp.kind.clone(),
        version,
        confidence: fp.confidence,
        tls,
        detail,
        monitored_target_id: None,
        prefill: fp.prefill.as_ref().map(|spec| build_prefill(spec, ip, port, tls)),
    }
}

/// Construit l'objet `prefill` du formulaire de cible : `address` et `tags`,
/// jamais de secret.
fn build_prefill(spec: &PrefillSpec, ip: &str, port: u16, tls: bool) -> serde_json::Value {
    use dumbmonit_proto::AddressForm;
    let address = match &spec.address {
        AddressForm::Host => ip.to_string(),
        AddressForm::HostPort => format!("{ip}:{port}"),
        AddressForm::Url { path } => {
            let scheme = if tls { "https" } else { "http" };
            format!("{scheme}://{ip}:{port}{path}")
        }
    };
    let mut tags = serde_json::Map::new();
    for (key, value) in &spec.tags {
        let v = match value {
            PrefillValue::Port => port.to_string(),
            PrefillValue::Scheme => if tls { "https" } else { "http" }.to_string(),
            PrefillValue::InsecureTls => tls.to_string(),
            PrefillValue::Text { text } => text.clone(),
        };
        tags.insert(key.clone(), serde_json::Value::String(v));
    }
    let mut out = serde_json::Map::new();
    out.insert("address".into(), serde_json::Value::String(address));
    if !tags.is_empty() {
        out.insert("tags".into(), serde_json::Value::Object(tags));
    }
    serde_json::Value::Object(out)
}

/// Retient la meilleure reconnaissance par port : la plus précise d'abord
/// (empreinte non générique), puis la confiance la plus haute. Les services non
/// HTTP (une base derrière un HTTP sur le même port, rare) ne se chassent pas.
pub fn keep_best(mut services: Vec<DiscoveredService>) -> Vec<DiscoveredService> {
    // Un seul service par (port, protocole) : on garde le mieux classé.
    services.sort_by(|a, b| {
        (a.port, a.protocol as u8).cmp(&(b.port, b.protocol as u8)).then(rank(b).cmp(&rank(a)))
    });
    services.dedup_by(|a, b| a.port == b.port && a.protocol as u8 == b.protocol as u8);
    services
}

/// Plus c'est grand, mieux c'est : une empreinte spécifique bat une générique,
/// une haute confiance bat une basse.
fn rank(service: &DiscoveredService) -> u8 {
    let confidence = match service.confidence {
        Confidence::High => 3,
        Confidence::Medium => 2,
        Confidence::Low => 1,
    };
    // Un service rattaché à un kind connu est préféré à un service sans kind.
    let has_kind = service.kind.is_some() as u8;
    confidence * 2 + has_kind
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http_evidence(server: &str, title: &str) -> PortEvidence {
        let mut http = std::collections::HashMap::new();
        http.insert(
            "/".to_string(),
            HttpResponse {
                status: 200,
                headers: vec![("server".into(), server.into())],
                title: Some(title.into()),
                body: String::new(),
            },
        );
        PortEvidence { tls: true, cert: None, banner: None, http }
    }

    #[test]
    fn an_http_fingerprint_matches_and_extracts_version_and_prefill() {
        let fp = Fingerprint::http("proxmox.web", "Proxmox VE", &[8006], "/")
            .header("server", "pve-api-daemon")
            .title("Proxmox Virtual Environment")
            .version_header("server", "pve-api-daemon/")
            .kind("proxmox")
            .prefill(PrefillSpec::host().tag("port", PrefillValue::Port));
        let evidence = http_evidence("pve-api-daemon/3.0", "Proxmox Virtual Environment");
        let service = eval_tcp(&fp, "192.168.1.20", 8006, &evidence).expect("reconnu");
        assert_eq!(service.kind.as_deref(), Some("proxmox"));
        assert_eq!(service.version.as_deref(), Some("3.0"));
        assert!(service.tls);
        let prefill = service.prefill.unwrap();
        assert_eq!(prefill["address"], "192.168.1.20");
        assert_eq!(prefill["tags"]["port"], "8006");
    }

    #[test]
    fn a_wrong_header_does_not_match() {
        let fp = Fingerprint::http("x", "X", &[8006], "/").header("server", "pve-api-daemon");
        let evidence = http_evidence("nginx", "Welcome");
        assert!(eval_tcp(&fp, "10.0.0.1", 8006, &evidence).is_none());
    }

    #[test]
    fn a_specific_match_beats_a_generic_one_on_the_same_port() {
        let specific = DiscoveredService {
            port: 443,
            protocol: Transport::Tcp,
            product: "UniFi".into(),
            kind: Some("unifi".into()),
            version: None,
            confidence: Confidence::High,
            tls: true,
            detail: None,
            monitored_target_id: None,
            prefill: None,
        };
        let generic = DiscoveredService {
            port: 443,
            product: "HTTP service".into(),
            kind: Some("http".into()),
            confidence: Confidence::Low,
            ..specific.clone()
        };
        let kept = keep_best(vec![generic, specific]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].product, "UniFi");
    }

    #[test]
    fn cut_reads_a_version_after_a_prefix() {
        assert_eq!(cut("pve-api-daemon/8.2.2", "pve-api-daemon/").as_deref(), Some("8.2.2"));
        assert_eq!(
            cut("SSH-2.0-OpenSSH_9.6p1 Debian", "SSH-2.0-").as_deref(),
            Some("OpenSSH_9.6p1")
        );
        assert_eq!(cut("nginx", "apache/"), None);
    }
}
