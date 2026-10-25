//! Moniteurs de disponibilité : HTTP, TCP, DNS, ICMP, expiration de certificat,
//! SMTP, PostgreSQL, MySQL/MariaDB, MQTT, WebSocket, NTP, UDP, SSH, IMAP et POP3.
//!
//! Là où les collecteurs SNMP, Proxmox ou l'agent surveillent des *équipements*,
//! ceux-ci surveillent des *services* : une page web répond-elle, un port
//! s'ouvre-t-il, un nom se résout-il vers la bonne adresse, un certificat va-t-il
//! expirer. C'est l'équivalent d'Uptime Kuma, branché sur le planificateur, les
//! alertes et les graphiques déjà en place.
//!
//! # Un collecteur par type de sonde
//!
//! Chaque sonde est un [`Collector`](dumbmonit_proto::Collector) à part entière —
//! `http`, `tcp`, `dns`, `ping`, `tls`, `smtp`, `postgres`, `mysql`, `mqtt`,
//! `websocket`, `ntp`, `udp`, `ssh`, `imap`, `pop3` — plutôt qu'un collecteur unique
//! paramétré. Elle hérite ainsi sans rien écrire du planificateur, de l'intervalle
//! par cible, de la découverte, des alertes et de l'interface ; et l'utilisateur
//! choisit un type dans une liste au lieu de deviner une étiquette.
//!
//! # Le point de conception : `probe_success`
//!
//! Toutes les sondes émettent `probe_success` (0 ou 1) **en plus** du `up` posé par
//! le registre, et renvoient `Ok` même quand le service est en panne. Le raisonnement
//! complet est dans [`outcome`], mais il tient en deux phrases : un service qui
//! répond 500 est en panne sans être injoignable, et un taux de disponibilité se
//! calcule sur des points écrits, pas sur des trous dans la série. `Err` est réservé
//! aux erreurs de configuration de l'utilisateur.
//!
//! ```text
//! # Taux de disponibilité sur trente jours, par cible :
//! avg_over_time(dumbmonit_probe_success[30d])
//! ```
//!
//! # Métriques produites
//!
//! Communes à toutes les sondes, étiquetées
//! `probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket|ntp|udp|ssh|imap|pop3"` :
//!
//! | Métrique | Unité | Sens |
//! |---|---|---|
//! | `probe_success` | 0 / 1 | Le service a répondu correctement. |
//! | `probe_duration_seconds` | s | Durée totale de la sonde. |
//! | `probe_failure_info` | 1 | Présence ; l'étiquette `reason` porte le motif. |
//!
//! Selon la sonde :
//!
//! | Métrique | Sondes | Sens |
//! |---|---|---|
//! | `probe_http_status_code` | http, websocket | Code de statut obtenu. |
//! | `probe_http_first_byte_seconds` | http | Délai jusqu'aux en-têtes de réponse. |
//! | `probe_http_phase_seconds` | http | Phases de la requête mesurée ; étiquette `phase` : dns, connect, tls, processing, transfer. |
//! | `probe_http_content_bytes` | http | Taille du corps. |
//! | `probe_connect_seconds` | toutes sauf dns, ping, ntp, udp | Établissement de la connexion. |
//! | `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket, imap, pop3 | Négociation TLS seule. |
//! | `probe_tls_version_info` | tls, http, smtp, mqtt, websocket, imap, pop3 | Présence ; étiquette `version`. |
//! | `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket, imap, pop3 | Jours avant expiration, négatif si périmé. |
//! | `probe_http_security_header` | http (https seulement) | 0/1 par étiquette `header` : HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
//! | `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket, imap, pop3 | La chaîne remonte à une autorité connue. |
//! | `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket, imap, pop3 | Présence ; étiquette `issuer`. |
//! | `probe_dns_lookup_seconds` | dns | Temps de résolution. |
//! | `probe_dns_answer_records` | dns | Enregistrements du type demandé. |
//! | `probe_icmp_rtt_seconds` | ping | Aller-retour moyen. |
//! | `probe_icmp_rtt_min_seconds` | ping | Aller-retour le plus court. |
//! | `probe_icmp_rtt_max_seconds` | ping | Aller-retour le plus long. |
//! | `probe_icmp_packet_loss_ratio` | ping | Perte entre 0 et 1. |
//! | `probe_icmp_packets_sent` | ping | Échos partis. |
//! | `probe_icmp_packets_received` | ping | Échos revenus. |
//! | `probe_smtp_greeting_seconds` | smtp | Délai jusqu'à la bannière `220`. |
//! | `probe_smtp_ehlo_seconds` | smtp | Aller-retour de la commande `EHLO`. |
//! | `probe_smtp_capabilities` | smtp | Extensions annoncées. |
//! | `probe_smtp_authenticated` | smtp | 1 quand `AUTH` a réussi. |
//! | `probe_sql_query_seconds` | postgres, mysql | Exécution de la requête seule. |
//! | `probe_sql_rows` | postgres, mysql | Lignes renvoyées. |
//! | `probe_sql_value` | postgres, mysql | Première colonne, si elle est numérique. |
//! | `probe_mqtt_connack_seconds` | mqtt | Délai jusqu'au `CONNACK`. |
//! | `probe_mqtt_suback_seconds` | mqtt | Délai jusqu'au `SUBACK`. |
//! | `probe_mqtt_message_bytes` | mqtt | Taille du message retenu. |
//! | `probe_mqtt_message_value` | mqtt | Contenu du message, s'il est numérique. |
//! | `probe_ws_handshake_seconds` | websocket | Négociation d'ouverture. |
//! | `probe_ws_message_bytes` | websocket | Taille de la trame reçue. |
//! | `probe_ntp_offset_seconds` | ntp | Écart entre l'horloge locale et le serveur. |
//! | `probe_ntp_delay_seconds` | ntp | Délai aller-retour réseau. |
//! | `probe_ntp_stratum` | ntp | Distance à une horloge de référence (16 : non synchronisé). |
//! | `probe_ntp_leap_indicator` | ntp | 0 à 3, voir `ntp::packet::LeapIndicator`. |
//! | `probe_ntp_root_delay_seconds` | ntp | Délai cumulé jusqu'à la racine de la hiérarchie. |
//! | `probe_ntp_root_dispersion_seconds` | ntp | Dispersion cumulée jusqu'à la racine. |
//! | `probe_ntp_reference_info` | ntp | Présence ; étiquette `reference_id`. |
//! | `probe_udp_response_seconds` | udp | Délai de la réponse (si une réponse est reçue). |
//! | `probe_udp_response_bytes` | udp | Taille de la réponse. |
//! | `probe_ssh_banner_seconds` | ssh | Délai jusqu'à la bannière `SSH-2.0-…`. |
//! | `probe_ssh_protocol_info` | ssh | Présence ; étiquette `version` (protocole). |
//! | `probe_ssh_software_info` | ssh | Présence ; étiquette `software` (logiciel du serveur). |
//! | `probe_ssh_kex_seconds` | ssh | Durée du début d'échange de clés (si la clé est demandée). |
//! | `probe_ssh_host_key_fingerprint` | ssh | 32 bits de l'empreinte de la clé d'hôte (voir `alert_on_change`). |
//! | `probe_ssh_host_key_info` | ssh | Présence ; étiquette `fingerprint` (`SHA256:…`). |
//! | `probe_ssh_host_key_match` | ssh | 1 si la clé correspond à `expected_host_key`, 0 sinon. |
//! | `probe_imap_greeting_seconds` | imap | Délai jusqu'à la bannière `* OK`. |
//! | `probe_imap_login_seconds` | imap | Aller-retour de la commande `LOGIN`. |
//! | `probe_imap_authenticated` | imap | 1 quand `LOGIN` a réussi. |
//! | `probe_pop3_greeting_seconds` | pop3 | Délai jusqu'à la bannière `+OK`. |
//! | `probe_pop3_login_seconds` | pop3 | Durée de `USER` + `PASS`. |
//! | `probe_pop3_authenticated` | pop3 | 1 quand l'identification a réussi. |
//!
//! Toutes sont des jauges : rien ici n'est cumulatif, chaque interrogation mesure
//! un instant. Le préfixe `dumbmonit_` est ajouté à l'écriture, comme partout.
//!
//! Les étiquettes d'identité restent de faible cardinalité : `probe` prend quinze
//! valeurs, `reason` dix-huit, `record_type` dix, `security` trois, `sslmode`
//! quatre, et `url`, `port`, `server_name`, `resolver` en prennent une par cible.
//! S'y ajoutent `target`, `host` et les `tag_*` posés par le registre.

