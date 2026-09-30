//! Connexion TCP, chiffrée ou non, des intégrations qui parlent un protocole
//! binaire plutôt que HTTP : Redis / Valkey (RESP) et MongoDB (OP_MSG).
//!
//! Contrairement aux moniteurs de disponibilité, ces intégrations visent un
//! équipement : une erreur se traduit en [`ProbeError`], et c'est l'interruption
//! de `dumbmonit_up` qui porte la panne. Le classement suit la règle commune :
//!
//! * connexion refusée, hôte injoignable, délai dépassé : `Unreachable` ou
//!   `Timeout`, c'est-à-dire « équipement en panne » ;
//! * poignée de main TLS impossible (un port en clair interrogé en TLS, ou
//!   l'inverse) : `Protocol`, erreur de configuration ;
//! * certificat refusé : `Config`, avec la marche à suivre. Le refus a lieu
//!   **avant** le premier identifiant envoyé, pour qu'un intercepteur n'emporte
//!   pas le mot de passe du compte de supervision.
//!
//! La négociation TLS reprend celle des sondes (`uptime::session::upgrade`) :
//! mêmes racines embarquées, même fournisseur cryptographique.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::uptime::session::{self, Deadline, SessionError, Stream};
use crate::uptime::tags;

/// Délai par défaut de toute l'interrogation (connexion, authentification,
/// commandes).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Réglages communs, lus sur les étiquettes de la cible.
#[derive(Debug, Clone)]
pub struct SocketOptions {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub insecure_tls: bool,
    pub timeout: Duration,
}

impl SocketOptions {
    /// `port`, `tls`, `insecure_tls` et `request_timeout_seconds`.
    pub fn from_target(target: &Target, default_port: u16) -> Result<Self, ProbeError> {
        let port = match tags::tag(target, "port") {
            None => default_port,
            Some(raw) => raw
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| ProbeError::Config(format!("Invalid port: \"{raw}\"")))?,
        };
        let address = strip_scheme(&target.address);
        let (host, port) = tags::split_host_port(address, port)?;
        Ok(Self {
            host,
            port,
            tls: tags::parse_bool(target, "tls", false)?,
            insecure_tls: tags::parse_bool(target, "insecure_tls", false)?,
            timeout: parse_timeout(tags::tag(target, "request_timeout_seconds"))?,
        })
    }
}

/// Une adresse collée depuis une chaîne de connexion (`redis://cache.lan:6379`,
/// `mongodb://db.lan`) garde son hôte et son port ; le reste est ignoré.
fn strip_scheme(address: &str) -> &str {
    let address = address.trim();
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    // Des identifiants dans l'adresse n'ont rien à faire là : ils sont écartés.
    let rest = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    rest.split(['/', '?']).next().unwrap_or(rest)
}

fn parse_timeout(value: Option<&str>) -> Result<Duration, ProbeError> {
    let Some(raw) = value else { return Ok(DEFAULT_TIMEOUT) };
    let seconds: u64 =
        raw.parse().map_err(|_| ProbeError::Config(format!("Invalid timeout: \"{raw}\"")))?;
    if !(1..=120).contains(&seconds) {
        return Err(ProbeError::Config(
            "Request timeout must be between 1 and 120 seconds".to_string(),
        ));
    }
    Ok(Duration::from_secs(seconds))
}

/// Une connexion ouverte, avec son échéance.
pub struct Connection {
    stream: Stream,
    deadline: Deadline,
    timeout: Duration,
    product: &'static str,
}

