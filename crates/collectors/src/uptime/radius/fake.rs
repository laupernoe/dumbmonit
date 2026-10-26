//! Serveur RADIUS factice pour les tests : une socket UDP sur la boucle locale
//! qui lit les `Access-Request` avec le vrai décodage de la sonde et répond
//! selon le scénario demandé.

use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;

use super::packet::{
    ACCESS_ACCEPT, ACCESS_CHALLENGE, ACCESS_REJECT, ParsedRequest, build_response, parse_request,
};

/// Ce que le serveur factice fait d'une requête.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    /// Accepte le bon compte, rejette les autres.
    Normal,
    /// Répond `Access-Challenge` à tout le monde.
    Challenge,
    /// Ne répond jamais.
    Silent,
    /// Envoie d'abord une réponse portant un autre identifiant, puis la vraie.
    WrongIdFirst,
    /// Répond par des octets qui ne sont pas du RADIUS.
    Garbage,
}

pub struct FakeServer {
    pub address: String,
    last: Arc<Mutex<Option<ParsedRequest>>>,
}

impl FakeServer {
    pub async fn spawn(secret: &str, username: &str, password: &str, behaviour: Behaviour) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap().to_string();
        let last = Arc::new(Mutex::new(None));
        let shared = last.clone();
        let (secret, username, password) =
            (secret.to_string(), username.to_string(), password.to_string());

        tokio::spawn(async move {
            let mut buffer = vec![0u8; 4096];
            loop {
                let Ok((read, peer)) = socket.recv_from(&mut buffer).await else { return };
                let Ok(request) = parse_request(&buffer[..read], secret.as_bytes()) else {
                    continue;
                };
                let accepted = request.signature_valid
                    && request.username == username
                    && request.password == password;
                let (id, authenticator) = (request.id, request.authenticator);
                *shared.lock().unwrap() = Some(request);

                let sign = |code: u8, id: u8| {
                    build_response(code, id, &authenticator, secret.as_bytes(), &[])
                };
                let verdict = if accepted { ACCESS_ACCEPT } else { ACCESS_REJECT };
                match behaviour {
                    Behaviour::Silent => {}
                    Behaviour::Normal => {
                        let _ = socket.send_to(&sign(verdict, id), peer).await;
                    }
                    Behaviour::Challenge => {
                        let _ = socket.send_to(&sign(ACCESS_CHALLENGE, id), peer).await;
                    }
                    Behaviour::WrongIdFirst => {
                        let _ =
                            socket.send_to(&sign(ACCESS_REJECT, id.wrapping_add(1)), peer).await;
                        let _ = socket.send_to(&sign(verdict, id), peer).await;
                    }
                    Behaviour::Garbage => {
                        let _ = socket.send_to(b"HTTP/1.1 200 OK\r\n\r\nnot radius", peer).await;
                    }
                }
            }
        });
        Self { address, last }
    }

    /// La dernière requête lue, pour vérifier ce que la sonde a réellement envoyé.
    pub fn last_request(&self) -> Option<ParsedRequest> {
        self.last.lock().unwrap().take()
    }
}
