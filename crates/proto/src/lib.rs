//! Types partagés entre le serveur, les collecteurs et l'agent.
//!
//! Ce crate ne dépend d'aucun runtime ni d'aucune base de données : il définit
//! uniquement le contrat de données. C'est ce qui garantit qu'un collecteur SNMP,
//! l'agent distant et le serveur parlent tous exactement le même langage.

mod collector;
mod command;
mod credential;
mod description;
mod discovery;
pub mod env;
mod push;
mod sample;
mod target;
mod tls;

pub use collector::{Collector, ProbeError};
pub use command::{
    AgentCommand, CMD_CONTAINER_RESTART, CMD_CONTAINER_UPDATE, CMD_PROBE, COMMAND_MAX_AGE_SECS,
    COMMAND_RESULT_MAX_BYTES, COMMAND_RESULT_TRUNCATED, COMMANDS_PATH, CommandReport,
    CommandStatus, ProbeJob, ProbeOutcome, RELAY_PATH, RELAY_POLL_HOLD_SECS, truncate_result,
};
pub use credential::{
    Credential, CredentialInput, Resolved, SnmpV3Auth, SnmpV3AuthInput, SnmpV3AuthProtocol,
    SnmpV3Privacy, SnmpV3PrivacyInput, SnmpV3PrivacyProtocol,
};
pub use description::{
    CredentialDescription, CredentialFieldDescription, KindDescription, OptionColumn,
    OptionDescription, OptionItem, PortDescription, Requirement, SetupDescription, Text,
};
pub use discovery::{
    AddressForm, CMD_DISCOVERY_SCAN, Confidence, DISCOVERY_REPORT_PATH, DiscoveredHost,
    DiscoveredService, DiscoveryReport, DiscoveryScanJob, Fingerprint, FingerprintProbe, HttpMatch,
    MAX_DISCOVERY_HOSTS, PrefillSpec, PrefillValue, ScanDepth, ScanPhase, ScanProgress, Transport,
    VersionRule,
};
pub use push::{
    AGENT_KEY_HEADER, AGENT_SECRET_HEADER, AGENT_SECRET_PREFIX, AgentIdentity, BINDING_MISMATCH,
    DeclaredMonitor, INGEST_PATH, InstalledPackage, MAX_BATCH_SAMPLES, MAX_DECLARED_MONITORS,
    MAX_INVENTORY_PACKAGES,
    PUSH_PROTOCOL_VERSION, PushAck, PushBatch, RelaySetting, SoftwareInventory, TOKEN_PREFIX,
};
pub use sample::{MetricKind, Sample};
pub use target::{
    EXPORTED_LABEL_PREFIX, MAX_TAG_LABEL_LEN, TAG_LABEL_PREFIX, Target, TargetId, apply_identity,
    looks_secret,
};
pub use tls::{
    AGENT_TLS_HEADER, AGENT_TLS_SERVER_NAME, CERT_REFUSED, CLIENT_CERT_DAYS,
    DEFAULT_AGENT_TLS_PORT, MTLS_REQUIRED, RENEW_BEFORE_DAYS, TLS_ENROL_PATH, TLS_RENEW_PATH,
    TlsCertificate, TlsEnrolRequest, TlsOffer, TlsRenewRequest, agent_channel_url, is_pin,
    normalise_pin,
};
