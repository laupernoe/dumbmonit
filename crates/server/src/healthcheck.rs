//! Sous-commande `dumbmonit healthcheck` : sonde de santé du conteneur.
//!
//! L'image finale est `FROM scratch` (ni shell ni curl), donc le `HEALTHCHECK`
//! du Dockerfile rappelle le binaire lui-même. La sonde n'ouvre ni la base ni
//! VictoriaMetrics et n'écrit aucun log : elle fait un `GET /api/health` en
//! HTTP/1.1 sur la boucle locale et répond par son code de sortie.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::config::env_var;

const TIMEOUT: Duration = Duration::from_secs(3);

/// Adresse à sonder d'après `DUMBMONIT_BIND` (défaut `0.0.0.0:8080`) : une
/// adresse « toutes interfaces » devient la boucle locale correspondante.
pub fn target_from_bind(bind: &str) -> Option<SocketAddr> {
    let mut addr: SocketAddr = bind.trim().parse().ok()?;
    if addr.ip().is_unspecified() {
        addr.set_ip(match addr.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        });
    }
    Some(addr)
}

/// `true` si la première ligne de la réponse est un `HTTP/1.x 200`.
pub fn is_ok_status_line(response: &[u8]) -> bool {
    let line = response.split(|b| *b == b'\r' || *b == b'\n').next().unwrap_or_default();
    let mut parts = line.split(|b| *b == b' ');
    matches!(
        (parts.next(), parts.next()),
        (Some(version), Some(b"200")) if version.starts_with(b"HTTP/1.")
    )
}

/// Sonde `addr` ; `Ok(())` si `/api/health` répond 200 dans le délai imparti.
pub async fn probe(addr: SocketAddr) -> Result<(), String> {
    let attempt = async {
        let mut stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
        let request = format!(
            "GET /api/health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\
             Accept: application/json\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.map_err(|e| e.to_string())?;
        // La ligne d'état tient dans les premiers octets : inutile de tout lire.
        let mut head = [0u8; 64];
        let mut filled = 0;
        while filled < head.len() {
            let n = stream.read(&mut head[filled..]).await.map_err(|e| e.to_string())?;
            filled += n;
            if n == 0 || head[..filled].contains(&b'\n') {
                break;
            }
        }
        if is_ok_status_line(&head[..filled]) { Ok(()) } else { Err("unexpected status".into()) }
    };
    tokio::time::timeout(TIMEOUT, attempt).await.map_err(|_| "timeout".to_string())?
}

/// Point d'entrée de la sous-commande ; renvoie le code de sortie du processus.
pub fn run() -> i32 {
    let bind = env_var("DUMBMONIT_BIND").unwrap_or_else(|| "0.0.0.0:8080".into());
    let Some(addr) = target_from_bind(&bind) else {
        eprintln!("healthcheck: invalid DUMBMONIT_BIND \"{bind}\"");
        return 1;
    };
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
        return 1;
    };
    match runtime.block_on(probe(addr)) {
        Ok(()) => 0,
        Err(reason) => {
            eprintln!("healthcheck: {addr}: {reason}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn bind_wildcard_becomes_loopback() {
        assert_eq!(target_from_bind("0.0.0.0:8080"), "127.0.0.1:8080".parse().ok());
        assert_eq!(target_from_bind("[::]:9000"), "[::1]:9000".parse().ok());
        assert_eq!(target_from_bind("10.0.0.5:81"), "10.0.0.5:81".parse().ok());
        assert_eq!(target_from_bind("pas une adresse"), None);
    }

    #[test]
    fn status_line_parsing() {
        assert!(is_ok_status_line(b"HTTP/1.1 200 OK\r\n"));
        assert!(is_ok_status_line(b"HTTP/1.0 200 OK\r\n"));
        assert!(!is_ok_status_line(b"HTTP/1.1 503 Service Unavailable\r\n"));
        assert!(!is_ok_status_line(b"HTTP/1.1 2000 X\r\n"));
        assert!(!is_ok_status_line(b""));
    }

    async fn serve_once(status: &'static str) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 512];
            let n = socket.read(&mut buf).await.unwrap();
            assert!(buf[..n].starts_with(b"GET /api/health HTTP/1.1"));
            let reply = format!("HTTP/1.1 {status}\r\nContent-Length: 2\r\n\r\n{{}}");
            socket.write_all(reply.as_bytes()).await.unwrap();
        });
        addr
    }

    #[tokio::test]
    async fn probe_accepts_200_and_rejects_others() {
        assert!(probe(serve_once("200 OK").await).await.is_ok());
        assert!(probe(serve_once("503 Service Unavailable").await).await.is_err());
    }

    #[tokio::test]
    async fn probe_fails_when_nothing_listens() {
        let addr = {
            let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
            l.local_addr().unwrap()
        };
        assert!(probe(addr).await.is_err());
    }
}
