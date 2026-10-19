//! Les ports balayés, et les empreintes génériques qu'aucun collecteur ne porte.

use dumbmonit_proto::{Fingerprint, PrefillSpec, PrefillValue, ScanDepth};

/// Ports TCP d'un balayage `standard` : ce qu'un homelab ou une PME expose le
/// plus souvent. Une trentaine, choisis pour couvrir les produits reconnus par
/// les empreintes et les services courants (administration, bases, files,
/// imprimantes, partage, accès distant).
pub const STANDARD_TCP: &[u16] = &[
    22,    // SSH
    53,    // DNS/TCP
    80,    // HTTP
    443,   // HTTPS
    445,   // SMB
    631,   // IPP (CUPS, imprimantes)
    1883,  // MQTT
    2283,  // Immich
    2375,  // Docker API (clair)
    2376,  // Docker API (TLS)
    3000,  // Grafana / Forgejo / divers
    3306,  // MySQL/MariaDB
    3389,  // RDP
    5000,  // Synology DSM (HTTP)
    5001,  // Synology DSM (HTTPS)
    5432,  // PostgreSQL
    6379,  // Redis
    8006,  // Proxmox VE / Mail Gateway
    8007,  // Proxmox Backup Server
    8080,  // HTTP alternatif / Traefik
    8123,  // Home Assistant
    8291,  // MikroTik Winbox
    8443,  // HTTPS alternatif / PDM / UniFi
    8728,  // MikroTik API
    9000,  // Portainer / Graylog / MinIO / RustFS
    9001,  // Console MinIO / RustFS
    9090,  // Prometheus / Cockpit
    9100,  // JetDirect (imprimantes)
    9443,  // Portainer (HTTPS)
    27017, // MongoDB
];

/// Ports ajoutés par un balayage `extended` : moins fréquents, mais utiles.
pub const EXTENDED_EXTRA_TCP: &[u16] = &[
    21,    // FTP
    25,    // SMTP
    81,    // Nginx Proxy Manager
    88,    // Kerberos
    123,   // NTP (rarement en TCP)
    139,   // NetBIOS
    389,   // LDAP
    515,   // LPD (imprimantes)
    636,   // LDAPS
    1433,  // MSSQL
    2019,  // Caddy admin
    2049,  // NFS
    3100,  // Loki
    3493,  // NUT upsd
    4444,  // Sophos Firewall
    5060,  // SIP
    5201,  // iperf
    5900,  // VNC
    6060,  // CrowdSec
    7878,  // Radarr
    8000,  // Paperless / divers
    8086,  // InfluxDB
    8096,  // Jellyfin
    8428,  // VictoriaMetrics
    8729,  // MikroTik API-SSL
    8989,  // Sonarr
    9000,  // (déjà en standard, ignoré par la déduplication)
    9200,  // Elasticsearch
    9419,  // Veeam REST
    9428,  // VictoriaLogs
    9440,  // Nutanix Prism
    15672, // RabbitMQ management
    32400, // Plex
];

/// Les ports à balayer pour une profondeur donnée, en incluant ceux qu'une
/// empreinte réclame (`extended` seulement), sans doublon et triés.
pub fn tcp_ports(depth: ScanDepth, fingerprints: &[Fingerprint]) -> Vec<u16> {
    let mut set: std::collections::BTreeSet<u16> = STANDARD_TCP.iter().copied().collect();
    if depth == ScanDepth::Extended {
        set.extend(EXTENDED_EXTRA_TCP.iter().copied());
        for fp in fingerprints {
            set.extend(fp.ports.iter().copied());
        }
    }
    set.into_iter().collect()
}

/// Empreintes génériques, celles qu'aucun collecteur concret ne porte : un
/// service reconnu « de loin » (SSH, HTTP, SNMP, un port ouvert typique) quand
/// rien de plus précis n'a répondu. Toutes `generic`.
pub fn generic_fingerprints() -> Vec<Fingerprint> {
    vec![
        // SSH : la bannière d'ouverture suit la RFC 4253 (« SSH-2.0-… »).
        Fingerprint::banner("ssh", "SSH", &[22], &["SSH-"])
            .kind("tcp")
            .version_banner("SSH-2.0-")
            .prefill(PrefillSpec::host_port())
            .generic(),
        // Imprimante JetDirect : on n'envoie jamais rien sur 9100.
        Fingerprint::port("printer.raw", "Printer (JetDirect)", &[9100])
            .kind("tcp")
            .prefill(PrefillSpec { address: dumbmonit_proto::AddressForm::HostPort, tags: vec![] })
            .generic(),
        Fingerprint::port("smb", "SMB / file sharing", &[445]).kind("tcp").generic(),
        Fingerprint::port("rdp", "Remote Desktop (RDP)", &[3389]).kind("tcp").generic(),
        // SNMP générique : un agent qui a répondu, sans profil connu.
        Fingerprint::snmp("snmp", "SNMP agent", None, None)
            .kind("snmp")
            .prefill(PrefillSpec::host())
            .generic(),
        // HTTP générique : tout serveur web non reconnu. Port vide = toute
        // empreinte HTTP ; `status` large pour couvrir 200/401/403/302.
        Fingerprint::http("http", "HTTP service", &[], "/")
            .kind("http")
            .confidence(dumbmonit_proto::Confidence::Low)
            .prefill(PrefillSpec::url("").tag("insecure_tls", PrefillValue::InsecureTls))
            .generic(),
    ]
}

/// Ports sur lesquels on n'envoie jamais de requête HTTP : soit un autre
/// protocole (bases, SSH, MQTT), soit un matériel à ne surtout pas solliciter
/// (JetDirect 9100, qui imprimerait n'importe quel octet reçu).
pub const NO_HTTP: &[u16] = &[
    22, 25, 53, 445, 1883, 2049, 3306, 3389, 5432, 6379, 9100, 27017, 8291, 8728, 8729, 1433, 123,
    139, 389, 636, 515, 5900, 3493,
];

/// Vrai si l'on peut raisonnablement tenter une requête HTTP sur ce port.
pub fn http_allowed(port: u16) -> bool {
    !NO_HTTP.contains(&port)
}
