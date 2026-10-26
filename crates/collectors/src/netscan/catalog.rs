//! Catalogue des empreintes de découverte, par produit.
//!
//! Comme les notices et options des types (`api/collectors.rs` côté serveur),
//! ce catalogue est des *données* indexées par `kind` : il ne connaît aucun type
//! Rust concret, seulement les chaînes de `kind` et les signaux à reconnaître.
//! Chaque empreinte a été vérifiée contre le code source ou la documentation du
//! produit (voir les tests et les fichiers `testdata/`).
//!
//! Un collecteur peut compléter cette liste par
//! [`Collector::fingerprints`](dumbmonit_proto::Collector::fingerprints)
//! (paquets d'intégration, surtout) : le registre réunit les deux.

use dumbmonit_proto::{Confidence, Fingerprint, PrefillSpec, PrefillValue};

use super::ports::generic_fingerprints;

/// Prefill d'un produit atteint par son hôte : adresse = IP, port et TLS en
/// étiquettes (l'interface n'applique que les champs qu'elle connaît).
fn host_prefill() -> PrefillSpec {
    PrefillSpec::host()
        .tag("port", PrefillValue::Port)
        .tag("insecure_tls", PrefillValue::InsecureTls)
        .tag("scheme", PrefillValue::Scheme)
}

/// Toutes les empreintes connues : génériques, produits HTTP, bannières, ports,
/// SNMP.
pub fn all() -> Vec<Fingerprint> {
    let mut out = generic_fingerprints();
    out.extend(http_products());
    out.extend(banners());
    out.extend(ports_only());
    out.extend(snmp());
    out
}

