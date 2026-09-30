//! Le protocole réseau de NUT (`upsd`, TCP 3493), réduit à la lecture.
//!
//! Un échange est une ligne de commande terminée par `\n`, et une réponse qui
//! est soit une ligne (`OK`, `ERR <CODE>`), soit un bloc
//! `BEGIN LIST …` / `END LIST …`. Les valeurs sont entre guillemets doubles,
//! avec `\"` et `\\` comme seuls échappements (« Network protocol
//! information », documentation développeur de NUT).
//!
//! Seules `LIST UPS`, `LIST VAR`, `VER` et, si un identifiant est configuré,
//! `USERNAME` / `PASSWORD` sont envoyées. `LOGIN` — qui compte le client comme
//! un secondaire et le fait attendre à l'arrêt de l'onduleur — ne l'est jamais,
//! pas plus que `SET`, `INSTCMD` ou `FSD`.

use std::time::Duration;

use dumbmonit_proto::ProbeError;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// Longueur maximale d'une ligne de réponse. Une variable NUT tient en
/// quelques dizaines d'octets ; la limite ne sert qu'à borner un pair bavard.
const MAX_LINE: usize = 4096;

/// Nombre maximal de lignes d'une liste. Un onduleur réel publie 40 à 150
/// variables ; un serveur en publie rarement plus d'une dizaine.
const MAX_LIST_LINES: usize = 2000;

/// Une erreur que `upsd` a répondue (`ERR <CODE> [détail]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NutError {
    pub code: String,
}

impl NutError {
    /// Traduit un refus en erreur de sonde. `DATA-STALE` et
    /// `DRIVER-NOT-CONNECTED` ne passent pas par ici : ce sont des mesures.
    pub fn into_probe_error(self, what: &str) -> ProbeError {
        match self.code.as_str() {
            "ACCESS-DENIED" => ProbeError::Auth(format!(
                "upsd refused {what} (ACCESS-DENIED): check that it lets this host read, and the \
                 user name and password if one is set."
            )),
            "USERNAME-REQUIRED" | "PASSWORD-REQUIRED" | "INVALID-USERNAME" | "INVALID-PASSWORD" => {
                ProbeError::Auth(format!("upsd refused {what} ({}).", self.code))
            }
            "UNKNOWN-UPS" => ProbeError::Config(format!(
                "upsd does not know the UPS asked for in {what} (UNKNOWN-UPS): check the UPS \
                 names in the device options."
            )),
            code => ProbeError::Protocol(format!("upsd answered ERR {code} to {what}")),
        }
    }
}

/// Réponse à une commande : la liste attendue, ou un refus de `upsd`.
pub type Answer<T> = Result<T, NutError>;

/// Découpe une ligne en mots, en respectant les guillemets et les échappements.
///
/// `VAR ups1 device.model "Back-UPS \"ES\" 700"` donne
/// `["VAR", "ups1", "device.model", "Back-UPS \"ES\" 700"]`.
pub fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut chars = line.trim_end_matches(['\r', '\n']).chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| *c == ' ' || *c == '\t') {
            chars.next();
        }
        let Some(&first) = chars.peek() else { break };
        let mut word = String::new();
        if first == '"' {
            chars.next();
            let mut closed = false;
            while let Some(c) = chars.next() {
                match c {
                    '\\' => match chars.next() {
                        Some(escaped) => word.push(escaped),
                        None => return Err("escape at the end of the line".to_string()),
                    },
                    '"' => {
                        closed = true;
                        break;
                    }
                    other => word.push(other),
                }
            }
            if !closed {
                return Err("unterminated quoted string".to_string());
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c == ' ' || c == '\t' {
                    break;
                }
                word.push(c);
                chars.next();
            }
        }
        words.push(word);
    }
    Ok(words)
}

