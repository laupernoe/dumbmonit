//! Résolution des noms par les serveurs DNS propres au moniteur.
//!
//! Un contrôleur de domaine s'adresse presque toujours par son nom (le
//! certificat LDAPS porte ce nom), or le DNS de la machine qui supervise ne
//! connaît pas forcément le domaine. Quand `dns_servers` est renseigné, **toutes**
//! les résolutions du moniteur passent par un résolveur dédié, qui n'interroge
//! que ces serveurs (UDP, puis TCP si la réponse est tronquée ou l'UDP muet).
//! Vide : le résolveur du système, comme avant.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use dumbmonit_proto::ProbeError;
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolveHosts, ResolverConfig};
use hickory_resolver::net::NetError;
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::{Resolver, TokioResolver};

const DEFAULT_DNS_PORT: u16 = 53;

/// Les serveurs DNS à interroger, dans l'ordre saisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsServers(Vec<SocketAddr>);

/// Pourquoi un nom n'a pas donné d'adresse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Les serveurs ont répondu : le nom n'existe pas chez eux.
    NotFound(String),
    /// Aucun serveur n'a répondu à temps.
    Timeout(String),
}

impl ResolveError {
    pub fn message(&self) -> &str {
        match self {
            Self::NotFound(text) | Self::Timeout(text) => text,
        }
    }
}

impl DnsServers {
    /// Lit la liste saisie : adresses IP séparées par des virgules, des
    /// points-virgules, des espaces ou des retours à la ligne. `10.0.0.10:5353`
    /// et `[fd00::10]:53` précisent un port. `None` si la liste est vide.
    pub fn parse(raw: Option<&str>) -> Result<Option<Self>, ProbeError> {
        let Some(raw) = raw else { return Ok(None) };
        let mut servers = Vec::new();
        for item in raw.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
            if item.is_empty() {
                continue;
            }
            let address = item
                .parse::<SocketAddr>()
                .ok()
                .or_else(|| {
                    item.parse::<IpAddr>().ok().map(|ip| SocketAddr::new(ip, DEFAULT_DNS_PORT))
                })
                .or_else(|| {
                    // IPv6 entre crochets sans port.
                    let inner = item.strip_prefix('[')?.strip_suffix(']')?;
                    Some(SocketAddr::new(inner.parse().ok()?, DEFAULT_DNS_PORT))
                })
                .ok_or_else(|| {
                    ProbeError::Config(format!(
                        "\"dns_servers\" expects IP addresses of DNS servers (for example \
                         \"10.0.0.10, 10.0.0.11\"), got \"{item}\""
                    ))
                })?;
            if !servers.contains(&address) {
                servers.push(address);
            }
        }
        Ok((!servers.is_empty()).then_some(Self(servers)))
    }

    /// Pour les messages : `10.0.0.10, 10.0.0.11`, le port seulement s'il est
    /// inhabituel.
    pub fn label(&self) -> String {
        self.0
            .iter()
            .map(|a| if a.port() == DEFAULT_DNS_PORT { a.ip().to_string() } else { a.to_string() })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn resolver(&self, timeout: Duration) -> Result<TokioResolver, ProbeError> {
        let servers = self
            .0
            .iter()
            .map(|address| {
                let mut udp = ConnectionConfig::udp();
                udp.port = address.port();
                let mut tcp = ConnectionConfig::tcp();
                tcp.port = address.port();
                NameServerConfig::new(address.ip(), true, vec![udp, tcp])
            })
            .collect();
        let mut builder = Resolver::builder_with_config(
            ResolverConfig::from_parts(None, Vec::new(), servers),
            TokioRuntimeProvider::default(),
        );
        {
            let opts = builder.options_mut();
            opts.attempts = 2;
            opts.timeout = (timeout / 2).max(Duration::from_millis(500));
            opts.cache_size = 0;
            // Le fichier hosts du conteneur ne dit rien du domaine : on veut la
            // réponse des serveurs indiqués, et seulement la leur.
            opts.use_hosts_file = ResolveHosts::Never;
        }
        builder.build().map_err(|error| ProbeError::Config(format!("unusable resolver: {error}")))
    }

    /// Les adresses de `host` d'après ces serveurs uniquement.
    pub async fn lookup(&self, host: &str, timeout: Duration) -> Result<Vec<IpAddr>, ResolveError> {
        let label = self.label();
        let not_found =
            || ResolveError::NotFound(format!("{host} did not resolve through {label}"));
        let resolver = self.resolver(timeout).map_err(|error| {
            ResolveError::NotFound(format!("{host} did not resolve through {label} ({error})"))
        })?;
        let timed_out =
            || ResolveError::Timeout(format!("{label} did not answer while resolving {host}"));
        match tokio::time::timeout(timeout, resolver.lookup_ip(host)).await {
            Err(_) | Ok(Err(NetError::Timeout)) => Err(timed_out()),
            Ok(Err(_)) => Err(not_found()),
            Ok(Ok(lookup)) => {
                let addresses: Vec<IpAddr> = lookup.iter().collect();
                if addresses.is_empty() { Err(not_found()) } else { Ok(addresses) }
            }
        }
    }
}