/// Produits reconnus par une requête HTTP anonyme.
fn http_products() -> Vec<Fingerprint> {
    vec![
        // --- Proxmox ---
        Fingerprint::http("proxmox.web", "Proxmox VE", &[8006], "/")
            .header("server", "pve-api-daemon")
            .title("Proxmox Virtual Environment")
            .kind("proxmox")
            .prefill(host_prefill()),
        Fingerprint::http("pbs.web", "Proxmox Backup Server", &[8007], "/")
            .title("Proxmox Backup Server")
            .kind("pbs")
            .prefill(host_prefill()),
        Fingerprint::http("pmg.web", "Proxmox Mail Gateway", &[8006], "/")
            .header("server", "pve-api-daemon")
            .title("Proxmox Mail Gateway")
            .kind("pmg")
            .prefill(host_prefill()),
        Fingerprint::http("pdm.web", "Proxmox Datacenter Manager", &[8443], "/")
            .title("Proxmox Datacenter Manager")
            .kind("pdm")
            .prefill(host_prefill()),
        // --- NAS ---
        Fingerprint::http(
            "synology.dsm",
            "Synology DSM",
            &[5000, 5001],
            "/webapi/query.cgi?api=SYNO.API.Info&version=1&method=query&query=SYNO.API.Auth",
        )
        .status(200)
        .body("\"SYNO.API.Auth\"")
        .body("\"success\":true")
        .kind("synology")
        .prefill(host_prefill()),
        Fingerprint::http("truenas.scale", "TrueNAS", &[80, 443], "/ui/")
            .status(200)
            .body("id=\"main-page-title\"")
            .body("<ix-root>")
            .version(dumbmonit_proto::VersionRule::Header {
                name: "etag".into(),
                after: "TrueNAS-".into(),
            })
            .kind("truenas")
            .prefill(host_prefill()),
        Fingerprint::http("truenas.core", "TrueNAS CORE", &[80, 443], "/")
            .status(200)
            .body("id=\"main-page-title\"")
            .body("<app-root>")
            .confidence(Confidence::Medium)
            .kind("truenas")
            .prefill(host_prefill()),
        // QNAP : sans session, `authLogin.cgi` répond par un document XML
        // `QDocRoot` (authPassed à 0) ; c'est la page que lit l'intégration.
        Fingerprint::http("qnap.qts", "QNAP QTS", &[443, 8080], "/cgi-bin/authLogin.cgi")
            .status(200)
            .body("<QDocRoot version=\"1.0\">")
            .confidence(Confidence::Medium)
            .kind("qnap")
            .prefill(host_prefill()),
        Fingerprint::http("unraid.web", "Unraid", &[80, 443], "/login")
            .status(200)
            .body("Unraid webGUI")
            .kind("unraid")
            .prefill(host_prefill()),
        // --- Pare-feu / réseau ---
        Fingerprint::http("opnsense.web", "OPNsense", &[443, 80], "/")
            .header("server", "OPNsense")
            .kind("opnsense")
            .prefill(host_prefill()),
        Fingerprint::http("pfsense.web", "pfSense", &[443, 80], "/")
            .title("pfSense")
            .body("/js/pfSense.js")
            .kind("pfsense")
            .prefill(host_prefill()),
        Fingerprint::http("fortigate.web", "Fortinet FortiGate", &[443, 8443, 10443], "/")
            .status(200)
            .body("top.location=\"/remote/login\";")
            .kind("fortigate")
            .prefill(host_prefill()),
        Fingerprint::http(
            "sophos.fw",
            "Sophos Firewall",
            &[4444, 443],
            "/webconsole/webpages/login.jsp",
        )
        .status(200)
        .body("uiLangToHTMLLangAttributeValueMapping")
        .confidence(Confidence::Medium)
        .kind("sophos")
        .prefill(host_prefill()),
        // Le contrôleur Omada publie, sans authentification, sa version et son
        // identifiant : le même identifiant que celui de l'API ouverte.
        Fingerprint::http(
            "omada.controller",
            "TP-Link Omada Controller",
            &[8043, 443],
            "/api/info",
        )
        .status(200)
        .body("\"controllerVer\"")
        .body("\"omadacId\"")
        .version_json("/result/controllerVer")
        .kind("omada")
        .prefill(host_prefill()),
        Fingerprint::http("paloalto.login", "Palo Alto Networks PAN-OS", &[443], "/php/login.php")
            .status(200)
            .body("Palo Alto Networks - Login")
            .confidence(Confidence::Medium)
            .kind("paloalto")
            .prefill(host_prefill()),
        Fingerprint::http("sonicwall.web", "SonicWall SonicOS", &[443, 8443], "/")
            .header("server", "SonicWALL")
            .confidence(Confidence::Medium)
            .kind("sonicwall")
            .prefill(host_prefill()),
        Fingerprint::http("mikrotik.web", "MikroTik RouterOS", &[80, 443], "/")
            .status(200)
            .title("RouterOS router configuration page")
            .version(dumbmonit_proto::VersionRule::Body { after: "<h1>RouterOS v".into() })
            .kind("mikrotik")
            .prefill(host_prefill()),
        // --- Virtualisation / BMC ---
        Fingerprint::http("unifi.network", "UniFi Network Application", &[8443], "/status")
            .status(200)
            .body("\"server_version\"")
            .body("\"rc\":\"ok\"")
            .version_json("/meta/server_version")
            .kind("unifi")
            .prefill(host_prefill()),
        Fingerprint::http("unifi.os", "UniFi OS console", &[443, 11443], "/login")
            .status(200)
            .title("UniFi OS")
            .confidence(Confidence::Medium)
            .kind("unifi")
            .prefill(host_prefill()),
        Fingerprint::http("redfish.root", "Redfish BMC", &[443], "/redfish/v1/")
            .status(200)
            .body("#ServiceRoot.")
            .body("/redfish/v1")
            .version_json("/RedfishVersion")
            .kind("redfish")
            .prefill(host_prefill()),
        Fingerprint::http(
            "vsphere.sdk",
            "VMware vSphere (ESXi/vCenter)",
            &[443],
            "/sdk/vimServiceVersions.xml",
        )
        .status(200)
        .body("<namespaces")
        .body("urn:vim25")
        .kind("vsphere")
        .prefill(host_prefill()),
        Fingerprint::http("esxi.ui", "VMware ESXi", &[443], "/ui/")
            .status(200)
            .body("ng-app=\"esxUiApp\"")
            .kind("vsphere")
            .prefill(host_prefill()),
        Fingerprint::http("xcpng.xolite", "XCP-ng (XO Lite)", &[80, 443], "/")
            .status(200)
            .title("XO Lite")
            .kind("xcpng")
            .prefill(host_prefill()),
        Fingerprint::http("xo.web", "Xen Orchestra", &[80, 443], "/")
            .status(200)
            .title("Xen Orchestra")
            .prefill(host_prefill()),
        Fingerprint::http("nutanix.prism", "Nutanix Prism", &[9440], "/console/")
            .status(200)
            .title("Nutanix Web Console")
            .confidence(Confidence::Medium)
            .kind("nutanix")
            .prefill(host_prefill()),
        Fingerprint::http("veeam.em", "Veeam Backup Enterprise Manager", &[9443], "/login.aspx")
            .status(200)
            .body("Veeam Backup Enterprise Manager : Login")
            .confidence(Confidence::Medium)
            .prefill(host_prefill()),
        // --- Applications ---
        Fingerprint::http("homeassistant.web", "Home Assistant", &[8123], "/")
            .status(200)
            .title("Home Assistant")
            .kind("homeassistant")
            .prefill(host_prefill()),
        Fingerprint::http("portainer.api", "Portainer", &[9443, 9000], "/api/system/status")
            .status(200)
            .body("\"InstanceID\"")
            .body("\"Version\"")
            .version_json("/Version")
            .prefill(host_prefill()),
        Fingerprint::http(
            "docker.engine",
            "Docker Engine API (unprotected)",
            &[2375, 2376],
            "/_ping",
        )
        .status(200)
        .header("server", "Docker/")
        .version_header("server", "Docker/")
        .prefill(host_prefill()),
        Fingerprint::http("immich.api", "Immich", &[2283], "/api/server/ping")
            .status(200)
            .body("\"res\"")
            .body("pong")
            .kind("immich")
            .prefill(host_prefill()),
        Fingerprint::http("jellyfin.api", "Jellyfin", &[8096, 8920], "/System/Info/Public")
            .status(200)
            .body("Jellyfin Server")
            .body("StartupWizardCompleted")
            .version_json("/Version")
            .kind("jellyfin")
            .prefill(host_prefill()),
        Fingerprint::http("plex.identity", "Plex Media Server", &[32400], "/identity")
            .status(200)
            .body("MediaContainer")
            .body("machineIdentifier")
            .version_body("version=\"")
            .kind("plex")
            .prefill(host_prefill()),
        Fingerprint::http("nextcloud.status", "Nextcloud", &[80, 443, 8080, 8443], "/status.php")
            .status(200)
            .body("needsDbUpgrade")
            .body("\"productname\":\"Nextcloud\"")
            .version_json("/versionstring")
            .kind("nextcloud")
            .prefill(host_prefill()),
        Fingerprint::http("gitlab.signin", "GitLab", &[80, 443, 8929], "/users/sign_in")
            .status(200)
            .title("GitLab")
            .header("set-cookie", "_gitlab_session")
            .kind("gitlab")
            .prefill(host_prefill()),
        // Forgejo avant Gitea : ils partagent le port 3000.
        Fingerprint::http("forgejo.web", "Forgejo", &[3000, 80, 443], "/")
            .status(200)
            .body("git,forge,forgejo")
            .kind("forgejo")
            .prefill(host_prefill()),
        Fingerprint::http("gitea.web", "Gitea", &[3000, 80, 443], "/")
            .status(200)
            .body("go,git,self-hosted,gitea")
            .kind("forgejo")
            .prefill(host_prefill()),
        Fingerprint::http("paperless.login", "Paperless-ngx", &[8000], "/accounts/login/")
            .status(200)
            .title("Paperless-ng")
            .kind("paperless")
            .prefill(host_prefill()),
        // Pi-hole v6 avant v5.
        // GET /api/auth : 401 avec mot de passe, 200 sans ; le corps reste le même.
        Fingerprint::http("pihole.v6", "Pi-hole", &[80, 443, 8080, 8081], "/api/auth")
            .body("\"session\"")
            .body("\"validity\"")
            .body("\"totp\"")
            .kind("pihole")
            .prefill(host_prefill()),
        Fingerprint::http("pihole.v5", "Pi-hole (v5)", &[80], "/admin/api.php?version")
            .status(200)
            .body("{\"version\":3}")
            .prefill(host_prefill()),
        Fingerprint::http(
            "adguard.install",
            "AdGuard Home (setup)",
            &[3000, 80],
            "/control/install/get_addresses",
        )
        .status(200)
        .header("server", "AdGuardHome/")
        .version_header("server", "AdGuardHome/")
        .kind("adguard")
        .prefill(host_prefill()),
        // Page de connexion servie sans session : titre « Login » et couleur du
        // logo propre à AdGuard Home.
        Fingerprint::http("adguard.web", "AdGuard Home", &[80, 3000, 443, 8080], "/login.html")
            .status(200)
            .title("Login")
            .body("color=\"#67B279\"")
            .confidence(Confidence::Medium)
            .kind("adguard")
            .prefill(host_prefill()),
        Fingerprint::http("traefik.api", "Traefik", &[8080, 80, 443], "/api/version")
            .status(200)
            .body("\"Codename\"")
            .body("\"startDate\"")
            .version_json("/Version")
            .kind("traefik")
            .prefill(host_prefill()),
        Fingerprint::http("caddy.admin", "Caddy (admin API)", &[2019], "/config/")
            .status(200)
            .header("content-type", "application/json")
            .header("etag", "/config/")
            .kind("caddy")
            .prefill(host_prefill()),
        Fingerprint::http("caddy.web", "Caddy", &[80, 443, 8080], "/")
            .header("server", "Caddy")
            .confidence(Confidence::Medium)
            .prefill(host_prefill()),
        Fingerprint::http("npm.web", "Nginx Proxy Manager", &[81], "/")
            .status(200)
            .title("Nginx Proxy Manager")
            .kind("npm")
            .prefill(host_prefill()),
        Fingerprint::http("crowdsec.metrics", "CrowdSec", &[6060], "/metrics")
            .status(200)
            .body("# HELP cs_")
            .body("cs_info{version=")
            .version_body("cs_info{version=\"")
            .kind("crowdsec")
            .prefill(host_prefill()),
        Fingerprint::http("rabbitmq.mgmt", "RabbitMQ (management)", &[15672, 15671], "/")
            .status(200)
            .title("RabbitMQ Management")
            .kind("rabbitmq")
            .prefill(host_prefill()),
        Fingerprint::http("graylog.api", "Graylog", &[9000], "/api/")
            .status(200)
            .body("\"cluster_id\"")
            .body("\"node_id\"")
            .body("\"tagline\"")
            .version_json("/version")
            .kind("graylog")
            .prefill(host_prefill()),
        Fingerprint::http(
            "loki.buildinfo",
            "Grafana Loki",
            &[3100],
            "/loki/api/v1/status/buildinfo",
        )
        .status(200)
        .body("\"goVersion\"")
        .body("\"revision\"")
        .version_json("/version")
        .kind("loki")
        .prefill(host_prefill()),
        Fingerprint::http("victoriametrics.root", "VictoriaMetrics", &[8428], "/")
            .status(200)
            .body("Single-node VictoriaMetrics")
            .version_body("Version ")
            .kind("victoriametrics")
            .prefill(host_prefill()),
        Fingerprint::http("victorialogs.root", "VictoriaLogs", &[9428], "/")
            .status(200)
            .body("<h2>VictoriaLogs</h2>")
            .version_body("Version ")
            .kind("victorialogs")
            .prefill(host_prefill()),
        Fingerprint::http("grafana.health", "Grafana", &[3000], "/api/health")
            .status(200)
            .body("\"database\"")
            .body("\"commit\"")
            .version_json("/version")
            .prefill(host_prefill()),
        Fingerprint::http("prometheus.healthy", "Prometheus", &[9090], "/-/healthy")
            .status(200)
            .body("Prometheus")
            .body("is Healthy.")
            .prefill(host_prefill()),
        // --- Administration, stockage objet, DumbMonit ---
        // Cockpit : /ping répond `{ "service": "cockpit" }` (cockpithandlers.c).
        Fingerprint::http("cockpit.ping", "Cockpit", &[9090], "/ping")
            .status(200)
            .body("\"service\": \"cockpit\"")
            .prefill(host_prefill()),
        // Page de connexion de Cockpit : titre « Loading... » tant que le script
        // de connexion n'a pas tourné, plus ses ressources caractéristiques.
        Fingerprint::http("cockpit.login", "Cockpit", &[9090], "/")
            .status(200)
            .title("Loading...")
            .body("cockpit/static/login.js")
            .confidence(Confidence::Medium)
            .prefill(host_prefill()),
        // DumbMonit lui-même : /api/health est ouvert et répond toujours 200.
        // Le moniteur HTTP convient (URL de la page de santé).
        Fingerprint::http("dumbmonit.health", "DumbMonit", &[8080, 80, 443], "/api/health")
            .status(200)
            .body("\"status\":")
            .body("\"victoria\":{")
            .body("\"demo\":")
            .version_json("/version")
            .kind("http")
            .prefill(
                PrefillSpec::url("/api/health").tag("insecure_tls", PrefillValue::InsecureTls),
            ),
        // RustFS : /health publie `service: rustfs…` (health.rs) ; il imite aussi
        // les sondes MinIO mais n'envoie pas d'en-tête `Server: MinIO`.
        Fingerprint::http("rustfs.health", "RustFS", &[9000, 9001], "/health")
            .status(200)
            .body("\"service\":\"rustfs")
            .body("\"status\":")
            .version_json("/version")
            .prefill(host_prefill()),
        // MinIO : l'en-tête `Server: MinIO` accompagne toute réponse S3.
        Fingerprint::http("minio.live", "MinIO", &[9000], "/minio/health/live")
            .status(200)
            .header("server", "MinIO")
            .kind("minio")
            .prefill(host_prefill()),
        Fingerprint::http("minio.console", "MinIO Console", &[9001], "/")
            .status(200)
            .title("MinIO Console")
            .confidence(Confidence::Medium)
            .prefill(host_prefill()),
        // nginx / apache génériques : utiles comme indice, confiance moyenne.
        Fingerprint::http("nginx.server", "nginx", &[80, 443, 8080, 8443], "/")
            .header("server", "nginx")
            .version_header("server", "nginx/")
            .confidence(Confidence::Medium)
            .kind("nginx")
            .prefill(host_prefill()),
        Fingerprint::http("apache.server", "Apache HTTP Server", &[80, 443, 8080], "/")
            .header("server", "Apache/")
            .version_header("server", "Apache/")
            .confidence(Confidence::Medium)
            .kind("apache")
            .prefill(host_prefill()),
    ]
}