impl Connection {
    /// Ouvre la connexion et, si demandé, la chiffre.
    pub async fn open(options: &SocketOptions, product: &'static str) -> Result<Self, ProbeError> {
        let deadline = Deadline::starting_now(options.timeout);
        let endpoint = format!("{}:{}", options.host, options.port);
        let tcp = match deadline
            .wait(TcpStream::connect((options.host.as_str(), options.port)))
            .await
        {
            Err(_) => return Err(ProbeError::Timeout(options.timeout)),
            Ok(Err(error)) => {
                return Err(ProbeError::Unreachable(format!("{product} at {endpoint}: {error}")));
            }
            Ok(Ok(stream)) => stream,
        };
        let _ = tcp.set_nodelay(true);
        let stream = if options.tls {
            let secured = match session::upgrade(tcp, &options.host, deadline).await {
                Ok(secured) => secured,
                Err(SessionError::Timeout) => return Err(ProbeError::Timeout(options.timeout)),
                Err(error) => {
                    let (_, detail) = error.failure();
                    return Err(ProbeError::Protocol(format!(
                        "TLS handshake with {product} at {endpoint} failed: {detail}. Check that \
                         TLS is enabled on that port, or turn the TLS option off."
                    )));
                }
            };
            if !secured.trusted && !options.insecure_tls {
                let motif = secured.trust_error.as_deref().unwrap_or("chain cannot be verified");
                return Err(ProbeError::Config(format!(
                    "{product} at {endpoint} presented a certificate DumbMonit cannot verify \
                     ({motif}). Nothing was sent. Enable \"Accept any certificate\" if it comes \
                     from a private authority."
                )));
            }
            Stream::Tls(Box::new(secured.stream))
        } else {
            Stream::Plain(tcp)
        };
        Ok(Self { stream, deadline, timeout: options.timeout, product })
    }

    pub fn product(&self) -> &'static str {
        self.product
    }

    pub async fn write_all(&mut self, bytes: &[u8]) -> Result<(), ProbeError> {
        let timeout = self.timeout;
        let product = self.product;
        let write = async {
            self.stream.write_all(bytes).await?;
            self.stream.flush().await
        };
        match self.deadline.wait(write).await {
            Err(_) => Err(ProbeError::Timeout(timeout)),
            Ok(Err(error)) => Err(ProbeError::Unreachable(format!("{product}: {error}"))),
            Ok(Ok(())) => Ok(()),
        }
    }

    /// Lit exactement `buf.len()` octets.
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ProbeError> {
        let timeout = self.timeout;
        let product = self.product;
        match self.deadline.wait(self.stream.read_exact(buf)).await {
            Err(_) => Err(ProbeError::Timeout(timeout)),
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                Err(ProbeError::Protocol(format!(
                    "{product} closed the connection. Check the port, and whether the server \
                     expects TLS."
                )))
            }
            Ok(Err(error)) => Err(ProbeError::Unreachable(format!("{product}: {error}"))),
            Ok(Ok(_)) => Ok(()),
        }
    }

    /// Lit ce qui arrive, au plus `buf.len()` octets ; `0` à la fermeture.
    pub async fn read_some(&mut self, buf: &mut [u8]) -> Result<usize, ProbeError> {
        let timeout = self.timeout;
        let product = self.product;
        match self.deadline.wait(self.stream.read(buf)).await {
            Err(_) => Err(ProbeError::Timeout(timeout)),
            Ok(Err(error)) => Err(ProbeError::Unreachable(format!("{product}: {error}"))),
            Ok(Ok(read)) => Ok(read),
        }
    }

    pub async fn close(self) {
        self.stream.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    #[test]
    fn l_adresse_accepte_une_chaine_de_connexion() {
        let target = cible("redis", "redis://user:pw@cache.lan:6380/0", &[]);
        let options = SocketOptions::from_target(&target, 6379).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("cache.lan", 6380));
        let target = cible("mongodb", "db.lan", &[("port", "27018"), ("tls", "true")]);
        let options = SocketOptions::from_target(&target, 27017).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("db.lan", 27018));
        assert!(options.tls);
        assert_eq!(options.timeout, DEFAULT_TIMEOUT);
        let target = cible("redis", "[fd00::5]:6390", &[]);
        let options = SocketOptions::from_target(&target, 6379).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("fd00::5", 6390));
    }

    #[test]
    fn une_option_invalide_est_une_erreur_de_configuration() {
        for (key, value) in [("port", "0"), ("tls", "peut-être"), ("request_timeout_seconds", "0")]
        {
            let target = cible("redis", "cache.lan", &[(key, value)]);
            let error = SocketOptions::from_target(&target, 6379).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{key} = {value}");
        }
    }

    #[tokio::test]
    async fn un_port_ferme_est_une_panne() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let target = cible("redis", &format!("127.0.0.1:{port}"), &[]);
        let options = SocketOptions::from_target(&target, 6379).unwrap();
        let error = Connection::open(&options, "Redis").await.err().unwrap();
        assert!(error.means_down(), "{error}");
    }
}