/// Résout `host` avec les serveurs indiqués, ou avec le système si aucun.
/// Une adresse IP littérale n'interroge personne.
pub async fn resolve(
    host: &str,
    port: u16,
    servers: Option<&DnsServers>,
    timeout: Duration,
) -> Result<Vec<SocketAddr>, ResolveError> {
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        return Ok(vec![SocketAddr::new(ip, port)]);
    }
    match servers {
        Some(servers) => Ok(servers
            .lookup(host, timeout)
            .await?
            .into_iter()
            .map(|ip| SocketAddr::new(ip, port))
            .collect()),
        None => match tokio::time::timeout(timeout, tokio::net::lookup_host((host, port))).await {
            Err(_) => Err(ResolveError::Timeout(format!("{host} did not resolve in time"))),
            Ok(Err(error)) => {
                Err(ResolveError::NotFound(format!("{host} does not resolve: {error}")))
            }
            Ok(Ok(addresses)) => {
                let addresses: Vec<_> = addresses.collect();
                if addresses.is_empty() {
                    Err(ResolveError::NotFound(format!("{host} resolves to no address")))
                } else {
                    Ok(addresses)
                }
            }
        },
    }
}

#[cfg(test)]
pub mod fake {
    //! Un faux serveur DNS local : `dc01.corp.local` pointe sur 127.0.0.1,
    //! tout autre nom est inconnu (`NXDOMAIN`).

    use std::net::SocketAddr;

    use hickory_resolver::proto::op::{Message, OpCode, ResponseCode};
    use hickory_resolver::proto::rr::rdata::A;
    use hickory_resolver::proto::rr::{RData, Record, RecordType};

    pub async fn serve() -> SocketAddr {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buffer = [0u8; 512];
            while let Ok((length, peer)) = socket.recv_from(&mut buffer).await {
                let Ok(query) = Message::from_vec(&buffer[..length]) else { continue };
                let mut reply = Message::response(query.metadata.id, OpCode::Query);
                reply.metadata.recursion_desired = query.metadata.recursion_desired;
                reply.metadata.recursion_available = true;
                for question in &query.queries {
                    reply.add_query(question.clone());
                    let known =
                        question.name().to_string().eq_ignore_ascii_case("dc01.corp.local.");
                    if !known {
                        reply.metadata.response_code = ResponseCode::NXDomain;
                    } else if question.query_type() == RecordType::A {
                        let name = question.name().clone();
                        let ip = std::net::Ipv4Addr::LOCALHOST;
                        reply.add_answer(Record::from_rdata(name, 60, RData::A(A(ip))));
                    }
                }
                let _ = socket.send_to(&reply.to_vec().unwrap(), peer).await;
            }
        });
        address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_liste_accepte_ip_port_et_separateurs_varies() {
        assert_eq!(DnsServers::parse(None).unwrap(), None);
        assert_eq!(DnsServers::parse(Some("  ")).unwrap(), None);
        let servers = DnsServers::parse(Some("10.0.0.10, 10.0.0.11;10.0.0.10\n[fd00::10]:5353"))
            .unwrap()
            .unwrap();
        assert_eq!(servers.0.len(), 3, "le doublon est écarté");
        assert_eq!(servers.label(), "10.0.0.10, 10.0.0.11, [fd00::10]:5353");
        let v6 = DnsServers::parse(Some("[fd00::1]")).unwrap().unwrap();
        assert_eq!(v6.label(), "fd00::1");
    }

    #[test]
    fn un_nom_n_est_pas_un_serveur() {
        let error = DnsServers::parse(Some("10.0.0.10, dns.corp.local")).unwrap_err();
        assert!(matches!(error, ProbeError::Config(text) if text.contains("dns.corp.local")));
    }

    #[tokio::test]
    async fn le_nom_se_resout_par_le_serveur_indique() {
        let server = fake::serve().await;
        let servers = DnsServers::parse(Some(&server.to_string())).unwrap();
        let found = resolve("dc01.corp.local", 636, servers.as_ref(), Duration::from_secs(3)).await;
        assert_eq!(found, Ok(vec!["127.0.0.1:636".parse().unwrap()]));
    }

    #[tokio::test]
    async fn un_nom_inconnu_du_serveur_donne_une_erreur_claire() {
        let server = fake::serve().await;
        let servers = DnsServers::parse(Some(&server.to_string())).unwrap().unwrap();
        let error = resolve("dc02.corp.local", 636, Some(&servers), Duration::from_secs(3))
            .await
            .unwrap_err();
        assert_eq!(
            error.message(),
            format!("dc02.corp.local did not resolve through {}", servers.label())
        );
    }

    #[tokio::test]
    async fn une_ip_litterale_n_interroge_personne() {
        let servers = DnsServers::parse(Some("127.0.0.1:9")).unwrap().unwrap();
        let found = resolve("192.0.2.7", 389, Some(&servers), Duration::from_secs(1)).await;
        assert_eq!(found, Ok(vec!["192.0.2.7:389".parse().unwrap()]));
    }
}