mod dns;
pub mod guard;
mod http;
mod imap;
mod mailbox;
mod mqtt;
mod ntp;
mod outcome;
mod ping;
mod pop3;
pub(crate) mod session;
mod smtp;
mod sql;
mod ssh;
pub(crate) mod tags;
mod tcp;
pub(crate) mod tls;
mod udp;
pub(crate) mod websocket;

pub use dns::DnsCollector;
pub use http::HttpCollector;
pub use http::security_headers::security_headers;
pub use imap::ImapCollector;
pub use mqtt::MqttCollector;
pub use ntp::NtpCollector;
pub use ping::{PingCollector, icmp_available};
pub use pop3::Pop3Collector;
pub use smtp::SmtpCollector;
pub use sql::{MysqlCollector, PostgresCollector};
pub use ssh::SshCollector;
pub use tcp::TcpCollector;
pub use tls::TlsCollector;
pub use udp::UdpCollector;
pub use websocket::WebsocketCollector;

/// Nom annoncé par défaut dans la commande `EHLO` de la sonde SMTP.
/// Réexporté pour que l'API puisse l'afficher sans dupliquer la valeur.
pub use smtp::options::DEFAULT_HELO as SMTP_DEFAULT_HELO;
/// Requête exécutée par défaut par les sondes de base de données.
pub use sql::options::DEFAULT_QUERY as SQL_DEFAULT_QUERY;

