//! Le strict nécessaire de RESP (le protocole de Redis et Valkey) pour
//! s'authentifier et lire `INFO`.
//!
//! Une commande part en tableau de chaînes « bulk » ; une réponse est une
//! chaîne simple (`+OK`), une erreur (`-WRONGPASS …`), un entier (`:1`) ou une
//! chaîne bulk (`$5\r\nhello`). Les tableaux ne servent à aucune des deux
//! commandes employées et sont refusés comme réponse inattendue.

use dumbmonit_proto::ProbeError;

use crate::socket::Connection;

/// Taille maximale d'une réponse acceptée. `INFO` pèse quelques kilo-octets ;
/// la borne protège d'un service qui ne serait pas celui qu'on croit.
const MAX_REPLY: usize = 4 * 1024 * 1024;

#[derive(Debug, PartialEq)]
pub enum Reply {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
}

/// Encode une commande en tableau de chaînes bulk.
pub fn encode(args: &[&str]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
        out.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        out.extend_from_slice(arg.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// Lecteur tamponné au-dessus de la connexion.
pub struct Client {
    conn: Connection,
    buffer: Vec<u8>,
}

impl Client {
    pub fn new(conn: Connection) -> Self {
        Self { conn, buffer: Vec::with_capacity(8192) }
    }

    pub async fn command(&mut self, args: &[&str]) -> Result<Reply, ProbeError> {
        self.conn.write_all(&encode(args)).await?;
        self.read_reply().await
    }

    pub async fn close(self) {
        self.conn.close().await;
    }

    async fn fill(&mut self) -> Result<(), ProbeError> {
        let mut chunk = [0u8; 8192];
        let read = self.conn.read_some(&mut chunk).await?;
        if read == 0 {
            return Err(ProbeError::Protocol(format!(
                "{} closed the connection. Check the port, and whether the server expects TLS.",
                self.conn.product()
            )));
        }
        self.buffer.extend_from_slice(&chunk[..read]);
        if self.buffer.len() > MAX_REPLY {
            return Err(ProbeError::Protocol("reply larger than 4 MiB".to_string()));
        }
        Ok(())
    }

    async fn line(&mut self) -> Result<String, ProbeError> {
        loop {
            if let Some(end) = self.buffer.windows(2).position(|w| w == b"\r\n") {
                let line: Vec<u8> = self.buffer.drain(..end + 2).take(end).collect();
                return Ok(String::from_utf8_lossy(&line).into_owned());
            }
            self.fill().await?;
        }
    }

    async fn read_reply(&mut self) -> Result<Reply, ProbeError> {
        let line = self.line().await?;
        let (kind, rest) = line.split_at(line.len().min(1));
        match kind {
            "+" => Ok(Reply::Simple(rest.to_string())),
            "-" => Ok(Reply::Error(rest.to_string())),
            ":" => rest.parse().map(Reply::Integer).map_err(|_| unexpected(&line)),
            "$" => {
                let length: i64 = rest.parse().map_err(|_| unexpected(&line))?;
                if length < 0 {
                    return Ok(Reply::Bulk(None));
                }
                let length = usize::try_from(length).map_err(|_| unexpected(&line))?;
                if length > MAX_REPLY {
                    return Err(ProbeError::Protocol("reply larger than 4 MiB".to_string()));
                }
                while self.buffer.len() < length + 2 {
                    self.fill().await?;
                }
                let body: Vec<u8> = self.buffer.drain(..length + 2).take(length).collect();
                Ok(Reply::Bulk(Some(body)))
            }
            _ => Err(unexpected(&line)),
        }
    }
}

fn unexpected(line: &str) -> ProbeError {
    let excerpt: String = line.chars().take(80).collect();
    ProbeError::Protocol(format!(
        "The server does not speak the Redis protocol (it answered \"{excerpt}\"). Check the \
         device type and the port."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_commande_part_en_tableau_de_chaines() {
        assert_eq!(encode(&["AUTH", "u", "p w"]), b"*3\r\n$4\r\nAUTH\r\n$1\r\nu\r\n$3\r\np w\r\n");
    }
}
