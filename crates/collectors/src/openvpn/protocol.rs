//! L'interface de gestion d'OpenVPN (le réglage `management`), réduite à la
//! lecture.
//!
//! Protocole texte, apparenté à telnet : une invite d'accueil, puis des
//! commandes d'une ligne dont la réponse est soit un bloc terminé par `END`,
//! soit une ligne `ERROR: …`. Particularité qui complique la lecture : quand
//! un mot de passe est exigé, l'invite `ENTER PASSWORD:` n'est **pas** suivie
//! d'un retour à la ligne — elle est faite pour un humain qui tape sa réponse
//! juste derrière. La bannière d'accueil (`>INFO:…`), elle, en a un. Une simple
//! lecture ligne à ligne bloquerait donc indéfiniment face à une interface
//! protégée par mot de passe ; [`Session::greet`] cherche les deux motifs à la
//! fois dans un tampon qu'elle fait grossir au besoin.
//!
//! Seules `version`, `state` et `status 3` sont envoyées, avec le mot de passe
//! si l'interface en demande un. Jamais `signal`, `kill`, `client-kill` ni rien
//! qui changerait l'état du serveur ou d'un client.

use std::time::Duration;

use dumbmonit_proto::ProbeError;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Tampon maximal avant qu'une bannière ou une ligne soit jugée anormale.
const MAX_LINE: usize = 4096;

/// Lignes maximales d'un bloc (`status 3` sur un serveur très chargé reste à
/// quelques centaines de lignes).
const MAX_BLOCK_LINES: usize = 4096;

/// Ce que l'accueil annonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greeting {
    /// `ENTER PASSWORD:` : l'interface exige un mot de passe avant toute commande.
    PasswordRequired,
    /// `>INFO:` : prête, sans mot de passe.
    Ready,
}

/// Une connexion à l'interface de gestion, sur n'importe quel flux (TCP en
/// production, un tampon en mémoire dans les tests).
pub struct Session<S> {
    stream: S,
    /// Octets déjà lus sur le réseau mais pas encore consommés par l'appelant.
    pending: Vec<u8>,
    timeout: Duration,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    pub fn new(stream: S, timeout: Duration) -> Self {
        Self { stream, pending: Vec::new(), timeout }
    }