#[cfg(test)]
mod tests {
    use dumbmonit_proto::Collector;

    use super::*;

    /// Les types sont écrits en toutes lettres dans le `main.rs`, dans l'interface
    /// et dans les cibles enregistrées en base : les renommer casserait des cibles
    /// existantes sans que rien ne le signale.
    #[test]
    fn les_types_de_sonde_sont_ceux_annonces() {
        assert_eq!(HttpCollector::new().kind(), "http");
        assert_eq!(TcpCollector::new().kind(), "tcp");
        assert_eq!(DnsCollector::new().kind(), "dns");
        assert_eq!(PingCollector::new().kind(), "ping");
        assert_eq!(TlsCollector::new().kind(), "tls");
        assert_eq!(SmtpCollector::new().kind(), "smtp");
        assert_eq!(PostgresCollector::new().kind(), "postgres");
        assert_eq!(MysqlCollector::new().kind(), "mysql");
        assert_eq!(MqttCollector::new().kind(), "mqtt");
        assert_eq!(WebsocketCollector::new().kind(), "websocket");
        assert_eq!(NtpCollector::new().kind(), "ntp");
        assert_eq!(UdpCollector::new().kind(), "udp");
        assert_eq!(SshCollector::new().kind(), "ssh");
        assert_eq!(ImapCollector::new().kind(), "imap");
        assert_eq!(Pop3Collector::new().kind(), "pop3");
    }

    #[test]
    fn chaque_sonde_a_son_type_bien_a_elle() {
        let kinds = [
            HttpCollector::new().kind().to_owned(),
            TcpCollector::new().kind().to_owned(),
            DnsCollector::new().kind().to_owned(),
            PingCollector::new().kind().to_owned(),
            TlsCollector::new().kind().to_owned(),
            SmtpCollector::new().kind().to_owned(),
            PostgresCollector::new().kind().to_owned(),
            MysqlCollector::new().kind().to_owned(),
            MqttCollector::new().kind().to_owned(),
            WebsocketCollector::new().kind().to_owned(),
            NtpCollector::new().kind().to_owned(),
            UdpCollector::new().kind().to_owned(),
            SshCollector::new().kind().to_owned(),
            ImapCollector::new().kind().to_owned(),
            Pop3Collector::new().kind().to_owned(),
        ];
        let mut uniques = kinds.to_vec();
        uniques.sort_unstable();
        uniques.dedup();
        assert_eq!(uniques.len(), kinds.len(), "deux sondes se disputent le même type");
    }
}