/// Met une valeur entre guillemets pour l'envoyer (`USERNAME`, `PASSWORD`,
/// nom d'onduleur).
pub fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Un onduleur annoncé par `LIST UPS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsEntry {
    pub name: String,
    pub description: String,
}

/// Une connexion à `upsd`, sur n'importe quel flux (TCP en production, un
/// tampon en mémoire dans les tests).
pub struct Session<S> {
    stream: BufReader<S>,
    timeout: Duration,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    pub fn new(stream: S, timeout: Duration) -> Self {
        Self { stream: BufReader::new(stream), timeout }
    }

    async fn send(&mut self, command: &str) -> Result<(), ProbeError> {
        let line = format!("{command}\n");
        let write = self.stream.get_mut().write_all(line.as_bytes());
        match tokio::time::timeout(self.timeout, write).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(ProbeError::Unreachable(format!("upsd: {error}"))),
            Err(_) => Err(ProbeError::Timeout(self.timeout)),
        }
    }

    async fn read_line(&mut self) -> Result<String, ProbeError> {
        let mut buffer = Vec::new();
        let read = async {
            // `take` borne la ligne : un pair qui n'envoie jamais de `\n` ne fait
            // pas grossir le tampon indéfiniment.
            let mut limited = (&mut self.stream).take(MAX_LINE as u64 + 1);
            limited.read_until(b'\n', &mut buffer).await
        };
        let count = match tokio::time::timeout(self.timeout, read).await {
            Ok(Ok(count)) => count,
            Ok(Err(error)) => return Err(ProbeError::Unreachable(format!("upsd: {error}"))),
            Err(_) => return Err(ProbeError::Timeout(self.timeout)),
        };
        if count == 0 {
            return Err(ProbeError::Unreachable("upsd closed the connection".to_string()));
        }
        if buffer.len() > MAX_LINE {
            return Err(ProbeError::Protocol("upsd sent an overlong line".to_string()));
        }
        Ok(String::from_utf8_lossy(&buffer).trim_end_matches(['\r', '\n']).to_string())
    }

    /// Une commande à réponse d'une ligne : `OK …` ou `ERR …`.
    async fn simple(&mut self, command: &str, what: &str) -> Result<Answer<String>, ProbeError> {
        self.send(command).await?;
        let line = self.read_line().await?;
        if let Some(error) = parse_error(&line) {
            return Ok(Err(error));
        }
        if line.starts_with("OK") {
            return Ok(Ok(line));
        }
        Err(ProbeError::Protocol(format!("unexpected answer to {what}: {}", excerpt(&line))))
    }

    /// `USERNAME` puis `PASSWORD`. `upsd` ne vérifie le couple qu'au moment
    /// d'une commande qui l'exige ; il répond `OK` à la lecture seule.
    pub async fn authenticate(&mut self, username: &str, password: &str) -> Result<(), ProbeError> {
        self.simple(&format!("USERNAME {}", quote(username)), "USERNAME")
            .await?
            .map_err(|e| e.into_probe_error("USERNAME"))?;
        self.simple(&format!("PASSWORD {}", quote(password)), "PASSWORD")
            .await?
            .map_err(|e| e.into_probe_error("PASSWORD"))?;
        Ok(())
    }

    /// `VER` : une phrase libre, `Network UPS Tools upsd 2.8.2 - https://…`.
    pub async fn version(&mut self) -> Result<Option<String>, ProbeError> {
        self.send("VER").await?;
        let line = self.read_line().await?;
        if parse_error(&line).is_some() {
            return Ok(None);
        }
        Ok(parse_version(&line))
    }

    /// Lit un bloc `BEGIN LIST <quoi>` … `END LIST <quoi>` et en rend les
    /// lignes intermédiaires, découpées en mots.
    async fn list(&mut self, what: &str) -> Result<Answer<Vec<Vec<String>>>, ProbeError> {
        self.send(&format!("LIST {what}")).await?;
        let first = self.read_line().await?;
        if let Some(error) = parse_error(&first) {
            return Ok(Err(error));
        }
        // Le nom de l'onduleur revient tel que `upsd` l'écrit, pas forcément
        // tel qu'il a été envoyé : seul le début de la ligne est vérifié.
        if !first.starts_with("BEGIN LIST ") {
            return Err(ProbeError::Protocol(format!(
                "expected \"BEGIN LIST {what}\", upsd answered: {}",
                excerpt(&first)
            )));
        }
        let mut rows = Vec::new();
        loop {
            let line = self.read_line().await?;
            if line.starts_with("END LIST ") {
                return Ok(Ok(rows));
            }
            if rows.len() >= MAX_LIST_LINES {
                return Err(ProbeError::Protocol(format!("LIST {what}: too many lines")));
            }
            let words = tokenize(&line).map_err(|error| {
                ProbeError::Protocol(format!("LIST {what}: {error} in {}", excerpt(&line)))
            })?;
            rows.push(words);
        }
    }

    pub async fn list_ups(&mut self) -> Result<Answer<Vec<UpsEntry>>, ProbeError> {
        Ok(self.list("UPS").await?.map(|rows| {
            rows.into_iter()
                .filter(|words| words.len() >= 2 && words[0] == "UPS")
                .map(|mut words| UpsEntry {
                    description: words.get(2).cloned().unwrap_or_default(),
                    name: std::mem::take(&mut words[1]),
                })
                .collect()
        }))
    }

    /// `LIST VAR <ups>`, en couples (nom, valeur) dans l'ordre reçu.
    pub async fn list_vars(
        &mut self,
        ups: &str,
    ) -> Result<Answer<Vec<(String, String)>>, ProbeError> {
        Ok(self.list(&format!("VAR {}", ups_token(ups))).await?.map(|rows| {
            rows.into_iter()
                .filter(|words| words.len() >= 4 && words[0] == "VAR")
                .map(|mut words| (std::mem::take(&mut words[2]), std::mem::take(&mut words[3])))
                .collect()
        }))
    }

    /// Termine poliment : `upsd` journalise sinon une déconnexion brutale.
    pub async fn logout(&mut self) {
        if self.send("LOGOUT").await.is_ok() {
            let _ = self.read_line().await;
        }
    }
}

/// Un nom d'onduleur tel qu'il s'écrit dans une commande : nu s'il ne contient
/// que des caractères sûrs (le cas de tous les noms de `ups.conf`), sinon entre
/// guillemets. Le `BEGIN LIST VAR` renvoyé par `upsd` reprend le nom nu.
fn ups_token(name: &str) -> String {
    if !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '@'))
    {
        name.to_string()
    } else {
        quote(name)
    }
}