    async fn send(&mut self, command: &str) -> Result<(), ProbeError> {
        let line = format!("{command}\n");
        let write = self.stream.write_all(line.as_bytes());
        match tokio::time::timeout(self.timeout, write).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(ProbeError::Unreachable(format!("OpenVPN: {error}"))),
            Err(_) => Err(ProbeError::Timeout(self.timeout)),
        }
    }

    /// Lit jusqu'à ce qu'un des motifs apparaisse dans ce qui a été reçu, et
    /// rend ce qui le précède ; le motif lui-même et tout ce qui suit sont
    /// consommés pour le premier, gardés en réserve pour la suite.
    async fn read_until_any(&mut self, needles: &[&[u8]]) -> Result<(Vec<u8>, usize), ProbeError> {
        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            for (index, needle) in needles.iter().enumerate() {
                if let Some(pos) = find(&self.pending, needle) {
                    let before = self.pending[..pos].to_vec();
                    self.pending.drain(..pos + needle.len());
                    return Ok((before, index));
                }
            }
            if self.pending.len() > MAX_LINE {
                return Err(ProbeError::Protocol("OpenVPN sent an overlong line".to_string()));
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(ProbeError::Timeout(self.timeout));
            }
            let mut chunk = [0u8; 512];
            match tokio::time::timeout(remaining, self.stream.read(&mut chunk)).await {
                Ok(Ok(0)) => {
                    return Err(ProbeError::Unreachable(
                        "OpenVPN closed the management connection".to_string(),
                    ));
                }
                Ok(Ok(count)) => self.pending.extend_from_slice(&chunk[..count]),
                Ok(Err(error)) => return Err(ProbeError::Unreachable(format!("OpenVPN: {error}"))),
                Err(_) => return Err(ProbeError::Timeout(self.timeout)),
            }
        }
    }

    async fn read_line(&mut self) -> Result<String, ProbeError> {
        let (line, _) = self.read_until_any(&[b"\n"]).await?;
        Ok(String::from_utf8_lossy(&line).trim_end_matches('\r').to_string())
    }

    /// Lit l'accueil : jusqu'à dix lignes de notifications ignorées (`>…`
    /// autres que l'invite ou le message prêt), avant l'un des deux motifs
    /// attendus.
    pub async fn greet(&mut self) -> Result<Greeting, ProbeError> {
        for _ in 0..10 {
            let (before, which) = self.read_until_any(&[b"ENTER PASSWORD:", b"\n"]).await?;
            if which == 0 {
                return Ok(Greeting::PasswordRequired);
            }
            let line = String::from_utf8_lossy(&before).trim_end_matches('\r').to_string();
            if line.starts_with(">INFO:") {
                return Ok(Greeting::Ready);
            }
            // Une autre notification temps réel (`>HOLD:`, `>CLIENT:`, …) :
            // ignorée, l'accueil continue.
        }
        Err(ProbeError::Protocol("OpenVPN: no usable greeting within ten lines".to_string()))
    }

    /// Envoie le mot de passe tel quel : ce n'est pas une commande, l'invite
    /// ne s'attend à rien d'autre sur cette ligne.
    pub async fn authenticate(&mut self, password: &str) -> Result<(), ProbeError> {
        self.send(password).await?;
        let line = self.read_line().await?;
        if line.starts_with("SUCCESS") {
            return Ok(());
        }
        Err(ProbeError::Auth(format!(
            "the management interface refused the password: {}",
            excerpt(&line)
        )))
    }

    /// Une commande dont la réponse est un bloc de lignes terminé par `END`.
    async fn block(&mut self, command: &str) -> Result<Vec<String>, ProbeError> {
        self.send(command).await?;
        let mut lines = Vec::new();
        loop {
            let line = self.read_line().await?;
            if line == "END" {
                return Ok(lines);
            }
            if let Some(reason) = line.strip_prefix("ERROR: ") {
                return Err(ProbeError::Protocol(format!("{command}: {reason}")));
            }
            if lines.len() >= MAX_BLOCK_LINES {
                return Err(ProbeError::Protocol(format!("{command}: too many lines")));
            }
            lines.push(line);
        }
    }

    /// `version` : une ligne `OpenVPN Version: OpenVPN 2.6.9 […]`.
    pub async fn version(&mut self) -> Result<Option<String>, ProbeError> {
        let lines = self.block("version").await?;
        Ok(lines.iter().find_map(|line| parse_version(line)))
    }

    /// `state` : une ligne `<epoch>,<ÉTAT>,…`.
    pub async fn state(&mut self) -> Result<Option<StateInfo>, ProbeError> {
        let lines = self.block("state").await?;
        Ok(lines.first().and_then(|line| parse_state(line)))
    }

    /// `status 3` : les lignes `CLIENT_LIST,…`.
    pub async fn status3(&mut self) -> Result<Vec<ClientRow>, ProbeError> {
        let lines = self.block("status 3").await?;
        Ok(lines.iter().filter_map(|line| parse_client_list(line)).collect())
    }

    /// Termine poliment : une déconnexion brutale finit dans le journal du
    /// serveur, pour rien.
    pub async fn quit(&mut self) {
        let _ = self.send("quit").await;
    }
}

/// Position de la première occurrence de `needle` dans `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn excerpt(line: &str) -> String {
    line.chars().take(120).collect()
}

/// `OpenVPN Version: OpenVPN 2.6.9 [git:…] …` → `2.6.9`.
pub fn parse_version(line: &str) -> Option<String> {
    let after = line.strip_prefix("OpenVPN Version: OpenVPN ")?;
    after.split_whitespace().next().map(str::to_string)
}

/// L'état du serveur, tel que rend `state` sans argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInfo {
    pub since: i64,
    /// `CONNECTED`, `RECONNECTING`, … tel qu'OpenVPN l'écrit.
    pub state: String,
}

/// `1700000000,CONNECTED,SUCCESS,,,,,,` → l'horodatage et l'état.
pub fn parse_state(line: &str) -> Option<StateInfo> {
    let mut fields = line.split(',');
    let since = fields.next()?.trim().parse().ok()?;
    let state = fields.next()?.trim().to_string();
    if state.is_empty() {
        return None;
    }
    Some(StateInfo { since, state })
}

/// Un client, tel qu'une ligne `CLIENT_LIST` de `status 3` le décrit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRow {
    pub common_name: String,
    pub real_address: String,
    pub virtual_address: String,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub connected_since: i64,
}

