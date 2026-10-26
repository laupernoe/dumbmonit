//! Vérifie le catalogue d'empreintes contre des réponses réelles (ou fidèlement
//! reconstruites) rangées dans `testdata/`. Chaque fichier porte en commentaire
//! la source exacte (code upstream, doc, capture).
//!
//! Le but : qu'une modification d'empreinte qui casserait une reconnaissance
//! soit prise en faute ici, sans lab ni réseau.

use std::collections::HashMap;

use dumbmonit_proto::FingerprintProbe;

use super::catalog;
use super::match_engine;
use super::probe::{self, HttpResponse, PortEvidence};

/// Charge un fichier `testdata/`.
fn load(name: &str) -> String {
    let path = format!("{}/src/netscan/testdata/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("lecture de {path}: {e}"))
}

/// Réponse HTTP d'un fichier : on saute les commentaires `#` de tête jusqu'à la
/// ligne `HTTP/…`.
fn http_of(name: &str) -> HttpResponse {
    let raw = load(name);
    let start = raw.find("HTTP/").unwrap_or_else(|| panic!("{name}: pas de ligne de statut HTTP"));
    probe::parse_http(&raw.as_bytes()[start..]).unwrap_or_else(|| panic!("{name}: HTTP illisible"))
}

/// Texte de bannière d'un fichier : la ligne `# hex:` décodée si elle existe,
/// sinon la première ligne non commentée (les `\r\n` littéraux sont retirés).
fn banner_of(name: &str) -> String {
    let raw = load(name);
    if let Some(line) = raw.lines().find(|l| l.trim_start().starts_with("# hex:")) {
        let hex: String = line.split("hex:").nth(1).unwrap().split_whitespace().collect();
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
            .collect();
        return String::from_utf8_lossy(&bytes).replace('\u{fffd}', ".");
    }
    raw.lines()
        .find(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .unwrap_or("")
        .replace("\\r\\n", "")
        .replace("\\n", "")
}

fn fingerprint(id: &str) -> dumbmonit_proto::Fingerprint {
    catalog::all()
        .into_iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("empreinte \"{id}\" absente du catalogue"))
}

/// Confronte un échantillon HTTP à l'empreinte attendue.
fn http_matches(file: &str, id: &str, tls: bool) -> bool {
    let fp = fingerprint(id);
    let FingerprintProbe::Http { path, .. } = &fp.probe else {
        panic!("{id} n'est pas une empreinte HTTP");
    };
    let mut http = HashMap::new();
    http.insert(path.clone(), http_of(file));
    // Et aussi sous `/`, au cas où le chemin déclaré serait `/`.
    let evidence = PortEvidence { tls, cert: None, banner: None, http };
    let port = fp.ports.first().copied().unwrap_or(0);
    match_engine::eval_tcp(&fp, "192.168.1.9", port, &evidence).is_some()
}

/// Confronte un échantillon de bannière (spontanée ou provoquée) à l'empreinte.
fn assert_banner(file: &str, id: &str) {
    let fp = fingerprint(id);
    let banner = banner_of(file);
    let port = fp.ports.first().copied().unwrap_or(0);
    let service = match &fp.probe {
        FingerprintProbe::Banner { send: Some(_), .. } => {
            match_engine::eval_exchange(&fp, "10.0.0.1", port, &banner)
        }
        FingerprintProbe::Banner { send: None, .. } => {
            let evidence =
                PortEvidence { tls: false, cert: None, banner: Some(banner), ..Default::default() };
            match_engine::eval_tcp(&fp, "10.0.0.1", port, &evidence)
        }
        _ => panic!("{id} n'est pas une empreinte de bannière"),
    };
    assert!(service.is_some(), "{id} aurait dû reconnaître {file}");
}

#[test]
fn http_samples_are_recognised() {
    // (fichier, id d'empreinte, tls)
    let cases = [
        ("proxmox.web.txt", "proxmox.web", true),
        ("pbs.web.txt", "pbs.web", true),
        ("pmg.web.txt", "pmg.web", true),
        ("pdm.web.txt", "pdm.web", true),
        ("synology.dsm.txt", "synology.dsm", false),
        ("truenas.scale.txt", "truenas.scale", true),
        ("opnsense.web.txt", "opnsense.web", true),
        ("pfsense.web.txt", "pfsense.web", true),
        ("unraid.web.txt", "unraid.web", false),
        ("fortigate.web.txt", "fortigate.web", true),
        ("sophos.fw.txt", "sophos.fw", true),
        ("omada.controller.txt", "omada.controller", true),
        ("mikrotik.web.txt", "mikrotik.web", false),
        ("unifi.network.txt", "unifi.network", true),
        ("unifi.os.txt", "unifi.os", true),
        ("redfish.root.txt", "redfish.root", true),
        ("vsphere.sdk.txt", "vsphere.sdk", true),
        ("esxi.ui.txt", "esxi.ui", true),
        ("xcpng.xolite.txt", "xcpng.xolite", true),
        ("xo.web.txt", "xo.web", false),
        ("nutanix.prism.txt", "nutanix.prism", true),
        ("veeam.em.txt", "veeam.em", true),
        ("homeassistant.web.txt", "homeassistant.web", false),
        ("portainer.api.txt", "portainer.api", true),
        ("docker.engine.txt", "docker.engine", false),
        ("immich.api.txt", "immich.api", false),
        ("jellyfin.api.txt", "jellyfin.api", false),
        ("plex.identity.txt", "plex.identity", false),
        ("nextcloud.status.txt", "nextcloud.status", true),
        ("gitlab.signin.txt", "gitlab.signin", true),
        ("forgejo.web.txt", "forgejo.web", false),
        ("gitea.web.txt", "gitea.web", false),
        ("paperless.login.txt", "paperless.login", false),
        ("pihole.v6.txt", "pihole.v6", false),
        ("pihole.v5.txt", "pihole.v5", false),
        ("adguard.install.txt", "adguard.install", false),
        ("adguard.web.txt", "adguard.web", false),
        ("traefik.api.txt", "traefik.api", false),
        ("caddy.admin.txt", "caddy.admin", false),
        ("npm.web.txt", "npm.web", false),
        ("crowdsec.metrics.txt", "crowdsec.metrics", false),
        ("rabbitmq.mgmt.txt", "rabbitmq.mgmt", false),
        ("graylog.api.txt", "graylog.api", false),
        ("loki.buildinfo.txt", "loki.buildinfo", false),
        ("victoriametrics.root.txt", "victoriametrics.root", false),
        ("victorialogs.root.txt", "victorialogs.root", false),
        ("grafana.health.txt", "grafana.health", false),
        ("prometheus.healthy.txt", "prometheus.healthy", false),
        ("nginx.server.txt", "nginx.server", false),
        ("apache.server.txt", "apache.server", false),
        ("cockpit.ping.txt", "cockpit.ping", true),
        ("cockpit.login.txt", "cockpit.login", true),
        ("dumbmonit.health.txt", "dumbmonit.health", false),
        ("rustfs.health.txt", "rustfs.health", false),
        ("minio.live.txt", "minio.live", false),
        ("minio.console.txt", "minio.console", false),
    ];
    let missed: Vec<&str> =
        cases.iter().filter(|(file, id, tls)| !http_matches(file, id, *tls)).map(|c| c.1).collect();
    assert!(missed.is_empty(), "empreintes qui ne reconnaissent pas leur échantillon : {missed:?}");
}

#[test]
fn banner_samples_are_recognised() {
    assert_banner("ssh.openssh.txt", "ssh");
    assert_banner("mysql.mariadb.txt", "mysql.mariadb");
    assert_banner("mysql.mysql.txt", "mysql.mysql");
    assert_banner("redis.noauth.txt", "redis.noauth");
    assert_banner("redis.pong.txt", "redis.pong");
    assert_banner("nut.upsd-ver.txt", "nut.upsd");
    assert_banner("smtp.postfix.txt", "smtp.postfix");
    assert_banner("smtp.exim.txt", "smtp.exim");
}

#[test]
fn version_is_extracted_from_a_real_response() {
    // TrueNAS : la version est dans l'ETag.
    let fp = fingerprint("truenas.scale");
    let mut http = HashMap::new();
    let FingerprintProbe::Http { path, .. } = &fp.probe else { unreachable!() };
    http.insert(path.clone(), http_of("truenas.scale.txt"));
    let evidence = PortEvidence { tls: true, cert: None, banner: None, http };
    let service = match_engine::eval_tcp(&fp, "10.0.0.1", 443, &evidence).unwrap();
    assert!(service.version.is_some(), "version TrueNAS attendue");
}

#[test]
fn object_stores_are_told_apart() {
    // Le RustFS ne passe pas pour MinIO, ni l'inverse.
    let probe = |file: &str, id: &str| {
        let fp = fingerprint(id);
        let FingerprintProbe::Http { path, .. } = &fp.probe else { unreachable!() };
        let mut http = HashMap::new();
        http.insert(path.clone(), http_of(file));
        let evidence = PortEvidence { tls: false, cert: None, banner: None, http };
        match_engine::eval_tcp(&fp, "10.0.0.1", 9000, &evidence).is_some()
    };
    assert!(!probe("rustfs.health.txt", "minio.live"));
    assert!(!probe("minio.live.txt", "rustfs.health"));
    assert!(!probe("minio.live.txt", "dumbmonit.health"));
    assert!(!probe("rustfs.health.txt", "dumbmonit.health"));
}

#[test]
fn dumbmonit_version_and_kind_are_extracted() {
    let fp = fingerprint("dumbmonit.health");
    let FingerprintProbe::Http { path, .. } = &fp.probe else { unreachable!() };
    let mut http = HashMap::new();
    http.insert(path.clone(), http_of("dumbmonit.health.txt"));
    let evidence = PortEvidence { tls: false, cert: None, banner: None, http };
    let service = match_engine::eval_tcp(&fp, "10.0.0.1", 8080, &evidence).unwrap();
    assert_eq!(service.version.as_deref(), Some("0.12.0"));
    assert_eq!(service.kind.as_deref(), Some("http"));
}

#[test]
fn a_generic_http_response_is_only_generic() {
    // Un serveur nginx nu : reconnu « HTTP service » (générique) ou « nginx »,
    // jamais un produit précis sans ses marqueurs.
    let generic = fingerprint("http");
    let FingerprintProbe::Http { path, .. } = &generic.probe else { unreachable!() };
    let mut http = HashMap::new();
    http.insert(path.clone(), http_of("http.generic.txt"));
    let evidence = PortEvidence { tls: false, cert: None, banner: None, http };
    assert!(match_engine::eval_tcp(&generic, "10.0.0.1", 80, &evidence).is_some());
    // L'empreinte Proxmox ne doit pas s'y tromper.
    let proxmox = fingerprint("proxmox.web");
    assert!(match_engine::eval_tcp(&proxmox, "10.0.0.1", 8006, &evidence).is_none());
}
