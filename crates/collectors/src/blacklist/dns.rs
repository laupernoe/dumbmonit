//! Le résolveur des vérifications, derrière un trait pour pouvoir le simuler.
//!
//! Les réponses sont rendues en texte (`192.0.2.1`, `10 mx.example.com`, le
//! contenu d'un `TXT`) : c'est tout ce dont les vérifications ont besoin, et un
//! faux résolveur s'écrit alors en une table.

use std::net::SocketAddr;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::ProbeError;
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolveHosts, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::net::{DnsError, NetError};
use hickory_resolver::proto::rr::{RData, RecordType};
use hickory_resolver::{Resolver, TokioResolver};

/// Les types interrogés.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Qtype {
    A,
    Aaaa,
    Txt,
    Mx,
    Ptr,
}

/// Pourquoi une question est restée sans réponse. Une absence de réponse n'est
/// jamais un « non listé » : la liste est alors marquée inconnue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Timeout,
    Error(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => f.write_str("no answer in time"),
            Self::Error(detail) => f.write_str(detail),
        }
    }
}

#[async_trait]
pub trait Dns: Send + Sync {
    /// Les valeurs du type demandé. Vide : le nom n'existe pas (`NXDOMAIN`) ou
    /// n'a pas d'enregistrement de ce type.
    async fn query(&self, name: &str, qtype: Qtype) -> Result<Vec<String>, Failure>;
}

/// Le résolveur réel : celui du système, ou l'adresse donnée.
pub struct HickoryDns {
    resolver: TokioResolver,
}

impl HickoryDns {
    pub fn new(server: Option<SocketAddr>, timeout: Duration) -> Result<Self, ProbeError> {
        let mut builder = match server {
            Some(address) => {
                let mut udp = ConnectionConfig::udp();
                udp.port = address.port();
                let mut tcp = ConnectionConfig::tcp();
                tcp.port = address.port();
                let server = NameServerConfig::new(address.ip(), true, vec![udp, tcp]);
                Resolver::builder_with_config(
                    ResolverConfig::from_parts(None, Vec::new(), vec![server]),
                    TokioRuntimeProvider::default(),
                )
            }
            None => Resolver::builder_tokio().map_err(|error| {
                ProbeError::Config(format!(
                    "no usable system resolver ({error}): set Resolver to the address of a \
                     recursive resolver of your own"
                ))
            })?,
        };
        {
            let opts = builder.options_mut();
            // Deux essais : une liste noire répond en UDP, et une perte de paquet
            // ne doit pas suffire à rendre une liste « inconnue ».
            opts.attempts = 2;
            opts.timeout = timeout;
            opts.cache_size = 0;
            opts.use_hosts_file = ResolveHosts::Never;
            opts.num_concurrent_reqs = 1;
        }
        let resolver = builder
            .build()
            .map_err(|error| ProbeError::Config(format!("unusable resolver: {error}")))?;
        Ok(Self { resolver })
    }
}

fn record_type(qtype: Qtype) -> RecordType {
    match qtype {
        Qtype::A => RecordType::A,
        Qtype::Aaaa => RecordType::AAAA,
        Qtype::Txt => RecordType::TXT,
        Qtype::Mx => RecordType::MX,
        Qtype::Ptr => RecordType::PTR,
    }
}

fn host(name: String) -> String {
    name.trim_end_matches('.').to_ascii_lowercase()
}

#[async_trait]
impl Dns for HickoryDns {
    async fn query(&self, name: &str, qtype: Qtype) -> Result<Vec<String>, Failure> {
        let lookup = match self.resolver.lookup(name, record_type(qtype)).await {
            Ok(lookup) => lookup,
            Err(NetError::Dns(DnsError::NoRecordsFound(_))) => return Ok(Vec::new()),
            Err(NetError::Timeout) => return Err(Failure::Timeout),
            Err(error) => return Err(Failure::Error(error.to_string())),
        };
        Ok(lookup
            .answers()
            .iter()
            .filter_map(|record| match (&record.data, qtype) {
                (RData::A(a), Qtype::A) => Some(a.0.to_string()),
                (RData::AAAA(aaaa), Qtype::Aaaa) => Some(aaaa.0.to_string()),
                (RData::TXT(txt), Qtype::Txt) => {
                    Some(txt.txt_data.iter().map(|part| String::from_utf8_lossy(part)).collect())
                }
                (RData::MX(mx), Qtype::Mx) => {
                    Some(format!("{} {}", mx.preference, host(mx.exchange.to_string())))
                }
                (RData::PTR(ptr), Qtype::Ptr) => Some(host(ptr.0.to_string())),
                _ => None,
            })
            .collect())
    }
}

#[cfg(test)]
pub mod fake {
    //! Un faux résolveur : une table de réponses, `NXDOMAIN` pour le reste.

    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    type Answer = Result<Vec<String>, Failure>;

    #[derive(Default)]
    pub struct FakeDns {
        answers: Mutex<HashMap<(String, Qtype), Answer>>,
        /// Toute question dont le nom finit ainsi expire.
        pub timeout_suffix: Mutex<Option<String>>,
        pub queries: AtomicUsize,
    }

    impl FakeDns {
        pub fn answer(&self, name: &str, qtype: Qtype, values: &[&str]) -> &Self {
            self.answers.lock().unwrap().insert(
                (name.to_string(), qtype),
                Ok(values.iter().map(|v| v.to_string()).collect()),
            );
            self
        }

        pub fn fail(&self, name: &str, qtype: Qtype, failure: Failure) -> &Self {
            self.answers.lock().unwrap().insert((name.to_string(), qtype), Err(failure));
            self
        }
    }

    #[async_trait]
    impl Dns for FakeDns {
        async fn query(&self, name: &str, qtype: Qtype) -> Result<Vec<String>, Failure> {
            self.queries.fetch_add(1, Ordering::SeqCst);
            if let Some(suffix) = self.timeout_suffix.lock().unwrap().as_deref()
                && name.ends_with(suffix)
            {
                return Err(Failure::Timeout);
            }
            let answers = self.answers.lock().unwrap();
            answers.get(&(name.to_string(), qtype)).cloned().unwrap_or_else(|| Ok(Vec::new()))
        }
    }
}