/// Produits qui parlent (spontanément ou après une sonde inoffensive).
fn banners() -> Vec<Fingerprint> {
    vec![
        // MariaDB avant MySQL (la version MariaDB contient le mot « MariaDB »).
        Fingerprint::banner("mysql.mariadb", "MariaDB", &[3306], &["mariadb"])
            .kind("mysql")
            .prefill(host_prefill()),
        // MySQL 8+ : greffon d'authentification par défaut, absent de MariaDB.
        Fingerprint::banner("mysql.mysql", "MySQL", &[3306], &["caching_sha2_password"])
            .kind("mysql")
            .prefill(host_prefill()),
        // NUT upsd répond à VER sans authentification.
        Fingerprint::exchange(
            "nut.upsd",
            "NUT (upsd)",
            &[3493],
            b"VER\n",
            &["Network UPS Tools upsd"],
        )
        .confidence(Confidence::High)
        .version_banner("Network UPS Tools upsd ")
        .kind("nut")
        .prefill(host_prefill()),
        // Redis / Valkey : PING -> +PONG, ou refus d'authentification.
        Fingerprint::exchange("redis.noauth", "Redis", &[6379], b"PING\r\n", &["noauth"])
            .confidence(Confidence::High)
            .kind("redis")
            .prefill(host_prefill()),
        Fingerprint::banner("redis.protected", "Redis", &[6379], &["protected mode"])
            .kind("redis")
            .prefill(host_prefill()),
        Fingerprint::exchange("redis.pong", "Redis", &[6379], b"PING\r\n", &["+pong"])
            .confidence(Confidence::Medium)
            .kind("redis")
            .prefill(host_prefill()),
        // SMTP (Postfix, Exim) : bannière spontanée.
        Fingerprint::banner("smtp.postfix", "Postfix (SMTP)", &[25, 587], &["ESMTP Postfix"])
            .kind("smtp")
            .prefill(host_prefill()),
        Fingerprint::banner("smtp.exim", "Exim (SMTP)", &[25, 587], &["ESMTP Exim"])
            .version_banner("ESMTP Exim ")
            .kind("smtp")
            .prefill(host_prefill()),
        // FTP : bannière spontanée, pas de collecteur dédié.
        Fingerprint::banner("ftp.vsftpd", "vsFTPd", &[21], &["(vsFTPd "])
            .version_banner("(vsFTPd "),
        Fingerprint::banner("ftp.proftpd", "ProFTPD", &[21], &["ProFTPD"]),
        Fingerprint::banner("ftp.pureftpd", "Pure-FTPd", &[21], &["Pure-FTPd"]),
    ]
}