/// `ERR <CODE> [détail]`, ou rien.
pub fn parse_error(line: &str) -> Option<NutError> {
    let rest = line.strip_prefix("ERR ")?;
    let code = rest.split_whitespace().next().unwrap_or("UNKNOWN").to_string();
    Some(NutError { code })
}

/// Extrait `2.8.2` de `Network UPS Tools upsd 2.8.2 - https://…`.
pub fn parse_version(line: &str) -> Option<String> {
    let after = line.split("upsd").nth(1)?;
    let word = after.split_whitespace().next()?;
    let version: String = word
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
        .collect();
    (!version.is_empty() && version.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .then_some(version)
}

fn excerpt(line: &str) -> String {
    line.chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_guillemets_et_les_echappements_sont_respectes() {
        let words = tokenize(r#"VAR ups1 device.model "Back-UPS \"ES\" 700 \\ G2""#).unwrap();
        assert_eq!(words, ["VAR", "ups1", "device.model", r#"Back-UPS "ES" 700 \ G2"#]);
        assert_eq!(tokenize(r#"UPS a """#).unwrap(), ["UPS", "a", ""]);
        assert!(tokenize(r#"VAR a b "ouvert"#).is_err());
        assert_eq!(tokenize("  END   LIST UPS\r\n").unwrap(), ["END", "LIST", "UPS"]);
    }

    #[test]
    fn une_valeur_envoyee_est_echappee() {
        assert_eq!(quote(r#"p"a\ss"#), r#""p\"a\\ss""#);
        assert_eq!(ups_token("rack-1"), "rack-1");
        assert_eq!(ups_token("salle serveur"), "\"salle serveur\"");
    }

    #[test]
    fn les_erreurs_sont_classees() {
        let err = |line: &str| parse_error(line).unwrap().into_probe_error("LIST VAR x");
        assert!(matches!(err("ERR ACCESS-DENIED"), ProbeError::Auth(_)));
        assert!(matches!(err("ERR UNKNOWN-UPS"), ProbeError::Config(_)));
        assert!(matches!(err("ERR FEATURE-NOT-SUPPORTED extra"), ProbeError::Protocol(_)));
        assert!(!err("ERR ACCESS-DENIED").means_down());
        assert!(parse_error("OK").is_none());
        assert_eq!(parse_error("ERR DATA-STALE").unwrap().code, "DATA-STALE");
    }

    #[test]
    fn la_version_est_lue_dans_la_phrase_de_ver() {
        assert_eq!(
            parse_version("Network UPS Tools upsd 2.8.2 - https://www.networkupstools.org/"),
            Some("2.8.2".to_string())
        );
        assert_eq!(parse_version("Network UPS Tools upsd 2.7.4 - http://x/"), Some("2.7.4".into()));
        assert_eq!(parse_version("something else"), None);
    }

    /// Transcriptions réelles de `upsd` 2.8.2, rejouées sur un flux en mémoire.
    #[tokio::test]
    async fn les_listes_reelles_se_lisent() {
        let script = format!(
            "{}{}",
            include_str!("testdata/list_ups.txt"),
            include_str!("testdata/var_rack_on_battery.txt")
        );
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        server.write_all(script.as_bytes()).await.unwrap();
        let mut session = Session::new(client, Duration::from_secs(1));
        let ups = session.list_ups().await.unwrap().unwrap();
        assert_eq!(
            ups,
            [
                UpsEntry { name: "rack".into(), description: "Rack UPS".into() },
                UpsEntry { name: "office".into(), description: "Office UPS".into() },
            ]
        );
        let vars = session.list_vars("rack").await.unwrap().unwrap();
        assert!(vars.contains(&("ups.status".into(), "OB DISCHRG".into())));
        assert!(vars.contains(&("device.model".into(), "5P 1550".into())));
    }

    #[tokio::test]
    async fn un_refus_de_upsd_est_une_reponse_et_non_une_panne() {
        let (client, mut server) = tokio::io::duplex(1024);
        server.write_all(include_str!("testdata/err_data_stale.txt").as_bytes()).await.unwrap();
        let mut session = Session::new(client, Duration::from_secs(1));
        let answer = session.list_vars("office").await.unwrap();
        assert_eq!(answer.unwrap_err().code, "DATA-STALE");
    }

    #[tokio::test]
    async fn un_pair_muet_donne_un_delai_depasse() {
        let (client, _server) = tokio::io::duplex(1024);
        let mut session = Session::new(client, Duration::from_millis(50));
        let error = session.list_ups().await.unwrap_err();
        assert!(matches!(error, ProbeError::Timeout(_)));
        assert!(error.means_down());
    }

    #[tokio::test]
    async fn une_ligne_sans_fin_est_bornee() {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        server.write_all(&vec![b'A'; MAX_LINE + 10]).await.unwrap();
        let mut session = Session::new(client, Duration::from_secs(1));
        assert!(matches!(session.list_ups().await.unwrap_err(), ProbeError::Protocol(_)));
    }
}
