//! Faux serveurs pour les tests : un démon Docker sur socket Unix et un serveur
//! HTTP sur TCP. Ils répondent selon un scénario donné par le test et gardent la
//! trace de chaque requête reçue, dans l'ordre.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Une requête reçue par un faux serveur.
#[derive(Debug, Clone)]
pub struct Req {
    pub method: String,
    /// Chemin et paramètres, tels que le client les a écrits.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Req {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

/// Réponse du scénario : statut et corps.
pub type Resp = (u16, String);

type Handler = Arc<dyn Fn(&Req) -> Resp + Send + Sync>;

/// Historique partagé des requêtes reçues.
#[derive(Clone, Default)]
pub struct Calls(Arc<Mutex<Vec<Req>>>);

impl Calls {
    pub fn all(&self) -> Vec<Req> {
        self.0.lock().unwrap().clone()
    }

    /// `METHODE chemin` de chaque requête, dans l'ordre d'arrivée.
    pub fn lines(&self) -> Vec<String> {
        self.all().iter().map(|r| format!("{} {}", r.method, r.path)).collect()
    }

    pub fn count(&self, method: &str, path_prefix: &str) -> usize {
        self.all().iter().filter(|r| r.method == method && r.path.starts_with(path_prefix)).count()
    }

    pub fn bodies(&self, method: &str, path_prefix: &str) -> Vec<String> {
        self.all()
            .into_iter()
            .filter(|r| r.method == method && r.path.starts_with(path_prefix))
            .map(|r| r.body)
            .collect()
    }
}

async fn read_request<S: AsyncRead + Unpin>(stream: &mut S) -> Option<Req> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    let split = loop {
        if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&data[..split]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_string(), v.trim().to_string()))
        .collect();
    let length = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = data[split + 4..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    Some(Req { method, path, headers, body: String::from_utf8_lossy(&body).to_string() })
}

async fn answer<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, handler: Handler, calls: Calls) {
    let Some(mut request) = read_request(&mut stream).await else { return };
    // Le client Docker préfixe ses chemins de la version de l'API.
    if let Some(rest) = request.path.strip_prefix("/v1.")
        && let Some(slash) = rest.find('/')
    {
        request.path = rest[slash..].to_string();
    }
    let (status, body) = handler(&request);
    calls.0.lock().unwrap().push(request);
    let reply = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Faux démon Docker. Le répertoire doit vivre aussi longtemps que le test.
#[cfg(unix)]
pub fn fake_docker(
    dir: &tempfile::TempDir,
    handler: impl Fn(&Req) -> Resp + Send + Sync + 'static,
) -> (PathBuf, Calls) {
    let socket = dir.path().join("docker.sock");
    let listener = tokio::net::UnixListener::bind(&socket).expect("socket");
    let calls = Calls::default();
    let handler: Handler = Arc::new(handler);
    let seen = calls.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { return };
            tokio::spawn(answer(stream, handler.clone(), seen.clone()));
        }
    });
    (socket, calls)
}

/// Faux serveur HTTP ; rend son URL de base.
pub async fn fake_http(handler: impl Fn(&Req) -> Resp + Send + Sync + 'static) -> (String, Calls) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let calls = Calls::default();
    let handler: Handler = Arc::new(handler);
    let seen = calls.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { return };
            tokio::spawn(answer(stream, handler.clone(), seen.clone()));
        }
    });
    (url, calls)
}