/// Services reconnus par la seule ouverture d'un port (confiance basse).
fn ports_only() -> Vec<Fingerprint> {
    vec![
        Fingerprint::port("db.postgres", "PostgreSQL", &[5432])
            .confidence(Confidence::Medium)
            .kind("postgres")
            .prefill(host_prefill()),
        Fingerprint::port("db.mysql", "MySQL / MariaDB", &[3306])
            .confidence(Confidence::Medium)
            .kind("mysql")
            .prefill(host_prefill()),
        Fingerprint::port("db.redis", "Redis", &[6379]).kind("redis").prefill(host_prefill()),
        Fingerprint::port("db.mongodb", "MongoDB", &[27017])
            .confidence(Confidence::Medium)
            .kind("mongodb")
            .prefill(host_prefill()),
        Fingerprint::port("queue.mqtt", "MQTT broker", &[1883])
            .kind("mqtt")
            .prefill(host_prefill()),
        Fingerprint::port("mikrotik.api", "MikroTik API", &[8728, 8729]).kind("mikrotik"),
        Fingerprint::port("mikrotik.winbox", "MikroTik WinBox", &[8291]).kind("mikrotik"),
        Fingerprint::port("veeam.vbr", "Veeam Backup & Replication", &[9419]).kind("veeam"),
        Fingerprint::port("kopia.server", "Kopia server", &[51515]).kind("kopia"),
        Fingerprint::port("print.ipp", "Printer (IPP)", &[631])
            .kind("tcp")
            .prefill(PrefillSpec::host_port()),
    ]
}