/// `CLIENT_LIST,common_name,real_address,virtual_address,virtual_ipv6_address,
/// bytes_received,bytes_sent,connected_since,connected_since (time_t),
/// username,client_id,peer_id,data_channel_cipher`.
pub fn parse_client_list(line: &str) -> Option<ClientRow> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.first() != Some(&"CLIENT_LIST") || fields.len() < 9 {
        return None;
    }
    Some(ClientRow {
        common_name: fields[1].to_string(),
        real_address: fields[2].to_string(),
        virtual_address: fields[3].to_string(),
        bytes_received: fields[5].trim().parse().unwrap_or(0),
        bytes_sent: fields[6].trim().parse().unwrap_or(0),
        connected_since: fields[8].trim().parse().unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Transcription réelle (identifiants remplacés) : accueil sans mot de
    /// passe, puis `version`, `state` et `status 3`.
    const SCRIPT: &str = include_str!("testdata/management_session.txt");

    #[tokio::test]
    async fn an_open_management_interface_needs_no_password() {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        server.write_all(SCRIPT.as_bytes()).await.unwrap();
        let mut session = Session::new(client, Duration::from_secs(1));

        assert_eq!(session.greet().await.unwrap(), Greeting::Ready);
        assert_eq!(session.version().await.unwrap(), Some("2.6.9".to_string()));

        let state = session.state().await.unwrap().expect("state");
        assert_eq!(state.state, "CONNECTED");
        assert_eq!(state.since, 1_700_000_000);

        let clients = session.status3().await.unwrap();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].common_name, "laptop-noe");
        assert_eq!(clients[0].real_address, "203.0.113.5:54321");
        assert_eq!(clients[0].virtual_address, "10.8.0.2");
        assert_eq!(clients[0].bytes_received, 98_765);
        assert_eq!(clients[0].bytes_sent, 147_852);
        assert_eq!(clients[0].connected_since, 1_699_996_800);
    }

    #[tokio::test]
    async fn a_password_prompt_has_no_trailing_newline() {
        // L'invite est envoyée seule, sans retour à la ligne : c'est le cas qui
        // ferait patienter indéfiniment une lecture ligne à ligne ordinaire.
        let (client, mut server) = tokio::io::duplex(1024);
        server.write_all(b"ENTER PASSWORD:").await.unwrap();
        let mut session = Session::new(client, Duration::from_secs(1));
        assert_eq!(session.greet().await.unwrap(), Greeting::PasswordRequired);

        server.write_all(b"SUCCESS: password is correct\n").await.unwrap();
        session.authenticate("secret").await.unwrap();
    }

    #[tokio::test]
    async fn a_wrong_password_is_an_auth_error() {
        let (client, mut server) = tokio::io::duplex(1024);
        server.write_all(b"ENTER PASSWORD:").await.unwrap();
        let mut session = Session::new(client, Duration::from_secs(1));
        session.greet().await.unwrap();

        server.write_all(b"ERROR: bad password\n").await.unwrap();
        let error = session.authenticate("wrong").await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn realtime_notifications_before_the_banner_are_skipped() {
        let (client, mut server) = tokio::io::duplex(1024);
        server
            .write_all(b">CLIENT:CONNECT,1,1\n>INFO:OpenVPN Management Interface Version 5\n")
            .await
            .unwrap();
        let mut session = Session::new(client, Duration::from_secs(1));
        assert_eq!(session.greet().await.unwrap(), Greeting::Ready);
    }

    #[tokio::test]
    async fn a_silent_peer_times_out() {
        let (client, _server) = tokio::io::duplex(1024);
        let mut session = Session::new(client, Duration::from_millis(50));
        let error = session.greet().await.unwrap_err();
        assert!(matches!(error, ProbeError::Timeout(_)));
        assert!(error.means_down());
    }

    #[test]
    fn version_lines_are_parsed() {
        assert_eq!(
            parse_version(
                "OpenVPN Version: OpenVPN 2.6.9 [git:v2.6.9/abc] x86_64-pc-linux-gnu [SSL]"
            ),
            Some("2.6.9".to_string())
        );
        assert_eq!(parse_version("Management Version: 5"), None);
    }

    #[test]
    fn a_client_list_line_without_enough_fields_is_skipped() {
        assert_eq!(parse_client_list("CLIENT_LIST,a,b"), None);
        assert_eq!(parse_client_list("HEADER,CLIENT_LIST,Common Name"), None);
    }
}