/// Produits identifiés par SNMP (sysObjectID / sysDescr). Tous `kind = snmp`.
fn snmp() -> Vec<Fingerprint> {
    let f = |id: &str, product: &str, oid: Option<&str>, descr: Option<&str>| {
        Fingerprint::snmp(id, product, oid, descr).kind("snmp").prefill(PrefillSpec::host())
    };
    vec![
        f("snmp.routeros", "MikroTik RouterOS", Some("1.3.6.1.4.1.14988.1"), None),
        f("snmp.swos", "MikroTik SwOS", Some("1.3.6.1.4.1.14988.2"), None),
        f("snmp.fortigate", "Fortinet FortiGate", Some("1.3.6.1.4.1.12356.101"), None),
        f("snmp.qnap", "QNAP", Some("1.3.6.1.4.1.24681"), None),
        f("snmp.unifi", "Ubiquiti UniFi", Some("1.3.6.1.4.1.41112"), None),
        f("snmp.edgeos", "Ubiquiti EdgeOS", Some("1.3.6.1.4.1.41112.1.5"), None),
        f("snmp.cisco", "Cisco", Some("1.3.6.1.4.1.9.1"), Some("Cisco")),
        f("snmp.procurve", "HPE/Aruba switch", Some("1.3.6.1.4.1.11.2.3.7"), None),
        f("snmp.aruba-cx", "Aruba CX", Some("1.3.6.1.4.1.47196.4"), None),
        f("snmp.junos", "Juniper Junos", Some("1.3.6.1.4.1.2636"), None),
        f("snmp.apc", "APC UPS", Some("1.3.6.1.4.1.318.1"), None),
        f("snmp.eaton", "Eaton/MGE UPS", Some("1.3.6.1.4.1.534.1"), None),
        f("snmp.mge", "MGE UPS", Some("1.3.6.1.4.1.705.1"), None),
        f("snmp.hp-printer", "HP printer", Some("1.3.6.1.4.1.11.2.3.9"), None),
        f("snmp.brother", "Brother printer", Some("1.3.6.1.4.1.2435"), None),
        f("snmp.canon", "Canon printer", Some("1.3.6.1.4.1.1602"), None),
        f("snmp.epson", "Epson printer", Some("1.3.6.1.4.1.1248"), None),
        f("snmp.kyocera", "Kyocera printer", Some("1.3.6.1.4.1.1347"), None),
        f("snmp.ricoh", "Ricoh printer", Some("1.3.6.1.4.1.367"), None),
        f("snmp.lexmark", "Lexmark printer", Some("1.3.6.1.4.1.641"), None),
        f("snmp.xerox", "Xerox printer", Some("1.3.6.1.4.1.253"), None),
        f("snmp.opnsense-pfsense", "OPNsense / pfSense", Some("1.3.6.1.4.1.12325"), None),
        f("snmp.netgear", "Netgear switch", Some("1.3.6.1.4.1.4526"), None),
        f("snmp.tplink", "TP-Link switch", Some("1.3.6.1.4.1.11863"), None),
        f("snmp.zyxel", "Zyxel", Some("1.3.6.1.4.1.890"), None),
        f("snmp.watchguard", "WatchGuard Firebox", Some("1.3.6.1.4.1.3097"), None),
        f("snmp.stormshield", "Stormshield SNS", Some("1.3.6.1.4.1.11256"), None),
        // Produits reconnus par le sysDescr (OID Net-SNMP générique, partagé).
        f("snmp.synology", "Synology DSM", None, Some("synology")),
        f("snmp.truenas", "TrueNAS", None, Some("TrueNAS")),
        f("snmp.routeros-descr", "MikroTik RouterOS", None, Some("RouterOS")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fingerprint_has_a_unique_id() {
        let fingerprints = all();
        let mut ids: Vec<&str> = fingerprints.iter().map(|f| f.id.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "doublon d'identifiant d'empreinte");
        assert!(count > 50, "catalogue étonnamment court: {count}");
    }

    #[test]
    fn http_fingerprints_carry_conditions_except_the_generic_one() {
        for fp in all() {
            if let dumbmonit_proto::FingerprintProbe::Http { all, .. } = &fp.probe
                && !fp.generic
            {
                assert!(!all.is_empty(), "{} n'a aucune condition HTTP", fp.id);
            }
        }
    }
}
